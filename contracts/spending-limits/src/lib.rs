#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, symbol_short, Address, Env, Symbol};

mod storage;
#[cfg(test)]
mod test;
pub mod types;
pub mod validation;

/// Typed errors for the spending_limits contract.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Contract has already been initialized.
    AlreadyInitialized = 1,
    /// Caller is not authorized to perform this action.
    Unauthorized = 2,
    /// Amount validation failed (must be strictly positive).
    InvalidAmount = 3,
    /// `period` is not one of "daily" | "weekly" | "monthly".
    InvalidPeriod = 4,
    /// No limit has been configured for this (user, asset) pair.
    LimitNotFound = 5,
    /// Recording this spend would exceed the configured limit.
    LimitExceeded = 6,
    /// `asset` is not on the supported-asset allowlist.
    UnsupportedAsset = 7,
    /// Accumulating this spend would overflow i128.
    Overflow = 8,
}

/// The `spending_limits` smart contract.
#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Initializes the contract with an administrator. The admin has no
    /// special power over per-user limits (those are entirely self-service,
    /// authorized by the `user` parameter on each call) — this exists for
    /// deployment-tooling consistency with the rest of the workspace, and as
    /// a seam for future contract-wide administrative actions (e.g. pausing).
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if storage::read_admin(&env).is_some() {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        storage::write_admin(&env, &admin);
        Ok(())
    }

    /// Sets (or replaces) `user`'s spending cap for `asset` over `period`
    /// ("daily" | "weekly" | "monthly"). Only `user` may set their own limit.
    /// Replacing an existing limit updates only the cap — spend already
    /// accumulated this period is untouched (see `storage::write_limit`).
    pub fn set_limit(
        env: Env,
        user: Address,
        asset: Symbol,
        amount: i128,
        period: Symbol,
    ) -> Result<(), Error> {
        user.require_auth();
        validation::validate_amount(amount)?;
        validation::validate_asset(&asset)?;
        let period = types::Period::from_symbol(&env, &period).ok_or(Error::InvalidPeriod)?;

        storage::write_limit(&env, &user, &asset, &types::Limit { amount, period });

        env.events().publish(
            (symbol_short!("limit"), symbol_short!("set"), user.clone()),
            (asset, amount),
        );
        Ok(())
    }

    /// Records a spend of `amount` against `user`'s current-period cap for
    /// `asset`. Rejected in full (no partial application) if it would push
    /// the period's accumulated spend over the configured limit.
    pub fn record_spend(env: Env, user: Address, asset: Symbol, amount: i128) -> Result<(), Error> {
        user.require_auth();
        validation::validate_amount(amount)?;

        let limit = storage::read_limit(&env, &user, &asset).ok_or(Error::LimitNotFound)?;
        let idx = limit.period.index(&env);
        let current = storage::read_spent(&env, &user, &asset, idx);
        let new_spent = current.checked_add(amount).ok_or(Error::Overflow)?;
        if new_spent > limit.amount {
            return Err(Error::LimitExceeded);
        }

        storage::write_spent(&env, &user, &asset, idx, new_spent);

        env.events().publish(
            (symbol_short!("limit"), symbol_short!("spend"), user.clone()),
            (asset, amount, new_spent),
        );
        Ok(())
    }

    /// Returns `user`'s remaining allowance for `asset` in the current
    /// period. Returns 0 if no limit is configured — the safe default is
    /// "nothing allowed," never an implicit unlimited allowance.
    pub fn get_remaining(env: Env, user: Address, asset: Symbol) -> i128 {
        match storage::read_limit(&env, &user, &asset) {
            Some(limit) => {
                let idx = limit.period.index(&env);
                let spent = storage::read_spent(&env, &user, &asset, idx);
                // record_spend never lets spent exceed limit.amount, so this
                // subtraction cannot go negative in practice; saturating_sub
                // is a defensive floor rather than a load-bearing check.
                limit.amount.saturating_sub(spent)
            }
            None => 0,
        }
    }

    /// Read-only check: would recording `amount` now stay within `user`'s
    /// current-period limit for `asset`? Callable by other contracts (e.g.
    /// the spending-rules composition layer) with no authorization required,
    /// since it changes no state.
    pub fn check_limit(env: Env, user: Address, asset: Symbol, amount: i128) -> bool {
        if amount <= 0 {
            return false;
        }
        match storage::read_limit(&env, &user, &asset) {
            Some(limit) => {
                let idx = limit.period.index(&env);
                let spent = storage::read_spent(&env, &user, &asset, idx);
                match spent.checked_add(amount) {
                    Some(total) => total <= limit.amount,
                    None => false,
                }
            }
            None => false,
        }
    }
}
