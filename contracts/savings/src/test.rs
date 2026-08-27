#[cfg(test)]
mod tests {
    use crate::{Contract, ContractClient, Error};
    use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

    fn setup(env: &Env) -> ContractClient<'static> {
        env.mock_all_auths();
        let contract_id = env.register(Contract, ());
        let client = ContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize(&admin);
        client
    }

    #[test]
    fn deposit_increases_balance() {
        let env = Env::default();
        let client = setup(&env);
        let user = Address::generate(&env);
        let asset = Symbol::new(&env, "XLM");

        client.deposit(&user, &500_i128, &asset);
        assert_eq!(client.get_balance(&user, &asset), 500);

        client.deposit(&user, &250_i128, &asset);
        assert_eq!(client.get_balance(&user, &asset), 750);
    }

    #[test]
    fn withdraw_decreases_balance() {
        let env = Env::default();
        let client = setup(&env);
        let user = Address::generate(&env);
        let asset = Symbol::new(&env, "XLM");

        client.deposit(&user, &500_i128, &asset);
        client.withdraw(&user, &200_i128, &asset);
        assert_eq!(client.get_balance(&user, &asset), 300);
    }

    #[test]
    fn withdraw_full_balance_leaves_zero() {
        let env = Env::default();
        let client = setup(&env);
        let user = Address::generate(&env);
        let asset = Symbol::new(&env, "XLM");

        client.deposit(&user, &500_i128, &asset);
        client.withdraw(&user, &500_i128, &asset);
        assert_eq!(client.get_balance(&user, &asset), 0);
    }

    #[test]
    fn withdraw_rejects_insufficient_balance() {
        let env = Env::default();
        let client = setup(&env);
        let user = Address::generate(&env);
        let asset = Symbol::new(&env, "XLM");

        client.deposit(&user, &100_i128, &asset);
        let result = client.try_withdraw(&user, &200_i128, &asset);
        assert_eq!(result, Err(Ok(Error::InsufficientBalance)));
        // Balance must be unchanged after a rejected withdrawal.
        assert_eq!(client.get_balance(&user, &asset), 100);
    }

    #[test]
    fn withdraw_rejects_when_no_balance_exists() {
        let env = Env::default();
        let client = setup(&env);
        let user = Address::generate(&env);
        let asset = Symbol::new(&env, "XLM");

        let result = client.try_withdraw(&user, &1_i128, &asset);
        assert_eq!(result, Err(Ok(Error::InsufficientBalance)));
    }

    #[test]
    fn deposit_rejects_non_positive_amount() {
        let env = Env::default();
        let client = setup(&env);
        let user = Address::generate(&env);
        let asset = Symbol::new(&env, "XLM");

        let result = client.try_deposit(&user, &0_i128, &asset);
        assert_eq!(result, Err(Ok(Error::InvalidAmount)));

        let result = client.try_deposit(&user, &(-10_i128), &asset);
        assert_eq!(result, Err(Ok(Error::InvalidAmount)));
    }

    #[test]
    fn withdraw_rejects_non_positive_amount() {
        let env = Env::default();
        let client = setup(&env);
        let user = Address::generate(&env);
        let asset = Symbol::new(&env, "XLM");
        client.deposit(&user, &100_i128, &asset);

        let result = client.try_withdraw(&user, &0_i128, &asset);
        assert_eq!(result, Err(Ok(Error::InvalidAmount)));
    }

    #[test]
    fn balances_are_tracked_per_asset() {
        let env = Env::default();
        let client = setup(&env);
        let user = Address::generate(&env);
        let xlm = Symbol::new(&env, "XLM");
        let usdc = Symbol::new(&env, "USDC");

        client.deposit(&user, &500_i128, &xlm);
        client.deposit(&user, &100_i128, &usdc);

        assert_eq!(client.get_balance(&user, &xlm), 500);
        assert_eq!(client.get_balance(&user, &usdc), 100);
    }

    #[test]
    fn balances_are_tracked_per_user() {
        let env = Env::default();
        let client = setup(&env);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let asset = Symbol::new(&env, "XLM");

        client.deposit(&alice, &500_i128, &asset);
        assert_eq!(client.get_balance(&alice, &asset), 500);
        assert_eq!(client.get_balance(&bob, &asset), 0);
    }

    #[test]
    fn get_balance_defaults_to_zero() {
        let env = Env::default();
        let client = setup(&env);
        let user = Address::generate(&env);
        let asset = Symbol::new(&env, "XLM");

        assert_eq!(client.get_balance(&user, &asset), 0);
    }

    #[test]
    fn double_initialize_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(Contract, ());
        let client = ContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        let result = client.try_initialize(&admin);
        assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
    }
}
