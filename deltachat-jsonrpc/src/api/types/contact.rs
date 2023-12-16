use anyhow::Result;
use deltachat::contact::VerifiedStatus;
use deltachat::context::Context;
use serde::Serialize;
use typescript_type_def::TypeDef;

use super::color_int_to_hex_string;

#[derive(Serialize, TypeDef, schemars::JsonSchema)]
#[serde(rename = "Contact", rename_all = "camelCase")]
pub struct ContactObject {
    address: String,
    color: String,
    auth_name: String,
    status: String,
    display_name: String,
    id: u32,
    name: String,
    profile_image: Option<String>, // BLOBS
    name_and_addr: String,
    is_blocked: bool,
    is_verified: bool,
    /// the address that verified this contact
    verifier_addr: Option<String>,
    /// the id of the contact that verified this contact
    verifier_id: Option<u32>,
    /// the contact's last seen timestamp
    last_seen: i64,
    was_seen_recently: bool,
}

impl ContactObject {
    pub async fn try_from_dc_contact(
        context: &Context,
        contact: deltachat::contact::Contact,
    ) -> Result<Self> {
        let profile_image = match contact.get_profile_image(context).await? {
            Some(path_buf) => path_buf.to_str().map(|s| s.to_owned()),
            None => None,
        };
        let is_verified = contact.is_verified(context).await? == VerifiedStatus::BidirectVerified;

        let (verifier_addr, verifier_id) = if is_verified {
            (
                contact.get_verifier_addr(context).await?,
                contact
                    .get_verifier_id(context)
                    .await?
                    .map(|contact_id| contact_id.to_u32()),
            )
        } else {
            (None, None)
        };

        Ok(ContactObject {
            address: contact.get_addr().to_owned(),
            color: color_int_to_hex_string(contact.get_color()),
            auth_name: contact.get_authname().to_owned(),
            status: contact.get_status().to_owned(),
            display_name: contact.get_display_name().to_owned(),
            id: contact.id.to_u32(),
            name: contact.get_name().to_owned(),
            profile_image, //BLOBS
            name_and_addr: contact.get_name_n_addr(),
            is_blocked: contact.is_blocked(),
            is_verified,
            verifier_addr,
            verifier_id,
            last_seen: contact.last_seen(),
            was_seen_recently: contact.was_seen_recently(),
        })
    }
}
