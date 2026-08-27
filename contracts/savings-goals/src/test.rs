#[cfg(test)]
mod tests {
    use crate::{Contract, ContractClient, Error, ScheduleStatus};
    use soroban_sdk::{
        testutils::{Address as _, Events, Ledger},
        Address, Env, Symbol,
    };

    fn setup(env: &Env) -> (ContractClient<'static>, Address, Address) {
        env.mock_all_auths();
        let contract_id = env.register(Contract, ());
        let client = ContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let user = Address::generate(env);
        client.initialize(&admin);
        (client, admin, user)
    }

    fn xlm(env: &Env) -> Symbol {
        Symbol::new(env, "XLM")
    }

    #[test]
    fn create_and_get_goal() {
        let env = Env::default();
        let (client, _admin, user) = setup(&env);
        let name = Symbol::new(&env, "vacation");
        let asset = xlm(&env);

        let goal_id = client.create_goal(&user, &name, &1000_i128, &asset, &2_000_000_000_u64);
        assert_eq!(goal_id, 1);

        let goal = client.get_goal(&user, &goal_id);
        assert_eq!(goal.goal_id, 1);
        assert_eq!(goal.user, user);
        assert_eq!(goal.target, 1000);
        assert_eq!(goal.current_amount, 0);
        assert!(!goal.is_complete);
        assert_eq!(goal.schedule_status, ScheduleStatus::Active);
    }

