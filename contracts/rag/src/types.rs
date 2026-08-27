//! Core data structures for the RAG contract.
//!
//! Design notes (see acceptance criteria):
//! - All types derive `#[contracttype]`, so they are Soroban-compatible and
//!   can be stored/retrieved directly via `env.storage()` and passed across
//!   contract-fn boundaries.
//! - Variable-length fields (`String`, `Vec<..>`) are bounded by explicit
//!   constants, validated in each type's constructor rather than enforced
//!   at the type level (Soroban has no const-generic length bound on
//!   `String`/`Vec`).
//! - Identifiers are content-addressed: they are derived deterministically
//!   by SHA-256 hashing the canonical (XDR) encoding of a type's identity
//!   fields, so the same logical input always yields the same id, and ids
//!   never depend on caller-supplied nonces or ledger state.

#![allow(dead_code)]

use soroban_sdk::{contracterror, contracttype, xdr::ToXdr, Address, Bytes, BytesN, Env, String, Vec};

// ---------------------------------------------------------------------
// Bounds
// ---------------------------------------------------------------------

pub const MAX_NAME_LEN: u32 = 128;
pub const MAX_DESCRIPTION_LEN: u32 = 512;
pub const MAX_LOCATOR_LEN: u32 = 256;
pub const MAX_MODEL_ID_LEN: u32 = 64;
pub const MAX_COLLECTION_IDS_PER_QUERY: u32 = 16;
pub const MAX_TOP_K: u32 = 100;
pub const MAX_SCORE_BPS: u32 = 10_000;

// ---------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum TypesError {
    NameTooLong = 1,
    DescriptionTooLong = 2,
    LocatorTooLong = 3,
    ModelIdTooLong = 4,
    TooManyCollectionIds = 5,
    TopKOutOfRange = 6,
    ScoreOutOfRange = 7,
    InvalidByteRange = 8,
}

pub type RagResult<T> = Result<T, TypesError>;

// ---------------------------------------------------------------------
// Identifier helpers
// ---------------------------------------------------------------------

/// Deterministically derive a 32-byte id by hashing a set of concatenated
/// canonical byte slices. Same inputs, in the same order, always produce
/// the same id.
fn derive_id(env: &Env, parts: &[Bytes]) -> BytesN<32> {
    let mut buf = Bytes::new(env);
    for part in parts.iter() {
        buf.append(part);
    }
    env.crypto().sha256(&buf)
}

/// Canonical byte encoding of a `BytesN<32>` for use as `derive_id` input.
fn bytes32(env: &Env, v: &BytesN<32>) -> Bytes {
    Bytes::from_array(env, &v.to_array())
}

/// Canonical big-endian byte encoding of a `u32` for use as `derive_id` input.
fn u32_bytes(env: &Env, v: u32) -> Bytes {
    Bytes::from_array(env, &v.to_be_bytes())
}

/// Canonical big-endian byte encoding of a `u64` for use as `derive_id` input.
fn u64_bytes(env: &Env, v: u64) -> Bytes {
    Bytes::from_array(env, &v.to_be_bytes())
}

// ---------------------------------------------------------------------
// SourceReference
// ---------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceKind {
    Url,
    Ipfs,
    Storage,
    Inline,
}

/// Pointer to where a `Document`'s raw content actually lives, plus a
/// checksum so tampering with the off-chain payload can be detected.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceReference {
    pub kind: SourceKind,
    pub locator: String,
    pub checksum: BytesN<32>,
}

impl SourceReference {
    pub fn new(kind: SourceKind, locator: String, checksum: BytesN<32>) -> RagResult<Self> {
        if locator.len() > MAX_LOCATOR_LEN {
            return Err(TypesError::LocatorTooLong);
        }
        Ok(Self { kind, locator, checksum })
    }
}

// ---------------------------------------------------------------------
// Collection
// ---------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Collection {
    pub id: BytesN<32>,
    pub owner: Address,
    pub name: String,
    pub description: String,
    pub created_at: u64,
    pub document_count: u32,
    pub current_version: Option<BytesN<32>>,
}

