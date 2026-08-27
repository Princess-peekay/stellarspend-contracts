use crate::Error;

/// Validates a budget amount: must be strictly positive.
pub fn validate_amount(amount: i128) -> Result<(), Error> {
    if amount <= 0 {
        Err(Error::InvalidAmount)
    } else {
        Ok(())
    }
}

/// Validates that a budget's end date is strictly after its start date.
pub fn validate_date_range(start_date: u64, end_date: u64) -> Result<(), Error> {
    if end_date <= start_date {
        Err(Error::InvalidDateRange)
    } else {
        Ok(())
    }
}
