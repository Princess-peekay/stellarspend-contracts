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
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn test_initialize_sets_admin_and_default_config() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);

        let res = AdminManager::initialize(&env, admin.clone());
        assert!(res.is_ok());

        assert_eq!(AdminManager::get_admin(&env).unwrap(), admin);
        assert_eq!(AdminManager::get_config(&env).unwrap().paused, false);
    }

    #[test]
    fn test_initialize_emits_event() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);

        AdminManager::initialize(&env, admin).unwrap();

        assert_eq!(env.events().all().len(), 1);
    }

    #[test]
    fn test_reinitialize_fails_and_admin_unchanged() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let other = Address::generate(&env);

        AdminManager::initialize(&env, admin.clone()).unwrap();

        let res = AdminManager::initialize(&env, other);
        assert!(res.is_err());
        assert_eq!(AdminManager::get_admin(&env).unwrap(), admin);
    }

    #[test]
    #[should_panic]
    fn test_initialize_without_auth_fails() {
        let env = Env::default();
        // Deliberately no env.mock_all_auths() — admin has not authorized
        // this call, so require_auth() must panic.
        let admin = Address::generate(&env);

        let _ = AdminManager::initialize(&env, admin);
    }

    #[test]
    fn test_get_admin_before_init_fails() {
        let env = Env::default();
        let res = AdminManager::get_admin(&env);
        assert!(res.is_err());
    }

    #[test]
    fn test_require_admin() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let stranger = Address::generate(&env);

        AdminManager::initialize(&env, admin.clone()).unwrap();

        assert!(AdminManager::require_admin(&env, &admin).is_ok());
        assert!(AdminManager::require_admin(&env, &stranger).is_err());
    }
}