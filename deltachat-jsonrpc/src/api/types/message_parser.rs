use deltachat::message_parser::parser::{self, Element};
use serde::{Deserialize, Serialize};
use typescript_type_def::TypeDef;

#[repr(u8)]
#[derive(Serialize, Deserialize, TypeDef, schemars::JsonSchema)]
pub enum MessageParserMode {
    OnlyText,
    DesktopSet,
    Markdown,
}

pub fn parse_text(input: &str, mode: MessageParserMode) -> std::vec::Vec<Element> {
    match mode {
        MessageParserMode::OnlyText => parser::parse_only_text(input),
        MessageParserMode::DesktopSet => parser::parse_desktop_set(input),
        MessageParserMode::Markdown => parser::parse_markdown_text(input),
    }
}
