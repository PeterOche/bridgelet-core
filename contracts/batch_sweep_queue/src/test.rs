#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, Vec};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn setup() -> (Env, BatchSweepQueueClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BatchSweepQueue, ());
    let client = BatchSweepQueueClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

// ─── Initialization ───────────────────────────────────────────────────────────

#[test]
fn test_initialize_sets_admin() {
    let (_env, client, admin) = setup();
    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
#[should_panic]
fn test_initialize_twice_fails() {
    let (env, client, _admin) = setup();
    let second_admin = Address::generate(&env);
    client.initialize(&second_admin); // should panic with AlreadyInitialized
}

#[test]
fn test_initialize_twice_error_variant() {
    let (env, client, _admin) = setup();
    let second_admin = Address::generate(&env);
    let err = client
        .try_initialize(&second_admin)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::AlreadyInitialized);
}

#[test]
fn test_operations_before_init_fail() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BatchSweepQueue, ());
    let client = BatchSweepQueueClient::new(&env, &contract_id);

    let account = Address::generate(&env);
    let destination = Address::generate(&env);
    let admin = Address::generate(&env);
    let mut ids: Vec<u64> = Vec::new(&env);
    ids.push_back(0u64);

    assert_eq!(
        client.try_enqueue(&account, &destination).unwrap_err().unwrap(),
        Error::NotInitialized
    );
    assert_eq!(
        client.try_peek_batch(&1).unwrap_err().unwrap(),
        Error::NotInitialized
    );
    assert_eq!(
        client.try_mark_processed(&admin, &ids).unwrap_err().unwrap(),
        Error::NotInitialized
    );
    assert_eq!(
        client.try_queue_length().unwrap_err().unwrap(),
        Error::NotInitialized
    );
}

// ─── Enqueue ──────────────────────────────────────────────────────────────────

#[test]
fn test_enqueue_returns_incrementing_ids() {
    let (env, client, _admin) = setup();
    let account = Address::generate(&env);
    let dest = Address::generate(&env);

    let id0 = client.enqueue(&account, &dest);
    let id1 = client.enqueue(&account, &dest);
    let id2 = client.enqueue(&account, &dest);

    assert_eq!(id0, 0u64);
    assert_eq!(id1, 1u64);
    assert_eq!(id2, 2u64);
}

#[test]
fn test_enqueue_increases_queue_length() {
    let (env, client, _admin) = setup();
    let account = Address::generate(&env);
    let dest = Address::generate(&env);

    assert_eq!(client.queue_length(), 0u32);
    client.enqueue(&account, &dest);
    assert_eq!(client.queue_length(), 1u32);
    client.enqueue(&account, &dest);
    assert_eq!(client.queue_length(), 2u32);
}

// ─── peek_batch (read-only, FIFO order) ───────────────────────────────────────

#[test]
fn test_peek_batch_returns_fifo_order() {
    let (env, client, _admin) = setup();

    let account1 = Address::generate(&env);
    let account2 = Address::generate(&env);
    let account3 = Address::generate(&env);
    let dest = Address::generate(&env);

    let id0 = client.enqueue(&account1, &dest);
    let id1 = client.enqueue(&account2, &dest);
    let id2 = client.enqueue(&account3, &dest);

    let batch = client.peek_batch(&3);
    assert_eq!(batch.len(), 3);

    let (b_id0, b_account0, _) = batch.get(0).unwrap();
    let (b_id1, b_account1, _) = batch.get(1).unwrap();
    let (b_id2, b_account2, _) = batch.get(2).unwrap();

    assert_eq!(b_id0, id0);
    assert_eq!(b_account0, account1);
    assert_eq!(b_id1, id1);
    assert_eq!(b_account1, account2);
    assert_eq!(b_id2, id2);
    assert_eq!(b_account2, account3);
}

#[test]
fn test_peek_batch_does_not_mutate_queue() {
    let (env, client, _admin) = setup();
    let account = Address::generate(&env);
    let dest = Address::generate(&env);

    client.enqueue(&account, &dest);
    client.enqueue(&account, &dest);

    let before = client.queue_length();
    client.peek_batch(&10);
    let after = client.queue_length();

    assert_eq!(before, after, "peek_batch must not mutate queue state");
}

#[test]
fn test_peek_batch_respects_max() {
    let (env, client, _admin) = setup();
    let account = Address::generate(&env);
    let dest = Address::generate(&env);

    for _ in 0..5 {
        client.enqueue(&account, &dest);
    }

    let batch = client.peek_batch(&3);
    assert_eq!(batch.len(), 3);
}

