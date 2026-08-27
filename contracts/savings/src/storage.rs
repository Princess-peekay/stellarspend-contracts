use crate::types::{Config, DataKey};
use soroban_sdk::{Address, Env, Symbol};

/// Reads contract configuration from instance storage.
pub fn read_config(env: &Env) -> Option<Config> {
    env.storage().instance().get(&DataKey::Config)
}

/// Writes contract configuration to instance storage.
pub fn write_config(env: &Env, config: &Config) {
    env.storage().instance().set(&DataKey::Config, config);
}

/// Returns `user`'s balance for `asset`, or zero if none has ever been
/// recorded.
pub fn read_balance(env: &Env, user: &Address, asset: &Symbol) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::Balance(user.clone(), asset.clone()))
        .unwrap_or(0)
}

/// Writes `user`'s balance for `asset`.
pub fn write_balance(env: &Env, user: &Address, asset: &Symbol, balance: i128) {
    env.storage()
        .persistent()
        .set(&DataKey::Balance(user.clone(), asset.clone()), &balance);
}
