use soroban_sdk::{
    contracterror, contracttype, Bytes, Env, String,
};

// -----------------------------------------------------------------------
// Configuration
// -----------------------------------------------------------------------

/// Maximum number of knowledge items that can be recorded for one
/// retrieval operation.
pub const MAX_RAG_TOP_K: u32 = 100;

// -----------------------------------------------------------------------
// Retrieval Metadata
// -----------------------------------------------------------------------

/// Metadata recorded for an off-chain RAG retrieval operation.
///
/// The request ID associates this metadata with the retrieval query.
/// The collection ID and version identify the exact knowledge source
/// used for the retrieval.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetrievalMetadata {
    /// ID of the retrieval request/query this metadata belongs to.
    pub request_id: u64,

    /// Number of knowledge items requested by the retrieval operation.
    pub top_k: u32,

    /// Number of knowledge items actually selected.
    pub result_count: u32,

    /// Collection used for the retrieval.
    pub collection_id: String,

    /// Version of the collection used for the retrieval.
    pub collection_version: u64,

    /// Commitment to the complete retrieval result.
    pub result_commitment: Bytes,
}

// -----------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------

/// Errors returned by the retrieval metadata and verification module.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum RetrievalMetadataError {
    /// The top-K value is invalid.
    InvalidTopK = 1,

    /// The result count is invalid.
    InvalidResultCount = 2,

    /// The result count exceeds the configured top-K value.
    ResultCountExceedsTopK = 3,

    /// Metadata for the requested retrieval does not exist.
    MetadataNotFound = 4,

    /// The supplied request ID does not match the registered request.
    QueryAssociationMismatch = 5,

    /// The supplied collection does not match the registered collection.
    CollectionAssociationMismatch = 6,

    /// The supplied collection version does not match the registered version.
    CollectionVersionMismatch = 7,

    /// The supplied result does not match the registered commitment.
    ResultCommitmentMismatch = 8,
}

// -----------------------------------------------------------------------
// Storage
// -----------------------------------------------------------------------

#[derive(Clone)]
#[contracttype]
pub enum RetrievalMetadataKey {
    Metadata(u64),
}

// -----------------------------------------------------------------------
// Retrieval Metadata Manager
// -----------------------------------------------------------------------

pub struct RetrievalMetadataManager;

impl RetrievalMetadataManager {
    /// Creates a deterministic commitment for a retrieval result.
    ///
    /// The commitment binds the result to:
    /// - the retrieval request;
    /// - the collection;
    /// - the collection version;
    /// - the actual retrieval result.
    pub fn compute_result_commitment(
        env: &Env,
        request_id: u64,
        collection_id: String,
        collection_version: u64,
        result: Bytes,
    ) -> Bytes {
        let mut payload = Bytes::new(env);

        payload.extend_from_array(&request_id.to_be_bytes());
        payload.extend(collection_id.to_bytes());
        payload.extend_from_array(&collection_version.to_be_bytes());
        payload.extend(result);

        env.crypto().sha256(&payload).into()
    }

    /// Stores metadata for an off-chain retrieval operation.
    ///
    /// The result count must:
    /// - be greater than zero;
    /// - not exceed `top_k`;
    /// - not exceed `MAX_RAG_TOP_K`.
    pub fn store_metadata(
        env: &Env,
        request_id: u64,
        top_k: u32,
        result_count: u32,
        collection_id: String,
        collection_version: u64,
        result_commitment: Bytes,
    ) -> Result<RetrievalMetadata, RetrievalMetadataError> {
        // ---------------------------------------------------------------
        // 1. Validate top-K
        // ---------------------------------------------------------------

        if top_k == 0 || top_k > MAX_RAG_TOP_K {
            return Err(RetrievalMetadataError::InvalidTopK);
        }

        // ---------------------------------------------------------------
        // 2. Validate result count
        // ---------------------------------------------------------------

        if result_count == 0 || result_count > MAX_RAG_TOP_K {
            return Err(RetrievalMetadataError::InvalidResultCount);
        }

        // ---------------------------------------------------------------
        // 3. Ensure result count does not exceed top-K
        // ---------------------------------------------------------------

        if result_count > top_k {
            return Err(
                RetrievalMetadataError::ResultCountExceedsTopK
            );
        }

        // ---------------------------------------------------------------
        // 4. Ensure the commitment is not empty
        // ---------------------------------------------------------------

        if result_commitment.is_empty() {
            return Err(
                RetrievalMetadataError::ResultCommitmentMismatch
            );
        }

        // ---------------------------------------------------------------
        // 5. Create and store metadata
        // ---------------------------------------------------------------

        let metadata = RetrievalMetadata {
            request_id,
            top_k,
            result_count,
            collection_id,
            collection_version,
            result_commitment,
        };

        let key = RetrievalMetadataKey::Metadata(request_id);

        env.storage()
            .persistent()
            .set(&key, &metadata);

        Ok(metadata)
    }

