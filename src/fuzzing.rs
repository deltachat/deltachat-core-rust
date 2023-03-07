//! # Fuzzing module.
//!
//! This module exposes private APIs for fuzzing.

/// Fuzzing target for simplify().
///
/// Calls simplify() and panics if simplify() panics.
/// Does not return any value to avoid exposing internal crate types.
#[cfg(fuzzing)]
pub fn simplify(input: String, is_chat_message: bool) {
    crate::simplify::simplify(input, is_chat_message);
}
