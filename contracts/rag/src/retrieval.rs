use soroban_sdk::{
    contracterror, contracttype, Address, Bytes, Env, String,
};

// -----------------------------------------------------------------------
// Configuration
// -----------------------------------------------------------------------

/// Maximum number of knowledge items that can be included in a
/// retrieval result.
pub const MAX_RAG_RESULT_COUNT: u32 = 100;

// -----------------------------------------------------------------------
// Retrieval Result
// -----------------------------------------------------------------------

/// Represents a committed result from an off-chain RAG retrieval.
///
/// The result itself is not stored on-chain. Instead, its commitment
/// is stored together with the query, collection, knowledge version,
/// executor, and result count.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetrievalResult {
    /// ID of the query that produced this result.
    pub query_id: u64,

    /// Collection used by the retrieval operation.
    pub collection_id: String,

    /// Knowledge version used by the retrieval operation.
    pub knowledge_version: u64,

    /// Commitment to the off-chain retrieval result.
    pub result_commitment: Bytes,

    /// Executor that submitted the result commitment.
    pub executor: Address,

    /// Number of knowledge items returned by the retrieval operation.
    pub result_count: u32,
}

// -----------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------

/// Errors returned by the retrieval result module.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum RetrievalResultError {
    /// The referenced query does not exist.
    QueryNotFound = 1,

    /// The executor is not authorized to submit retrieval results.
    UnauthorizedExecutor = 2,

    /// The result commitment is empty.
    InvalidCommitment = 3,

    /// The result count is invalid.
    InvalidResultCount = 4,

    /// A result has already been submitted for the query.
    DuplicateResult = 5,

    /// The collection association is invalid.
    InvalidCollection = 6,

    /// The knowledge version is invalid.
    InvalidKnowledgeVersion = 7,

    /// The requested retrieval result does not exist.
    ResultNotFound = 8,
}

// -----------------------------------------------------------------------
// Storage
// -----------------------------------------------------------------------

#[derive(Clone)]
#[contracttype]
pub enum RetrievalResultKey {
    Result(u64),
}

// -----------------------------------------------------------------------
// Retrieval Result Manager
// -----------------------------------------------------------------------

pub struct RetrievalResultManager;

impl RetrievalResultManager {
    /// Checks whether a retrieval result count is within the
    /// configured limit.
    pub fn is_valid_result_count(result_count: u32) -> bool {
        result_count > 0
            && result_count <= MAX_RAG_RESULT_COUNT
    }

    /// Checks whether a result commitment is valid.
    pub fn is_valid_commitment(commitment: &Bytes) -> bool {
        !commitment.is_empty()
    }

    /// Checks whether the supplied executor is authorized.
    ///
    /// This standalone implementation treats the supplied executor
    /// as the authorized executor for the operation.
    ///
    /// When the RAG contract has an existing executor registry,
    /// this function can be connected to that authorization layer
    /// without changing the stored result structure.
    pub fn is_authorized_executor(
        _env: &Env,
        _executor: &Address,
    ) -> bool {
        true
    }

    /// Checks whether a query has been registered.
    ///
    /// This standalone implementation uses the retrieval-result
    /// namespace to determine whether a query already has a result.
    ///
    /// In the integrated RAG contract this should be connected to
    /// the existing query registry.
    pub fn query_exists(
        _env: &Env,
        query_id: u64,
    ) -> bool {
        query_id > 0
    }

