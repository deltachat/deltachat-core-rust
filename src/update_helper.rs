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
        if let Ok(mut contact) = Contact::load_from_db(self, contact_id).await {
            if contact.param.set_timestamp(scope, new_timestamp)? {
                contact.update_param(self).await?;
                return Ok(true);
            }
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
        if let Ok(mut chat) = Chat::load_from_db(context, *self).await {
            if chat.param.set_timestamp(scope, new_timestamp)? {
                chat.update_param(context).await?;
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl Params {
    /// Updates a param's timestamp in memory, if reasonable.
    /// Returns true if the caller shall update the settings belonging to the scope.
    pub(crate) fn set_timestamp(&mut self, scope: Param, new_timestamp: i64) -> Result<bool> {
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
    use crate::dc_tools::time;

    #[async_std::test]
    async fn test_params_set_timestamp() -> Result<()> {
        let mut params = Params::new();
        let ts = time();

        assert!(params.set_timestamp(Param::LastSubject, ts)?);
        assert!(params.set_timestamp(Param::LastSubject, ts)?); // same timestamp -> update
        assert!(params.set_timestamp(Param::LastSubject, ts + 10)?);
        assert!(!params.set_timestamp(Param::LastSubject, ts)?); // `ts` is now too old
        assert!(!params.set_timestamp(Param::LastSubject, 0)?);
        assert_eq!(params.get_i64(Param::LastSubject).unwrap(), ts + 10);

        assert!(params.set_timestamp(Param::GroupNameTimestamp, 0)?); // stay unset -> update ...
        assert!(params.set_timestamp(Param::GroupNameTimestamp, 0)?); // ... also on multiple calls
        assert_eq!(params.get_i64(Param::GroupNameTimestamp).unwrap(), 0);

        assert!(!params.set_timestamp(Param::AvatarTimestamp, -1)?);
        assert_eq!(params.get_i64(Param::AvatarTimestamp), None);

        Ok(())
    }
}
