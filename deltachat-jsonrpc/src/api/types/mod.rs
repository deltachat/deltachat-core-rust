pub mod account;
pub mod chat;
pub mod chat_list;
pub mod contact;
pub mod message;
pub mod provider_info;
pub mod webxdc;

pub fn color_int_to_hex_string(color: u32) -> String {
    format!("{:#08x}", color).replace("0x", "#")
}

fn maybe_empty_string_to_option(string: String) -> Option<String> {
    if string.is_empty() {
        None
    } else {
        Some(string)
    }
}
