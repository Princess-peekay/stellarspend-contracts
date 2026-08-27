use crate::types::{Config, Contribution, DataKey, Goal};
use soroban_sdk::{Address, Env, Vec};

/// Reads contract configuration from instance storage.
pub fn read_config(env: &Env) -> Option<Config> {
    env.storage().instance().get(&DataKey::Config)
}

/// Writes contract configuration to instance storage.
pub fn write_config(env: &Env, config: &Config) {
    env.storage().instance().set(&DataKey::Config, config);
}

/// Reads a goal record from persistent storage.
pub fn read_goal(env: &Env, goal_id: u64) -> Option<Goal> {
    env.storage().persistent().get(&DataKey::Goal(goal_id))
}

/// Writes a goal record to persistent storage.
pub fn write_goal(env: &Env, goal: &Goal) {
    env.storage()
        .persistent()
        .set(&DataKey::Goal(goal.goal_id), goal);
}

/// Appends `goal_id` to the list of goal ids owned by `user`.
pub fn add_user_goal(env: &Env, user: &Address, goal_id: u64) {
    let key = DataKey::UserGoals(user.clone());
    let mut ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env));
    ids.push_back(goal_id);
    env.storage().persistent().set(&key, &ids);
}

/// Returns all goal ids owned by `user`.
pub fn user_goal_ids(env: &Env, user: &Address) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&DataKey::UserGoals(user.clone()))
        .unwrap_or(Vec::new(env))
}

/// Allocates and returns the next contribution id for `goal_id`.
pub fn next_contribution_id(env: &Env, goal_id: u64) -> u64 {
    let key = DataKey::LastContribId(goal_id);
    let next: u64 = env.storage().persistent().get(&key).unwrap_or(0u64) + 1;
    env.storage().persistent().set(&key, &next);
    next
}

/// Appends a contribution record to a goal's contribution history.
pub fn add_contribution(env: &Env, goal_id: u64, contribution: &Contribution) {
    let key = DataKey::GoalContributions(goal_id);
    let mut history: Vec<Contribution> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env));
    history.push_back(contribution.clone());
    env.storage().persistent().set(&key, &history);
}

/// Returns the full contribution history for a goal, oldest first.
pub fn goal_contributions(env: &Env, goal_id: u64) -> Vec<Contribution> {
    env.storage()
        .persistent()
        .get(&DataKey::GoalContributions(goal_id))
        .unwrap_or(Vec::new(env))
}
