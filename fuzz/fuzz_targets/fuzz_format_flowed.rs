use bolero::check;
use format_flowed::{format_flowed, unformat_flowed};

fn round_trip(input: &str) -> String {
    let mut input = format_flowed(input);
    input.retain(|c| c != '\r');
    unformat_flowed(&input, false)
}

fn main() {
    check!().for_each(|data: &[u8]| {
        if let Ok(input) = std::str::from_utf8(data) {
            let input = input.trim().to_string();

            // Only consider inputs that are the result of unformatting format=flowed text.
            // At least this means that lines don't contain any trailing whitespace.
            let input = round_trip(&input);
            let output = round_trip(&input);
            assert_eq!(input, output);
        }
    });
}
