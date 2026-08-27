use soroban_sdk::{contracttype, symbol_short, Address, Env};

#[contracttype]
#[derive(Clone, Debug)]
pub struct RagConfig {
    pub paused: bool,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Config,
}

pub struct AdminManager;

impl AdminManager {
    /// Initializes the contract: establishes the administrator and default
    /// configuration. Can only ever succeed once — subsequent calls are
    /// rejected to prevent re-initialization.
    pub fn initialize(env: &Env, admin: Address) -> Result<(), &'static str> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err("AlreadyInitialized: contract has already been initialized");
        }

        // Only the account being installed as admin can authorize its own
        // initialization.
        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::Config, &RagConfig { paused: false });

        env.events().publish((symbol_short!("init"),), admin);

        Ok(())
    }

    /// Retrieves the current administrator. Fails if not yet initialized.
    pub fn get_admin(env: &Env) -> Result<Address, &'static str> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or("NotInitialized: contract has not been initialized")
    }

    /// Retrieves the current configuration. Fails if not yet initialized.
    pub fn get_config(env: &Env) -> Result<RagConfig, &'static str> {
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or("NotInitialized: contract has not been initialized")
    }

    /// Convenience guard other managers can call to require that `caller`
    /// is the contract admin, e.g. for admin-gated operations.
    pub fn require_admin(env: &Env, caller: &Address) -> Result<(), &'static str> {
        let admin = Self::get_admin(env)?;
        if &admin != caller {
            return Err("Unauthorized: caller is not the contract admin");
        }
        Ok(())
    }
}

#[cfg(test)]
mod test;