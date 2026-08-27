#[cfg(test)]
mod chunk_ownership_tests {
    use super::*;
    use soroban_sdk::{vec, Env};

    #[test]
    fn test_unauthorized_chunk_registration_fails() {
        let env = Env::default();
        let owner = Address::generate(&env);
        let unauthorized_user = Address::generate(&env);
        let admin = Address::generate(&env);

        let document = Document {
            id: 100,
            owner: owner.clone(),
        };

        let admins = vec![&env, admin.clone()];

        // Unauthorized user attempt should return OwnershipError::Unauthorized
        let result = ChunkOwnershipManager::verify_chunk_operation(
            &env,
            &unauthorized_user,
            &document,
            &admins,
        );

        assert_eq!(result, Err(OwnershipError::Unauthorized));
    }
}