#![cfg(test)]

use super::*;
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

fn setup() -> (Env, AccessControllerClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AccessController, ());
    let client = AccessControllerClient::new(&env, &contract_id);
    let super_admin = Address::generate(&env);
    client.initialize(&super_admin);
    (env, client, super_admin)
}

// ── initialize ──────────────────────────────────────────────────────────────

#[test]
fn initialize_succeeds() {
    let (_env, client, super_admin) = setup();
    assert_eq!(client.get_super_admin(), Some(super_admin));
}

#[test]
fn initialize_twice_returns_already_initialized() {
    let (env, client, _super_admin) = setup();
    let another = Address::generate(&env);
    assert_eq!(
        client.try_initialize(&another),
        Err(Ok(Error::AlreadyInitialized))
    );
}

// ── grant_role ──────────────────────────────────────────────────────────────

#[test]
fn super_admin_can_grant_role() {
    let (env, client, super_admin) = setup();
    let operator = Address::generate(&env);

    client.grant_role(&super_admin, &symbol_short!("operator"), &operator);
    assert!(client.has_role(&symbol_short!("operator"), &operator));
}

#[test]
fn admin_can_grant_role() {
    let (env, client, super_admin) = setup();
    let admin = Address::generate(&env);
    let operator = Address::generate(&env);

    client.grant_role(&super_admin, &symbol_short!("admin"), &admin);
    client.grant_role(&admin, &symbol_short!("operator"), &operator);
    assert!(client.has_role(&symbol_short!("operator"), &operator));
}

#[test]
fn unauthorized_grant_returns_error() {
    let (env, client, _super_admin) = setup();
    let nobody = Address::generate(&env);
    let target = Address::generate(&env);

    let result = client.try_grant_role(&nobody, &symbol_short!("operator"), &target);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

// ── revoke_role ─────────────────────────────────────────────────────────────

#[test]
fn super_admin_can_revoke_role() {
    let (env, client, super_admin) = setup();
    let operator = Address::generate(&env);

    client.grant_role(&super_admin, &symbol_short!("operator"), &operator);
    assert!(client.has_role(&symbol_short!("operator"), &operator));

    client.revoke_role(&super_admin, &symbol_short!("operator"), &operator);
    assert!(!client.has_role(&symbol_short!("operator"), &operator));
}

#[test]
fn revoke_nonexistent_role_is_noop() {
    let (env, client, super_admin) = setup();
    let nobody = Address::generate(&env);

    client.revoke_role(&super_admin, &symbol_short!("operator"), &nobody);
}

#[test]
fn unauthorized_revoke_returns_error() {
    let (env, client, super_admin) = setup();
    let operator = Address::generate(&env);
    let nobody = Address::generate(&env);

    client.grant_role(&super_admin, &symbol_short!("operator"), &operator);

    let result = client.try_revoke_role(&nobody, &symbol_short!("operator"), &operator);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn cannot_revoke_super_admin() {
    let (env, client, super_admin) = setup();

    let result = client.try_revoke_role(
        &super_admin,
        &Symbol::new(&env, "super_admin"),
        &super_admin,
    );
    assert_eq!(result, Err(Ok(Error::CannotRevokeSuperAdmin)));
}

// ── re-grant ────────────────────────────────────────────────────────────────

#[test]
fn re_grant_same_role_works() {
    let (env, client, super_admin) = setup();
    let operator = Address::generate(&env);

    client.grant_role(&super_admin, &symbol_short!("operator"), &operator);
    client.revoke_role(&super_admin, &symbol_short!("operator"), &operator);
    assert!(!client.has_role(&symbol_short!("operator"), &operator));

    client.grant_role(&super_admin, &symbol_short!("operator"), &operator);
    assert!(client.has_role(&symbol_short!("operator"), &operator));
}

// ── has_role ────────────────────────────────────────────────────────────────

#[test]
fn has_role_false_for_ungranted() {
    let (env, client, _super_admin) = setup();
    let nobody = Address::generate(&env);
    assert!(!client.has_role(&symbol_short!("operator"), &nobody));
}

#[test]
fn has_role_independent_across_roles() {
    let (env, client, super_admin) = setup();
    let account = Address::generate(&env);

    client.grant_role(&super_admin, &symbol_short!("pauser"), &account);
    assert!(client.has_role(&symbol_short!("pauser"), &account));
    assert!(!client.has_role(&symbol_short!("operator"), &account));
}
