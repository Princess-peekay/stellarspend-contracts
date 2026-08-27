use soroban_sdk::{contracttype, Address, Env};

/// Keys used for contract instance storage.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Config,
}

/// Default configuration established at initialization time.
#[derive(Clone)]
#[contracttype]
pub struct Config {
    pub paused: bool,
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn get_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .expect("contract not initialized")
}

pub fn set_config(env: &Env, config: &Config) {
    env.storage().instance().set(&DataKey::Config, config);
}

pub fn get_config(env: &Env) -> Config {
    env.storage()
        .instance()
        .get(&DataKey::Config)
        .expect("contract not initialized")
}