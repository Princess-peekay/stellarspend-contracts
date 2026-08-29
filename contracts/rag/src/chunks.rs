```rust
use soroban_sdk::{contracttype, Address, Env, String};

use crate::document::DataKey as DocumentDataKey;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentChunk {
    pub id: u32,
    pub document_id: String,
    pub commitment: String,
    pub index: u32,
    pub metadata: String,
}

#[derive(Clone)]
#[contracttype]
pub enum ChunkDataKey {
    Chunk(String, u32),
}

pub struct ChunkRegistrationManager;

impl ChunkRegistrationManager {
    /// Registers a document chunk using a commitment rather than storing
    /// the full chunk content on-chain.
    ///
    /// Chunk IDs are scoped to their document, meaning the same chunk ID
    /// may be used by different documents but cannot be registered twice
    /// for the same document.
    pub fn register_chunk(
        env: &Env,
        document_id: String,
        chunk_id: u32,
        index: u32,
        commitment: String,
        metadata: String,
        caller: Address,
    ) -> Result<(), &'static str> {
        caller.require_auth();

        // A chunk can only belong to an existing document.
        let document = env
            .storage()
            .persistent()
            .get(&DocumentDataKey::VersionedDoc(document_id.clone()))
            .ok_or("DocumentNotFound")?;

        // Only the document owner may register chunks for the document.
        if document.owner != caller {
            return Err(
                "Unauthorized: only the document owner can register document chunks",
            );
        }

        // Do not allow chunks to be registered against a revoked document.
        if document.revoked {
            return Err(
                "DocumentRevoked: cannot register chunks for a revoked document",
            );
        }

        // A commitment is required because the actual chunk content is
        // intentionally not stored on-chain.
        if commitment.len() == 0 {
            return Err("InvalidCommitment: chunk commitment cannot be empty");
        }

        let chunk_key = ChunkDataKey::Chunk(document_id.clone(), chunk_id);

        // Chunk IDs are unique within a document.
        if env.storage().persistent().has(&chunk_key) {
            return Err("ChunkAlreadyExists");
        }

        let chunk = DocumentChunk {
            id: chunk_id,
            document_id,
            commitment,
            index,
            metadata,
        };

        env.storage().persistent().set(&chunk_key, &chunk);

        Ok(())
    }

    /// Retrieves a registered chunk by its document-scoped chunk ID.
    pub fn get_chunk(
        env: &Env,
        document_id: String,
        chunk_id: u32,
    ) -> Result<DocumentChunk, &'static str> {
        env.storage()
            .persistent()
            .get(&ChunkDataKey::Chunk(document_id, chunk_id))
            .ok_or("ChunkNotFound")
    }

    /// Checks whether a chunk has already been registered for a document.
    pub fn chunk_exists(
        env: &Env,
        document_id: String,
        chunk_id: u32,
    ) -> bool {
        env.storage()
            .persistent()
            .has(&ChunkDataKey::Chunk(document_id, chunk_id))
    }
}
```
