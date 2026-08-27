use crate::types::{Budget, Config, DataKey};
use soroban_sdk::{Address, Env, Vec};

/// Reads contract configuration from instance storage.
pub fn read_config(env: &Env) -> Option<Config> {
    env.storage().instance().get(&DataKey::Config)
}

/// Writes contract configuration to instance storage.
pub fn write_config(env: &Env, config: &Config) {
    env.storage().instance().set(&DataKey::Config, config);
}

/// Reads a budget record from persistent storage.
pub fn read_budget(env: &Env, budget_id: u64) -> Option<Budget> {
    env.storage().persistent().get(&DataKey::Budget(budget_id))
}

/// Writes a budget record to persistent storage.
pub fn write_budget(env: &Env, budget: &Budget) {
    env.storage()
        .persistent()
        .set(&DataKey::Budget(budget.budget_id), budget);
}

/// Removes a budget record from persistent storage.
pub fn delete_budget(env: &Env, budget_id: u64) {
    env.storage()
        .persistent()
        .remove(&DataKey::Budget(budget_id));
}

/// Appends `budget_id` to the list of budget ids owned by `user`.
pub fn add_user_budget(env: &Env, user: &Address, budget_id: u64) {
    let key = DataKey::UserBudgets(user.clone());
    let mut ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env));
    ids.push_back(budget_id);
    env.storage().persistent().set(&key, &ids);
}

/// Removes `budget_id` from the list of budget ids owned by `user`.
pub fn remove_user_budget(env: &Env, user: &Address, budget_id: u64) {
    let key = DataKey::UserBudgets(user.clone());
    let ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env));
    let mut updated = Vec::new(env);
    for id in ids.iter() {
        if id != budget_id {
            updated.push_back(id);
        }
    }
    env.storage().persistent().set(&key, &updated);
}

/// Returns all budget ids owned by `user`.
pub fn user_budget_ids(env: &Env, user: &Address) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&DataKey::UserBudgets(user.clone()))
        .unwrap_or(Vec::new(env))
}
