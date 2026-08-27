use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentVersion {
    pub version_id: u32,
    pub content_hash: String,
    pub creation_ledger: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct VersionedDocument {
    pub id: String,
    pub owner: Address,
    pub active_version_id: u32,
    pub versions: Vec<DocumentVersion>,
    pub revoked: bool,
    pub revoked_ledger: Option<u32>,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    VersionedDoc(String),
}

pub struct DocumentCommitmentManager;

impl DocumentCommitmentManager {
    /// Registers a document with a mandatory, non-empty content hash commitment.
    pub fn register_document(
        env: &Env,
        id: String,
        content_hash: String,
        owner: Address,
    ) -> Result<(), &'static str> {
        owner.require_auth();

        if content_hash.len() == 0 {
            return Err("InvalidHash: document content hash cannot be empty");
        }

        if env.storage().persistent().has(&DataKey::VersionedDoc(id.clone())) {
            return Err("DocumentAlreadyExists");
        }

        let initial_version = DocumentVersion {
            version_id: 1,
            content_hash,
            creation_ledger: env.ledger().sequence(),
        };

        let mut versions = Vec::new(env);
        versions.push_back(initial_version);

        let document = VersionedDocument {
            id: id.clone(),
            owner,
            active_version_id: 1,
            versions,
            revoked: false,
            revoked_ledger: None,
        };

        env.storage().persistent().set(&DataKey::VersionedDoc(id), &document);
        Ok(())
    }

    /// Appends a new version with a new hash commitment, leaving existing versions immutable.
    pub fn commit_new_version(
        env: &Env,
        id: String,
        new_content_hash: String,
        caller: Address,
    ) -> Result<u32, &'static str> {
        caller.require_auth();

        if new_content_hash.len() == 0 {
            return Err("InvalidHash: new content hash cannot be empty");
        }

        let mut document: VersionedDocument = env.storage()
            .persistent()
            .get(&DataKey::VersionedDoc(id.clone()))
            .ok_or("DocumentNotFound")?;

        if document.owner != caller {
            return Err("Unauthorized: only the document owner can commit a new version");
        }

        if document.revoked {
            return Err("DocumentRevoked: cannot commit new versions to a revoked document");
        }

        let next_version_id = document.versions.len() + 1;
        let new_version = DocumentVersion {
            version_id: next_version_id,
            content_hash: new_content_hash,
            creation_ledger: env.ledger().sequence(),
        };

        document.versions.push_back(new_version);
        document.active_version_id = next_version_id;

        env.storage().persistent().set(&DataKey::VersionedDoc(id), &document);
        Ok(next_version_id)
    }

    /// Revokes a document, permanently disqualifying it from active RAG use.
    /// Historical version data is untouched and remains queryable via
    /// `get_hash_for_version`. Only the document's owner may revoke it.
    pub fn revoke_document(
        env: &Env,
        id: String,
        caller: Address,
    ) -> Result<(), &'static str> {
        caller.require_auth();

        let mut document: VersionedDocument = env.storage()
            .persistent()
            .get(&DataKey::VersionedDoc(id.clone()))
            .ok_or("DocumentNotFound")?;

        if document.owner != caller {
            return Err("Unauthorized: only the document owner can revoke the document");
        }

        if document.revoked {
            return Err("AlreadyRevoked");
        }

        document.revoked = true;
        document.revoked_ledger = Some(env.ledger().sequence());

        env.storage().persistent().set(&DataKey::VersionedDoc(id.clone()), &document);

        // Emit revocation event: topics = (symbol, doc_id), data = owner
        env.events().publish(
            (symbol_short!("revoke"), id),
            document.owner.clone(),
        );

        Ok(())
    }

    /// Returns the content hash of the currently active version, but ONLY
    /// if the document has not been revoked. Use this for new retrieval
    /// requests — revoked documents must never be surfaced here.
    pub fn get_active_hash_for_retrieval(
        env: &Env,
        id: String,
    ) -> Result<String, &'static str> {
        let document: VersionedDocument = env.storage()
            .persistent()
            .get(&DataKey::VersionedDoc(id))
            .ok_or("DocumentNotFound")?;

        if document.revoked {
            return Err("DocumentRevoked: document is not available for retrieval");
        }

        for v in document.versions.iter() {
            if v.version_id == document.active_version_id {
                return Ok(v.content_hash);
            }
        }

        Err("VersionNotFound")
    }

    /// Retrieves the content hash for a specific version to perform integrity
    /// verification / historical provenance lookups. Intentionally does NOT
    /// check revocation status — revoked documents must remain auditable.
    pub fn get_hash_for_version(
        env: &Env,
        id: String,
        version_id: u32,
    ) -> Result<String, &'static str> {
        let document: VersionedDocument = env.storage()
            .persistent()
            .get(&DataKey::VersionedDoc(id))
            .ok_or("DocumentNotFound")?;

        for v in document.versions.iter() {
            if v.version_id == version_id {
                return Ok(v.content_hash);
            }
        }

        Err("VersionNotFound")
    }

    /// Convenience read-only check for revocation status.
    pub fn is_revoked(env: &Env, id: String) -> Result<bool, &'static str> {
        let document: VersionedDocument = env.storage()
            .persistent()
            .get(&DataKey::VersionedDoc(id))
            .ok_or("DocumentNotFound")?;

        Ok(document.revoked)
    }
}