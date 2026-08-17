#![cfg(test)]

use super::*;
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn setup() -> (Env, AuditLogClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AuditLog, ());
    let client = AuditLogClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

/// Deploy and initialize, and additionally authorize `writer`.
fn setup_with_writer() -> (Env, AuditLogClient<'static>, Address, Address) {
    let (env, client, admin) = setup();
    let writer = Address::generate(&env);
    client.authorize_writer(&admin, &writer);
    (env, client, admin, writer)
}

// ── initialize ───────────────────────────────────────────────────────────────

#[test]
fn initialize_succeeds_once() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AuditLog, ());
    let client = AuditLogClient::new(&env, &contract_id);
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

// ── authorize_writer ──────────────────────────────────────────────────────────

#[test]
fn authorize_writer_succeeds() {
    let (env, client, admin) = setup();
    let writer = Address::generate(&env);

    let result = client.try_authorize_writer(&admin, &writer);
    assert!(result.is_ok());
}

#[test]
fn authorize_writer_non_admin_returns_unauthorized() {
    let (env, client, _admin) = setup();
    let intruder = Address::generate(&env);
    let writer = Address::generate(&env);

    let result = client.try_authorize_writer(&intruder, &writer);
    assert_eq!(result.unwrap_err().unwrap(), Error::Unauthorized);
}

/// Authorizing the same writer twice must succeed (idempotent).
#[test]
fn authorize_writer_idempotent() {
    let (env, client, admin) = setup();
    let writer = Address::generate(&env);

    client.authorize_writer(&admin, &writer);
    let result = client.try_authorize_writer(&admin, &writer);
    assert!(result.is_ok());
}

// ── record ────────────────────────────────────────────────────────────────────

/// Unauthorized writer must be rejected (acceptance criterion from #460).
#[test]
fn record_unauthorized_writer_rejected() {
    let (env, client, _admin) = setup();
    let unauthorized = Address::generate(&env);
    let actor = Address::generate(&env);
    let subject = Address::generate(&env);

    let result = client.try_record(&unauthorized, &actor, &symbol_short!("sweep"), &subject);
    assert_eq!(result.unwrap_err().unwrap(), Error::UnauthorizedWriter);
}

/// IDs are assigned sequentially starting from 0 (acceptance criterion from #460).
#[test]
fn record_sequential_id_assignment() {
    let (env, client, _admin, writer) = setup_with_writer();
    let actor = Address::generate(&env);
    let subject = Address::generate(&env);

    let id0 = client.record(&writer, &actor, &symbol_short!("sweep"), &subject);
    let id1 = client.record(&writer, &actor, &symbol_short!("expire"), &subject);
    let id2 = client.record(&writer, &actor, &symbol_short!("init"), &subject);

    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

#[test]
fn record_increments_count() {
    let (env, client, _admin, writer) = setup_with_writer();
    let actor = Address::generate(&env);
    let subject = Address::generate(&env);

    assert_eq!(client.count(), 0);
    client.record(&writer, &actor, &symbol_short!("sweep"), &subject);
    assert_eq!(client.count(), 1);
    client.record(&writer, &actor, &symbol_short!("expire"), &subject);
    assert_eq!(client.count(), 2);
}

// ── get ───────────────────────────────────────────────────────────────────────

#[test]
fn get_returns_entry_after_record() {
    let (env, client, _admin, writer) = setup_with_writer();
    let actor = Address::generate(&env);
    let subject = Address::generate(&env);

    let id = client.record(&writer, &actor, &symbol_short!("sweep"), &subject);
    let entry = client.get(&id).expect("entry must exist");

    assert_eq!(entry.writer, writer);
    assert_eq!(entry.actor, actor);
    assert_eq!(entry.action, symbol_short!("sweep"));
    assert_eq!(entry.subject, subject);
}

#[test]
fn get_returns_none_for_missing_id() {
    let (_, client, _) = setup();
    assert_eq!(client.get(&99u64), None);
}

/// Entries are immutable — no update/delete is exposed (acceptance criterion #460).
/// Verify this by confirming get() after record returns the original entry unchanged.
#[test]
fn entries_are_immutable_once_written() {
    let (env, client, _admin, writer) = setup_with_writer();
    let actor = Address::generate(&env);
    let subject = Address::generate(&env);

    let id = client.record(&writer, &actor, &symbol_short!("sweep"), &subject);

    // Record another entry to confirm state is additive, not overwriting.
    let actor2 = Address::generate(&env);
    let subject2 = Address::generate(&env);
    client.record(&writer, &actor2, &symbol_short!("expire"), &subject2);

    // Original entry must be unchanged.
    let entry = client.get(&id).unwrap();
    assert_eq!(entry.actor, actor);
    assert_eq!(entry.subject, subject);
    assert_eq!(entry.action, symbol_short!("sweep"));
}

// ── count ─────────────────────────────────────────────────────────────────────

#[test]
fn count_starts_at_zero() {
    let (_, client, _) = setup();
    assert_eq!(client.count(), 0);
}

// ── state isolation ───────────────────────────────────────────────────────────

#[test]
fn two_instances_are_independent() {
    let env = Env::default();
    env.mock_all_auths();

    let id_a = env.register(AuditLog, ());
    let id_b = env.register(AuditLog, ());

    let client_a = AuditLogClient::new(&env, &id_a);
    let client_b = AuditLogClient::new(&env, &id_b);

    let admin_a = Address::generate(&env);
    let admin_b = Address::generate(&env);
    client_a.initialize(&admin_a);
    client_b.initialize(&admin_b);

    let writer_a = Address::generate(&env);
    client_a.authorize_writer(&admin_a, &writer_a);

    let actor = Address::generate(&env);
    let subject = Address::generate(&env);
    client_a.record(&writer_a, &actor, &symbol_short!("sweep"), &subject);

    // Contract B must still have count = 0.
    assert_eq!(client_b.count(), 0);
    // Contract A must have count = 1.
    assert_eq!(client_a.count(), 1);
}
