use anyhow::Result;
use deltachat::config::Config;
use deltachat::contact::{Contact, ContactId};
use serde::Serialize;
use typescript_type_def::TypeDef;

use super::color_int_to_hex_string;

#[derive(Serialize, TypeDef)]
#[serde(tag = "type")]
pub enum Account {
    #[serde(rename_all = "camelCase")]
    Configured {
        id: u32,
        display_name: Option<String>,
        addr: Option<String>,
        // size: u32,
        profile_image: Option<String>, // TODO: This needs to be converted to work with blob http server.
        color: String,
    },
    #[serde(rename_all = "camelCase")]
    Unconfigured {
        id: u32,
    },
}

impl Account {
    pub async fn from_context(ctx: &deltachat::context::Context, id: u32) -> Result<Self> {
        if ctx.is_configured().await? {
            let display_name = ctx.get_config(Config::Displayname).await?;
            let addr = ctx.get_config(Config::Addr).await?;
            let profile_image = ctx.get_config(Config::Selfavatar).await?;
            let color = color_int_to_hex_string(
                Contact::get_by_id(ctx, ContactId::SELF).await?.get_color(),
            );
            Ok(Account::Configured {
                id,
                display_name,
                addr,
                profile_image,
                color,
            })
        } else {
            Ok(Account::Unconfigured { id })
        }
    }
}
