use soroban_sdk::contracterror;

/// Common cross-contract errors returned by the shared helpers and by
/// contracts that reuse these error codes across their public surface.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SharedError {
    /// The caller is not authorized to perform the requested operation.
    Unauthorized = 1,
    /// A supplied amount is invalid (negative, or otherwise outside the
    /// contract's accepted range) for the operation being performed.
    InvalidAmount = 2,
    /// A supplied address does not match the expected Stellar address
    /// format and could not be accepted.
    InvalidAddress = 3,
    /// A supplied string does not match the expected format or value.
    InvalidString = 4,
    /// The computation exceeded the representable range and overflowed.
    Overflow = 5,
}
