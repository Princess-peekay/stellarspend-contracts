#[cfg(test)]
mod tests {
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{vec, Address, Env};

    // We import the batch-rewards contract under test.
    use crate::{Contract, ContractClient, Error};

    // We import the delegation contract so we can register it in the test env
    // and set up real delegation state for integration-style tests.
    use delegation::{DelegationContract, DelegationContractClient};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    /// Registers batch-rewards, initialises with `admin`, returns (client, admin).
    fn setup_rewards<'e>(env: &'e Env) -> (ContractClient<'e>, Address) {
        let id = env.register(Contract, ());
        let client = ContractClient::new(env, &id);
        let admin = Address::generate(env);
        // client.initialize() returns () on success and panics on error.
        client.initialize(&admin);
        (client, admin)
    }

    /// Registers the delegation contract and sets a delegation record.
    /// Returns (delegation_client, contract_address).
    fn setup_delegation<'e>(
        env: &'e Env,
        owner: &Address,
        delegate: &Address,
        limit: i128,
    ) -> (DelegationContractClient<'e>, Address) {
        let del_id = env.register(DelegationContract, ());
        let del_client = DelegationContractClient::new(env, &del_id);
        del_client.set_delegation(owner, delegate, &limit);
        (del_client, del_id)
    }

    // -----------------------------------------------------------------------
    // initialize
    // -----------------------------------------------------------------------

    #[test]
    fn initialize_happy_path() {
        let env = make_env();
        let id = env.register(Contract, ());
        let client = ContractClient::new(&env, &id);
        let admin = Address::generate(&env);
        // Soroban client strips Result — returns () on success, panics on error.
        client.initialize(&admin);
    }

    #[test]
    fn initialize_twice_returns_error() {
        let env = make_env();
        let (client, admin) = setup_rewards(&env);
        // try_initialize returns Result<Result<(), Error>, soroban_sdk::Error>
        let result = client.try_initialize(&admin);
        assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
    }

    // -----------------------------------------------------------------------
    // distribute_rewards — happy path
    // -----------------------------------------------------------------------

    #[test]
    fn distribute_rewards_passes_when_delegation_sufficient() {
        let env = make_env();
        let (client, admin) = setup_rewards(&env);

        let owner = Address::generate(&env);
        let (_, del_id) = setup_delegation(&env, &owner, &admin, 1_000_i128);

        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);
        let recipients = vec![&env, r1.clone(), r2.clone()];
        let amounts = vec![&env, 400_i128, 300_i128]; // total = 700 ≤ 1000 limit

        // distribute_rewards returns () on success and panics on error.
        client.distribute_rewards(&admin, &owner, &del_id, &recipients, &amounts);
    }

    #[test]
    fn distribute_rewards_passes_when_allowance_exactly_equals_total() {
        let env = make_env();
        let (client, admin) = setup_rewards(&env);

        let owner = Address::generate(&env);
        let (_, del_id) = setup_delegation(&env, &owner, &admin, 500_i128);

        let r1 = Address::generate(&env);
        let recipients = vec![&env, r1.clone()];
        let amounts = vec![&env, 500_i128]; // total == limit exactly

        client.distribute_rewards(&admin, &owner, &del_id, &recipients, &amounts);
    }

    // -----------------------------------------------------------------------
    // distribute_rewards — delegation gate
    // -----------------------------------------------------------------------

    #[test]
    fn distribute_rewards_fails_when_delegation_insufficient() {
        let env = make_env();
        let (client, admin) = setup_rewards(&env);

        let owner = Address::generate(&env);
        // Only 100 in allowance, but we try to distribute 200.
        let (_, del_id) = setup_delegation(&env, &owner, &admin, 100_i128);

        let r1 = Address::generate(&env);
        let recipients = vec![&env, r1.clone()];
        let amounts = vec![&env, 200_i128];

        let result =
            client.try_distribute_rewards(&admin, &owner, &del_id, &recipients, &amounts);
        assert_eq!(result, Err(Ok(Error::DelegationCheckFailed)));
    }

    #[test]
    fn distribute_rewards_fails_when_no_delegation_exists() {
        let env = make_env();
        let (client, admin) = setup_rewards(&env);

        let owner = Address::generate(&env);
        // Register the delegation contract but set no allowance.
        let del_id = env.register(DelegationContract, ());

        let r1 = Address::generate(&env);
        let recipients = vec![&env, r1.clone()];
        let amounts = vec![&env, 1_i128];

        let result =
            client.try_distribute_rewards(&admin, &owner, &del_id, &recipients, &amounts);
        assert_eq!(result, Err(Ok(Error::DelegationCheckFailed)));
    }

    #[test]
    fn distribute_rewards_fails_after_delegation_revoked() {
        let env = make_env();
        let (client, admin) = setup_rewards(&env);

        let owner = Address::generate(&env);
        let (del_client, del_id) = setup_delegation(&env, &owner, &admin, 500_i128);
        // Revoke the delegation — check_allowance should now return false.
        del_client.revoke_delegation(&owner, &admin);

        let r1 = Address::generate(&env);
        let recipients = vec![&env, r1.clone()];
        let amounts = vec![&env, 100_i128];

        let result =
            client.try_distribute_rewards(&admin, &owner, &del_id, &recipients, &amounts);
        assert_eq!(result, Err(Ok(Error::DelegationCheckFailed)));
    }

    // -----------------------------------------------------------------------
    // distribute_rewards — input validation
    // -----------------------------------------------------------------------

    #[test]
    fn distribute_rewards_rejects_mismatched_vec_lengths() {
        let env = make_env();
        let (client, admin) = setup_rewards(&env);

        let owner = Address::generate(&env);
        let (_, del_id) = setup_delegation(&env, &owner, &admin, 1_000_i128);

        let r1 = Address::generate(&env);
        let recipients = vec![&env, r1.clone()];
        let amounts = vec![&env, 100_i128, 200_i128]; // length mismatch

        let result =
            client.try_distribute_rewards(&admin, &owner, &del_id, &recipients, &amounts);
        assert_eq!(result, Err(Ok(Error::InvalidAmount)));
    }

    #[test]
    fn distribute_rewards_rejects_zero_amount() {
        let env = make_env();
        let (client, admin) = setup_rewards(&env);

        let owner = Address::generate(&env);
        let (_, del_id) = setup_delegation(&env, &owner, &admin, 1_000_i128);

        let r1 = Address::generate(&env);
        let recipients = vec![&env, r1.clone()];
        let amounts = vec![&env, 0_i128]; // zero is invalid

        let result =
            client.try_distribute_rewards(&admin, &owner, &del_id, &recipients, &amounts);
        assert_eq!(result, Err(Ok(Error::InvalidAmount)));
    }

    #[test]
    fn distribute_rewards_rejects_negative_amount() {
        let env = make_env();
        let (client, admin) = setup_rewards(&env);

        let owner = Address::generate(&env);
        let (_, del_id) = setup_delegation(&env, &owner, &admin, 1_000_i128);

        let r1 = Address::generate(&env);
        let recipients = vec![&env, r1.clone()];
        let amounts = vec![&env, -50_i128];

        let result =
            client.try_distribute_rewards(&admin, &owner, &del_id, &recipients, &amounts);
        assert_eq!(result, Err(Ok(Error::InvalidAmount)));
    }

    #[test]
    fn distribute_rewards_rejects_non_admin_caller() {
        let env = make_env();
        let (client, admin) = setup_rewards(&env);

        let owner = Address::generate(&env);
        let (_, del_id) = setup_delegation(&env, &owner, &admin, 1_000_i128);

        // Keep `admin` in scope to avoid unused variable warning.
        let _ = admin;
        let impostor = Address::generate(&env);
        let r1 = Address::generate(&env);
        let recipients = vec![&env, r1.clone()];
        let amounts = vec![&env, 100_i128];

        // `impostor` is not the stored admin.
        let result =
            client.try_distribute_rewards(&impostor, &owner, &del_id, &recipients, &amounts);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    // -----------------------------------------------------------------------
    // Pre-existing tests (preserved from original stub)
    // -----------------------------------------------------------------------

    #[test]
    fn happy_path_environment() {
        use soroban_sdk::testutils::Ledger as _;
        let env = Env::default();
        env.ledger().set_sequence_number(1);
        assert_eq!(env.ledger().sequence(), 1);
    }

    #[test]
    fn zero_boundary() {
        assert_eq!(0_i128.checked_add(0), Some(0));
    }

    #[test]
    fn overflow_boundary() {
        assert_eq!(i128::MAX.checked_add(1), None);
    }
}
