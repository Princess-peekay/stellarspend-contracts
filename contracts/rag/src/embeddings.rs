use soroban_sdk::{contracttype, Address, Env, String, Vec};

/// Maximum length of an embedding model identifier.
pub const MAX_MODEL_IDENTIFIER_LENGTH: u32 = 128;

/// Maximum length of model metadata.
pub const MAX_MODEL_METADATA_LENGTH: u32 = 256;

/// Maximum length of an embedding commitment.
pub const MAX_COMMITMENT_LENGTH: u32 = 128;

/// Maximum length of a dimension commitment.
pub const MAX_DIMENSION_COMMITMENT_LENGTH: u32 = 128;

/// Maximum supported embedding dimension.
///
/// This prevents malformed or unreasonable dimensions from being
/// stored on-chain.
pub const MAX_EMBEDDING_DIMENSION: u32 = 16_384;

/// Minimum supported embedding dimension.
pub const MIN_EMBEDDING_DIMENSION: u32 = 1;

// -----------------------------------------------------------------------------
// Embedding Model Registry (#1242)
// -----------------------------------------------------------------------------

/// A registered embedding model.
///
/// Models are identified by a unique `model_id` and human-readable
/// identifier. Metadata is bounded to prevent unbounded storage.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddingModel {
    pub model_id: u64,
    pub identifier: String,
    pub metadata: String,
    pub active: bool,
}

/// ---------------------------------------------------------------------------
/// Embedding Commitment (#1241)
// ---------------------------------------------------------------------------
///
/// Represents an immutable commitment to an embedding generated off-chain.
///
/// The contract never generates or stores the actual embedding. It only
/// stores the commitments and bounded metadata required to identify and
/// verify the off-chain representation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddingCommitment {
    /// Source chunk associated with this embedding.
    pub chunk_id: u64,

    /// Embedding model used to generate the vector.
    pub model_id: u64,

    /// Embedding representation version.
    pub version: u64,

    /// Cryptographic commitment to the off-chain vector.
    pub commitment: Vec<u8>,

    /// Number of dimensions in the off-chain vector.
    pub dimension: u32,

    /// Commitment to the vector dimension.
    pub dimension_commitment: Vec<u8>,
}

/// Errors produced by the embedding module.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmbeddingError {
    Unauthorized,
    ModelIdentifierTooLong,
    MetadataTooLong,
    CommitmentTooLong,
    DimensionCommitmentTooLong,
    InvalidDimension,
    EmptyCommitment,
    EmptyDimensionCommitment,
    DuplicateModel,
    ModelNotFound,
    ModelInactive,
    CommitmentAlreadyExists,
    CommitmentNotFound,
}

/// Storage keys used by the embedding registry and commitment system.
#[contracttype]
pub enum EmbeddingKey {
    // Model registry
    Model(u64),
    ModelIdentifier(String),
    ModelCount,

    // Embedding commitments
    Commitment {
        chunk_id: u64,
        version: u64,
    },
}

// -----------------------------------------------------------------------------
// Model Registry
// -----------------------------------------------------------------------------

/// Registers a new embedding model.
///
/// Model identifiers must be unique and metadata must remain bounded.
///
/// The caller must be authenticated. The existing RAG authorization system
/// should additionally restrict this operation to administrators.
pub fn register_model(
    env: &Env,
    caller: Address,
    identifier: String,
    metadata: String,
) -> Result<u64, EmbeddingError> {
    caller.require_auth();

    if identifier.len() > MAX_MODEL_IDENTIFIER_LENGTH {
        return Err(EmbeddingError::ModelIdentifierTooLong);
    }

    if metadata.len() > MAX_MODEL_METADATA_LENGTH {
        return Err(EmbeddingError::MetadataTooLong);
    }

    let identifier_key =
        EmbeddingKey::ModelIdentifier(identifier.clone());

    if env.storage().persistent().has(&identifier_key) {
        return Err(EmbeddingError::DuplicateModel);
    }

    let model_id = get_model_count(env);

    let model = EmbeddingModel {
        model_id,
        identifier: identifier.clone(),
        metadata,
        active: true,
    };

    env.storage()
        .persistent()
        .set(&EmbeddingKey::Model(model_id), &model);

    env.storage()
        .persistent()
        .set(&identifier_key, &model_id);

    env.storage()
        .persistent()
        .set(
            &EmbeddingKey::ModelCount,
            &(model_id + 1),
        );

    Ok(model_id)
}

/// Retrieves a registered model by ID.
pub fn get_model(
    env: &Env,
    model_id: u64,
) -> Option<EmbeddingModel> {
    env.storage()
        .persistent()
        .get(&EmbeddingKey::Model(model_id))
}

/// Retrieves a model ID by its unique identifier.
pub fn get_model_id(
    env: &Env,
    identifier: String,
) -> Option<u64> {
    env.storage()
        .persistent()
        .get(&EmbeddingKey::ModelIdentifier(identifier))
}

/// Returns whether a registered model is active.
pub fn get_model_status(
    env: &Env,
    model_id: u64,
) -> Result<bool, EmbeddingError> {
    match get_model(env, model_id) {
        Some(model) => Ok(model.active),
        None => Err(EmbeddingError::ModelNotFound),
    }
}

