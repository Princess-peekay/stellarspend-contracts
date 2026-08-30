use soroban_sdk::{contracttype, Env, Vec};

#[contracttype]
pub enum VerificationKey {
    RegisteredCommitment,
}

/// Stores a registered embedding commitment.
pub fn set_registered_commitment(env: &Env, commitment: Vec<u8>) {
    env.storage()
        .persistent()
        .set(&VerificationKey::RegisteredCommitment, &commitment);
}

/// Retrieves the registered embedding commitment.
pub fn get_registered_commitment(env: &Env) -> Option<Vec<u8>> {
    env.storage()
        .persistent()
        .get(&VerificationKey::RegisteredCommitment)
}

/// Verifies that a supplied commitment matches the registered
/// on-chain commitment.
///
/// Returns `true` when the commitments match and `false` otherwise.
pub fn verify_commitment(env: &Env, supplied_commitment: Vec<u8>) -> bool {
    match get_registered_commitment(env) {
        Some(registered_commitment) => registered_commitment == supplied_commitment,
        None => false,
    }
}
