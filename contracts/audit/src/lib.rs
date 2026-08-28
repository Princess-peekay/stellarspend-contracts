#![no_std]
//! # StellarSpend audit contract
//!
//! Holds an administrator address and a single non-negative value in instance
//! storage, and gates every write on that administrator's authorization.
//!
//! ## Lifecycle
//!
//! 1. [`Contract::initialize`] records the administrator. It may only be
//!    called once.
//! 2. [`Contract::set_value`] updates the stored value; only the recorded
//!    administrator may call it.
//! 3. [`Contract::get_value`] reads the value. It is unauthenticated.
//!
//! ## Authorization
//!
//! Writes require a signature from the administrator via
//! [`Address::require_auth`], which panics — rather than returning an
//! [`Error`] — when the required authorization is absent. A caller that is
//! *authenticated but not the administrator* gets [`Error::Unauthorized`]
//! back instead, so the two failures are distinguishable.

use soroban_sdk::{contract, contracterror, contractimpl, Address, Env};

mod storage;
#[cfg(test)]
mod test;
/// Stored data types for this contract.
pub mod types;
/// Standalone input validation helpers.
pub mod validation;

/// Typed errors for the audit contract.
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

/// The audit contract.
#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Initializes the contract with an administrator.
    ///
    /// Records `admin` as the sole account permitted to call
    /// [`Contract::set_value`], and sets the stored value to `0`.
    ///
    /// # Arguments
    ///
    /// * `env` — the contract environment.
    /// * `admin` — the address to record as administrator.
    ///
    /// # Errors
    ///
    /// Returns [`Error::AlreadyInitialized`] if configuration is already
    /// present. There is no way to re-initialize or to transfer
    /// administration afterwards.
    ///
    /// # Panics
    ///
    /// Panics if `admin` has not authorized the call. Note that the
    /// already-initialized check runs *before* the authorization check, so a
    /// repeat call returns the error without requiring a signature.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if storage::read_config(&env).is_some() {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        storage::write_config(&env, &types::Config { admin, value: 0 });
        Ok(())
    }

    /// Updates the contract value after authenticating the administrator.
    ///
    /// # Arguments
    ///
    /// * `env` — the contract environment.
    /// * `admin` — the caller, which must match the recorded administrator.
    /// * `value` — the new value. Must be non-negative.
    ///
    /// # Errors
    ///
    /// * [`Error::InvalidAmount`] if `value` is negative.
    /// * [`Error::Unauthorized`] if `admin` is not the recorded
    ///   administrator, **or** if the contract has never been initialized —
    ///   an uninitialized contract has no administrator, so no caller
    ///   qualifies.
    ///
    /// # Panics
    ///
    /// Panics if `admin` has not authorized the call.
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

    /// Returns the current configured value.
    ///
    /// Requires no authorization.
    ///
    /// Returns `0` when the contract has not been initialized, which is
    /// indistinguishable from an initialized contract whose value is `0`.
    /// Callers that need to tell the two apart should read the configuration
    /// through a state-bearing entry point instead.
    pub fn get_value(env: Env) -> i128 {
        storage::read_config(&env).map(|c| c.value).unwrap_or(0)
    }
}
