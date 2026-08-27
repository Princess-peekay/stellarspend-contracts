#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, symbol_short, Address, Env, Symbol, Vec};

mod storage;
#[cfg(test)]
mod test;
pub mod types;
pub mod validation;

pub use types::{Contribution, Goal, ScheduleStatus};

/// Typed errors for the savings_goals contract.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Contract has already been initialized.
    AlreadyInitialized = 1,
    /// Contract has not been initialized.
    NotInitialized = 2,
    /// Caller is not authorized to perform this action on the given goal.
    Unauthorized = 3,
    /// Amount must be a strictly positive value.
    InvalidAmount = 4,
    /// Deadline must be strictly in the future.
    InvalidDeadline = 5,
    /// No goal exists with the given id.
    GoalNotFound = 6,
    /// Round-up "nearest unit" must be a strictly positive value when
    /// enabling the round-up rule.
    InvalidRoundUpUnit = 7,
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Initializes the contract with an administrator. None of the
    /// per-user goal operations below require the admin's authorization;
    /// this exists so deployments have a consistent init step across all
    /// StellarSpend contracts, and to guard against double-initialization.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if storage::read_config(&env).is_some() {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        storage::write_config(
            &env,
            &types::Config {
                admin,
                last_goal_id: 0,
            },
        );
        Ok(())
    }

    /// Creates a new savings goal for `user` and returns its id.
    pub fn create_goal(
        env: Env,
        user: Address,
        name: Symbol,
        target: i128,
        asset: Symbol,
        deadline: u64,
    ) -> Result<u64, Error> {
        user.require_auth();
        validation::validate_amount(target)?;
        validation::validate_deadline(&env, deadline)?;

        let mut config = storage::read_config(&env).ok_or(Error::NotInitialized)?;
        config.last_goal_id += 1;
        let goal_id = config.last_goal_id;
        storage::write_config(&env, &config);

        let goal = Goal {
            goal_id,
            user: user.clone(),
            name,
            target,
            current_amount: 0,
            asset,
            deadline,
            created_at: env.ledger().timestamp(),
            is_complete: false,
            round_up_enabled: false,
            round_up_nearest_unit: 0,
            schedule_status: ScheduleStatus::Active,
        };
        storage::write_goal(&env, &goal);
        storage::add_user_goal(&env, &user, goal_id);

        Ok(goal_id)
    }

    /// Records a contribution toward `goal_id` on behalf of `user`.
    /// Emits a `milestone` event the first time the goal's target is met
    /// or exceeded.
    pub fn contribute(env: Env, user: Address, goal_id: u64, amount: i128) -> Result<(), Error> {
        user.require_auth();
        validation::validate_amount(amount)?;

        let mut goal = storage::read_goal(&env, goal_id).ok_or(Error::GoalNotFound)?;
        if goal.user != user {
            return Err(Error::Unauthorized);
        }

        let contrib_id = storage::next_contribution_id(&env, goal_id);
        let contribution = Contribution {
            contribution_id: contrib_id,
            goal_id,
            user: user.clone(),
            amount,
            timestamp: env.ledger().timestamp(),
        };
        storage::add_contribution(&env, goal_id, &contribution);

        let was_complete = goal.is_complete;
        goal.current_amount += amount;
        if !was_complete && goal.current_amount >= goal.target {
            goal.is_complete = true;
            env.events().publish(
                (symbol_short!("milestone"), goal_id),
                (user.clone(), goal.current_amount),
            );
        }
        storage::write_goal(&env, &goal);

        Ok(())
    }

    /// Returns the current state of a goal owned by `user`.
    pub fn get_goal(env: Env, user: Address, goal_id: u64) -> Result<Goal, Error> {
        let goal = storage::read_goal(&env, goal_id).ok_or(Error::GoalNotFound)?;
        if goal.user != user {
            return Err(Error::Unauthorized);
        }
        Ok(goal)
    }

    /// Returns all goals belonging to `user`.
    pub fn get_all_goals(env: Env, user: Address) -> Vec<Goal> {
        let goal_ids = storage::user_goal_ids(&env, &user);
        let mut goals = Vec::new(&env);
        for goal_id in goal_ids.iter() {
            if let Some(goal) = storage::read_goal(&env, goal_id) {
                goals.push_back(goal);
            }
        }
        goals
    }

    /// Returns the contribution history for a goal owned by `user`.
    pub fn get_contribution_history(
        env: Env,
        goal_id: u64,
        user: Address,
    ) -> Result<Vec<Contribution>, Error> {
        let goal = storage::read_goal(&env, goal_id).ok_or(Error::GoalNotFound)?;
        if goal.user != user {
            return Err(Error::Unauthorized);
        }
        Ok(storage::goal_contributions(&env, goal_id))
    }

    /// Enables or disables round-up contributions for a goal and sets the
    /// unit contributions should be rounded up to. Disabling resets the
    /// stored unit back to zero.
    pub fn set_round_up_rule(
        env: Env,
        user: Address,
        goal_id: u64,
        enabled: bool,
        nearest_unit: i128,
    ) -> Result<(), Error> {
        user.require_auth();
        if enabled {
            validation::validate_round_up_unit(nearest_unit)?;
        }

        let mut goal = storage::read_goal(&env, goal_id).ok_or(Error::GoalNotFound)?;
        if goal.user != user {
            return Err(Error::Unauthorized);
        }
        goal.round_up_enabled = enabled;
        goal.round_up_nearest_unit = if enabled { nearest_unit } else { 0 };
        storage::write_goal(&env, &goal);
        Ok(())
    }

    /// Pauses the automated contribution schedule for a goal.
    pub fn pause_schedule(env: Env, user: Address, goal_id: u64) -> Result<(), Error> {
        set_schedule_status(&env, &user, goal_id, ScheduleStatus::Paused)
    }

    /// Resumes a previously paused contribution schedule.
    pub fn resume_schedule(env: Env, user: Address, goal_id: u64) -> Result<(), Error> {
        set_schedule_status(&env, &user, goal_id, ScheduleStatus::Active)
    }

    /// Cancels the automated contribution schedule for a goal. This is a
    /// terminal state; call `set_round_up_rule` again to start a new one.
    pub fn cancel_schedule(env: Env, user: Address, goal_id: u64) -> Result<(), Error> {
        set_schedule_status(&env, &user, goal_id, ScheduleStatus::Cancelled)
    }
}

/// Shared implementation for the three schedule-lifecycle endpoints.
fn set_schedule_status(
    env: &Env,
    user: &Address,
    goal_id: u64,
    status: ScheduleStatus,
) -> Result<(), Error> {
    user.require_auth();
    let mut goal = storage::read_goal(env, goal_id).ok_or(Error::GoalNotFound)?;
    if &goal.user != user {
        return Err(Error::Unauthorized);
    }
    goal.schedule_status = status;
    storage::write_goal(env, &goal);
    Ok(())
}