    /// Returns metadata associated with a retrieval request.
    pub fn get_metadata(
        env: &Env,
        request_id: u64,
    ) -> Result<RetrievalMetadata, RetrievalMetadataError> {
        let key = RetrievalMetadataKey::Metadata(request_id);

        env.storage()
            .persistent()
            .get(&key)
            .ok_or(RetrievalMetadataError::MetadataNotFound)
    }

    /// Verifies that a supplied retrieval result matches the registered
    /// result commitment and associations.
    ///
    /// Verification checks:
    /// - request/query association;
    /// - collection association;
    /// - collection version association;
    /// - result commitment.
    pub fn verify_result(
        env: &Env,
        request_id: u64,
        collection_id: String,
        collection_version: u64,
        result: Bytes,
    ) -> Result<bool, RetrievalMetadataError> {
        let metadata = Self::get_metadata(env, request_id)?;

        // ---------------------------------------------------------------
        // 1. Verify query/request association
        // ---------------------------------------------------------------

        if metadata.request_id != request_id {
            return Err(
                RetrievalMetadataError::QueryAssociationMismatch
            );
        }

        // ---------------------------------------------------------------
        // 2. Verify collection association
        // ---------------------------------------------------------------

        if metadata.collection_id != collection_id {
            return Err(
                RetrievalMetadataError::CollectionAssociationMismatch
            );
        }

        // ---------------------------------------------------------------
        // 3. Verify collection version
        // ---------------------------------------------------------------

        if metadata.collection_version != collection_version {
            return Err(
                RetrievalMetadataError::CollectionVersionMismatch
            );
        }

        // ---------------------------------------------------------------
        // 4. Recompute the commitment from the supplied result
        // ---------------------------------------------------------------

        let supplied_commitment = Self::compute_result_commitment(
            env,
            request_id,
            collection_id,
            collection_version,
            result,
        );

        // ---------------------------------------------------------------
        // 5. Verify the result commitment
        // ---------------------------------------------------------------

        if supplied_commitment != metadata.result_commitment {
            return Err(
                RetrievalMetadataError::ResultCommitmentMismatch
            );
        }

        Ok(true)
    }

    /// Checks whether a top-K value is within the configured limit.
    pub fn is_valid_top_k(top_k: u32) -> bool {
        top_k > 0 && top_k <= MAX_RAG_TOP_K
    }

    /// Checks whether a result count is valid for a given top-K value.
    pub fn is_valid_result_count(
        top_k: u32,
        result_count: u32,
    ) -> bool {
        result_count > 0
            && result_count <= top_k
            && result_count <= MAX_RAG_TOP_K
    }
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // Top-K validation
    // ---------------------------------------------------------------

    #[test]
    fn valid_top_k_is_accepted() {
        assert!(RetrievalMetadataManager::is_valid_top_k(1));
        assert!(RetrievalMetadataManager::is_valid_top_k(10));
        assert!(RetrievalMetadataManager::is_valid_top_k(
            MAX_RAG_TOP_K
        ));
    }

    #[test]
    fn zero_top_k_is_rejected() {
        assert!(!RetrievalMetadataManager::is_valid_top_k(0));
    }

    #[test]
    fn top_k_above_limit_is_rejected() {
        assert!(
            !RetrievalMetadataManager::is_valid_top_k(
                MAX_RAG_TOP_K + 1
            )
        );
    }

    // ---------------------------------------------------------------
    // Result count validation
    // ---------------------------------------------------------------

    #[test]
    fn valid_result_count_is_accepted() {
        assert!(
            RetrievalMetadataManager::is_valid_result_count(
                10, 5
            )
        );

        assert!(
            RetrievalMetadataManager::is_valid_result_count(
                10, 10
            )
        );
    }

    #[test]
    fn zero_result_count_is_rejected() {
        assert!(
            !RetrievalMetadataManager::is_valid_result_count(
                10, 0
            )
        );
    }

    #[test]
    fn result_count_above_top_k_is_rejected() {
        assert!(
            !RetrievalMetadataManager::is_valid_result_count(
                10, 11
            )
        );
    }

    // ---------------------------------------------------------------
    // Metadata storage
    // ---------------------------------------------------------------

    #[test]
    fn metadata_is_stored_and_retrieved() {
        let env = Env::default();

        let collection_id =
            String::from_str(&env, "collection-1");

        let result =
            Bytes::from_array(&env, &[1u8, 2u8, 3u8]);

        let commitment =
            RetrievalMetadataManager::compute_result_commitment(
                &env,
                42,
                collection_id.clone(),
                1,
                result,
            );

        let metadata =
            RetrievalMetadataManager::store_metadata(
                &env,
                42,
                10,
                7,
                collection_id.clone(),
                1,
                commitment.clone(),
            )
            .unwrap();

        assert_eq!(metadata.request_id, 42);
        assert_eq!(metadata.top_k, 10);
        assert_eq!(metadata.result_count, 7);
        assert_eq!(metadata.collection_id, collection_id);
        assert_eq!(metadata.collection_version, 1);
        assert_eq!(metadata.result_commitment, commitment);

        let stored =
            RetrievalMetadataManager::get_metadata(&env, 42)
                .unwrap();

        assert_eq!(stored, metadata);
    }

