// Issue #1130 — Collection Access Control
//
// Implements both fine-grained per-document access policies (allowed_users list)
// and a resource-level access level enum (OwnerOnly / MembersOnly / Public).
//
// Also implements trusted RAG embedding provider authorization.

use soroban_sdk::{contracttype, Address, Env, String, Vec};

// -----------------------------------------------------------------------
// Types
// -----------------------------------------------------------------------

/// Per-document access policy: stores the owner and an explicit allowlist.
#[contracttype]
#[derive(Clone, Debug)]
pub struct AccessPolicy {
    pub document_id: String,
    pub owner: Address,
    pub allowed_users: Vec<Address>,
}

/// Coarse-grained resource access level used by the RAG layer.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceAccessLevel {
    OwnerOnly,
    MembersOnly,
    Public,
}

#[derive(Clone)]
#[contracttype]
pub enum AccessDataKey {
    Policy(String),
    Level(String),
    Administrator,
    EmbeddingProvider(Address),
}

/// Errors returned by embedding provider authorization operations.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderAccessError {
    Unauthorized,
    ProviderAlreadyAuthorized,
    ProviderNotAuthorized,
    AdministratorNotConfigured,
}

// -----------------------------------------------------------------------
// AccessControlManager
// -----------------------------------------------------------------------

pub struct AccessControlManager;

impl AccessControlManager {
    /// Sets or updates the per-document access policy (owner-only).
    pub fn set_policy(
        env: &Env,
        document_id: String,
        allowed_users: Vec<Address>,
        caller: Address,
    ) -> Result<(), &'static str> {
        caller.require_auth();

        if let Some(existing) = env
            .storage()
            .persistent()
            .get::<_, AccessPolicy>(&AccessDataKey::Policy(document_id.clone()))
        {
            if existing.owner != caller {
                return Err("Unauthorized: only the document owner can modify access policy");
            }
        }

        let policy = AccessPolicy {
            document_id: document_id.clone(),
            owner: caller,
            allowed_users,
        };

        env.storage()
            .persistent()
            .set(&AccessDataKey::Policy(document_id), &policy);

