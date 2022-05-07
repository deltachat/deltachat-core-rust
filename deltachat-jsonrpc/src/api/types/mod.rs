pub mod account;
pub mod chat;
pub mod chat_list;
pub mod contact;
pub mod message;
pub mod provider_info;

pub fn color_int_to_hex_string(color: u32) -> String {
    format!("{:#08x}", color).replace("0x", "#")
}
