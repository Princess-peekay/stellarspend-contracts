use soroban_sdk::{symbol_short, Address, Env};

/// Emitted once, when the contract is successfully initialized.
///
/// topics: ("init",)
/// data:   admin address
pub fn emit_initialized(env: &Env, admin: &Address) {
    env.events()
        .publish((symbol_short!("init"),), admin.clone());
}