/// Returns the number of registered models.
pub fn get_model_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&EmbeddingKey::ModelCount)
        .unwrap_or(0)
}

// -----------------------------------------------------------------------------
// Embedding Commitments + Versioning
// -----------------------------------------------------------------------------

/// Registers an immutable embedding commitment.
///
/// A commitment is uniquely identified by:
///
///     chunk_id + version
///
/// Once a commitment exists for a chunk/version pair, it cannot be replaced.
///
/// The embedding itself is generated off-chain. Only bounded metadata and
/// cryptographic commitments are recorded on-chain.
pub fn register_commitment(
    env: &Env,
    caller: Address,
    chunk_id: u64,
    model_id: u64,
    version: u64,
    commitment: Vec<u8>,
    dimension: u32,
    dimension_commitment: Vec<u8>,
) -> Result<(), EmbeddingError> {
    caller.require_auth();

    // -------------------------------------------------------------------------
    // Commitment validation
    // -------------------------------------------------------------------------

    if commitment.is_empty() {
        return Err(EmbeddingError::EmptyCommitment);
    }

    if commitment.len() > MAX_COMMITMENT_LENGTH {
        return Err(EmbeddingError::CommitmentTooLong);
    }

    if dimension_commitment.is_empty() {
        return Err(
            EmbeddingError::EmptyDimensionCommitment
        );
    }

    if dimension_commitment.len()
        > MAX_DIMENSION_COMMITMENT_LENGTH
    {
        return Err(
            EmbeddingError::DimensionCommitmentTooLong
        );
    }

    // -------------------------------------------------------------------------
    // Dimension validation
    // -------------------------------------------------------------------------

    if dimension < MIN_EMBEDDING_DIMENSION
        || dimension > MAX_EMBEDDING_DIMENSION
    {
        return Err(EmbeddingError::InvalidDimension);
    }

    // -------------------------------------------------------------------------
    // Model validation
    // -------------------------------------------------------------------------

    let model = match get_model(env, model_id) {
        Some(model) => model,
        None => return Err(EmbeddingError::ModelNotFound),
    };

    if !model.active {
        return Err(EmbeddingError::ModelInactive);
    }

    // -------------------------------------------------------------------------
    // Chunk + version binding
    // -------------------------------------------------------------------------

    let key = EmbeddingKey::Commitment {
        chunk_id,
        version,
    };

    // Never overwrite a historical commitment.
    if env.storage().persistent().has(&key) {
        return Err(
            EmbeddingError::CommitmentAlreadyExists
        );
    }

    // -------------------------------------------------------------------------
    // Store bounded metadata
    // -------------------------------------------------------------------------

    let embedding = EmbeddingCommitment {
        chunk_id,
        model_id,
        version,
        commitment,
        dimension,
        dimension_commitment,
    };

    env.storage()
        .persistent()
        .set(&key, &embedding);

    Ok(())
}

/// Retrieves the commitment for a specific chunk and version.
///
/// Historical records remain available indefinitely unless explicitly
/// removed by a future storage policy.
pub fn get_commitment(
    env: &Env,
    chunk_id: u64,
    version: u64,
) -> Option<EmbeddingCommitment> {
    let key = EmbeddingKey::Commitment {
        chunk_id,
        version,
    };

    env.storage().persistent().get(&key)
}

// -----------------------------------------------------------------------------
// Commitment Verification
// -----------------------------------------------------------------------------

/// Verifies that a supplied vector commitment matches the commitment
/// stored for a chunk/version pair.
pub fn verify_commitment(
    env: &Env,
    chunk_id: u64,
    version: u64,
    commitment: Vec<u8>,
) -> Result<bool, EmbeddingError> {
    if commitment.len() > MAX_COMMITMENT_LENGTH {
        return Err(EmbeddingError::CommitmentTooLong);
    }

    let stored = get_commitment(env, chunk_id, version)
        .ok_or(EmbeddingError::CommitmentNotFound)?;

    Ok(stored.commitment == commitment)
}

/// Verifies the dimension associated with a chunk/version pair.
///
/// The actual cryptographic verification of `dimension_commitment`
/// should be implemented once the repository defines the commitment
/// algorithm being used.
pub fn verify_dimension(
    env: &Env,
    chunk_id: u64,
    version: u64,
    dimension: u32,
) -> Result<bool, EmbeddingError> {
    if dimension < MIN_EMBEDDING_DIMENSION
        || dimension > MAX_EMBEDDING_DIMENSION
    {
        return Err(EmbeddingError::InvalidDimension);
    }

    let stored = get_commitment(env, chunk_id, version)
        .ok_or(EmbeddingError::CommitmentNotFound)?;

    Ok(stored.dimension == dimension)
}

/// Verifies that embedding metadata belongs to the expected chunk.
///
/// The storage key itself is based on `chunk_id + version`, but this
/// explicit check prevents accidental metadata mismatch.
pub fn verify_chunk_binding(
    env: &Env,
    chunk_id: u64,
    version: u64,
) -> Result<bool, EmbeddingError> {
    let stored = get_commitment(env, chunk_id, version)
        .ok_or(EmbeddingError::CommitmentNotFound)?;

    Ok(stored.chunk_id == chunk_id)
}