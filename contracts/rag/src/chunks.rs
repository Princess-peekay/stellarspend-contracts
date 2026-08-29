```rust
use soroban_sdk::{contracttype, Address, Env, String};

use crate::document::DataKey as DocumentDataKey;

/// Maximum length of a chunk commitment.
///
/// This accommodates common hexadecimal and encoded hash representations
/// while preventing unbounded storage.
pub const MAX_CHUNK_COMMITMENT_LENGTH: u32 = 256;

/// Maximum length of chunk metadata fields.
pub const MAX_PAGE_REFERENCE_LENGTH: u32 = 64;
pub const MAX_SECTION_LENGTH: u32 = 256;
pub const MAX_HEADING_LENGTH: u32 = 256;
pub const MAX_SOURCE_REFERENCE_LENGTH: u32 = 512;
pub const MAX_SIZE_COMMITMENT_LENGTH: u32 = 256;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChunkMetadata {
    pub page: Option<String>,
    pub section: Option<String>,
    pub heading: Option<String>,
    pub chunk_index: u32,
    pub size_commitment: Option<String>,
    pub source_reference: Option<String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentChunk {
    /// Chunk identifier scoped to the document version.
    pub id: u32,

    /// Parent document identifier.
    pub document_id: String,

    /// Document version this chunk belongs to.
    pub version_id: u32,

    /// Cryptographic commitment to the chunk content.
    ///
    /// This field is immutable after registration.
    pub commitment: String,

    /// Bounded descriptive metadata.
    pub metadata: ChunkMetadata,
}

#[derive(Clone)]
#[contracttype]
pub enum ChunkDataKey {
    Chunk(String, u32, u32),
}

pub struct ChunkRegistrationManager;

impl ChunkRegistrationManager {
    /// Registers a chunk for a specific document version.
    ///
    /// The chunk content itself is never stored on-chain. Only its
    /// cryptographic commitment is persisted.
    pub fn register_chunk(
        env: &Env,
        document_id: String,
        version_id: u32,
        chunk_id: u32,
        commitment: String,
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
                "Unauthorized: only the document owner can register document chunks",
            );
        }

        if document.revoked {
            return Err(
                "DocumentRevoked: cannot register chunks for a revoked document",
            );
        }

        // The version must belong to the document.
        let version_exists = document
            .versions
            .iter()
            .any(|version| version.version_id == version_id);

        if !version_exists {
            return Err("VersionNotFound");
        }

        Self::validate_commitment(&commitment)?;
        Self::validate_metadata(&metadata)?;

        let chunk_key =
            ChunkDataKey::Chunk(document_id.clone(), version_id, chunk_id);

        // A chunk can only be registered once for a particular
        // document version.
        if env.storage().persistent().has(&chunk_key) {
            return Err("ChunkAlreadyExists");
        }

        let chunk = DocumentChunk {
            id: chunk_id,
            document_id,
            version_id,
            commitment,
            metadata,
        };

        env.storage().persistent().set(&chunk_key, &chunk);

        Ok(())
    }

    /// Returns the immutable cryptographic commitment for a chunk.
    pub fn get_chunk_commitment(
        env: &Env,
        document_id: String,
        version_id: u32,
        chunk_id: u32,
    ) -> Result<String, &'static str> {
        let chunk: DocumentChunk = env
            .storage()
            .persistent()
            .get(&ChunkDataKey::Chunk(
                document_id,
                version_id,
                chunk_id,
            ))
            .ok_or("ChunkNotFound")?;

        Ok(chunk.commitment)
    }

    /// Retrieves a complete chunk registration.
    pub fn get_chunk(
        env: &Env,
        document_id: String,
        version_id: u32,
        chunk_id: u32,
    ) -> Result<DocumentChunk, &'static str> {
        env.storage()
            .persistent()
            .get(&ChunkDataKey::Chunk(
                document_id,
                version_id,
                chunk_id,
            ))
            .ok_or("ChunkNotFound")
    }

    /// Checks whether a chunk has already been registered for a
    /// particular document version.
    pub fn chunk_exists(
        env: &Env,
        document_id: String,
        version_id: u32,
        chunk_id: u32,
    ) -> bool {
        env.storage()
            .persistent()
            .has(&ChunkDataKey::Chunk(
                document_id,
                version_id,
                chunk_id,
            ))
    }

    /// Validates a chunk commitment before it is persisted.
    fn validate_commitment(
        commitment: &String,
    ) -> Result<(), &'static str> {
        if commitment.len() == 0 {
            return Err(
                "InvalidHash: chunk hash commitment cannot be empty",
            );
        }

        if commitment.len() > MAX_CHUNK_COMMITMENT_LENGTH {
            return Err(
                "InvalidHash: chunk hash commitment exceeds maximum length",
            );
        }

        Ok(())
    }

    /// Validates bounded chunk metadata.
    fn validate_metadata(
        metadata: &ChunkMetadata,
    ) -> Result<(), &'static str> {
        if let Some(page) = &metadata.page {
            if page.len() > MAX_PAGE_REFERENCE_LENGTH {
                return Err(
                    "MetadataTooLong: page reference exceeds maximum length",
                );
            }
        }

        if let Some(section) = &metadata.section {
            if section.len() > MAX_SECTION_LENGTH {
                return Err(
                    "MetadataTooLong: section exceeds maximum length",
                );
            }
        }

        if let Some(heading) = &metadata.heading {
            if heading.len() > MAX_HEADING_LENGTH {
                return Err(
                    "MetadataTooLong: heading exceeds maximum length",
                );
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
```
