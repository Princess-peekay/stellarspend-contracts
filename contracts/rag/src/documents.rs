// Issues #1134 & #1139 — Document Registration & Ownership Transfer
//
// Implements versioned document storage with source provenance tracking,
// metadata classification, ownership transfer, and event emission.

use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env, String, Vec};

// -----------------------------------------------------------------------
// Types
// -----------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceType {
    IpfsCid,
    GitCommit,
    HttpsResource,
    AppGeneratedId,
    ExternalContentId,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct SourceReference {
    pub source_type: SourceType,
    pub reference: String,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct DocumentMetadata {
    pub title: String,
    pub mime_type: String,
    pub version: String,
    pub language: String,
    pub source: SourceReference,
    pub collection_id: String,
    pub creation_ledger: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct DocumentVersion {
    pub version_id: u32,
    pub content_hash: BytesN<32>,
    pub previous_version_id: Option<u32>,
    pub creation_ledger: u32,
}

/// Full document record with ownership and version history.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Document {
    pub id: String,
    pub owner: Address,
    pub metadata: DocumentMetadata,
    pub active_version_id: u32,
    pub versions: Vec<DocumentVersion>,
}

#[derive(Clone)]
#[contracttype]
pub enum DocumentDataKey {
    Document(String),
}

// -----------------------------------------------------------------------
// DocumentManager
// -----------------------------------------------------------------------

pub struct DocumentManager;

impl DocumentManager {
    /// Registers a new document with an initial content hash and metadata.
    /// `owner.require_auth()` is enforced. No raw content is stored on-chain.
    pub fn register_document(
        env: &Env,
        id: String,
        initial_content_hash: BytesN<32>,
        metadata: DocumentMetadata,
        owner: Address,
    ) -> Result<(), &'static str> {
        owner.require_auth();

        if env
            .storage()
            .persistent()
            .has(&DocumentDataKey::Document(id.clone()))
        {
            return Err("DocumentAlreadyExists");
        }

        let initial_version = DocumentVersion {
            version_id: 1,
            content_hash: initial_content_hash,
            previous_version_id: None,
            creation_ledger: env.ledger().sequence(),
        };
        let mut versions = Vec::new(env);
        versions.push_back(initial_version);

        let doc = Document {
            id: id.clone(),
            owner,
            metadata,
            active_version_id: 1,
            versions,
        };
        env.storage()
            .persistent()
            .set(&DocumentDataKey::Document(id), &doc);
        Ok(())
    }

    /// Returns the current owner of the document.
    pub fn get_document_owner(env: &Env, id: String) -> Result<Address, &'static str> {
        let doc: Document = env
            .storage()
            .persistent()
            .get(&DocumentDataKey::Document(id))
            .ok_or("DocumentNotFound")?;
        Ok(doc.owner)
    }

    /// Transfers document ownership to `new_owner`.
    /// Requires the current owner's authorization.
    /// Emits `DocumentOwnershipTransferredEvent`.
    pub fn transfer_document_ownership(
        env: &Env,
        id: String,
        new_owner: Address,
        caller: Address,
    ) -> Result<(), &'static str> {
        caller.require_auth();

        let mut doc: Document = env
            .storage()
            .persistent()
            .get(&DocumentDataKey::Document(id.clone()))
            .ok_or("DocumentNotFound")?;

        if doc.owner != caller {
            return Err("Unauthorized: only the document owner can transfer ownership");
        }

        doc.owner = new_owner.clone();
        env.storage()
            .persistent()
            .set(&DocumentDataKey::Document(id.clone()), &doc);

        env.events().publish(
            (symbol_short!("doc_xfer"), id),
            new_owner,
        );
        Ok(())
    }

    /// Appends a new immutable content version while preserving history.
    pub fn append_version(
        env: &Env,
        id: String,
        new_content_hash: BytesN<32>,
        caller: Address,
    ) -> Result<u32, &'static str> {
        caller.require_auth();

        let mut doc: Document = env
            .storage()
            .persistent()
            .get(&DocumentDataKey::Document(id.clone()))
            .ok_or("DocumentNotFound")?;

        if doc.owner != caller {
            return Err("Unauthorized: only owner can append versions");
        }

        let next_version_id = doc.versions.len() + 1;
        let new_version = DocumentVersion {
            version_id: next_version_id,
            content_hash: new_content_hash,
            previous_version_id: Some(doc.active_version_id),
            creation_ledger: env.ledger().sequence(),
        };
        doc.versions.push_back(new_version);
        doc.active_version_id = next_version_id;
        env.storage()
            .persistent()
            .set(&DocumentDataKey::Document(id), &doc);
        Ok(next_version_id)
    }

    /// Updates mutable metadata; only the owner can update.
    pub fn update_metadata(
        env: &Env,
        id: String,
        new_metadata: DocumentMetadata,
        caller: Address,
    ) -> Result<(), &'static str> {
        caller.require_auth();
        let mut doc: Document = env
            .storage()
            .persistent()
            .get(&DocumentDataKey::Document(id.clone()))
            .ok_or("DocumentNotFound")?;
        if doc.owner != caller {
            return Err("Unauthorized: only the document owner can update metadata");
        }
        doc.metadata = new_metadata;
        env.storage()
            .persistent()
            .set(&DocumentDataKey::Document(id), &doc);
        Ok(())
    }

    /// Retrieves the full document record.
    pub fn get_document(env: &Env, id: String) -> Result<Document, &'static str> {
        env.storage()
            .persistent()
            .get(&DocumentDataKey::Document(id))
            .ok_or("DocumentNotFound")
    }
}
