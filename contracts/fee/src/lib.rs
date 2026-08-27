#![no_std]

extern crate alloc;

use soroban_sdk::{contract, contracterror, contractimpl, Address, Env, Vec};

mod storage;
#[cfg(test)]
mod test;
pub mod types;
pub mod validation;

/// Typed errors for the fee contract.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Contract has already been initialized.
    AlreadyInitialized = 1,
    /// Caller is not the administrator.
    Unauthorized = 2,
    /// Amount validation failed.
    InvalidAmount = 3,
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Initializes the contract with an administrator.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if storage::read_config(&env).is_some() {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        storage::write_config(&env, &types::Config { admin, value: 0 });
        Ok(())
    }

    /// Updates the contract value after authenticating the administrator.
    pub fn set_value(env: Env, admin: Address, value: i128) -> Result<(), Error> {
        admin.require_auth();
        if value < 0 {
            return Err(Error::InvalidAmount);
        }
        let current = storage::read_config(&env).ok_or(Error::Unauthorized)?;
        if current.admin != admin {
            return Err(Error::Unauthorized);
        }
        storage::write_config(&env, &types::Config { admin, value });
        Ok(())
    }

    pub fn get_value(env: Env) -> i128 {
        storage::read_config(&env).map(|c| c.value).unwrap_or(0)
    }

    /// Returns the configured administrator, if the contract has been
    /// initialized.
    pub fn get_owner(env: Env) -> Option<Address> {
        storage::owner(&env)
    }

    /// Calculates the applicable fee tier rate for `value` using the shared
    /// tiered-rate utility (`shared::rate_curve::calculate_tiered_rate`), so
    /// fee tier calculations stay consistent with batch-rewards and rewards
    /// rather than duplicating the tier-matching logic here (#1109).
    pub fn calculate_tiered_rate(value: i128, tiers: Vec<shared::rate_curve::Tier>) -> i128 {
        let tiers: alloc::vec::Vec<shared::rate_curve::Tier> = tiers.iter().collect();
        shared::rate_curve::calculate_tiered_rate(value, &tiers)
    }
}
