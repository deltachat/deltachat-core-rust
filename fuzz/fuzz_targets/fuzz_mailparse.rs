use bolero::check;

fn main() {
    check!().for_each(|data: &[u8]| {
        mailparse::parse_mail(data).ok();
    });
}
