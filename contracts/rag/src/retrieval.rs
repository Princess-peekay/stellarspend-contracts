use soroban_sdk::{
    contracterror, contracttype, Env,
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
/// `top_k` represents the configured number of knowledge items to select,
/// while `result_count` represents the number of items actually selected.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetrievalMetadata {
    /// ID of the retrieval request/query this metadata belongs to.
    pub request_id: u64,

    /// Number of knowledge items requested by the retrieval operation.
    pub top_k: u32,

    /// Number of knowledge items actually selected.
    pub result_count: u32,
}

// -----------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------

/// Errors returned by the retrieval metadata module.
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
            return Err(RetrievalMetadataError::ResultCountExceedsTopK);
        }

        // ---------------------------------------------------------------
        // 4. Create and store metadata
        // ---------------------------------------------------------------

        let metadata = RetrievalMetadata {
            request_id,
            top_k,
            result_count,
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
        assert!(RetrievalMetadataManager::is_valid_top_k(MAX_RAG_TOP_K));
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

    #[test]
    fn result_count_above_global_limit_is_rejected() {
        assert!(
            !RetrievalMetadataManager::is_valid_result_count(
                MAX_RAG_TOP_K,
                MAX_RAG_TOP_K + 1
            )
        );
    }

    // ---------------------------------------------------------------
    // Metadata storage
    // ---------------------------------------------------------------

    #[test]
    fn metadata_is_stored_and_retrieved() {
        let env = Env::default();

        let metadata =
            RetrievalMetadataManager::store_metadata(
                &env,
                42,
                10,
                7,
            )
            .unwrap();

        assert_eq!(
            metadata,
            RetrievalMetadata {
                request_id: 42,
                top_k: 10,
                result_count: 7,
            }
        );

        let stored =
            RetrievalMetadataManager::get_metadata(&env, 42)
                .unwrap();

        assert_eq!(stored.request_id, 42);
        assert_eq!(stored.top_k, 10);
        assert_eq!(stored.result_count, 7);
    }

    // ---------------------------------------------------------------
    // Invalid metadata
    // ---------------------------------------------------------------

    #[test]
    fn zero_top_k_is_rejected_when_storing_metadata() {
        let env = Env::default();

        let result =
            RetrievalMetadataManager::store_metadata(
                &env,
                1,
                0,
                0,
            );

        assert_eq!(
            result,
            Err(RetrievalMetadataError::InvalidTopK)
        );
    }

    #[test]
    fn result_count_greater_than_top_k_is_rejected() {
        let env = Env::default();

        let result =
            RetrievalMetadataManager::store_metadata(
                &env,
                1,
                10,
                11,
            );

        assert_eq!(
            result,
            Err(
                RetrievalMetadataError::ResultCountExceedsTopK
            )
        );
    }

    #[test]
    fn zero_result_count_is_rejected_when_storing_metadata() {
        let env = Env::default();

        let result =
            RetrievalMetadataManager::store_metadata(
                &env,
                1,
                10,
                0,
            );

        assert_eq!(
            result,
            Err(RetrievalMetadataError::InvalidResultCount)
        );
    }

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