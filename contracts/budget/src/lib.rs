#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, Address, Env, Symbol, Vec};

mod storage;
#[cfg(test)]
mod test;
pub mod types;
pub mod validation;

pub use types::Budget;

/// Typed errors for the budget contract.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Contract has already been initialized.
    AlreadyInitialized = 1,
    /// Contract has not been initialized.
    NotInitialized = 2,
    /// Caller is not authorized to perform this action on the given
    /// budget.
    Unauthorized = 3,
    /// Amount must be a strictly positive value.
    InvalidAmount = 4,
    /// `end_date` must be strictly after `start_date`.
    InvalidDateRange = 5,
    /// No budget exists with the given id.
    BudgetNotFound = 6,
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
        storage::write_config(
            &env,
            &types::Config {
                admin,
                last_budget_id: 0,
            },
        );
        Ok(())
    }

    /// Creates a new budget for `user` and returns its id.
    #[allow(clippy::too_many_arguments)]
    pub fn create_budget(
        env: Env,
        user: Address,
        name: Symbol,
        amount: i128,
        category: Symbol,
        asset: Symbol,
        start_date: u64,
        end_date: u64,
    ) -> Result<u64, Error> {
        user.require_auth();
        validation::validate_amount(amount)?;
        validation::validate_date_range(start_date, end_date)?;

        let mut config = storage::read_config(&env).ok_or(Error::NotInitialized)?;
        config.last_budget_id += 1;
        let budget_id = config.last_budget_id;
        storage::write_config(&env, &config);

        let budget = Budget {
            budget_id,
            user: user.clone(),
            name,
            amount,
            category,
            asset,
            start_date,
            end_date,
            created_at: env.ledger().timestamp(),
        };
        storage::write_budget(&env, &budget);
        storage::add_user_budget(&env, &user, budget_id);

        Ok(budget_id)
    }

    /// Updates the amount allocated to an existing budget.
    pub fn update_budget(
        env: Env,
        user: Address,
        budget_id: u64,
        amount: i128,
    ) -> Result<(), Error> {
        user.require_auth();
        validation::validate_amount(amount)?;

        let mut budget = storage::read_budget(&env, budget_id).ok_or(Error::BudgetNotFound)?;
        if budget.user != user {
            return Err(Error::Unauthorized);
        }
        budget.amount = amount;
        storage::write_budget(&env, &budget);
        Ok(())
    }

    /// Deletes a budget owned by `user`.
    pub fn delete_budget(env: Env, user: Address, budget_id: u64) -> Result<(), Error> {
        user.require_auth();
        let budget = storage::read_budget(&env, budget_id).ok_or(Error::BudgetNotFound)?;
        if budget.user != user {
            return Err(Error::Unauthorized);
        }
        storage::delete_budget(&env, budget_id);
        storage::remove_user_budget(&env, &user, budget_id);
        Ok(())
    }

    /// Returns all budgets belonging to `user`.
    pub fn get_budgets(env: Env, user: Address) -> Vec<Budget> {
        let ids = storage::user_budget_ids(&env, &user);
        let mut budgets = Vec::new(&env);
        for id in ids.iter() {
            if let Some(budget) = storage::read_budget(&env, id) {
                budgets.push_back(budget);
            }
        }
        budgets
    }
}
