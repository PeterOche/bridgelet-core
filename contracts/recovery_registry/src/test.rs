#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, Vec};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn setup() -> (Env, RecoveryRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(RecoveryRegistry, ());
    let client = RecoveryRegistryClient::new(&env, &contract_id);
    (env, client)
}

fn make_guardians(env: &Env, n: usize) -> Vec<Address> {
    let mut v: Vec<Address> = Vec::new(env);
    for _ in 0..n {
        v.push_back(Address::generate(env));
    }
    v
}

// ─── register ─────────────────────────────────────────────────────────────────

#[test]
fn test_register_stores_guardian_set() {
    let (env, client) = setup();
    let account = Address::generate(&env);
    let guardians = make_guardians(&env, 3);

    client.register(&account, &guardians, &2);

    let set = client.get_guardian_set(&account).unwrap();
    assert_eq!(set.threshold, 2);
    assert_eq!(set.guardians.len(), 3);
}

#[test]
fn test_register_twice_fails() {
    let (env, client) = setup();
    let account = Address::generate(&env);
    let guardians = make_guardians(&env, 2);

    client.register(&account, &guardians, &1);
    let err = client
        .try_register(&account, &guardians, &1)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::AlreadyRegistered);
}

#[test]
fn test_register_empty_guardians_fails() {
    let (env, client) = setup();
    let account = Address::generate(&env);
    let empty: Vec<Address> = Vec::new(&env);
    let err = client
        .try_register(&account, &empty, &1)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::NoGuardians);
}

#[test]
fn test_register_threshold_zero_fails() {
    let (env, client) = setup();
    let account = Address::generate(&env);
    let guardians = make_guardians(&env, 2);
    let err = client
        .try_register(&account, &guardians, &0)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::InvalidThreshold);
}

#[test]
fn test_register_threshold_exceeds_guardian_count_fails() {
    let (env, client) = setup();
    let account = Address::generate(&env);
    let guardians = make_guardians(&env, 2);
    let err = client
        .try_register(&account, &guardians, &3)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::InvalidThreshold);
}

#[test]
fn test_register_threshold_equal_to_guardian_count_succeeds() {
    let (env, client) = setup();
    let account = Address::generate(&env);
    let guardians = make_guardians(&env, 3);
    // threshold == len should be allowed (unanimous)
    client.register(&account, &guardians, &3);
    let set = client.get_guardian_set(&account).unwrap();
    assert_eq!(set.threshold, 3);
}

// ─── approve_recovery ────────────────────────────────────────────────────────

#[test]
fn test_approve_by_non_guardian_fails() {
    let (env, client) = setup();
    let account = Address::generate(&env);
    let guardians = make_guardians(&env, 3);
    client.register(&account, &guardians, &2);

    let outsider = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let err = client
        .try_approve_recovery(&outsider, &account, &new_owner)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::NotAGuardian);
}

#[test]
fn test_approve_unregistered_account_fails() {
    let (env, client) = setup();
    let account = Address::generate(&env);
    let guardian = Address::generate(&env);
    let new_owner = Address::generate(&env);

    let err = client
        .try_approve_recovery(&guardian, &account, &new_owner)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::NotRegistered);
}

#[test]
fn test_guardian_cannot_approve_twice() {
    let (env, client) = setup();
    let account = Address::generate(&env);
    let guardians = make_guardians(&env, 3);
    let guardian0 = guardians.get(0).unwrap();
    let new_owner = Address::generate(&env);

    client.register(&account, &guardians, &2);
    client.approve_recovery(&guardian0, &account, &new_owner);

    let err = client
        .try_approve_recovery(&guardian0, &account, &new_owner)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::AlreadyApproved);
}

#[test]
fn test_single_approval_increments_count() {
    let (env, client) = setup();
    let account = Address::generate(&env);
    let guardians = make_guardians(&env, 3);
    let guardian0 = guardians.get(0).unwrap();
    let new_owner = Address::generate(&env);

    client.register(&account, &guardians, &2);
    assert_eq!(client.approval_count(&account, &new_owner), 0u32);

    client.approve_recovery(&guardian0, &account, &new_owner);
    assert_eq!(client.approval_count(&account, &new_owner), 1u32);
}

