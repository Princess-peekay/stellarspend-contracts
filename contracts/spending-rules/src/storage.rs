use crate::types::{Config, DataKey, Rule};
use soroban_sdk::{Address, Env, Symbol};

const CONFIG: &str = "CONFIG";
const SPENDING_LIMITS: &str = "S_LIMITS";
const SPENDING_CATEGORIES: &str = "S_CATS";
const ZK_VERIFIER: &str = "ZK_VER";

// ─── Configuration ───────────────────────────────────────────────────────────

/// Reads contract configuration from instance storage.
pub fn read_config(env: &Env) -> Option<Config> {
    env.storage().instance().get(&CONFIG)
}

/// Writes contract configuration to instance storage.
pub fn write_config(env: &Env, config: &Config) {
    env.storage().instance().set(&CONFIG, config);
}

// ─── Rules ───────────────────────────────────────────────────────────────────

/// Reads the rule for `(user, category)`, if any.
pub fn read_rule(env: &Env, user: &Address, category: &Symbol) -> Option<Rule> {
    env.storage()
        .persistent()
        .get(&DataKey::Rule(user.clone(), category.clone()))
}

/// Writes (or replaces) the rule for `(user, category)`.
pub fn write_rule(env: &Env, user: &Address, category: &Symbol, rule: &Rule) {
    env.storage()
        .persistent()
        .set(&DataKey::Rule(user.clone(), category.clone()), rule);
}

// ─── Cross-contract addresses ────────────────────────────────────────────────

/// Reads the stored spending-limits contract address.
pub fn read_spending_limits_address(env: &Env) -> Option<Address> {
    env.storage().instance().get(&SPENDING_LIMITS)
}

/// Writes the spending-limits contract address to instance storage.
pub fn write_spending_limits_address(env: &Env, addr: &Address) {
    env.storage().instance().set(&SPENDING_LIMITS, addr);
}

/// Reads the stored spending-categories contract address.
pub fn read_spending_categories_address(env: &Env) -> Option<Address> {
    env.storage().instance().get(&SPENDING_CATEGORIES)
}

/// Writes the spending-categories contract address to instance storage.
pub fn write_spending_categories_address(env: &Env, addr: &Address) {
    env.storage().instance().set(&SPENDING_CATEGORIES, addr);
}

/// Reads the stored zk-verifier contract address.
pub fn read_zk_verifier_address(env: &Env) -> Option<Address> {
    env.storage().instance().get(&ZK_VERIFIER)
}

/// Writes the zk-verifier contract address to instance storage.
pub fn write_zk_verifier_address(env: &Env, addr: &Address) {
    env.storage().instance().set(&ZK_VERIFIER, addr);
}
