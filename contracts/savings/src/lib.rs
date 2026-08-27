#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, Address, Env, Symbol};

mod storage;
#[cfg(test)]
mod test;
pub mod types;
pub mod validation;

/// Typed errors for the savings contract.
/// Typed errors for the savings contract.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Contract has already been initialized.
    AlreadyInitialized = 1,
    /// Contract has not been initialized.
    NotInitialized = 2,
    /// Amount must be a strictly positive value.
    InvalidAmount = 3,
    /// Withdrawal amount exceeds the caller's current balance.
    InsufficientBalance = 4,
}

/// Savings contract entrypoint.
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
        storage::write_config(&env, &types::Config { admin });
        Ok(())
    }

    /// Deposits `amount` of `asset` into `user`'s savings balance. This
    /// contract tracks internal accounting only — it does not itself move
    /// any tokens on `user`'s behalf; callers are responsible for the
    /// corresponding token transfer alongside this call.
    pub fn deposit(env: Env, user: Address, amount: i128, asset: Symbol) -> Result<(), Error> {
        user.require_auth();
        validation::validate_amount(amount)?;

        let balance = storage::read_balance(&env, &user, &asset) + amount;
        storage::write_balance(&env, &user, &asset, balance);
        Ok(())
    }

    /// Withdraws `amount` of `asset` from `user`'s savings balance.
    pub fn withdraw(env: Env, user: Address, amount: i128, asset: Symbol) -> Result<(), Error> {
        user.require_auth();
        validation::validate_amount(amount)?;

        let current = storage::read_balance(&env, &user, &asset);
        if amount > current {
            return Err(Error::InsufficientBalance);
        }
        storage::write_balance(&env, &user, &asset, current - amount);
        Ok(())
    }

    /// Returns `user`'s current balance for `asset`.
    pub fn get_balance(env: Env, user: Address, asset: Symbol) -> i128 {
        storage::read_balance(&env, &user, &asset)
    }
}
