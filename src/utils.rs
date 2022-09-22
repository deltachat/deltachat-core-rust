const INVALID_CHARACTERS: [char; 5] = ['\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}'];
/// This method strips all occurances of the RTLO Unicode character.
/// [Why is this needed](https://github.com/deltachat/deltachat-core-rust/issues/3479)?
pub(crate) fn create_safe_string(input_str: &str) -> String {
    input_str.replace(|char| INVALID_CHARACTERS.contains(&char), "")
}
