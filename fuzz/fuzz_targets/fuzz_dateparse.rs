use bolero::check;

fn main() {
    check!().for_each(|data: &[u8]| match std::str::from_utf8(data) {
        Ok(input) => {
            mailparse::dateparse(input).ok();
        }
        Err(_err) => {}
    });
}
