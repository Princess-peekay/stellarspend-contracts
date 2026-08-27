// Issue #1130 — Collection Access Control
//
// Implements both fine-grained per-document access policies (allowed_users list)
// and a resource-level access level enum (OwnerOnly / MembersOnly / Public).

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
    pub fn get_resource_access_level(env: &Env, resource_id: String) -> ResourceAccessLevel {
        env.storage()
            .persistent()
            .get(&AccessDataKey::Level(resource_id))
            .unwrap_or(ResourceAccessLevel::OwnerOnly)
    }
}
