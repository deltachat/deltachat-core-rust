//! # Functions to update timestamps.

use crate::chat::{Chat, ChatId};
use crate::contact::Contact;
use crate::context::Context;
use crate::param::{Param, Params};
use anyhow::Result;

impl Context {
    /// Updates a contact's timestamp, if reasonable.
    /// Returns true if the caller shall update the settings belonging to the scope.
    /// (if we have a ContactId type at some point, the function should go there)
    pub(crate) async fn update_contacts_timestamp(
        &self,
        contact_id: u32,
        scope: Param,
        new_timestamp: i64,
    ) -> Result<bool> {
        let mut contact = Contact::load_from_db(self, contact_id).await?;
        if contact.param.update_timestamp(scope, new_timestamp)? {
            contact.update_param(self).await?;
            return Ok(true);
        }
        Ok(false)
    }
}

impl ChatId {
    /// Updates a chat id's timestamp on disk, if reasonable.
    /// Returns true if the caller shall update the settings belonging to the scope.
    pub(crate) async fn update_timestamp(
        &self,
        context: &Context,
        scope: Param,
        new_timestamp: i64,
    ) -> Result<bool> {
        let mut chat = Chat::load_from_db(context, *self).await?;
        if chat.param.update_timestamp(scope, new_timestamp)? {
            chat.update_param(context).await?;
            return Ok(true);
        }
        Ok(false)
    }
}

impl Params {
    /// Updates a param's timestamp in memory, if reasonable.
    /// Returns true if the caller shall update the settings belonging to the scope.
    pub(crate) fn update_timestamp(&mut self, scope: Param, new_timestamp: i64) -> Result<bool> {
        let old_timestamp = self.get_i64(scope).unwrap_or_default();
        if new_timestamp >= old_timestamp {
            self.set_i64(scope, new_timestamp);
            return Ok(true);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dc_receive_imf::dc_receive_imf;
    use crate::dc_tools::time;
    use crate::test_utils::TestContext;

    #[async_std::test]
    async fn test_params_update_timestamp() -> Result<()> {
        let mut params = Params::new();
        let ts = time();

        assert!(params.update_timestamp(Param::LastSubject, ts)?);
        assert!(params.update_timestamp(Param::LastSubject, ts)?); // same timestamp -> update
        assert!(params.update_timestamp(Param::LastSubject, ts + 10)?);
        assert!(!params.update_timestamp(Param::LastSubject, ts)?); // `ts` is now too old
        assert!(!params.update_timestamp(Param::LastSubject, 0)?);
        assert_eq!(params.get_i64(Param::LastSubject).unwrap(), ts + 10);

        assert!(params.update_timestamp(Param::GroupNameTimestamp, 0)?); // stay unset -> update ...
        assert!(params.update_timestamp(Param::GroupNameTimestamp, 0)?); // ... also on multiple calls
        assert_eq!(params.get_i64(Param::GroupNameTimestamp).unwrap(), 0);

        assert!(!params.update_timestamp(Param::AvatarTimestamp, -1)?);
        assert_eq!(params.get_i64(Param::AvatarTimestamp), None);

        Ok(())
    }

    #[async_std::test]
    async fn test_out_of_order_subject() -> Result<()> {
        let t = TestContext::new_alice().await;

        dc_receive_imf(
            &t,
            b"From: Bob Authname <bob@example.org>\n\
                 To: alice@example.org\n\
                 Subject: updated subject\n\
                 Message-ID: <msg2@example.org>\n\
                 Chat-Version: 1.0\n\
                 Date: Sun, 22 Mar 2021 23:37:57 +0000\n\
                 \n\
                 second message\n",
            "INBOX",
            1,
            false,
        )
        .await?;
        dc_receive_imf(
            &t,
            b"From: Bob Authname <bob@example.org>\n\
                 To: alice@example.org\n\
                 Subject: original subject\n\
                 Message-ID: <msg1@example.org>\n\
                 Chat-Version: 1.0\n\
                 Date: Sun, 22 Mar 2021 22:37:57 +0000\n\
                 \n\
                 first message\n",
            "INBOX",
            2,
            false,
        )
        .await?;

        let msg = t.get_last_msg().await;
        let chat = Chat::load_from_db(&t, msg.chat_id).await?;
        assert_eq!(
            chat.param.get(Param::LastSubject).unwrap(),
            "updated subject"
        );

        Ok(())
    }

    #[async_std::test]
    async fn test_out_of_order_group_name() -> Result<()> {
        let t = TestContext::new_alice().await;

        dc_receive_imf(
            &t,
            b"From: Bob Authname <bob@example.org>\n\
                 To: alice@example.org\n\
                 Message-ID: <msg1@example.org>\n\
                 Chat-Version: 1.0\n\
                 Chat-Group-ID: abcde\n\
                 Chat-Group-Name: initial name\n\
                 Date: Sun, 22 Mar 2021 01:00:00 +0000\n\
                 \n\
                 first message\n",
            "INBOX",
            1,
            false,
        )
        .await?;
        let msg = t.get_last_msg().await;
        let chat = Chat::load_from_db(&t, msg.chat_id).await?;
        assert_eq!(chat.name, "initial name");

        dc_receive_imf(
            &t,
            b"From: Bob Authname <bob@example.org>\n\
                 To: alice@example.org\n\
                 Message-ID: <msg3@example.org>\n\
                 Chat-Version: 1.0\n\
                 Chat-Group-ID: abcde\n\
                 Chat-Group-Name: another name update\n\
                 Chat-Group-Name-Changed: a name update\n\
                 Date: Sun, 22 Mar 2021 03:00:00 +0000\n\
                 \n\
                 third message\n",
            "INBOX",
            2,
            false,
        )
        .await?;
        dc_receive_imf(
            &t,
            b"From: Bob Authname <bob@example.org>\n\
                 To: alice@example.org\n\
                 Message-ID: <msg2@example.org>\n\
                 Chat-Version: 1.0\n\
                 Chat-Group-ID: abcde\n\
                 Chat-Group-Name: a name update\n\
                 Chat-Group-Name-Changed: initial name\n\
                 Date: Sun, 22 Mar 2021 02:00:00 +0000\n\
                 \n\
                 second message\n",
            "INBOX",
            3,
            false,
        )
        .await?;
        let msg = t.get_last_msg().await;
        let chat = Chat::load_from_db(&t, msg.chat_id).await?;
        assert_eq!(chat.name, "another name update");

        Ok(())
    }
}
