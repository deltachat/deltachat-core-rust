use anyhow::{anyhow, Result};
use deltachat::config::Config;
use deltachat::contact::{Contact, ContactId};
use serde::Serialize;
use typescript_type_def::TypeDef;

use super::color_int_to_hex_string;

#[derive(Serialize, TypeDef, schemars::JsonSchema)]
#[serde(tag = "kind")]
pub enum Account {
    #[serde(rename_all = "camelCase")]
    Configured {
        id: u32,
        display_name: Option<String>,
        addr: Option<String>,
        // size: u32,
        profile_image: Option<String>, // TODO: This needs to be converted to work with blob http server.
        color: String,

        /// Account IO is disabled when this account is not selected
        ///
        /// this means IO is stopped unless this account is selected
        /// and background fetch is also disabled for this account
        background_io_disabled: bool,
    },
    #[serde(rename_all = "camelCase")]
    Unconfigured { id: u32 },
}

impl Account {
    pub async fn load(accounts: &deltachat::accounts::Accounts, id: u32) -> Result<Self> {
        if let Some(ctx) = &accounts.get_account(id) {
            if ctx.is_configured().await? {
                let display_name = ctx.get_config(Config::Displayname).await?;
                let addr = ctx.get_config(Config::Addr).await?;
                let profile_image = ctx.get_config(Config::Selfavatar).await?;
                let color = color_int_to_hex_string(
                    Contact::get_by_id(ctx, ContactId::SELF).await?.get_color(),
                );
                Ok(Account::Configured {
                    id: ctx.get_id(),
                    display_name,
                    addr,
                    profile_image,
                    color,
                    background_io_disabled: accounts.get_disable_background_io(id).unwrap_or(false),
                })
            } else {
                Ok(Account::Unconfigured { id })
            }
        } else {
            Err(anyhow!(
                "account with id {} doesn't exist anymore",
                id
            ))
        }
    }
}
