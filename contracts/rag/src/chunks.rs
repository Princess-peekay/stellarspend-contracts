use soroban_sdk::{contracttype, Env, String, Symbol};

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
pub enum ChunkError {
    VersionMismatch,
    InvalidChunkId,
}

pub struct ChunkManager;

impl ChunkManager {
    /// Associates or updates a chunk ensuring it references the correct document version.
    /// Rejects modifications if the target version does not match the expected active version.
    pub fn upsert_chunk(
        env: &Env,
        existing_chunk: Option<Chunk>,
        new_id: u64,
        document_id: u64,
        target_version: u32,
        content: String,
    ) -> Result<Chunk, ChunkError> {
        if let Some(ref chunk) = existing_chunk {
            // Ensure old chunks remain immutable; reject mismatched document versions
            if chunk.version != target_version || chunk.document_id != document_id {
                return Err(ChunkError::VersionMismatch);
            }
        }

        Ok(Chunk {
            id: new_id,
            document_id,
            version: target_version,
            content,
        })
    }
}