        Ok(())
    }

    /// Returns Ok if the caller is the owner or is in the allowed_users list.
    pub fn verify_access(
        env: &Env,
        document_id: &String,
        caller: &Address,
    ) -> Result<(), &'static str> {
        let policy: AccessPolicy = env
            .storage()
            .persistent()
            .get(&AccessDataKey::Policy(document_id.clone()))
            .ok_or("AccessPolicyNotFound")?;

        if &policy.owner == caller {
            return Ok(());
        }

        for user in policy.allowed_users.iter() {
            if &user == caller {
                return Ok(());
            }
        }

        Err("AccessDenied")
    }

    /// Sets a coarse-grained access level for a resource (e.g. a collection).
    pub fn set_resource_access_level(
        env: &Env,
        resource_id: String,
        level: ResourceAccessLevel,
        caller: Address,
    ) {
        caller.require_auth();

        env.storage()
            .persistent()
            .set(&AccessDataKey::Level(resource_id), &level);
    }

    /// Gets the access level for a resource, defaulting to `OwnerOnly`.
    pub fn get_resource_access_level(
        env: &Env,
        resource_id: String,
    ) -> ResourceAccessLevel {
        env.storage()
            .persistent()
            .get(&AccessDataKey::Level(resource_id))
            .unwrap_or(ResourceAccessLevel::OwnerOnly)
    }

    // -------------------------------------------------------------------
    // Embedding Provider Authorization
    // -------------------------------------------------------------------

    /// Initializes the administrator responsible for managing
    /// trusted embedding providers.
    ///
    /// This can only be performed once.
    pub fn initialize_provider_admin(
        env: &Env,
        administrator: Address,
    ) -> Result<(), ProviderAccessError> {
        let key = AccessDataKey::Administrator;

        if env.storage().instance().has(&key) {
            return Err(ProviderAccessError::Unauthorized);
        }

        administrator.require_auth();

        env.storage()
            .instance()
            .set(&key, &administrator);

        Ok(())
    }

    /// Returns the administrator responsible for provider authorization.
    pub fn get_provider_admin(
        env: &Env,
    ) -> Result<Address, ProviderAccessError> {
        env.storage()
            .instance()
            .get(&AccessDataKey::Administrator)
            .ok_or(ProviderAccessError::AdministratorNotConfigured)
    }

    /// Authorizes a trusted embedding provider.
    ///
    /// Only the configured administrator can authorize providers.
    pub fn authorize_embedding_provider(
        env: &Env,
        administrator: Address,
        provider: Address,
    ) -> Result<(), ProviderAccessError> {
        Self::require_provider_admin(env, &administrator)?;

        let key = AccessDataKey::EmbeddingProvider(provider.clone());

        if env.storage().persistent().has(&key) {
            return Err(ProviderAccessError::ProviderAlreadyAuthorized);
        }

        env.storage().persistent().set(&key, &true);

        env.events().publish(
            (soroban_sdk::symbol_short!("provider"), provider),
            true,
        );

        Ok(())
    }

    /// Revokes an embedding provider.
    ///
    /// Only the configured administrator can revoke providers.
    pub fn revoke_embedding_provider(
        env: &Env,
        administrator: Address,
        provider: Address,
    ) -> Result<(), ProviderAccessError> {
        Self::require_provider_admin(env, &administrator)?;

        let key = AccessDataKey::EmbeddingProvider(provider.clone());

        if !env.storage().persistent().has(&key) {
            return Err(ProviderAccessError::ProviderNotAuthorized);
        }

        env.storage().persistent().remove(&key);

        env.events().publish(
            (soroban_sdk::symbol_short!("provider"), provider),
            false,
        );

        Ok(())
    }

    /// Returns true when the provider is currently authorized.
    pub fn is_authorized_embedding_provider(
        env: &Env,
        provider: Address,
    ) -> bool {
        env.storage()
            .persistent()
            .get::<_, bool>(
                &AccessDataKey::EmbeddingProvider(provider),
            )
            .unwrap_or(false)
    }

    /// Ensures that a provider is authorized before it can
    /// submit an embedding commitment.
    ///
    /// The provider must also authenticate the operation.
    pub fn require_authorized_embedding_provider(
        env: &Env,
        provider: &Address,
    ) -> Result<(), ProviderAccessError> {
        if !Self::is_authorized_embedding_provider(
            env,
            provider.clone(),
        ) {
            return Err(ProviderAccessError::Unauthorized);
        }

        provider.require_auth();

        Ok(())
    }

    /// Ensures the caller is the configured provider administrator.
    fn require_provider_admin(
        env: &Env,
        caller: &Address,
    ) -> Result<(), ProviderAccessError> {
        let administrator = Self::get_provider_admin(env)?;

        if &administrator != caller {
            return Err(ProviderAccessError::Unauthorized);
        }

        caller.require_auth();

        Ok(())
    }
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn provider_can_be_authorized() {
        let env = Env::default();

        let administrator = Address::generate(&env);
        let provider = Address::generate(&env);

        AccessControlManager::initialize_provider_admin(
            &env,
            administrator.clone(),
        )
        .unwrap();

        AccessControlManager::authorize_embedding_provider(
            &env,
            administrator,
            provider.clone(),
        )
        .unwrap();

        assert!(
            AccessControlManager::is_authorized_embedding_provider(
                &env,
                provider
            )
        );
    }

    #[test]
    fn provider_can_be_revoked() {
        let env = Env::default();

        let administrator = Address::generate(&env);
        let provider = Address::generate(&env);

        AccessControlManager::initialize_provider_admin(
            &env,
            administrator.clone(),
        )
        .unwrap();

        AccessControlManager::authorize_embedding_provider(
            &env,
            administrator.clone(),
            provider.clone(),
        )
        .unwrap();

        assert!(
            AccessControlManager::is_authorized_embedding_provider(
                &env,
                provider.clone()
            )
        );

        AccessControlManager::revoke_embedding_provider(
            &env,
            administrator,
            provider.clone(),
        )
        .unwrap();

        assert!(
            !AccessControlManager::is_authorized_embedding_provider(
                &env,
                provider
            )
        );
    }

    #[test]
    fn unauthorized_provider_cannot_submit_commitment() {
        let env = Env::default();

        let provider = Address::generate(&env);

        let result =
            AccessControlManager::require_authorized_embedding_provider(
                &env,
                &provider,
            );

        assert_eq!(
            result,
            Err(ProviderAccessError::Unauthorized)
        );
    }

    #[test]
    fn authorized_provider_can_submit_commitment() {
        let env = Env::default();

        let administrator = Address::generate(&env);
        let provider = Address::generate(&env);

        AccessControlManager::initialize_provider_admin(
            &env,
            administrator.clone(),
        )
        .unwrap();

        AccessControlManager::authorize_embedding_provider(
            &env,
            administrator,
            provider.clone(),
        )
        .unwrap();

        let result =
            AccessControlManager::require_authorized_embedding_provider(
                &env,
                &provider,
            );

        assert!(result.is_ok());
    }

    #[test]
    fn revoked_provider_cannot_submit_commitment() {
        let env = Env::default();

        let administrator = Address::generate(&env);
        let provider = Address::generate(&env);

        AccessControlManager::initialize_provider_admin(
            &env,
            administrator.clone(),
        )
        .unwrap();

        AccessControlManager::authorize_embedding_provider(
            &env,
            administrator.clone(),
            provider.clone(),
        )
        .unwrap();

        AccessControlManager::revoke_embedding_provider(
            &env,
            administrator,
            provider.clone(),
        )
        .unwrap();

        let result =
            AccessControlManager::require_authorized_embedding_provider(
                &env,
                &provider,
            );

        assert_eq!(
            result,
            Err(ProviderAccessError::Unauthorized)
        );
    }

    #[test]
    fn only_administrator_can_authorize_provider() {
        let env = Env::default();

        let administrator = Address::generate(&env);
        let attacker = Address::generate(&env);
        let provider = Address::generate(&env);

        AccessControlManager::initialize_provider_admin(
            &env,
            administrator,
        )
        .unwrap();

        let result =
            AccessControlManager::authorize_embedding_provider(
                &env,
                attacker,
                provider.clone(),
            );

        assert_eq!(
            result,
            Err(ProviderAccessError::Unauthorized)
        );

        assert!(
            !AccessControlManager::is_authorized_embedding_provider(
                &env,
                provider
            )
        );
    }

    #[test]
    fn only_administrator_can_revoke_provider() {
        let env = Env::default();

        let administrator = Address::generate(&env);
        let attacker = Address::generate(&env);
        let provider = Address::generate(&env);

        AccessControlManager::initialize_provider_admin(
            &env,
            administrator.clone(),
        )
        .unwrap();

        AccessControlManager::authorize_embedding_provider(
            &env,
            administrator,
            provider.clone(),
        )
        .unwrap();

        let result =
            AccessControlManager::revoke_embedding_provider(
                &env,
                attacker,
                provider.clone(),
            );

        assert_eq!(
            result,
            Err(ProviderAccessError::Unauthorized)
        );

        assert!(
            AccessControlManager::is_authorized_embedding_provider(
                &env,
                provider
            )
        );
    }

    #[test]
    fn duplicate_provider_authorization_fails() {
        let env = Env::default();

        let administrator = Address::generate(&env);
        let provider = Address::generate(&env);

        AccessControlManager::initialize_provider_admin(
            &env,
            administrator.clone(),
        )
        .unwrap();

        AccessControlManager::authorize_embedding_provider(
            &env,
            administrator.clone(),
            provider.clone(),
        )
        .unwrap();

        let result =
            AccessControlManager::authorize_embedding_provider(
                &env,
                administrator,
                provider,
            );

        assert_eq!(
            result,
            Err(ProviderAccessError::ProviderAlreadyAuthorized)
        );
    }

    #[test]
    fn revoking_unknown_provider_fails() {
        let env = Env::default();

        let administrator = Address::generate(&env);
        let provider = Address::generate(&env);

        AccessControlManager::initialize_provider_admin(
            &env,
            administrator.clone(),
        )
        .unwrap();

        let result =
            AccessControlManager::revoke_embedding_provider(
                &env,
                administrator,
                provider,
            );

        assert_eq!(
            result,
            Err(ProviderAccessError::ProviderNotAuthorized)
        );
    }

    #[test]
    fn provider_admin_can_be_initialized_only_once() {
        let env = Env::default();

        let administrator = Address::generate(&env);
        let second_administrator = Address::generate(&env);

        AccessControlManager::initialize_provider_admin(
            &env,
            administrator,
        )
        .unwrap();

        let result =
            AccessControlManager::initialize_provider_admin(
                &env,
                second_administrator,
            );

        assert_eq!(
            result,
            Err(ProviderAccessError::Unauthorized)
        );
    }
}