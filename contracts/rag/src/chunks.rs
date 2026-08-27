use soroban_sdk::{contracttype, Address, Env, String, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document {
    pub id: u64,
    pub owner: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Chunk {
    pub id: u64,
    pub document_id: u64,
    pub version: u32,
    pub content: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OwnershipError {
    Unauthorized,
    DocumentNotFound,
}

pub struct ChunkOwnershipManager;

impl ChunkOwnershipManager {
    /// Validates that the caller is either the document owner or an authorized administrator,
    /// requiring cryptographic authorization from the caller.
    pub fn verify_chunk_operation(
        env: &Env,
        caller: &Address,
        document: &Document,
        authorized_admins: &Vec<Address>,
    ) -> Result<(), OwnershipError> {
        // Enforce Soroban host authentication for the caller
        caller.require_auth();

        let is_owner = caller == &document.owner;
        let is_admin = authorized_admins.contains(caller.clone());

        if !is_owner && !is_admin {
            return Err(OwnershipError::Unauthorized);
        }

        Ok(())
    }
}