impl Collection {
    /// `id` is derived from `owner` + `name`, so a given owner cannot
    /// silently create two distinct collections that collide under the
    /// same name, and re-deriving the id later (e.g. for lookups) does
    /// not require reading anything from storage first.
    pub fn new(env: &Env, owner: Address, name: String, description: String, created_at: u64) -> RagResult<Self> {
        if name.len() > MAX_NAME_LEN {
            return Err(TypesError::NameTooLong);
        }
        if description.len() > MAX_DESCRIPTION_LEN {
            return Err(TypesError::DescriptionTooLong);
        }

        let id = derive_id(env, &[owner.to_xdr(env), name.to_xdr(env)]);

        Ok(Self {
            id,
            owner,
            name,
            description,
            created_at,
            document_count: 0,
            current_version: None,
        })
    }
}

// ---------------------------------------------------------------------
// Document
// ---------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentStatus {
    Pending,
    Indexed,
    Archived,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document {
    pub id: BytesN<32>,
    pub collection_id: BytesN<32>,
    pub source: SourceReference,
    pub content_hash: BytesN<32>,
    pub chunk_count: u32,
    pub status: DocumentStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

impl Document {
    /// `id` is derived from `collection_id` + `content_hash`, so
    /// re-ingesting byte-identical content into the same collection
    /// resolves to the same document instead of creating a duplicate.
    pub fn new(
        env: &Env,
        collection_id: BytesN<32>,
        source: SourceReference,
        content_hash: BytesN<32>,
        created_at: u64,
    ) -> Self {
        let id = derive_id(env, &[bytes32(env, &collection_id), bytes32(env, &content_hash)]);

        Self {
            id,
            collection_id,
            source,
            content_hash,
            chunk_count: 0,
            status: DocumentStatus::Pending,
            created_at,
            updated_at: created_at,
        }
    }
}

// ---------------------------------------------------------------------
// DocumentChunk
// ---------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ByteRange {
    pub start: u32,
    pub end: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentChunk {
    pub id: BytesN<32>,
    pub document_id: BytesN<32>,
    pub chunk_index: u32,
    pub content_hash: BytesN<32>,
    pub byte_range: ByteRange,
    pub embedding_commitment: Option<BytesN<32>>,
}

impl DocumentChunk {
    /// `id` is derived from `document_id` + `chunk_index`, so chunk
    /// identity is stable across re-runs of the chunking process as long
    /// as the chunking parameters (and therefore the index) don't change.
    pub fn new(
        env: &Env,
        document_id: BytesN<32>,
        chunk_index: u32,
        content_hash: BytesN<32>,
        byte_range: ByteRange,
    ) -> RagResult<Self> {
        if byte_range.end <= byte_range.start {
            return Err(TypesError::InvalidByteRange);
        }

        let id = derive_id(
            env,
            &[bytes32(env, &document_id), u32_bytes(env, chunk_index)],
        );

        Ok(Self {
            id,
            document_id,
            chunk_index,
            content_hash,
            byte_range,
            embedding_commitment: None,
        })
    }
}

// ---------------------------------------------------------------------
// EmbeddingCommitment
// ---------------------------------------------------------------------

/// An on-chain commitment to an off-chain embedding vector: the contract
/// never stores the vector itself, only a hash of it, plus enough
/// metadata (model + dimensionality) to make the commitment meaningful.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddingCommitment {
    pub id: BytesN<32>,
    pub chunk_id: BytesN<32>,
    pub model_id: String,
    pub dims: u32,
    pub vector_hash: BytesN<32>,
    pub created_at: u64,
}

impl EmbeddingCommitment {
    /// `id` is derived from `chunk_id` + `model_id`, so re-embedding a
    /// chunk with the same model resolves to the same commitment slot,
    /// while embedding it with a different model produces a distinct one.
    pub fn new(
        env: &Env,
        chunk_id: BytesN<32>,
        model_id: String,
        dims: u32,
        vector_hash: BytesN<32>,
        created_at: u64,
    ) -> RagResult<Self> {
        if model_id.len() > MAX_MODEL_ID_LEN {
            return Err(TypesError::ModelIdTooLong);
        }

        let id = derive_id(env, &[bytes32(env, &chunk_id), model_id.to_xdr(env)]);

        Ok(Self {
            id,
            chunk_id,
            model_id,
            dims,
            vector_hash,
            created_at,
        })
    }
}