// ─── recovery_ready ──────────────────────────────────────────────────────────

#[test]
fn test_recovery_not_ready_before_threshold() {
    let (env, client) = setup();
    let account = Address::generate(&env);
    let guardians = make_guardians(&env, 3);
    let new_owner = Address::generate(&env);

    client.register(&account, &guardians, &2);
    client.approve_recovery(&guardians.get(0).unwrap(), &account, &new_owner);

    assert!(
        !client.recovery_ready(&account, &new_owner),
        "should not be ready with only 1 of 2 approvals"
    );
}

#[test]
fn test_recovery_ready_false_for_unregistered_account() {
    let (env, client) = setup();
    let account = Address::generate(&env);
    let new_owner = Address::generate(&env);
    assert!(!client.recovery_ready(&account, &new_owner));
}

#[test]
fn test_recovery_ready_false_for_different_new_owner() {
    let (env, client) = setup();
    let account = Address::generate(&env);
    let guardians = make_guardians(&env, 2);
    let new_owner_a = Address::generate(&env);
    let new_owner_b = Address::generate(&env);

    client.register(&account, &guardians, &2);
    client.approve_recovery(&guardians.get(0).unwrap(), &account, &new_owner_a);
    client.approve_recovery(&guardians.get(1).unwrap(), &account, &new_owner_a);

    // Fully approved for new_owner_a, but not for new_owner_b
    assert!(client.recovery_ready(&account, &new_owner_a));
    assert!(!client.recovery_ready(&account, &new_owner_b));
}

// ─── 2-of-3 guardian recovery (issue acceptance criteria) ────────────────────

#[test]
fn test_2_of_3_guardian_recovery() {
    let (env, client) = setup();
    let account = Address::generate(&env);
    let guardians = make_guardians(&env, 3);
    let guardian0 = guardians.get(0).unwrap();
    let guardian1 = guardians.get(1).unwrap();
    let new_owner = Address::generate(&env);

    client.register(&account, &guardians, &2);

    // Not ready yet
    assert!(!client.recovery_ready(&account, &new_owner));

    // First approval
    client.approve_recovery(&guardian0, &account, &new_owner);
    assert_eq!(client.approval_count(&account, &new_owner), 1u32);
    assert!(
        !client.recovery_ready(&account, &new_owner),
        "1/2 approvals should not be ready"
    );

    // Second approval — threshold met
    client.approve_recovery(&guardian1, &account, &new_owner);
    assert_eq!(client.approval_count(&account, &new_owner), 2u32);
    assert!(
        client.recovery_ready(&account, &new_owner),
        "2/2 approvals should be ready"
    );
}

#[test]
fn test_3_of_3_unanimous_recovery() {
    let (env, client) = setup();
    let account = Address::generate(&env);
    let guardians = make_guardians(&env, 3);
    let new_owner = Address::generate(&env);

    client.register(&account, &guardians, &3);

    for i in 0..2u32 {
        assert!(!client.recovery_ready(&account, &new_owner));
        client.approve_recovery(&guardians.get(i).unwrap(), &account, &new_owner);
    }
    assert!(!client.recovery_ready(&account, &new_owner));

    client.approve_recovery(&guardians.get(2).unwrap(), &account, &new_owner);
    assert!(client.recovery_ready(&account, &new_owner));
}

#[test]
fn test_independent_recovery_proposals_dont_interfere() {
    let (env, client) = setup();
    let account = Address::generate(&env);
    let guardians = make_guardians(&env, 3);
    let new_owner_a = Address::generate(&env);
    let new_owner_b = Address::generate(&env);

    client.register(&account, &guardians, &2);

    // Two guardians approve owner_a; one approves owner_b
    client.approve_recovery(&guardians.get(0).unwrap(), &account, &new_owner_a);
    client.approve_recovery(&guardians.get(1).unwrap(), &account, &new_owner_a);
    client.approve_recovery(&guardians.get(0).unwrap(), &account, &new_owner_b);

    assert!(client.recovery_ready(&account, &new_owner_a));
    assert!(!client.recovery_ready(&account, &new_owner_b));
}
