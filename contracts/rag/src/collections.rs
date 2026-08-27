// Issue #1129 — Collection Membership
//
// Implements versioned knowledge collections with lifecycle management,
// metadata updates, and member address management.

use soroban_sdk::{contracttype, symbol_short, Address, Env, Map, String, Vec};

// -----------------------------------------------------------------------
// Types
// -----------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug)]
pub struct KnowledgeCollection {
    pub collection_id: String,
    pub owner: Address,
    pub name: String,
    pub description: String,
    pub current_version: u32,
    pub document_ids: Vec<String>,
    pub is_active: bool,
}

#[derive(Clone)]
#[contracttype]
pub enum CollectionDataKey {
    Collection(String),
    Members(String),
}

// -----------------------------------------------------------------------
// CollectionManager — lifecycle, versioning, metadata, membership
// -----------------------------------------------------------------------

pub struct CollectionManager;

impl CollectionManager {
    /// Creates a new active knowledge collection.
    pub fn create_collection(
        env: &Env,
        collection_id: String,
        name: String,
        description: String,
        owner: Address,
    ) -> Result<(), &'static str> {
        owner.require_auth();

        if env
            .storage()
            .persistent()
            .has(&CollectionDataKey::Collection(collection_id.clone()))
        {
            return Err("CollectionAlreadyExists");
        }

        let collection = KnowledgeCollection {
            collection_id: collection_id.clone(),
            owner,
            name,
            description,
            current_version: 1,
            document_ids: Vec::new(env),
            is_active: true,
        };
        env.storage()
            .persistent()
            .set(&CollectionDataKey::Collection(collection_id), &collection);
        Ok(())
    }

    /// Deactivates a collection — historical records stay queryable.
    pub fn deactivate_collection(
        env: &Env,
        collection_id: String,
        caller: Address,
    ) -> Result<(), &'static str> {
        caller.require_auth();
        let mut col: KnowledgeCollection = env
            .storage()
            .persistent()
            .get(&CollectionDataKey::Collection(collection_id.clone()))
            .ok_or("CollectionNotFound")?;
        if col.owner != caller {
            return Err("Unauthorized");
        }
        col.is_active = false;
        env.storage()
            .persistent()
            .set(&CollectionDataKey::Collection(collection_id), &col);
        Ok(())
    }

    /// Re-activates a previously deactivated collection.
    pub fn reactivate_collection(
        env: &Env,
        collection_id: String,
        caller: Address,
    ) -> Result<(), &'static str> {
        caller.require_auth();
        let mut col: KnowledgeCollection = env
            .storage()
            .persistent()
            .get(&CollectionDataKey::Collection(collection_id.clone()))
            .ok_or("CollectionNotFound")?;
        if col.owner != caller {
            return Err("Unauthorized");
        }
        col.is_active = true;
        env.storage()
            .persistent()
            .set(&CollectionDataKey::Collection(collection_id), &col);
        Ok(())
    }

    /// Updates mutable collection metadata (name and description).
    pub fn update_collection(
        env: &Env,
        collection_id: String,
        new_name: String,
        new_description: String,
        caller: Address,
    ) -> Result<(), &'static str> {
        caller.require_auth();
        let mut col: KnowledgeCollection = env
            .storage()
            .persistent()
            .get(&CollectionDataKey::Collection(collection_id.clone()))
            .ok_or("CollectionNotFound")?;
        if col.owner != caller {
            return Err("Unauthorized");
        }
        col.name = new_name;
        col.description = new_description;
        env.storage()
            .persistent()
            .set(&CollectionDataKey::Collection(collection_id.clone()), &col);
        env.events()
            .publish((symbol_short!("col_upd"), collection_id), caller);
        Ok(())
    }

    /// Adds a document to the collection and bumps the version.
    pub fn add_document_to_collection(
        env: &Env,
        collection_id: String,
        document_id: String,
        caller: Address,
    ) -> Result<u32, &'static str> {
        caller.require_auth();
        let mut col: KnowledgeCollection = env
            .storage()
            .persistent()
            .get(&CollectionDataKey::Collection(collection_id.clone()))
            .ok_or("CollectionNotFound")?;
        if !col.is_active {
            return Err("CollectionInactive");
        }
        if col.owner != caller {
            return Err("Unauthorized");
        }
        col.document_ids.push_back(document_id);
        col.current_version += 1;
        env.storage()
            .persistent()
            .set(&CollectionDataKey::Collection(collection_id), &col);
        Ok(col.current_version)
    }

    /// Retrieves the collection state.
    pub fn get_collection(
        env: &Env,
        collection_id: String,
    ) -> Result<KnowledgeCollection, &'static str> {
        env.storage()
            .persistent()
            .get(&CollectionDataKey::Collection(collection_id))
            .ok_or("CollectionNotFound")
    }

    // -------------------------------------------------------------------
    // Issue #1129 — Member management
    // -------------------------------------------------------------------

    /// Adds an address as a member of the collection.
    /// Panics if already a member to prevent duplicate entries.
    pub fn add_member(
        env: &Env,
        collection_id: String,
        member: Address,
        caller: Address,
    ) -> Result<(), &'static str> {
        caller.require_auth();
        let mut members: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&CollectionDataKey::Members(collection_id.clone()))
            .unwrap_or(Map::new(env));
        if members.contains_key(member.clone()) {
            return Err("MemberAlreadyExists");
        }
        members.set(member, true);
        env.storage()
            .persistent()
            .set(&CollectionDataKey::Members(collection_id), &members);
        Ok(())
    }

    /// Removes an existing member from the collection.
    pub fn remove_member(
        env: &Env,
        collection_id: String,
        member: Address,
        caller: Address,
    ) -> Result<(), &'static str> {
        caller.require_auth();
        let mut members: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&CollectionDataKey::Members(collection_id.clone()))
            .unwrap_or(Map::new(env));
        if members.contains_key(member.clone()) {
            members.remove(member);
            env.storage()
                .persistent()
                .set(&CollectionDataKey::Members(collection_id), &members);
        }
        Ok(())
    }

    /// Returns true if the address is a member of the collection.
    pub fn check_membership(env: &Env, collection_id: String, member: Address) -> bool {
        let members: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&CollectionDataKey::Members(collection_id))
            .unwrap_or(Map::new(env));
        members.contains_key(member)
    }
}