// ---------------------------------------------------------------------
// RetrievalQuery / RetrievalResult
// ---------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetrievalQuery {
    pub id: BytesN<32>,
    pub requester: Address,
    pub query_hash: BytesN<32>,
    pub collection_ids: Vec<BytesN<32>>,
    pub top_k: u32,
    pub created_at: u64,
}

impl RetrievalQuery {
    /// `id` is derived from `requester` + `query_hash` + `created_at`.
    /// Unlike content-addressed types above, the same requester may
    /// legitimately repeat the same query text at different times, so the
    /// timestamp is folded in to keep each submitted query distinguishable.
    pub fn new(
        env: &Env,
        requester: Address,
        query_hash: BytesN<32>,
        collection_ids: Vec<BytesN<32>>,
        top_k: u32,
        created_at: u64,
    ) -> RagResult<Self> {
        if collection_ids.len() > MAX_COLLECTION_IDS_PER_QUERY {
            return Err(TypesError::TooManyCollectionIds);
        }
        if top_k == 0 || top_k > MAX_TOP_K {
            return Err(TypesError::TopKOutOfRange);
        }

        let id = derive_id(
            env,
            &[
                requester.to_xdr(env),
                bytes32(env, &query_hash),
                u64_bytes(env, created_at),
            ],
        );

        Ok(Self {
            id,
            requester,
            query_hash,
            collection_ids,
            top_k,
            created_at,
        })
    }
}

/// A single scored hit for a `RetrievalQuery`. Identity is the
/// `(query_id, chunk_id)` pair, so no separate id field is needed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetrievalResult {
    pub query_id: BytesN<32>,
    pub chunk_id: BytesN<32>,
    pub document_id: BytesN<32>,
    /// Similarity score scaled to basis points, `0..=MAX_SCORE_BPS`,
    /// avoiding floats in contract state.
    pub score_bps: u32,
    pub rank: u32,
}

impl RetrievalResult {
    pub fn new(
        query_id: BytesN<32>,
        chunk_id: BytesN<32>,
        document_id: BytesN<32>,
        score_bps: u32,
        rank: u32,
    ) -> RagResult<Self> {
        if score_bps > MAX_SCORE_BPS {
            return Err(TypesError::ScoreOutOfRange);
        }
        Ok(Self { query_id, chunk_id, document_id, score_bps, rank })
    }
}

// ---------------------------------------------------------------------
// ProvenanceRecord
// ---------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProvenanceAction {
    Created,
    Updated,
    Indexed,
    Archived,
    VersionSealed,
}

/// An append-only audit entry chained to its predecessor via
/// `prev_record_id`, forming a per-subject provenance log.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProvenanceRecord {
    pub id: BytesN<32>,
    pub subject_id: BytesN<32>,
    pub action: ProvenanceAction,
    pub actor: Address,
    pub timestamp: u64,
    pub prev_record_id: Option<BytesN<32>>,
}

impl ProvenanceRecord {
    /// `id` is derived from `subject_id` + `actor` + `timestamp` +
    /// `prev_record_id` (or a zeroed root marker if this is the first
    /// record for the subject), which chains each entry to its history
    /// and keeps otherwise-identical actions from colliding.
    pub fn new(
        env: &Env,
        subject_id: BytesN<32>,
        action: ProvenanceAction,
        actor: Address,
        timestamp: u64,
        prev_record_id: Option<BytesN<32>>,
    ) -> Self {
        let prev_bytes = match &prev_record_id {
            Some(prev) => bytes32(env, prev),
            None => Bytes::from_array(env, &[0u8; 32]),
        };

        let id = derive_id(
            env,
            &[
                bytes32(env, &subject_id),
                actor.to_xdr(env),
                u64_bytes(env, timestamp),
                prev_bytes,
            ],
        );

        Self {
            id,
            subject_id,
            action,
            actor,
            timestamp,
            prev_record_id,
        }
    }
}

