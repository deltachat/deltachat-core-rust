use bolero::check;

use deltachat::fuzzing::simplify;

fn main() {
    check!().for_each(|data: &[u8]| match String::from_utf8(data.to_vec()) {
        Ok(input) => {
            simplify(input.clone(), true);
            simplify(input, false);
        }
        Err(_err) => {}
    });
}
