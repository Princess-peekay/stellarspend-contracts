use crate::Error;
use soroban_sdk::Env;

/// Validates a contribution or goal target amount: must be strictly
/// positive.
pub fn validate_amount(amount: i128) -> Result<(), Error> {
    if amount <= 0 {
        Err(Error::InvalidAmount)
    } else {
        Ok(())
    }
}

/// Validates a round-up "nearest unit": must be strictly positive.
pub fn validate_round_up_unit(nearest_unit: i128) -> Result<(), Error> {
    if nearest_unit <= 0 {
        Err(Error::InvalidRoundUpUnit)
    } else {
        Ok(())
    }
}

/// Validates that a deadline lies strictly in the future relative to the
/// current ledger timestamp.
pub fn validate_deadline(env: &Env, deadline: u64) -> Result<(), Error> {
    if deadline <= env.ledger().timestamp() {
        Err(Error::InvalidDeadline)
    } else {
        Ok(())
    }
}