// ---------------------------------------------------------------------
// KnowledgeVersion
// ---------------------------------------------------------------------

/// An immutable, sealed snapshot of a `Collection` at a point in time.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnowledgeVersion {
    pub id: BytesN<32>,
    pub collection_id: BytesN<32>,
    pub version_number: u32,
    pub snapshot_hash: BytesN<32>,
    pub parent_version_id: Option<BytesN<32>>,
    pub created_at: u64,
}

impl KnowledgeVersion {
    /// `id` is derived from `collection_id` + `version_number` +
    /// `snapshot_hash`, so the same snapshot sealed twice for the same
    /// collection/version resolves to the same version id.
    pub fn new(
        env: &Env,
        collection_id: BytesN<32>,
        version_number: u32,
        snapshot_hash: BytesN<32>,
        parent_version_id: Option<BytesN<32>>,
        created_at: u64,
    ) -> Self {
        let id = derive_id(
            env,
            &[
                bytes32(env, &collection_id),
                u32_bytes(env, version_number),
                bytes32(env, &snapshot_hash),
            ],
        );

        Self {
            id,
            collection_id,
            version_number,
            snapshot_hash,
            parent_version_id,
            created_at,
        }
    }
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{IntoVal, TryFromVal, Val};

    /// Round-trips a value through the host's `Val` conversion, which is
    /// the same path storage/cross-contract calls use, so this exercises
    /// real Soroban (de)serialization rather than just Rust's `Clone`.
    fn round_trip<T>(env: &Env, value: T) -> T
    where
        T: IntoVal<Env, Val> + TryFromVal<Env, Val>,
    {
        let val: Val = value.into_val(env);
        T::try_from_val(env, &val).unwrap()
    }

    #[test]
    fn collection_id_is_deterministic_and_bounds_are_enforced() {
        let env = Env::default();
        let owner = Address::generate(&env);
        let name = String::from_str(&env, "handbook");
        let desc = String::from_str(&env, "employee handbook");

        let a = Collection::new(&env, owner.clone(), name.clone(), desc.clone(), 100).unwrap();
        let b = Collection::new(&env, owner.clone(), name.clone(), desc.clone(), 999).unwrap();
        assert_eq!(a.id, b.id, "same owner+name must yield the same id regardless of timestamp");

        let too_long = String::from_str(&env, &"x".repeat((MAX_NAME_LEN + 1) as usize));
        assert_eq!(
            Collection::new(&env, owner, too_long, desc, 100).unwrap_err(),
            TypesError::NameTooLong
        );
    }

    #[test]
    fn document_id_is_content_addressed() {
        let env = Env::default();
        let collection_id = BytesN::<32>::from_array(&env, &[1u8; 32]);
        let content_hash = BytesN::<32>::from_array(&env, &[2u8; 32]);
        let source = SourceReference::new(
            SourceKind::Ipfs,
            String::from_str(&env, "ipfs://cid"),
            BytesN::<32>::from_array(&env, &[3u8; 32]),
        )
        .unwrap();

        let doc1 = Document::new(&env, collection_id.clone(), source.clone(), content_hash.clone(), 1);
        let doc2 = Document::new(&env, collection_id, source, content_hash, 2);
        assert_eq!(doc1.id, doc2.id);
    }

    #[test]
    fn chunk_rejects_invalid_byte_range() {
        let env = Env::default();
        let document_id = BytesN::<32>::from_array(&env, &[1u8; 32]);
        let content_hash = BytesN::<32>::from_array(&env, &[2u8; 32]);

        let err = DocumentChunk::new(
            &env,
            document_id,
            0,
            content_hash,
            ByteRange { start: 10, end: 10 },
        )
        .unwrap_err();
        assert_eq!(err, TypesError::InvalidByteRange);
    }