#[test]
fn test_peek_batch_max_larger_than_queue() {
    let (env, client, _admin) = setup();
    let account = Address::generate(&env);
    let dest = Address::generate(&env);

    client.enqueue(&account, &dest);
    client.enqueue(&account, &dest);

    let batch = client.peek_batch(&100);
    assert_eq!(batch.len(), 2);
}

#[test]
fn test_peek_batch_zero_max_fails() {
    let (_env, client, _admin) = setup();
    let err = client.try_peek_batch(&0).unwrap_err().unwrap();
    assert_eq!(err, Error::InvalidBatchSize);
}

// ─── mark_processed ───────────────────────────────────────────────────────────

#[test]
fn test_mark_processed_removes_exact_ids() {
    let (env, client, admin) = setup();
    let account = Address::generate(&env);
    let dest = Address::generate(&env);

    let id0 = client.enqueue(&account, &dest);
    let id1 = client.enqueue(&account, &dest);
    let id2 = client.enqueue(&account, &dest);

    let mut to_remove: Vec<u64> = Vec::new(&env);
    to_remove.push_back(id0);
    to_remove.push_back(id2);
    client.mark_processed(&admin, &to_remove);

    assert_eq!(client.queue_length(), 1u32);
    let remaining = client.peek_batch(&10);
    assert_eq!(remaining.len(), 1);
    let (remaining_id, _, _) = remaining.get(0).unwrap();
    assert_eq!(remaining_id, id1);
}

#[test]
fn test_mark_processed_partial_batch() {
    let (env, client, admin) = setup();
    let account = Address::generate(&env);
    let dest = Address::generate(&env);

    // Enqueue 5 items, mark first 2 processed.
    for _ in 0..5 {
        client.enqueue(&account, &dest);
    }

    let mut ids: Vec<u64> = Vec::new(&env);
    ids.push_back(0u64);
    ids.push_back(1u64);
    client.mark_processed(&admin, &ids);

    assert_eq!(client.queue_length(), 3u32);

    let batch = client.peek_batch(&10);
    let first_id = batch.get(0).unwrap().0;
    assert_eq!(first_id, 2u64, "remaining entries should start at id=2");
}

#[test]
fn test_mark_processed_nonexistent_ids_are_ignored() {
    let (env, client, admin) = setup();
    let account = Address::generate(&env);
    let dest = Address::generate(&env);

    client.enqueue(&account, &dest);

    let mut ids: Vec<u64> = Vec::new(&env);
    ids.push_back(999u64); // does not exist
    client.mark_processed(&admin, &ids);

    assert_eq!(client.queue_length(), 1u32);
}

#[test]
fn test_mark_processed_requires_admin() {
    let (env, client, _admin) = setup();
    let account = Address::generate(&env);
    let dest = Address::generate(&env);
    client.enqueue(&account, &dest);

    let non_admin = Address::generate(&env);
    let mut ids: Vec<u64> = Vec::new(&env);
    ids.push_back(0u64);

    let err = client
        .try_mark_processed(&non_admin, &ids)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::Unauthorized);
}

#[test]
fn test_mark_processed_all_leaves_empty_queue() {
    let (env, client, admin) = setup();
    let account = Address::generate(&env);
    let dest = Address::generate(&env);

    client.enqueue(&account, &dest);
    client.enqueue(&account, &dest);

    let mut ids: Vec<u64> = Vec::new(&env);
    ids.push_back(0u64);
    ids.push_back(1u64);
    client.mark_processed(&admin, &ids);

    assert_eq!(client.queue_length(), 0u32);
    assert_eq!(client.peek_batch(&10).len(), 0);
}

// ─── FIFO ordering preserved after partial removal ────────────────────────────

#[test]
fn test_fifo_ordering_after_partial_removal() {
    let (env, client, admin) = setup();
    let account = Address::generate(&env);
    let dest = Address::generate(&env);

    // Enqueue ids 0..4
    for _ in 0..5 {
        client.enqueue(&account, &dest);
    }

    // Remove ids 1 and 3
    let mut ids: Vec<u64> = Vec::new(&env);
    ids.push_back(1u64);
    ids.push_back(3u64);
    client.mark_processed(&admin, &ids);

    let batch = client.peek_batch(&10);
    assert_eq!(batch.len(), 3);
    assert_eq!(batch.get(0).unwrap().0, 0u64);
    assert_eq!(batch.get(1).unwrap().0, 2u64);
    assert_eq!(batch.get(2).unwrap().0, 4u64);
}
