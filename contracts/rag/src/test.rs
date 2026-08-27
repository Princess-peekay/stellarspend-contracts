use super::*;
use soroban_sdk::{testutils::Address as _, Env};

#[test]
fn test_initialize_sets_admin_and_default_config() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);

    let res = AdminManager::initialize(&env, admin.clone());
    assert!(res.is_ok());

    assert_eq!(AdminManager::get_admin(&env).unwrap(), admin);
    assert_eq!(AdminManager::get_config(&env).unwrap().paused, false);
}

#[test]
fn test_initialize_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);

    AdminManager::initialize(&env, admin).unwrap();

    assert_eq!(env.events().all().len(), 1);
}

#[test]
fn test_reinitialize_fails_and_admin_unchanged() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let other = Address::generate(&env);

    AdminManager::initialize(&env, admin.clone()).unwrap();

    // Re-initialization must be rejected, even with a different admin.
    let res = AdminManager::initialize(&env, other);
    assert!(res.is_err());

    // Original admin must remain unchanged.
    assert_eq!(AdminManager::get_admin(&env).unwrap(), admin);

    // Re-initialization must not overwrite config either.
    assert_eq!(AdminManager::get_config(&env).unwrap().paused, false);
}

#[test]
#[should_panic]
fn test_initialize_without_auth_fails() {
    let env = Env::default();
    // Deliberately no env.mock_all_auths() — admin has not authorized
    // this call, so require_auth() must panic.
    let admin = Address::generate(&env);

    let _ = AdminManager::initialize(&env, admin);
}

#[test]
fn test_get_admin_before_init_fails() {
    let env = Env::default();
    let res = AdminManager::get_admin(&env);
    assert!(res.is_err());
}

#[test]
fn test_get_config_before_init_fails() {
    let env = Env::default();
    let res = AdminManager::get_config(&env);
    assert!(res.is_err());
}

#[test]
fn test_require_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let stranger = Address::generate(&env);

    AdminManager::initialize(&env, admin.clone()).unwrap();

    assert!(AdminManager::require_admin(&env, &admin).is_ok());
    assert!(AdminManager::require_admin(&env, &stranger).is_err());
}