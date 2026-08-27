#[cfg(test)]
mod chunk_deactivation_tests {
    use super::*;
    use soroban_sdk::{Env, String};

    #[test]
    fn test_chunk_deactivation_and_provenance_preservation() {
        let env = Env::default();
        let owner = Address::generate(&env);
        let unauthorized = Address::generate(&env);

        let mut chunk = Chunk {
            id: 1,
            document_id: 10,
            version: 1,
            content: String::from_str(&env, "Historical content record"),
            is_active: true,
        };

        // Unauthorized user attempt should fail
        let unauthorized_result = ChunkLifecycleManager::deactivate_chunk(
            &env,
            &unauthorized,
            &owner,
            &mut chunk,
        );
        assert_eq!(unauthorized_result, Err(DeactivationError::Unauthorized));

        // Authorized owner attempt should succeed
        let success_result = ChunkLifecycleManager::deactivate_chunk(
            &env,
            &owner,
            &owner,
            &mut chunk,
        );
        assert!(success_result.is_ok());
        assert_eq!(chunk.is_active, false);
        // Ensure historical content provenance is preserved and not destroyed
        assert_eq!(chunk.content, String::from_str(&env, "Historical content record"));
    }
}