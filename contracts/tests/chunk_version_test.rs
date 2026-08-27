#[cfg(test)]
mod chunk_version_tests {
    use super::*;
    use soroban_sdk::{Env, String};

    #[test]
    fn test_chunk_version_matching_and_mutation_prevention() {
        let env = Env::default();
        
        let initial_chunk = Chunk {
            id: 1,
            document_id: 100,
            version: 1,
            content: String::from_str(&env, "Initial chunk content v1"),
        };

        // Attempting to update with a mismatched version should fail
        let result = ChunkManager::upsert_chunk(
            &env,
            Some(initial_chunk.clone()),
            1,
            100,
            2, // Mismatched version
            String::from_str(&env, "Mutated content"),
        );

        assert_eq!(result, Err(ChunkError::VersionMismatch));

        // Updating with the correct matching version should succeed
        let success_result = ChunkManager::upsert_chunk(
            &env,
            Some(initial_chunk),
            1,
            100,
            1, // Matching version
            String::from_str(&env, "Updated v1 content"),
        );

        assert!(success_result.is_ok());
        assert_eq!(success_result.unwrap().version, 1);
    }
}