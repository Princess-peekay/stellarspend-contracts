```rust
use soroban_sdk::{contracttype, Address, Env, String};

use crate::document::DataKey as DocumentDataKey;

/// Maximum length of chunk metadata fields.
///
/// These limits prevent arbitrarily large metadata from being persisted
/// alongside a chunk.
pub const MAX_PAGE_REFERENCE_LENGTH: u32 = 64;
pub const MAX_SECTION_LENGTH: u32 = 256;
pub const MAX_HEADING_LENGTH: u32 = 256;
pub const MAX_SOURCE_REFERENCE_LENGTH: u32 = 512;
pub const MAX_SIZE_COMMITMENT_LENGTH: u32 = 256;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChunkMetadata {
    /// Source document page containing this chunk.
    pub page: Option<String>,

    /// Section containing this chunk.
    pub section: Option<String>,

    /// Heading associated with this chunk.
    pub heading: Option<String>,

    /// Position of the chunk within the document.
    pub chunk_index: u32,

    /// Optional commitment to the token/size information for the chunk.
    pub size_commitment: Option<String>,

    /// Reference to the original source location.
    pub source_reference: Option<String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentChunk {
    /// Unique identifier scoped to the parent document.
    pub id: u32,

    /// ID of the document containing this chunk.
    pub document_id: String,

    /// Commitment to the actual chunk content.
    pub commitment: String,

    /// Chunk metadata.
    pub metadata: ChunkMetadata,
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
    pub fn register_chunk(
        env: &Env,
        document_id: String,
        chunk_id: u32,
        metadata: ChunkMetadata,
        commitment: String,
        caller: Address,
    ) -> Result<(), &'static str> {
        caller.require_auth();

        let document = env
            .storage()
            .persistent()
            .get(&DocumentDataKey::VersionedDoc(document_id.clone()))
            .ok_or("DocumentNotFound")?;

        if document.owner != caller {
            return Err(
                "Unauthorized: only the document owner can register document chunks",
            );
        }

        if document.revoked {
            return Err(
                "DocumentRevoked: cannot register chunks for a revoked document",
            );
        }

        if commitment.len() == 0 {
            return Err("InvalidCommitment: chunk commitment cannot be empty");
        }

        Self::validate_metadata(&metadata)?;

        let chunk_key = ChunkDataKey::Chunk(document_id.clone(), chunk_id);

        if env.storage().persistent().has(&chunk_key) {
            return Err("ChunkAlreadyExists");
        }

        let chunk = DocumentChunk {
            id: chunk_id,
            document_id,
            commitment,
            metadata,
        };

        env.storage().persistent().set(&chunk_key, &chunk);

        Ok(())
    }

    /// Updates metadata for an existing chunk.
    ///
    /// Only the owner of the parent document may update chunk metadata.
    pub fn update_chunk_metadata(
        env: &Env,
        document_id: String,
        chunk_id: u32,
        metadata: ChunkMetadata,
        caller: Address,
    ) -> Result<(), &'static str> {
        caller.require_auth();

        let document = env
            .storage()
            .persistent()
            .get(&DocumentDataKey::VersionedDoc(document_id.clone()))
            .ok_or("DocumentNotFound")?;

        if document.owner != caller {
            return Err(
                "Unauthorized: only the document owner can update chunk metadata",
            );
        }

        let chunk_key = ChunkDataKey::Chunk(document_id.clone(), chunk_id);

        let mut chunk: DocumentChunk = env
            .storage()
            .persistent()
            .get(&chunk_key)
            .ok_or("ChunkNotFound")?;

        Self::validate_metadata(&metadata)?;

        chunk.metadata = metadata;

        env.storage()
            .persistent()
            .set(&chunk_key, &chunk);

        Ok(())
    }

    /// Retrieves the metadata associated with a chunk.
    pub fn get_chunk_metadata(
        env: &Env,
        document_id: String,
        chunk_id: u32,
    ) -> Result<ChunkMetadata, &'static str> {
        let chunk: DocumentChunk = env
            .storage()
            .persistent()
            .get(&ChunkDataKey::Chunk(document_id, chunk_id))
            .ok_or("ChunkNotFound")?;

        Ok(chunk.metadata)
    }

    /// Retrieves a complete registered chunk.
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

    /// Checks whether a chunk exists for a document.
    pub fn chunk_exists(
        env: &Env,
        document_id: String,
        chunk_id: u32,
    ) -> bool {
        env.storage()
            .persistent()
            .has(&ChunkDataKey::Chunk(document_id, chunk_id))
    }

    /// Validates all bounded metadata fields before persistence.
    fn validate_metadata(metadata: &ChunkMetadata) -> Result<(), &'static str> {
        if let Some(page) = &metadata.page {
            if page.len() > MAX_PAGE_REFERENCE_LENGTH {
                return Err("MetadataTooLong: page reference exceeds maximum length");
            }
        }

        if let Some(section) = &metadata.section {
            if section.len() > MAX_SECTION_LENGTH {
                return Err("MetadataTooLong: section exceeds maximum length");
            }
        }

        if let Some(heading) = &metadata.heading {
            if heading.len() > MAX_HEADING_LENGTH {
                return Err("MetadataTooLong: heading exceeds maximum length");
            }
        }

        if let Some(size_commitment) = &metadata.size_commitment {
            if size_commitment.len() > MAX_SIZE_COMMITMENT_LENGTH {
                return Err(
                    "MetadataTooLong: size commitment exceeds maximum length",
                );
            }
        }

        if let Some(source_reference) = &metadata.source_reference {
            if source_reference.len() > MAX_SOURCE_REFERENCE_LENGTH {
                return Err(
                    "MetadataTooLong: source reference exceeds maximum length",
                );
            }
        }

        Ok(())
    }
}
