use soroban_sdk::{contracttype, Env, String, Vec};

pub const MAX_MODEL_METADATA_LENGTH: u32 = 256;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddingModel {
    pub model_id: u64,
    pub identifier: String,
    pub metadata: String,
    pub active: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddingRecord {
    pub commitment: Vec<u8>,
    pub model_id: u64,
    pub chunk_version: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmbeddingError {
    DuplicateModel,
    MetadataTooLong,
    ModelNotFound,
}

#[contracttype]
pub enum EmbeddingKey {
    Model(u64),
    ModelIdentifier(String),
    ModelCount,
    Record(u64),
    RecordCount,
}

/// Registers a supported embedding model.
///
/// Duplicate identifiers are rejected and model metadata is bounded.
pub fn register_model(
    env: &Env,
    identifier: String,
    metadata: String,
) -> Result<u64, EmbeddingError> {
    if metadata.len() > MAX_MODEL_METADATA_LENGTH {
        return Err(EmbeddingError::MetadataTooLong);
    }

    if env
        .storage()
        .persistent()
        .has(&EmbeddingKey::ModelIdentifier(identifier.clone()))
    {
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
        .set(&EmbeddingKey::ModelIdentifier(identifier), &model_id);

    env.storage()
        .persistent()
        .set(&EmbeddingKey::ModelCount, &(model_id + 1));

    Ok(model_id)
}

/// Retrieves a registered embedding model.
pub fn get_model(env: &Env, model_id: u64) -> Option<EmbeddingModel> {
    env.storage()
        .persistent()
        .get(&EmbeddingKey::Model(model_id))
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

/// Returns the total number of registered models.
pub fn get_model_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&EmbeddingKey::ModelCount)
        .unwrap_or(0)
}