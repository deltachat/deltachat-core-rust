use crate::chat::ChatId;
use crate::config::Config;
use crate::context::Context;
use crate::message::{Message, MsgId, Viewtype};
use crate::param::Param;
use crate::webxdc::{maps_integration, StatusUpdateItem, StatusUpdateSerial};
use anyhow::Result;

impl Message {
    /// Mark Webxdc message shipped with the main app as a default integration.
    pub fn set_webxdc_integration(&mut self) {
        self.hidden = true;
        self.param.set_int(Param::WebxdcIntegration, 1);
    }
}

impl Context {
    /// Get Webxdc instance used for optional integrations.
    /// If there is no integration, the caller may decide to add a default one.
    pub async fn init_webxdc_integration(
        &self,
        integrate_for: Option<ChatId>,
    ) -> Result<Option<MsgId>> {
        let Some(instance_id) = self
            .get_config_parsed::<u32>(Config::WebxdcIntegration)
            .await?
        else {
            return Ok(None);
        };

        if let Some(mut instance) =
            Message::load_from_db_optional(self, MsgId::new(instance_id)).await?
        {
            if instance.viewtype == Viewtype::Webxdc && !instance.chat_id.is_trash() {
                let integrate_for = integrate_for.unwrap_or_default().to_u32() as i32;
                if instance.param.get_int(Param::WebxdcIntegrateFor) != Some(integrate_for) {
                    instance
                        .param
                        .set_int(Param::WebxdcIntegrateFor, integrate_for);
                    instance.update_param(self).await?;
                }
                return Ok(Some(instance.id));
            }
        }

        Ok(None)
    }

    // Check if a Webxdc shall be used as an integration and remember that.
    pub(crate) async fn update_webxdc_integration_database(&self, msg: &Message) -> Result<()> {
        if msg.viewtype == Viewtype::Webxdc && msg.param.get_int(Param::WebxdcIntegration).is_some()
        {
            self.set_config_internal(
                Config::WebxdcIntegration,
                Some(&msg.id.to_u32().to_string()),
            )
            .await?;
        }
        Ok(())
    }

    // Intercept sending updates from Webxdc to core.
    pub(crate) async fn intercept_send_webxdc_status_update(
        &self,
        instance: Message,
        status_update: StatusUpdateItem,
    ) -> Result<()> {
        let chat_id = self.integrate_for(&instance)?;
        maps_integration::intercept_send_update(self, chat_id, status_update).await
    }

    // Intercept Webxdc requesting updates from core.
    pub(crate) async fn intercept_get_webxdc_status_updates(
        &self,
        instance: Message,
        last_known_serial: StatusUpdateSerial,
    ) -> Result<String> {
        let chat_id = self.integrate_for(&instance)?;
        maps_integration::intercept_get_updates(self, chat_id, last_known_serial).await
    }

    // Get chat the Webxdc is integrated for.
    // This is the chat given to `init_webxdc_integration()`.
    fn integrate_for(&self, instance: &Message) -> Result<Option<ChatId>> {
        let raw_id = instance
            .param
            .get_int(Param::WebxdcIntegrateFor)
            .unwrap_or(0) as u32;
        let chat_id = if raw_id > 0 {
            Some(ChatId::new(raw_id))
        } else {
            None
        };
        Ok(chat_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::chat::send_msg;
    use crate::config::Config;
    use crate::message::{Message, Viewtype};
    use crate::test_utils::TestContext;
    use anyhow::Result;
    use std::time::Duration;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_default_integrations_are_single_device() -> Result<()> {
        let t = TestContext::new_alice().await;
        t.set_config_bool(Config::BccSelf, false).await?;
        let chat = t.get_self_chat().await;

        let bytes = include_bytes!("../../test-data/webxdc/minimal.xdc");
        let mut msg = Message::new(Viewtype::Webxdc);
        msg.set_file_from_bytes(&t, "my-maps.xdc", bytes, None)
            .await?;
        msg.set_webxdc_integration();
        send_msg(&t, chat.id, &mut msg).await?;

        // default integrations are shipped with the apps and should not be sent over the wire
        let sent = t.pop_sent_msg_opt(Duration::from_secs(1)).await;
        assert!(sent.is_none());

        Ok(())
    }
}
