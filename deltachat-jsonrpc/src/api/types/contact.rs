use anyhow::Result;
use deltachat::contact::VerifiedStatus;
use deltachat::context::Context;
use serde::Serialize;
use typescript_type_def::TypeDef;

use super::color_int_to_hex_string;

#[derive(Serialize, TypeDef)]
#[serde(rename = "Contact")]
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
        })
    }
}
