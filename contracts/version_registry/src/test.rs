#![cfg(test)]

use super::*;
use soroban_sdk::{
    symbol_short, testutils::Address as _, testutils::Ledger, Address, BytesN, Env, Symbol,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn setup() -> (Env, VersionRegistryClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(VersionRegistry, ());
    let client = VersionRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

fn sample_hash(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[1u8; 32])
}

fn sample_hash_2(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[2u8; 32])
}

fn zero_hash(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0u8; 32])
}

// ── initialize ───────────────────────────────────────────────────────────────

#[test]
fn initialize_succeeds_once() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(VersionRegistry, ());
    let client = VersionRegistryClient::new(&env, &contract_id);
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

// ── publish ──────────────────────────────────────────────────────────────────

#[test]
fn publish_succeeds() {
    let (env, client, admin) = setup();
    let name = Symbol::new(&env, "my_contract");
    let hash = sample_hash(&env);
    let version = symbol_short!("v1");

    let result = client.try_publish(&admin, &name, &hash, &version);
    assert!(result.is_ok());

    let current = client.current(&name);
    assert!(current.is_some());
    let (current_hash, current_version) = current.unwrap();
    assert_eq!(current_hash, hash);
    assert_eq!(current_version, version);
}

#[test]
fn publish_non_admin_returns_unauthorized() {
    let (env, client, _admin) = setup();
    let intruder = Address::generate(&env);
    let name = Symbol::new(&env, "my_contract");
    let hash = sample_hash(&env);
    let version = symbol_short!("v1");

    let result = client.try_publish(&intruder, &name, &hash, &version);
    assert_eq!(result.unwrap_err().unwrap(), Error::Unauthorized);
}

#[test]
fn publish_before_init_returns_not_initialized() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(VersionRegistry, ());
    let client = VersionRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    let name = Symbol::new(&env, "my_contract");
    let hash = sample_hash(&env);
    let version = symbol_short!("v1");

    let result = client.try_publish(&admin, &name, &hash, &version);
    assert_eq!(result.unwrap_err().unwrap(), Error::NotInitialized);
}

#[test]
fn publish_zero_hash_returns_invalid_wasm_hash() {
    let (env, client, admin) = setup();
    let name = Symbol::new(&env, "my_contract");
    let hash = zero_hash(&env);
    let version = symbol_short!("v1");

    let result = client.try_publish(&admin, &name, &hash, &version);
    assert_eq!(result.unwrap_err().unwrap(), Error::InvalidWasmHash);
}

#[test]
fn publish_updates_history() {
    let (env, client, admin) = setup();
    let name = Symbol::new(&env, "my_contract");
    let hash = sample_hash(&env);
    let version = symbol_short!("v1");

    client.publish(&admin, &name, &hash, &version);

    let history = client.history(&name);
    assert_eq!(history.len(), 1);
    let (h, v, _seq) = history.get(0).unwrap();
    assert_eq!(h, hash);
    assert_eq!(v, version);
}

#[test]
fn publish_multiple_versions_for_same_name() {
    let (env, client, admin) = setup();
    let name = Symbol::new(&env, "my_contract");
    let hash1 = sample_hash(&env);
    let version1 = symbol_short!("v1");
    let hash2 = sample_hash_2(&env);
    let version2 = symbol_short!("v2");

    client.publish(&admin, &name, &hash1, &version1);
    client.publish(&admin, &name, &hash2, &version2);

    // current should return the latest
    let current = client.current(&name).unwrap();
    assert_eq!(current.0, hash2);
    assert_eq!(current.1, version2);

    // history should have both entries in order
    let history = client.history(&name);
    assert_eq!(history.len(), 2);

    let (h1, v1, _seq1) = history.get(0).unwrap();
    assert_eq!(h1, hash1);
    assert_eq!(v1, version1);

    let (h2, v2, _seq2) = history.get(1).unwrap();
    assert_eq!(h2, hash2);
    assert_eq!(v2, version2);
}

#[test]
fn history_returns_publish_order_with_ledger_sequence() {
    let (env, client, admin) = setup();
    let name = Symbol::new(&env, "my_contract");

    env.ledger().with_mut(|li| {
        li.sequence_number = 100;
    });

    let hash1 = sample_hash(&env);
    let version1 = symbol_short!("v1");
    client.publish(&admin, &name, &hash1, &version1);

    env.ledger().with_mut(|li| {
        li.sequence_number = 200;
    });

    let hash2 = sample_hash_2(&env);
    let version2 = symbol_short!("v2");
    client.publish(&admin, &name, &hash2, &version2);

    let history = client.history(&name);
    assert_eq!(history.len(), 2);

    let (_, _, seq1) = history.get(0).unwrap();
    assert_eq!(seq1, 100);

    let (_, _, seq2) = history.get(1).unwrap();
    assert_eq!(seq2, 200);
}

// ── current ──────────────────────────────────────────────────────────────────

#[test]
fn current_returns_none_for_unknown_name() {
    let (env, client, _admin) = setup();
    let name = symbol_short!("unknown");

    assert_eq!(client.current(&name), None);
}

#[test]
fn current_returns_latest_after_multiple_publishes() {
    let (env, client, admin) = setup();
    let name = Symbol::new(&env, "my_contract");
    let hash1 = sample_hash(&env);
    let version1 = symbol_short!("v1");
    let hash2 = sample_hash_2(&env);
    let version2 = symbol_short!("v2");

    client.publish(&admin, &name, &hash1, &version1);
    client.publish(&admin, &name, &hash2, &version2);

    let (hash, version) = client.current(&name).unwrap();
    assert_eq!(hash, hash2);
    assert_eq!(version, version2);
}

// ── history ──────────────────────────────────────────────────────────────────

#[test]
fn history_empty_for_unknown_name() {
    let (env, client, _admin) = setup();
    let name = symbol_short!("unknown");

    assert_eq!(client.history(&name).len(), 0);
}

#[test]
fn history_independent_across_names() {
    let (env, client, admin) = setup();
    let name_a = Symbol::new(&env, "contract_a");
    let name_b = Symbol::new(&env, "contract_b");
    let hash_a = sample_hash(&env);
    let hash_b = sample_hash_2(&env);
    let version = symbol_short!("v1");

    client.publish(&admin, &name_a, &hash_a, &version);
    client.publish(&admin, &name_b, &hash_b, &version);

    assert_eq!(client.history(&name_a).len(), 1);
    assert_eq!(client.history(&name_b).len(), 1);

    let (h_a, _, _) = client.history(&name_a).get(0).unwrap();
    assert_eq!(h_a, hash_a);

    let (h_b, _, _) = client.history(&name_b).get(0).unwrap();
    assert_eq!(h_b, hash_b);
}

// ── get_admin ────────────────────────────────────────────────────────────────

#[test]
fn get_admin_returns_none_before_init() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(VersionRegistry, ());
    let client = VersionRegistryClient::new(&env, &contract_id);

    assert_eq!(client.get_admin(), None);
}
