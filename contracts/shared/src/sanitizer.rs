use alloc::string::String;

/// Removes null bytes and control characters from a memo.
pub fn sanitize_memo(input: String) -> String {
    input
        .chars()
        .filter(|c| *c != '\0' && !c.is_control())
        .collect()
}
