#![no_std]

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

mod engine;
mod storage;
#[cfg(test)]
mod test;
pub mod types;
pub mod validation;

pub use types::Error;

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Initializes the contract with an administrator and sets the addresses of
    /// the three contracts this engine composes.
    pub fn initialize(
        env: Env,
        admin: Address,
        spending_limits: Address,
        spending_categories: Address,
        zk_verifier: Address,
    ) -> Result<(), Error> {
        if storage::read_config(&env).is_some() {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        storage::write_config(&env, &types::Config { admin });
        storage::write_spending_limits_address(&env, &spending_limits);
        storage::write_spending_categories_address(&env, &spending_categories);
        storage::write_zk_verifier_address(&env, &zk_verifier);
        Ok(())
    }

    /// Creates or replaces a spending rule for `user` in `category`.
    ///
    /// Only the administrator may create rules. Users cannot self-serve rule
    /// creation — rules are policy-level configuration managed by the admin.
    pub fn set_rule(
        env: Env,
        admin: Address,
        user: Address,
        category: Symbol,
        weekly_limit: i128,
        zk_required_above: i128,
    ) -> Result<(), Error> {
        admin.require_auth();
        let current = storage::read_config(&env).ok_or(Error::Unauthorized)?;
        if current.admin != admin {
            return Err(Error::Unauthorized);
        }
        if weekly_limit <= 0 {
            return Err(Error::InvalidAmount);
        }
        let rule = types::Rule {
            category: category.clone(),
            weekly_limit,
            zk_required_above,
        };
        storage::write_rule(&env, &user, &category, &rule);

        env.events().publish(
            (symbol_short!("rule"), symbol_short!("set"), user),
            (category, weekly_limit, zk_required_above),
        );
        Ok(())
    }

    /// Evaluates a proposed transaction against all active spending rules for
    /// the given `user` and `category`.
    ///
    /// This is the core composition function. It performs a sequence of
    /// cross-contract calls:
    ///
    /// 1. Loads the `Rule` for `(user, category)` from this contract's storage.
    /// 2. If `amount > rule.zk_required_above`, verifies the `zk_proof` via the
    ///    zk-verifier contract. A missing proof returns `ZkProofRequired`; an
    ///    invalid proof returns `ZkProofInvalid`.
    /// 3. Fetches the user's current weekly spend in this category from the
    ///    spending-categories contract. If `current_spent + amount` exceeds
    ///    `rule.weekly_limit`, returns `CategoryLimitExceeded`.
    /// 4. Checks the general per-asset spending cap via the spending-limits
    ///    contract. If the cap would be exceeded, returns `CategoryLimitExceeded`.
    ///
    /// Returns `Ok(())` when all checks pass, or the appropriate `Error`
    /// variant on failure.
    pub fn evaluate(
        env: Env,
        user: Address,
        category: Symbol,
        amount: i128,
        zk_proof: Option<soroban_sdk::Bytes>,
    ) -> Result<(), Error> {
        engine::evaluate(&env, &user, &category, amount, zk_proof)
    }

    // ─── Read-only queries ─────────────────────────────────────────────────

    /// Returns the rule for `(user, category)`, if one exists.
    pub fn get_rule(env: Env, user: Address, category: Symbol) -> Option<types::Rule> {
        storage::read_rule(&env, &user, &category)
    }

    /// Returns the stored spending-limits contract address.
    pub fn get_spending_limits_address(env: Env) -> Option<Address> {
        storage::read_spending_limits_address(&env)
    }

    /// Returns the stored spending-categories contract address.
    pub fn get_spending_categories_address(env: Env) -> Option<Address> {
        storage::read_spending_categories_address(&env)
    }

    /// Returns the stored zk-verifier contract address.
    pub fn get_zk_verifier_address(env: Env) -> Option<Address> {
        storage::read_zk_verifier_address(&env)
    }
}
