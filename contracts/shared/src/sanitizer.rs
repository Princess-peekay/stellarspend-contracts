//! Shared input-sanitization helpers for StellarSpend contracts.
//!
//! These utilities strip unsafe characters from user-supplied strings
//! (e.g. payment memos) before they are stored on-chain, preventing
//! null-byte injection and control-character pollution.
use alloc::string::String;

/// Removes null bytes (`\0`) and ASCII/Unicode control characters from a memo
/// string before it is stored on-chain.
///
/// # What is stripped
/// * Null bytes (`\0`, `U+0000`)
/// * All Unicode control characters as defined by [`char::is_control`]
///   (code points `U+0000`–`U+001F` and `U+007F`–`U+009F`)
///
/// Printable characters, whitespace that is not a control character,
/// and all non-ASCII Unicode are preserved unchanged.
///
/// # Examples
/// ```
/// let clean = sanitize_memo("hello\0world".to_string());
/// assert_eq!(clean, "helloworld");
/// ```
pub fn sanitize_memo(input: String) -> String {
    input
        .chars()
        .filter(|c| *c != '\0' && !c.is_control())
        .collect()
}
