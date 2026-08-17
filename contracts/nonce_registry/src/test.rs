#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn setup() -> (Env, NonceRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(NonceRegistry, ());
    let client = NonceRegistryClient::new(&env, &contract_id);
    (env, client)
}

// ── consume ──────────────────────────────────────────────────────────────────

#[test]
fn consume_new_nonce_succeeds() {
    let (_, client) = setup();
    let signer = Address::generate(&client.env);
    let result = client.try_consume(&signer, &0u64);
    assert!(result.is_ok());
}

#[test]
fn consume_replay_returns_error() {
    let (_, client) = setup();
    let signer = Address::generate(&client.env);

    client.consume(&signer, &42u64);

    let result = client.try_consume(&signer, &42u64);
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::NonceAlreadyConsumed,
        "second consume must return NonceAlreadyConsumed"
    );
}

#[test]
fn consume_different_nonces_both_succeed() {
    let (_, client) = setup();
    let signer = Address::generate(&client.env);

    client.consume(&signer, &0u64);
    client.consume(&signer, &1u64);
}

#[test]
fn consume_same_nonce_different_signers_both_succeed() {
    let (_, client) = setup();
    let signer_a = Address::generate(&client.env);
    let signer_b = Address::generate(&client.env);

    client.consume(&signer_a, &0u64);
    client.consume(&signer_b, &0u64);
}

// ── is_consumed ──────────────────────────────────────────────────────────────

#[test]
fn is_consumed_returns_false_before_consume() {
    let (_, client) = setup();
    let signer = Address::generate(&client.env);
    assert!(!client.is_consumed(&signer, &99u64));
}

#[test]
fn is_consumed_returns_true_after_consume() {
    let (_, client) = setup();
    let signer = Address::generate(&client.env);

    client.consume(&signer, &5u64);

    assert!(client.is_consumed(&signer, &5u64));
}

#[test]
fn is_consumed_returns_false_for_adjacent_nonce() {
    let (_, client) = setup();
    let signer = Address::generate(&client.env);

    client.consume(&signer, &5u64);

    assert!(!client.is_consumed(&signer, &6u64));
}

// ── next_nonce ───────────────────────────────────────────────────────────────

#[test]
fn next_nonce_starts_at_zero() {
    let (_, client) = setup();
    let signer = Address::generate(&client.env);
    assert_eq!(client.next_nonce(&signer), 0);
}

#[test]
fn next_nonce_advances_after_consume() {
    let (_, client) = setup();
    let signer = Address::generate(&client.env);

    client.consume(&signer, &0u64);

    assert_eq!(
        client.next_nonce(&signer),
        1,
        "next_nonce must be 1 after consuming nonce 0"
    );
}

#[test]
fn next_nonce_is_monotonically_increasing() {
    let (_, client) = setup();
    let signer = Address::generate(&client.env);

    for n in 0u64..5 {
        assert_eq!(client.next_nonce(&signer), n);
        client.consume(&signer, &n);
    }
    assert_eq!(client.next_nonce(&signer), 5);
}

#[test]
fn next_nonce_advances_past_gap() {
    let (_, client) = setup();
    let signer = Address::generate(&client.env);

    // Skip nonces 0-9 and consume nonce 10 directly.
    client.consume(&signer, &10u64);

    assert_eq!(
        client.next_nonce(&signer),
        11,
        "next_nonce must jump to 11 after consuming nonce 10 directly"
    );
}

#[test]
fn next_nonce_does_not_regress_when_lower_nonce_consumed() {
    let (_, client) = setup();
    let signer = Address::generate(&client.env);

    // Consume a high nonce first.
    client.consume(&signer, &100u64);
    assert_eq!(client.next_nonce(&signer), 101);

    // Consuming an older nonce must not reduce next_nonce.
    client.consume(&signer, &5u64);
    assert_eq!(
        client.next_nonce(&signer),
        101,
        "next_nonce must not regress after consuming an older nonce"
    );
}

#[test]
fn next_nonce_independent_per_signer() {
    let (_, client) = setup();
    let signer_a = Address::generate(&client.env);
    let signer_b = Address::generate(&client.env);

    client.consume(&signer_a, &7u64);

    assert_eq!(client.next_nonce(&signer_a), 8);
    assert_eq!(
        client.next_nonce(&signer_b),
        0,
        "signer_b's counter must be unaffected by signer_a's consumes"
    );
}
