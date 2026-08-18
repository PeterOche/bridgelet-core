#![cfg(test)]

use super::*;
use soroban_sdk::{symbol_short, testutils::Address as _, testutils::Ledger, Address, Env, Symbol};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn setup() -> (Env, ComplianceOracleClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(ComplianceOracle, ());
    let client = ComplianceOracleClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let attestor = Address::generate(&env);
    client.initialize(&admin, &attestor);

    (env, client, admin, attestor)
}

fn status_clear(env: &Env) -> Symbol {
    Symbol::new(env, "CLEAR")
}

fn status_blocked(env: &Env) -> Symbol {
    Symbol::new(env, "BLOCKED")
}

// ─── initialize ──────────────────────────────────────────────────────────────

#[test]
fn test_initialize_twice_fails() {
    let (_env, client, admin, attestor) = setup();
    let err = client
        .try_initialize(&admin, &attestor)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::AlreadyInitialized);
}

// ─── attest ──────────────────────────────────────────────────────────────────

#[test]
fn test_attest_stores_status() {
    let (env, client, _admin, attestor) = setup();
    let target = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1000;

    client.attest(&attestor, &target, &status_clear(&env), &expiry);

    let (s, e) = client.status(&target).unwrap();
    assert_eq!(s, status_clear(&env));
    assert_eq!(e, expiry);
}

#[test]
fn test_attest_overwrites_previous_attestation() {
    let (env, client, _admin, attestor) = setup();
    let target = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1000;

    client.attest(&attestor, &target, &status_blocked(&env), &expiry);
    client.attest(&attestor, &target, &status_clear(&env), &(expiry + 500));

    let (s, e) = client.status(&target).unwrap();
    assert_eq!(s, status_clear(&env));
    assert_eq!(e, expiry + 500);
}

#[test]
fn test_attest_unauthorized_attestor_fails() {
    let (env, client, _admin, _attestor) = setup();
    let impostor = Address::generate(&env);
    let target = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1000;

    let err = client
        .try_attest(&impostor, &target, &status_clear(&env), &expiry)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::Unauthorized);
}

#[test]
fn test_attest_expiry_at_current_ledger_fails() {
    let (env, client, _admin, attestor) = setup();
    let target = Address::generate(&env);
    // expiry == current ledger sequence is not strictly greater → invalid
    let expiry = env.ledger().sequence();

    let err = client
        .try_attest(&attestor, &target, &status_clear(&env), &expiry)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::InvalidExpiry);
}

#[test]
fn test_attest_expiry_in_past_fails() {
    let (env, client, _admin, attestor) = setup();
    let target = Address::generate(&env);

    // Advance ledger a bit so we can supply an already-past expiry.
    env.ledger().set_sequence_number(100);
    let expiry = 50u32;

    let err = client
        .try_attest(&attestor, &target, &status_clear(&env), &expiry)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::InvalidExpiry);
}

// ─── status ───────────────────────────────────────────────────────────────────

#[test]
fn test_status_returns_none_for_unknown_address() {
    let (env, client, _admin, _attestor) = setup();
    let target = Address::generate(&env);
    assert!(client.status(&target).is_none());
}

#[test]
fn test_status_returns_none_after_expiry() {
    let (env, client, _admin, attestor) = setup();
    let target = Address::generate(&env);

    // Attest with expiry at ledger 200.
    env.ledger().set_sequence_number(100);
    client.attest(&attestor, &target, &status_clear(&env), &200);

    // Advance past the expiry.
    env.ledger().set_sequence_number(201);
    assert!(
        client.status(&target).is_none(),
        "stale attestation should be treated as absent"
    );
}

#[test]
fn test_status_returns_some_before_expiry() {
    let (env, client, _admin, attestor) = setup();
    let target = Address::generate(&env);

    env.ledger().set_sequence_number(100);
    client.attest(&attestor, &target, &status_clear(&env), &200);

    // Still within the validity window.
    env.ledger().set_sequence_number(150);
    let result = client.status(&target);
    assert!(result.is_some());
    let (s, _) = result.unwrap();
    assert_eq!(s, status_clear(&env));
}

