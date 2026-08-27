/// Validates a financial amount: must be strictly positive.
pub fn validate_amount(amount: i128) -> Result<(), &'static str> {
    if amount <= 0 {
        Err("amount must be positive")
    } else {
        Ok(())
    }
}
