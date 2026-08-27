use crate::Error;

/// Validates a deposit/withdrawal amount: must be strictly positive.
pub fn validate_amount(amount: i128) -> Result<(), Error> {
    if amount <= 0 {
        Err(Error::InvalidAmount)
    } else {
        Ok(())
    }
}