    /// Commits the result of an off-chain RAG retrieval operation.
    ///
    /// A query can only have one committed result.
    pub fn commit_result(
        env: &Env,
        query_id: u64,
        collection_id: String,
        knowledge_version: u64,
        result_commitment: Bytes,
        executor: Address,
        result_count: u32,
    ) -> Result<RetrievalResult, RetrievalResultError> {
        // ---------------------------------------------------------------
        // 1. Validate query ID
        // ---------------------------------------------------------------

        if !Self::query_exists(env, query_id) {
            return Err(RetrievalResultError::QueryNotFound);
        }

        // ---------------------------------------------------------------
        // 2. Validate executor
        // ---------------------------------------------------------------

        executor.require_auth();

        if !Self::is_authorized_executor(env, &executor) {
            return Err(
                RetrievalResultError::UnauthorizedExecutor
            );
        }

        // ---------------------------------------------------------------
        // 3. Reject duplicate submissions
        // ---------------------------------------------------------------

        let key = RetrievalResultKey::Result(query_id);

        if env.storage().persistent().has(&key) {
            return Err(RetrievalResultError::DuplicateResult);
        }

        // ---------------------------------------------------------------
        // 4. Validate commitment
        // ---------------------------------------------------------------

        if !Self::is_valid_commitment(&result_commitment) {
            return Err(
                RetrievalResultError::InvalidCommitment
            );
        }

        // ---------------------------------------------------------------
        // 5. Validate result count
        // ---------------------------------------------------------------

        if !Self::is_valid_result_count(result_count) {
            return Err(
                RetrievalResultError::InvalidResultCount
            );
        }

        // ---------------------------------------------------------------
        // 6. Validate collection
        // ---------------------------------------------------------------

        if collection_id.is_empty() {
            return Err(
                RetrievalResultError::InvalidCollection
            );
        }

        // ---------------------------------------------------------------
        // 7. Validate knowledge version
        // ---------------------------------------------------------------

        if knowledge_version == 0 {
            return Err(
                RetrievalResultError::InvalidKnowledgeVersion
            );
        }

        // ---------------------------------------------------------------
        // 8. Store retrieval result commitment
        // ---------------------------------------------------------------

        let result = RetrievalResult {
            query_id,
            collection_id,
            knowledge_version,
            result_commitment,
            executor,
            result_count,
        };

        env.storage()
            .persistent()
            .set(&key, &result);

        Ok(result)
    }

    /// Returns the committed result for a query.
    pub fn get_result(
        env: &Env,
        query_id: u64,
    ) -> Result<RetrievalResult, RetrievalResultError> {
        let key = RetrievalResultKey::Result(query_id);

        env.storage()
            .persistent()
            .get(&key)
            .ok_or(RetrievalResultError::ResultNotFound)
    }
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn setup() -> (Env, Address) {
        let env = Env::default();
        let executor = Address::generate(&env);

        (env, executor)
    }

    // ---------------------------------------------------------------
    // Result count validation
    // ---------------------------------------------------------------

    #[test]
    fn valid_result_count_is_accepted() {
        assert!(
            RetrievalResultManager::is_valid_result_count(1)
        );

        assert!(
            RetrievalResultManager::is_valid_result_count(50)
        );

        assert!(
            RetrievalResultManager::is_valid_result_count(
                MAX_RAG_RESULT_COUNT
            )
        );
    }

    #[test]
    fn zero_result_count_is_rejected() {
        assert!(
            !RetrievalResultManager::is_valid_result_count(0)
        );
    }

    #[test]
    fn result_count_above_limit_is_rejected() {
        assert!(
            !RetrievalResultManager::is_valid_result_count(
                MAX_RAG_RESULT_COUNT + 1
            )
        );
    }

    // ---------------------------------------------------------------
    // Commitment validation
    // ---------------------------------------------------------------

    #[test]
    fn non_empty_commitment_is_valid() {
        let env = Env::default();

        let commitment =
            Bytes::from_array(&env, &[1u8; 32]);

        assert!(
            RetrievalResultManager::is_valid_commitment(
                &commitment
            )
        );
    }

    #[test]
    fn empty_commitment_is_invalid() {
        let env = Env::default();

        let commitment = Bytes::new(&env);

        assert!(
            !RetrievalResultManager::is_valid_commitment(
                &commitment
            )
        );
    }

    // ---------------------------------------------------------------
    // Result commitment
    // ---------------------------------------------------------------

    #[test]
    fn retrieval_result_is_stored() {
        let (env, executor) = setup();

        let collection_id =
            String::from_str(&env, "collection-1");

        let commitment =
            Bytes::from_array(&env, &[7u8; 32]);

        let result =
            RetrievalResultManager::commit_result(
                &env,
                1,
                collection_id.clone(),
                1,
                commitment.clone(),
                executor.clone(),
                5,
            )
            .unwrap();

        assert_eq!(result.query_id, 1);
        assert_eq!(result.collection_id, collection_id);
        assert_eq!(result.knowledge_version, 1);
        assert_eq!(
            result.result_commitment,
            commitment
        );
        assert_eq!(result.executor, executor);
        assert_eq!(result.result_count, 5);
    }

    // ---------------------------------------------------------------
    // Stored result retrieval
    // ---------------------------------------------------------------

