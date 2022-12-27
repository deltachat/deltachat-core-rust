/// Fuzzing target for simplify().
///
/// Calls simplify() and panics if simplify() panics.
/// Does not return any vaule to avoid exposing internal crate types.
#[cfg(fuzzing)]
pub fn simplify(mut input: String, is_chat_message: bool) {
    crate::simplify::simplify(input, is_chat_message);
}
