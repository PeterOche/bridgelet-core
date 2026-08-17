#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn setup() -> (Env, ClaimableBalanceRegistryClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(ClaimableBalanceRegistry, ());
    let client = ClaimableBalanceRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let writer = Address::generate(&env);
    client.initialize(&admin, &writer);

    (env, client, admin, writer)
}

// ─── initialize ──────────────────────────────────────────────────────────────

#[test]
fn test_initialize_twice_fails() {
    let (env, client, admin, writer) = setup();
    let err = client
        .try_initialize(&admin, &writer)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::AlreadyInitialized);
}

// ─── record ──────────────────────────────────────────────────────────────────

#[test]
fn test_record_stores_balance() {
    let (env, client, _admin, writer) = setup();
    let recovery = Address::generate(&env);
    let asset = Address::generate(&env);

    client.record(&writer, &recovery, &asset, &1000);
    assert_eq!(client.balance_of(&recovery, &asset), 1000i128);
}

#[test]
fn test_record_accumulates_balance() {
    let (env, client, _admin, writer) = setup();
    let recovery = Address::generate(&env);
    let asset = Address::generate(&env);

    client.record(&writer, &recovery, &asset, &500);
    client.record(&writer, &recovery, &asset, &300);
    assert_eq!(client.balance_of(&recovery, &asset), 800i128);
}

#[test]
fn test_record_different_assets_are_independent() {
    let (env, client, _admin, writer) = setup();
    let recovery = Address::generate(&env);
    let asset_a = Address::generate(&env);
    let asset_b = Address::generate(&env);

    client.record(&writer, &recovery, &asset_a, &100);
    client.record(&writer, &recovery, &asset_b, &200);

    assert_eq!(client.balance_of(&recovery, &asset_a), 100i128);
    assert_eq!(client.balance_of(&recovery, &asset_b), 200i128);
}

#[test]
fn test_record_unauthorized_writer_fails() {
    let (env, client, _admin, _writer) = setup();
    let impostor = Address::generate(&env);
    let recovery = Address::generate(&env);
    let asset = Address::generate(&env);

    let err = client
        .try_record(&impostor, &recovery, &asset, &100)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::Unauthorized);
}

#[test]
fn test_record_zero_amount_fails() {
    let (env, client, _admin, writer) = setup();
    let recovery = Address::generate(&env);
    let asset = Address::generate(&env);

    let err = client
        .try_record(&writer, &recovery, &asset, &0)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::InvalidAmount);
}

#[test]
fn test_record_negative_amount_fails() {
    let (env, client, _admin, writer) = setup();
    let recovery = Address::generate(&env);
    let asset = Address::generate(&env);

    let err = client
        .try_record(&writer, &recovery, &asset, &-50)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::InvalidAmount);
}

// ─── claim ────────────────────────────────────────────────────────────────────

#[test]
fn test_claim_returns_recorded_amount_and_zeroes_balance() {
    let (env, client, _admin, writer) = setup();
    let recovery = Address::generate(&env);
    let asset = Address::generate(&env);

    client.record(&writer, &recovery, &asset, &750);
    let claimed = client.claim(&recovery, &asset);

    assert_eq!(claimed, 750i128);
    assert_eq!(client.balance_of(&recovery, &asset), 0i128);
}

#[test]
fn test_double_claim_fails() {
    let (env, client, _admin, writer) = setup();
    let recovery = Address::generate(&env);
    let asset = Address::generate(&env);

    client.record(&writer, &recovery, &asset, &750);
    client.claim(&recovery, &asset);

    let err = client
        .try_claim(&recovery, &asset)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::NothingToClaim);
}

#[test]
fn test_claim_with_no_record_fails() {
    let (env, client, _admin, _writer) = setup();
    let recovery = Address::generate(&env);
    let asset = Address::generate(&env);

    let err = client
        .try_claim(&recovery, &asset)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::NothingToClaim);
}

#[test]
fn test_record_then_claim_then_record_again() {
    let (env, client, _admin, writer) = setup();
    let recovery = Address::generate(&env);
    let asset = Address::generate(&env);

    // First cycle
    client.record(&writer, &recovery, &asset, &400);
    assert_eq!(client.claim(&recovery, &asset), 400i128);
    assert_eq!(client.balance_of(&recovery, &asset), 0i128);

    // Second cycle — new record after previous claim
    client.record(&writer, &recovery, &asset, &200);
    assert_eq!(client.balance_of(&recovery, &asset), 200i128);
    assert_eq!(client.claim(&recovery, &asset), 200i128);
}

// ─── balance_of ──────────────────────────────────────────────────────────────

#[test]
fn test_balance_of_returns_zero_for_unknown() {
    let (env, client, _admin, _writer) = setup();
    let recovery = Address::generate(&env);
    let asset = Address::generate(&env);

    assert_eq!(client.balance_of(&recovery, &asset), 0i128);
}

// ─── set_writer ───────────────────────────────────────────────────────────────

#[test]
fn test_set_writer_allows_new_writer() {
    let (env, client, admin, old_writer) = setup();
    let new_writer = Address::generate(&env);
    let recovery = Address::generate(&env);
    let asset = Address::generate(&env);

    client.set_writer(&admin, &new_writer);

    // Old writer should now be rejected.
    let err = client
        .try_record(&old_writer, &recovery, &asset, &100)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::Unauthorized);

    // New writer should succeed.
    client.record(&new_writer, &recovery, &asset, &100);
    assert_eq!(client.balance_of(&recovery, &asset), 100i128);
}

#[test]
fn test_set_writer_unauthorized_fails() {
    let (env, client, _admin, _writer) = setup();
    let impostor = Address::generate(&env);
    let new_writer = Address::generate(&env);

    let err = client
        .try_set_writer(&impostor, &new_writer)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::Unauthorized);
}
