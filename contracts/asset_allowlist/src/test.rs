#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn setup() -> (Env, AssetAllowlistClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AssetAllowlist, ());
    let client = AssetAllowlistClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

// ── initialize ───────────────────────────────────────────────────────────────

#[test]
fn initialize_succeeds_once() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AssetAllowlist, ());
    let client = AssetAllowlistClient::new(&env, &contract_id);
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

// ── allow ─────────────────────────────────────────────────────────────────────

#[test]
fn allow_new_asset_succeeds() {
    let (env, client, admin) = setup();
    let asset = Address::generate(&env);

    let result = client.try_allow(&admin, &asset);
    assert!(result.is_ok());
    assert!(client.is_allowed(&asset));
}

#[test]
fn allow_non_admin_returns_unauthorized() {
    let (env, client, _admin) = setup();
    let intruder = Address::generate(&env);
    let asset = Address::generate(&env);

    let result = client.try_allow(&intruder, &asset);
    assert_eq!(result.unwrap_err().unwrap(), Error::Unauthorized);
}

#[test]
fn allow_updates_list() {
    let (env, client, admin) = setup();
    let asset = Address::generate(&env);

    client.allow(&admin, &asset);

    let list = client.list();
    assert_eq!(list.len(), 1);
    assert_eq!(list.get(0).unwrap(), asset);
}

/// Duplicate `allow` calls for an already-allowed asset must be a no-op,
/// not an error (acceptance criterion from issue #462).
#[test]
fn allow_idempotent_no_duplicate_in_list() {
    let (env, client, admin) = setup();
    let asset = Address::generate(&env);

    client.allow(&admin, &asset);
    // Allow the same asset again — must succeed and list must not grow.
    let result = client.try_allow(&admin, &asset);
    assert!(result.is_ok());

    assert_eq!(client.list().len(), 1);
    assert!(client.is_allowed(&asset));
}

// ── disallow ──────────────────────────────────────────────────────────────────

#[test]
fn disallow_existing_asset_succeeds() {
    let (env, client, admin) = setup();
    let asset = Address::generate(&env);

    client.allow(&admin, &asset);
    assert!(client.is_allowed(&asset));

    client.disallow(&admin, &asset);
    assert!(!client.is_allowed(&asset));
}

#[test]
fn disallow_non_admin_returns_unauthorized() {
    let (env, client, admin) = setup();
    let intruder = Address::generate(&env);
    let asset = Address::generate(&env);

    client.allow(&admin, &asset);

    let result = client.try_disallow(&intruder, &asset);
    assert_eq!(result.unwrap_err().unwrap(), Error::Unauthorized);
}

#[test]
fn disallow_updates_list() {
    let (env, client, admin) = setup();
    let asset = Address::generate(&env);

    client.allow(&admin, &asset);
    client.disallow(&admin, &asset);

    assert_eq!(client.list().len(), 0);
}

#[test]
fn disallow_nonexistent_asset_is_noop() {
    let (env, client, admin) = setup();
    let asset = Address::generate(&env);

    // disallow without having allowed first — should not panic
    let result = client.try_disallow(&admin, &asset);
    assert!(result.is_ok());
    assert!(!client.is_allowed(&asset));
}

// ── is_allowed ────────────────────────────────────────────────────────────────

/// Query for a never-allowed asset must return false (acceptance criterion).
#[test]
fn is_allowed_returns_false_for_unknown_asset() {
    let (env, client, _admin) = setup();
    let asset = Address::generate(&env);
    assert!(!client.is_allowed(&asset));
}

// ── list ──────────────────────────────────────────────────────────────────────

#[test]
fn list_empty_before_any_allow() {
    let (_, client, _) = setup();
    assert_eq!(client.list().len(), 0);
}

#[test]
fn list_contains_all_allowed_assets() {
    let (env, client, admin) = setup();
    let asset_a = Address::generate(&env);
    let asset_b = Address::generate(&env);
    let asset_c = Address::generate(&env);

    client.allow(&admin, &asset_a);
    client.allow(&admin, &asset_b);
    client.allow(&admin, &asset_c);

    assert_eq!(client.list().len(), 3);
}

#[test]
fn list_excludes_disallowed_asset() {
    let (env, client, admin) = setup();
    let asset_a = Address::generate(&env);
    let asset_b = Address::generate(&env);

    client.allow(&admin, &asset_a);
    client.allow(&admin, &asset_b);
    client.disallow(&admin, &asset_a);

    let list = client.list();
    assert_eq!(list.len(), 1);
    assert_eq!(list.get(0).unwrap(), asset_b);
}

// ── re-allow after disallow ───────────────────────────────────────────────────

#[test]
fn reallow_after_disallow_works() {
    let (env, client, admin) = setup();
    let asset = Address::generate(&env);

    client.allow(&admin, &asset);
    client.disallow(&admin, &asset);
    assert!(!client.is_allowed(&asset));

    client.allow(&admin, &asset);
    assert!(client.is_allowed(&asset));
    assert_eq!(client.list().len(), 1);
}