    #[test]
    fn retrieval_query_bounds_are_enforced() {
        let env = Env::default();
        let requester = Address::generate(&env);
        let query_hash = BytesN::<32>::from_array(&env, &[4u8; 32]);
        let mut ids = Vec::new(&env);
        for i in 0..(MAX_COLLECTION_IDS_PER_QUERY + 1) {
            ids.push_back(BytesN::<32>::from_array(&env, &[i as u8; 32]));
        }

        let err = RetrievalQuery::new(&env, requester.clone(), query_hash.clone(), ids, 5, 1).unwrap_err();
        assert_eq!(err, TypesError::TooManyCollectionIds);

        let ok_ids = Vec::new(&env);
        let err = RetrievalQuery::new(&env, requester, query_hash, ok_ids, 0, 1).unwrap_err();
        assert_eq!(err, TypesError::TopKOutOfRange);
    }

    #[test]
    fn all_types_round_trip_through_storage_conversion() {
        let env = Env::default();
        let owner = Address::generate(&env);

        let collection = Collection::new(
            &env,
            owner.clone(),
            String::from_str(&env, "docs"),
            String::from_str(&env, "general docs"),
            10,
        )
        .unwrap();
        let restored: Collection = round_trip(&env, collection.clone());
        assert_eq!(collection, restored);

        let source = SourceReference::new(
            SourceKind::Storage,
            String::from_str(&env, "bucket/key"),
            BytesN::<32>::from_array(&env, &[9u8; 32]),
        )
        .unwrap();
        let document = Document::new(
            &env,
            collection.id.clone(),
            source,
            BytesN::<32>::from_array(&env, &[8u8; 32]),
            10,
        );
        let restored: Document = round_trip(&env, document.clone());
        assert_eq!(document, restored);

        let chunk = DocumentChunk::new(
            &env,
            document.id.clone(),
            0,
            BytesN::<32>::from_array(&env, &[7u8; 32]),
            ByteRange { start: 0, end: 128 },
        )
        .unwrap();
        let restored: DocumentChunk = round_trip(&env, chunk.clone());
        assert_eq!(chunk, restored);

        let commitment = EmbeddingCommitment::new(
            &env,
            chunk.id.clone(),
            String::from_str(&env, "text-embed-small"),
            384,
            BytesN::<32>::from_array(&env, &[6u8; 32]),
            11,
        )
        .unwrap();
        let restored: EmbeddingCommitment = round_trip(&env, commitment.clone());
        assert_eq!(commitment, restored);

        let mut collection_ids = Vec::new(&env);
        collection_ids.push_back(collection.id.clone());
        let query = RetrievalQuery::new(
            &env,
            owner.clone(),
            BytesN::<32>::from_array(&env, &[5u8; 32]),
            collection_ids,
            10,
            12,
        )
        .unwrap();
        let restored: RetrievalQuery = round_trip(&env, query.clone());
        assert_eq!(query, restored);

        let result = RetrievalResult::new(query.id.clone(), chunk.id.clone(), document.id.clone(), 9000, 1).unwrap();
        let restored: RetrievalResult = round_trip(&env, result.clone());
        assert_eq!(result, restored);

        let provenance = ProvenanceRecord::new(
            &env,
            collection.id.clone(),
            ProvenanceAction::Created,
            owner,
            10,
            None,
        );
        let restored: ProvenanceRecord = round_trip(&env, provenance.clone());
        assert_eq!(provenance, restored);

        let version = KnowledgeVersion::new(
            &env,
            collection.id.clone(),
            1,
            BytesN::<32>::from_array(&env, &[4u8; 32]),
            None,
            13,
        );
        let restored: KnowledgeVersion = round_trip(&env, version.clone());
        assert_eq!(version, restored);
    }

    #[test]
    fn storage_persist_and_load_round_trip() {
        let env = Env::default();
        let contract_id = env.register(crate::RagContract, ());

        let owner = Address::generate(&env);
        let collection = Collection::new(
            &env,
            owner,
            String::from_str(&env, "docs"),
            String::from_str(&env, "general docs"),
            10,
        )
        .unwrap();

        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&collection.id, &collection);
            let loaded: Collection = env.storage().persistent().get(&collection.id).unwrap();
            assert_eq!(loaded, collection);
        });
    }
}