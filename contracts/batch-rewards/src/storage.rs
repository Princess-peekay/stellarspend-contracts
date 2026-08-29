use crate::types::Config;
use soroban_sdk::{Address, Env};

const CONFIG: &str = "CONFIG";

/// Reads contract configuration from instance storage.
pub fn read_config(env: &Env) -> Option<Config> {
    // Instance storage is appropriate for contract-wide configuration.
    env.storage().instance().get(&CONFIG)
}

/// Writes contract configuration to instance storage.
pub fn write_config(env: &Env, config: &Config) {
    // Instance storage keeps configuration attached to the contract instance.
    env.storage().instance().set(&CONFIG, config);
}

/// Returns the configured owner.
#[allow(dead_code)]
pub fn owner(env: &Env) -> Option<Address> {
    read_config(env).map(|c| c.admin)
}