#[test]
fn test_status_at_exact_expiry_ledger_is_valid() {
    let (env, client, _admin, attestor) = setup();
    let target = Address::generate(&env);

    env.ledger().set_sequence_number(100);
    client.attest(&attestor, &target, &status_clear(&env), &200);

    // At the exact expiry ledger, sequence == expiry, which is NOT > expiry.
    env.ledger().set_sequence_number(200);
    let result = client.status(&target);
    assert!(
        result.is_some(),
        "attestation should still be valid at its exact expiry ledger"
    );
}

// ─── set_attestor (rotation) ─────────────────────────────────────────────────

#[test]
fn test_attestor_rotation_allows_new_attestor() {
    let (env, client, admin, old_attestor) = setup();
    let new_attestor = Address::generate(&env);
    let target = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1000;

    client.set_attestor(&admin, &new_attestor);

    // Old attestor should now be rejected.
    let err = client
        .try_attest(&old_attestor, &target, &status_clear(&env), &expiry)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::Unauthorized);

    // New attestor should succeed.
    client.attest(&new_attestor, &target, &status_clear(&env), &expiry);
    let (s, _) = client.status(&target).unwrap();
    assert_eq!(s, status_clear(&env));
}

#[test]
fn test_set_attestor_unauthorized_fails() {
    let (env, client, _admin, _attestor) = setup();
    let impostor = Address::generate(&env);
    let new_attestor = Address::generate(&env);

    let err = client
        .try_set_attestor(&impostor, &new_attestor)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::Unauthorized);
}

#[test]
fn test_set_attestor_by_non_admin_fails() {
    let (env, client, _admin, attestor) = setup();
    let new_attestor = Address::generate(&env);

    // Current attestor (not admin) tries to rotate.
    let err = client
        .try_set_attestor(&attestor, &new_attestor)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::Unauthorized);
}

// ─── Acceptance-criteria smoke tests ─────────────────────────────────────────

/// Issue #468 AC: only the configured attestor can call `attest`.
#[test]
fn test_ac_only_attestor_can_attest() {
    let (env, client, _admin, attestor) = setup();
    let target = Address::generate(&env);
    let expiry = env.ledger().sequence() + 500;
    let random = Address::generate(&env);

    // Attestor succeeds.
    client.attest(&attestor, &target, &symbol_short!("CLEAR"), &expiry);

    // Random address fails.
    assert_eq!(
        client
            .try_attest(&random, &target, &symbol_short!("CLEAR"), &expiry)
            .unwrap_err()
            .unwrap(),
        Error::Unauthorized
    );
}

/// Issue #468 AC: `status` returns None once `expiry_ledger` has passed.
#[test]
fn test_ac_stale_attestation_returns_none() {
    let (env, client, _admin, attestor) = setup();
    let target = Address::generate(&env);

    env.ledger().set_sequence_number(50);
    client.attest(&attestor, &target, &symbol_short!("CLEAR"), &100);

    env.ledger().set_sequence_number(101);
    assert!(
        client.status(&target).is_none(),
        "stale attestations must be treated as absent"
    );
}

/// Issue #468 AC: unit tests cover attest, query, expiry, and attestor rotation.
#[test]
fn test_ac_full_lifecycle() {
    let (env, client, admin, attestor) = setup();
    let target = Address::generate(&env);

    env.ledger().set_sequence_number(100);

    // 1. Attest.
    client.attest(&attestor, &target, &symbol_short!("CLEAR"), &200);

    // 2. Query (valid).
    let (s, e) = client.status(&target).unwrap();
    assert_eq!(s, Symbol::new(&env, "CLEAR"));
    assert_eq!(e, 200u32);

    // 3. Expiry.
    env.ledger().set_sequence_number(201);
    assert!(client.status(&target).is_none());

    // 4. Rotate attestor.
    let new_attestor = Address::generate(&env);
    client.set_attestor(&admin, &new_attestor);

    // Re-attest with the new attestor.
    env.ledger().set_sequence_number(201);
    client.attest(&new_attestor, &target, &symbol_short!("CLEAR"), &400);
    assert!(client.status(&target).is_some());
}
