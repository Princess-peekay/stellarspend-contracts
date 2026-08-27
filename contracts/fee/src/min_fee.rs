use soroban_sdk::Env;
use crate::storage;

/// Returns the configured minimum fee, falling back to 1 when no value is stored.
pub fn get_min_fee(env: &Env) -> i128 {
    storage::get_min_fee(env).unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_get_min_fee_default() {
        let env = Env::default();
        let fee = get_min_fee(&env);
        assert!(fee >= 1);
    }
}