    #[test]
    fn stored_result_can_be_retrieved() {
        let (env, executor) = setup();

        let collection_id =
            String::from_str(&env, "collection-1");

        let commitment =
            Bytes::from_array(&env, &[9u8; 32]);

        RetrievalResultManager::commit_result(
            &env,
            42,
            collection_id.clone(),
            2,
            commitment.clone(),
            executor.clone(),
            3,
        )
        .unwrap();

        let stored =
            RetrievalResultManager::get_result(
                &env,
                42,
            )
            .unwrap();

        assert_eq!(stored.query_id, 42);
        assert_eq!(stored.collection_id, collection_id);
        assert_eq!(
            stored.knowledge_version,
            2
        );
        assert_eq!(
            stored.result_commitment,
            commitment
        );
        assert_eq!(
            stored.executor,
            executor
        );
        assert_eq!(
            stored.result_count,
            3
        );
    }

    // ---------------------------------------------------------------
    // Duplicate submission
    // ---------------------------------------------------------------

    #[test]
    fn duplicate_result_submission_is_rejected() {
        let (env, executor) = setup();

        let collection_id =
            String::from_str(&env, "collection-1");

        let first_commitment =
            Bytes::from_array(&env, &[1u8; 32]);

        RetrievalResultManager::commit_result(
            &env,
            1,
            collection_id.clone(),
            1,
            first_commitment,
            executor.clone(),
            5,
        )
        .unwrap();

        let second_commitment =
            Bytes::from_array(&env, &[2u8; 32]);

        let result =
            RetrievalResultManager::commit_result(
                &env,
                1,
                collection_id,
                1,
                second_commitment,
                executor,
                5,
            );

        assert_eq!(
            result,
            Err(
                RetrievalResultError::DuplicateResult
            )
        );
    }

    // ---------------------------------------------------------------
    // Invalid result count
    // ---------------------------------------------------------------

    #[test]
    fn invalid_result_count_is_rejected() {
        let (env, executor) = setup();

        let collection_id =
            String::from_str(&env, "collection-1");

        let commitment =
            Bytes::from_array(&env, &[1u8; 32]);

        let result =
            RetrievalResultManager::commit_result(
                &env,
                1,
                collection_id,
                1,
                commitment,
                executor,
                0,
            );

        assert_eq!(
            result,
            Err(
                RetrievalResultError::InvalidResultCount
            )
        );
    }

    // ---------------------------------------------------------------
    // Invalid commitment
    // ---------------------------------------------------------------

    #[test]
    fn empty_commitment_is_rejected() {
        let (env, executor) = setup();

        let collection_id =
            String::from_str(&env, "collection-1");

        let commitment = Bytes::new(&env);

        let result =
            RetrievalResultManager::commit_result(
                &env,
                1,
                collection_id,
                1,
                commitment,
                executor,
                5,
            );

        assert_eq!(
            result,
            Err(
                RetrievalResultError::InvalidCommitment
            )
        );
    }

    // ---------------------------------------------------------------
    // Invalid collection
    // ---------------------------------------------------------------

    #[test]
    fn empty_collection_is_rejected() {
        let (env, executor) = setup();

        let collection_id =
            String::from_str(&env, "");

        let commitment =
            Bytes::from_array(&env, &[1u8; 32]);

        let result =
            RetrievalResultManager::commit_result(
                &env,
                1,
                collection_id,
                1,
                commitment,
                executor,
                5,
            );

        assert_eq!(
            result,
            Err(
                RetrievalResultError::InvalidCollection
            )
        );
    }

    // ---------------------------------------------------------------
    // Invalid knowledge version
    // ---------------------------------------------------------------

    #[test]
    fn zero_knowledge_version_is_rejected() {
        let (env, executor) = setup();

        let collection_id =
            String::from_str(&env, "collection-1");

        let commitment =
            Bytes::from_array(&env, &[1u8; 32]);

        let result =
            RetrievalResultManager::commit_result(
                &env,
                1,
                collection_id,
                0,
                commitment,
                executor,
                5,
            );

        assert_eq!(
            result,
            Err(
                RetrievalResultError::InvalidKnowledgeVersion
            )
        );
    }

    // ---------------------------------------------------------------
    // Missing result
    // ---------------------------------------------------------------

    #[test]
    fn nonexistent_result_returns_error() {
        let env = Env::default();

        let result =
            RetrievalResultManager::get_result(
                &env,
                999,
            );

        assert_eq!(
            result,
            Err(
                RetrievalResultError::ResultNotFound
            )
        );
    }
}