#![cfg(test)]

use super::*;
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn setup() -> (Env, AllowlistRegistryClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AllowlistRegistry, ());
    let client = AllowlistRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

// ── initialize ───────────────────────────────────────────────────────────────

#[test]
fn initialize_succeeds_once() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AllowlistRegistry, ());
    let client = AllowlistRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    let result = client.try_initialize(&admin);
    assert!(result.is_ok());
    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
fn initialize_twice_returns_error() {
    let (_, client, admin) = setup();

    let result = client.try_initialize(&admin);
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::AlreadyInitialized,
        "second initialize must return AlreadyInitialized"
    );
}

// ── add ──────────────────────────────────────────────────────────────────────

#[test]
fn add_new_address_succeeds() {
    let (env, client, admin) = setup();
    let addr = Address::generate(&env);

    let result = client.try_add(&admin, &addr, &symbol_short!("wallet"));
    assert!(result.is_ok());
    assert!(client.is_allowed(&addr));
}

#[test]
fn add_non_admin_returns_unauthorized() {
    let (env, client, _admin) = setup();
    let intruder = Address::generate(&env);
    let addr = Address::generate(&env);

    let result = client.try_add(&intruder, &addr, &symbol_short!("x"));
    assert_eq!(result.unwrap_err().unwrap(), Error::Unauthorized);
}

#[test]
fn add_updates_list() {
    let (env, client, admin) = setup();
    let addr = Address::generate(&env);

    client.add(&admin, &addr, &symbol_short!("dest"));

    let list = client.list();
    assert_eq!(list.len(), 1);
    assert_eq!(list.get(0).unwrap(), addr);
}

#[test]
fn add_idempotent_no_duplicate_in_list() {
    let (env, client, admin) = setup();
    let addr = Address::generate(&env);

    client.add(&admin, &addr, &symbol_short!("first"));
    // Add the same address again; list must not grow.
    client.add(&admin, &addr, &symbol_short!("second"));

    assert_eq!(client.list().len(), 1);
    assert!(client.is_allowed(&addr));
}

// ── remove ───────────────────────────────────────────────────────────────────

#[test]
fn remove_existing_address_succeeds() {
    let (env, client, admin) = setup();
    let addr = Address::generate(&env);

    client.add(&admin, &addr, &symbol_short!("wallet"));
    assert!(client.is_allowed(&addr));

    client.remove(&admin, &addr);
    assert!(!client.is_allowed(&addr));
}

#[test]
fn remove_non_admin_returns_unauthorized() {
    let (env, client, admin) = setup();
    let intruder = Address::generate(&env);
    let addr = Address::generate(&env);

    client.add(&admin, &addr, &symbol_short!("wallet"));

    let result = client.try_remove(&intruder, &addr);
    assert_eq!(result.unwrap_err().unwrap(), Error::Unauthorized);
}

#[test]
fn remove_updates_list() {
    let (env, client, admin) = setup();
    let addr = Address::generate(&env);

    client.add(&admin, &addr, &symbol_short!("dest"));
    client.remove(&admin, &addr);

    assert_eq!(client.list().len(), 0);
}

#[test]
fn remove_nonexistent_address_is_noop() {
    let (env, client, admin) = setup();
    let addr = Address::generate(&env);

    // remove without having added first — should not panic
    let result = client.try_remove(&admin, &addr);
    assert!(result.is_ok());
    assert!(!client.is_allowed(&addr));
}

// ── re-add after remove ──────────────────────────────────────────────────────

#[test]
fn readd_after_remove_works() {
    let (env, client, admin) = setup();
    let addr = Address::generate(&env);

    client.add(&admin, &addr, &symbol_short!("dest"));
    client.remove(&admin, &addr);
    assert!(!client.is_allowed(&addr));

    client.add(&admin, &addr, &symbol_short!("dest2"));
    assert!(client.is_allowed(&addr));
    assert_eq!(client.list().len(), 1);
}

// ── is_allowed ───────────────────────────────────────────────────────────────

#[test]
fn is_allowed_returns_false_for_unknown_address() {
    let (env, client, _admin) = setup();
    let addr = Address::generate(&env);
    assert!(!client.is_allowed(&addr));
}

// ── list ─────────────────────────────────────────────────────────────────────

#[test]
fn list_empty_before_any_add() {
    let (_, client, _) = setup();
    assert_eq!(client.list().len(), 0);
}

#[test]
fn list_contains_all_added_addresses() {
    let (env, client, admin) = setup();
    let addr_a = Address::generate(&env);
    let addr_b = Address::generate(&env);
    let addr_c = Address::generate(&env);

    client.add(&admin, &addr_a, &symbol_short!("a"));
    client.add(&admin, &addr_b, &symbol_short!("b"));
    client.add(&admin, &addr_c, &symbol_short!("c"));

    let list = client.list();
    assert_eq!(list.len(), 3);
}

#[test]
fn list_excludes_removed_address() {
    let (env, client, admin) = setup();
    let addr_a = Address::generate(&env);
    let addr_b = Address::generate(&env);

    client.add(&admin, &addr_a, &symbol_short!("a"));
    client.add(&admin, &addr_b, &symbol_short!("b"));
    client.remove(&admin, &addr_a);

    let list = client.list();
    assert_eq!(list.len(), 1);
    assert_eq!(list.get(0).unwrap(), addr_b);
}
