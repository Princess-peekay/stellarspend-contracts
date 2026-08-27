use soroban_sdk::{contract, contracterror, contractimpl, Address, Env};

use crate::events;
use crate::storage::{self, Config};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RagError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
}

#[contract]
pub struct RagContract;

#[contractimpl]
impl RagContract {
    /// Initializes the contract: sets the administrator and default
    /// configuration. Can only ever succeed once — subsequent calls
    /// return `RagError::AlreadyInitialized`.
    pub fn initialize(env: Env, admin: Address) -> Result<(), RagError> {
        if storage::has_admin(&env) {
            return Err(RagError::AlreadyInitialized);
        }

        // Only the account being set as admin can authorize its own
        // installation as administrator.
        admin.require_auth();

        storage::set_admin(&env, &admin);
        storage::set_config(&env, &Config { paused: false });

        events::emit_initialized(&env, &admin);

        Ok(())
    }

    /// Returns the current administrator. Panics if not yet initialized.
    pub fn get_admin(env: Env) -> Address {
        storage::get_admin(&env)
    }

    /// Returns whether the contract has been initialized.
    pub fn is_initialized(env: Env) -> bool {
        storage::has_admin(&env)
    }
}