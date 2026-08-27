#[cfg(test)]
mod chunk_access_tests {
    use super::*;
    use soroban_sdk::{vec, Env};

    #[test]
    fn test_restricted_chunk_access_enforcement() {
        let env = Env::default();
        let authorized_user = Address::generate(&env);
        let unauthorized_user = Address::generate(&env);

        let policy = ChunkAccessPolicy {
            chunk_id: 10,
            is_restricted: true,
            allowed_viewers: vec![&env, authorized_user.clone()],
        };

        // Authorized user should pass access check
        assert!(ChunkAccessManager::verify_chunk_access(&env, &authorized_user, &policy).is_ok());

        // Unauthorized user should be rejected with AccessDenied
        assert_eq!(
            ChunkAccessManager::verify_chunk_access(&env, &unauthorized_user, &policy),
            Err(AccessError::AccessDenied)
        );
    }
}