    #[test]
    fn create_goal_ids_increment_per_contract() {
        let env = Env::default();
        let (client, _admin, user) = setup(&env);
        let asset = xlm(&env);

        let id1 = client.create_goal(
            &user,
            &Symbol::new(&env, "vacation"),
            &1000_i128,
            &asset,
            &2_000_000_000_u64,
        );
        let id2 = client.create_goal(
            &user,
            &Symbol::new(&env, "car"),
            &5000_i128,
            &asset,
            &2_000_000_000_u64,
        );

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn contribute_below_target_does_not_complete() {
        let env = Env::default();
        let (client, _admin, user) = setup(&env);
        let asset = xlm(&env);
        let goal_id = client.create_goal(
            &user,
            &Symbol::new(&env, "vacation"),
            &1000_i128,
            &asset,
            &2_000_000_000_u64,
        );

        client.contribute(&user, &goal_id, &600_i128);

        let goal = client.get_goal(&user, &goal_id);
        assert_eq!(goal.current_amount, 600);
        assert!(!goal.is_complete);
    }

    #[test]
    fn contribute_crossing_target_completes_and_emits_milestone() {
        let env = Env::default();
        let (client, _admin, user) = setup(&env);
        let asset = xlm(&env);
        let goal_id = client.create_goal(
            &user,
            &Symbol::new(&env, "vacation"),
            &1000_i128,
            &asset,
            &2_000_000_000_u64,
        );

        client.contribute(&user, &goal_id, &600_i128);
        client.contribute(&user, &goal_id, &400_i128);

        let goal = client.get_goal(&user, &goal_id);
        assert_eq!(goal.current_amount, 1000);
        assert!(goal.is_complete);

        // A milestone event must have been published when the target was
        // crossed by the second contribution.
        let events = env.events().all();
        assert!(!events.is_empty());
    }

    #[test]
    fn milestone_is_only_emitted_once() {
        let env = Env::default();
        let (client, _admin, user) = setup(&env);
        let asset = xlm(&env);
        let goal_id = client.create_goal(
            &user,
            &Symbol::new(&env, "vacation"),
            &1000_i128,
            &asset,
            &2_000_000_000_u64,
        );

        client.contribute(&user, &goal_id, &1000_i128);
        let events_after_first_completion = env.events().all().len();

        // Contributing again after the goal is already complete must not
        // publish a second milestone event.
        client.contribute(&user, &goal_id, &100_i128);
        let events_after_second_contribution = env.events().all().len();

        assert_eq!(
            events_after_first_completion,
            events_after_second_contribution
        );

        let goal = client.get_goal(&user, &goal_id);
        assert_eq!(goal.current_amount, 1100);
        assert!(goal.is_complete);
    }

    #[test]
    fn contribution_history_records_each_contribution_in_order() {
        let env = Env::default();
        let (client, _admin, user) = setup(&env);
        let asset = xlm(&env);
        let goal_id = client.create_goal(
            &user,
            &Symbol::new(&env, "vacation"),
            &1000_i128,
            &asset,
            &2_000_000_000_u64,
        );

        client.contribute(&user, &goal_id, &600_i128);
        client.contribute(&user, &goal_id, &400_i128);

        let history = client.get_contribution_history(&goal_id, &user);
        assert_eq!(history.len(), 2);
        assert_eq!(history.get(0).unwrap().amount, 600);
        assert_eq!(history.get(1).unwrap().amount, 400);
        assert_eq!(history.get(0).unwrap().contribution_id, 1);
        assert_eq!(history.get(1).unwrap().contribution_id, 2);
    }

    #[test]
    fn get_all_goals_returns_every_goal_for_user() {
        let env = Env::default();
        let (client, _admin, user) = setup(&env);
        let asset = xlm(&env);
        client.create_goal(
            &user,
            &Symbol::new(&env, "vacation"),
            &1000_i128,
            &asset,
            &2_000_000_000_u64,
        );
        client.create_goal(
            &user,
            &Symbol::new(&env, "car"),
            &5000_i128,
            &asset,
            &2_000_000_000_u64,
        );

        let goals = client.get_all_goals(&user);
        assert_eq!(goals.len(), 2);
    }

    #[test]
    fn get_all_goals_is_empty_for_a_user_with_no_goals() {
        let env = Env::default();
        let (client, _admin, _user) = setup(&env);
        let other = Address::generate(&env);

        let goals = client.get_all_goals(&other);
        assert_eq!(goals.len(), 0);
    }

    #[test]
    fn set_round_up_rule_enables_and_disables() {
        let env = Env::default();
        let (client, _admin, user) = setup(&env);
        let asset = xlm(&env);
        let goal_id = client.create_goal(
            &user,
            &Symbol::new(&env, "vacation"),
            &1000_i128,
            &asset,
            &2_000_000_000_u64,
        );

        client.set_round_up_rule(&user, &goal_id, &true, &5_i128);
        let goal = client.get_goal(&user, &goal_id);
        assert!(goal.round_up_enabled);
        assert_eq!(goal.round_up_nearest_unit, 5);

        client.set_round_up_rule(&user, &goal_id, &false, &5_i128);
        let goal = client.get_goal(&user, &goal_id);
        assert!(!goal.round_up_enabled);
        assert_eq!(goal.round_up_nearest_unit, 0);
    }

    #[test]
    fn pause_resume_cancel_schedule_transitions() {
        let env = Env::default();
        let (client, _admin, user) = setup(&env);
        let asset = xlm(&env);
        let goal_id = client.create_goal(
            &user,
            &Symbol::new(&env, "vacation"),
            &1000_i128,
            &asset,
            &2_000_000_000_u64,
        );

        client.pause_schedule(&user, &goal_id);
        assert_eq!(
            client.get_goal(&user, &goal_id).schedule_status,
            ScheduleStatus::Paused
        );

        client.resume_schedule(&user, &goal_id);
        assert_eq!(
            client.get_goal(&user, &goal_id).schedule_status,
            ScheduleStatus::Active
        );

        client.cancel_schedule(&user, &goal_id);
        assert_eq!(
            client.get_goal(&user, &goal_id).schedule_status,
            ScheduleStatus::Cancelled
        );
    }

    #[test]
    fn get_goal_rejects_non_owner() {
        let env = Env::default();
        let (client, _admin, user) = setup(&env);
        let other = Address::generate(&env);
        let asset = xlm(&env);
        let goal_id = client.create_goal(
            &user,
            &Symbol::new(&env, "vacation"),
            &1000_i128,
            &asset,
            &2_000_000_000_u64,
        );

        let result = client.try_get_goal(&other, &goal_id);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    #[test]
    fn contribute_rejects_non_owner() {
        let env = Env::default();
        let (client, _admin, user) = setup(&env);
        let other = Address::generate(&env);
        let asset = xlm(&env);
        let goal_id = client.create_goal(
            &user,
            &Symbol::new(&env, "vacation"),
            &1000_i128,
            &asset,
            &2_000_000_000_u64,
        );

        let result = client.try_contribute(&other, &goal_id, &100_i128);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    #[test]
    fn contribute_rejects_unknown_goal() {
        let env = Env::default();
        let (client, _admin, user) = setup(&env);

        let result = client.try_contribute(&user, &999_u64, &100_i128);
        assert_eq!(result, Err(Ok(Error::GoalNotFound)));
    }

    #[test]
    fn create_goal_rejects_non_positive_target() {
        let env = Env::default();
        let (client, _admin, user) = setup(&env);
        let asset = xlm(&env);

        let result = client.try_create_goal(
            &user,
            &Symbol::new(&env, "vacation"),
            &0_i128,
            &asset,
            &2_000_000_000_u64,
        );
        assert_eq!(result, Err(Ok(Error::InvalidAmount)));
    }

    #[test]
    fn create_goal_rejects_past_deadline() {
        let env = Env::default();
        env.ledger().set_timestamp(1_000_000);
        let (client, _admin, user) = setup(&env);
        let asset = xlm(&env);

        let result = client.try_create_goal(
            &user,
            &Symbol::new(&env, "vacation"),
            &1000_i128,
            &asset,
            &500_000_u64,
        );
        assert_eq!(result, Err(Ok(Error::InvalidDeadline)));
    }

    #[test]
    fn set_round_up_rule_rejects_non_positive_unit_when_enabling() {
        let env = Env::default();
        let (client, _admin, user) = setup(&env);
        let asset = xlm(&env);
        let goal_id = client.create_goal(
            &user,
            &Symbol::new(&env, "vacation"),
            &1000_i128,
            &asset,
            &2_000_000_000_u64,
        );

        let result = client.try_set_round_up_rule(&user, &goal_id, &true, &0_i128);
        assert_eq!(result, Err(Ok(Error::InvalidRoundUpUnit)));

        // Disabling never validates the unit, even if it is non-positive.
        client.set_round_up_rule(&user, &goal_id, &false, &0_i128);
        let goal = client.get_goal(&user, &goal_id);
        assert!(!goal.round_up_enabled);
    }

    #[test]
    fn pause_schedule_rejects_non_owner() {
        let env = Env::default();
        let (client, _admin, user) = setup(&env);
        let other = Address::generate(&env);
        let asset = xlm(&env);
        let goal_id = client.create_goal(
            &user,
            &Symbol::new(&env, "vacation"),
            &1000_i128,
            &asset,
            &2_000_000_000_u64,
        );

        let result = client.try_pause_schedule(&other, &goal_id);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
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