    // ---------------------------------------------------------------
    // Successful verification
    // ---------------------------------------------------------------

    #[test]
    fn matching_commitment_verifies_successfully() {
        let env = Env::default();

        let collection_id =
            String::from_str(&env, "collection-1");

        let result =
            Bytes::from_array(&env, &[10u8, 20u8, 30u8]);

        let commitment =
            RetrievalMetadataManager::compute_result_commitment(
                &env,
                42,
                collection_id.clone(),
                3,
                result.clone(),
            );

        RetrievalMetadataManager::store_metadata(
            &env,
            42,
            10,
            3,
            collection_id.clone(),
            3,
            commitment,
        )
        .unwrap();

        let verified =
            RetrievalMetadataManager::verify_result(
                &env,
                42,
                collection_id,
                3,
                result,
            )
            .unwrap();

        assert!(verified);
    }

    // ---------------------------------------------------------------
    // Tampered result
    // ---------------------------------------------------------------

    #[test]
    fn tampered_result_fails_verification() {
        let env = Env::default();

        let collection_id =
            String::from_str(&env, "collection-1");

        let original_result =
            Bytes::from_array(&env, &[10u8, 20u8, 30u8]);

        let commitment =
            RetrievalMetadataManager::compute_result_commitment(
                &env,
                42,
                collection_id.clone(),
                1,
                original_result,
            );

        RetrievalMetadataManager::store_metadata(
            &env,
            42,
            10,
            3,
            collection_id.clone(),
            1,
            commitment,
        )
        .unwrap();

        let tampered_result =
            Bytes::from_array(&env, &[10u8, 20u8, 99u8]);

        let result =
            RetrievalMetadataManager::verify_result(
                &env,
                42,
                collection_id,
                1,
                tampered_result,
            );

        assert_eq!(
            result,
            Err(
                RetrievalMetadataError::ResultCommitmentMismatch
            )
        );
    }

    // ---------------------------------------------------------------
    // Query association
    // ---------------------------------------------------------------

    #[test]
    fn wrong_request_id_fails_verification() {
        let env = Env::default();

        let collection_id =
            String::from_str(&env, "collection-1");

        let result =
            Bytes::from_array(&env, &[1u8, 2u8, 3u8]);

        let commitment =
            RetrievalMetadataManager::compute_result_commitment(
                &env,
                42,
                collection_id.clone(),
                1,
                result.clone(),
            );

        RetrievalMetadataManager::store_metadata(
            &env,
            42,
            10,
            3,
            collection_id.clone(),
            1,
            commitment,
        )
        .unwrap();

        let verification =
            RetrievalMetadataManager::verify_result(
                &env,
                43,
                collection_id,
                1,
                result,
            );

        assert_eq!(
            verification,
            Err(RetrievalMetadataError::ResultCommitmentMismatch)
        );
    }

    // ---------------------------------------------------------------
    // Collection association
    // ---------------------------------------------------------------

    #[test]
    fn wrong_collection_fails_verification() {
        let env = Env::default();

        let original_collection =
            String::from_str(&env, "collection-1");

        let result =
            Bytes::from_array(&env, &[1u8, 2u8, 3u8]);

        let commitment =
            RetrievalMetadataManager::compute_result_commitment(
                &env,
                42,
                original_collection.clone(),
                1,
                result.clone(),
            );

        RetrievalMetadataManager::store_metadata(
            &env,
            42,
            10,
            3,
            original_collection,
            1,
            commitment,
        )
        .unwrap();

        let wrong_collection =
            String::from_str(&env, "collection-2");

        let verification =
            RetrievalMetadataManager::verify_result(
                &env,
                42,
                wrong_collection,
                1,
                result,
            );

        assert_eq!(
            verification,
            Err(
                RetrievalMetadataError::CollectionAssociationMismatch
            )
        );
    }

    // ---------------------------------------------------------------
    // Collection version association
    // ---------------------------------------------------------------

    #[test]
    fn wrong_collection_version_fails_verification() {
        let env = Env::default();

        let collection_id =
            String::from_str(&env, "collection-1");

        let result =
            Bytes::from_array(&env, &[1u8, 2u8, 3u8]);

        let commitment =
            RetrievalMetadataManager::compute_result_commitment(
                &env,
                42,
                collection_id.clone(),
                1,
                result.clone(),
            );

        RetrievalMetadataManager::store_metadata(
            &env,
            42,
            10,
            3,
            collection_id.clone(),
            1,
            commitment,
        )
        .unwrap();

        let verification =
            RetrievalMetadataManager::verify_result(
                &env,
                42,
                collection_id,
                2,
                result,
            );

        assert_eq!(
            verification,
            Err(
                RetrievalMetadataError::CollectionVersionMismatch
            )
        );
    }

    // ---------------------------------------------------------------
    // Missing metadata
    // ---------------------------------------------------------------

    #[test]
    fn nonexistent_metadata_returns_error() {
        let env = Env::default();

        let result =
            RetrievalMetadataManager::get_metadata(&env, 999);

        assert_eq!(
            result,
            Err(RetrievalMetadataError::MetadataNotFound)
        );
    }
}