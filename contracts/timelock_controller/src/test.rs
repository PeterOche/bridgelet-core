#[cfg(test)]
mod test {
    extern crate std;

    use crate::{TimelockController, TimelockControllerClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        Address, BytesN, Env,
    };

    const MIN_DELAY: u64 = 100;

    fn create_env() -> Env {
        let env = Env::default();
        env.ledger().with_mut(|li| {
            li.sequence_number = 1_000;
            li.min_persistent_entry_ttl = 50;
            li.min_temp_entry_ttl = 50;
            li.max_entry_ttl = 600_000;
        });
        env
    }

    fn setup() -> (Env, TimelockControllerClient<'static>, Address, Address) {
        let env = create_env();
        env.mock_all_auths();
        let contract_id = env.register(TimelockController, ());
        let client = TimelockControllerClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin, &MIN_DELAY);
        (env, client, admin, contract_id)
    }

    fn make_hash(env: &Env, seed: u8) -> BytesN<32> {
        BytesN::from_array(env, &[seed; 32])
    }

    // ── Initialization ────────────────────────────────────────────────────────

    #[test]
    fn test_initialize_stores_admin_and_delay() {
        let (env, client, admin, _) = setup();
        assert_eq!(client.get_admin(), Some(admin));
        assert_eq!(client.get_min_delay(), Some(MIN_DELAY));
        let _ = env;
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4000)")]
    fn test_double_initialize_panics() {
        let (env, client, _, _) = setup();
        let another = Address::generate(&env);
        client.initialize(&another, &10u64);
    }

    // ── Queue ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_queue_valid_action() {
        let (env, client, admin, _) = setup();
        let hash = make_hash(&env, 1);
        let target = Address::generate(&env);
        // now = 1000, min_delay = 100, so eta >= 1100
        client.queue(&admin, &target, &hash, &1200u64);
        assert!(!client.is_ready(&hash)); // not ready yet (now=1000 < eta=1200)
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4003)")]
    fn test_queue_eta_too_early() {
        let (env, client, admin, _) = setup();
        let hash = make_hash(&env, 2);
        let target = Address::generate(&env);
        // eta = 1099 < 1000 + 100 = 1100
        client.queue(&admin, &target, &hash, &1099u64);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4004)")]
    fn test_queue_duplicate_pending_hash() {
        let (env, client, admin, _) = setup();
        let hash = make_hash(&env, 3);
        let target = Address::generate(&env);
        client.queue(&admin, &target, &hash, &1200u64);
        // Second queue with same hash while still pending must fail
        client.queue(&admin, &target, &hash, &1300u64);
    }

    // ── Execute ───────────────────────────────────────────────────────────────

    #[test]
    fn test_execute_after_eta() {
        let (env, client, admin, _) = setup();
        let hash = make_hash(&env, 4);
        let target = Address::generate(&env);
        client.queue(&admin, &target, &hash, &1100u64);

        // Advance ledger past ETA
        env.ledger().with_mut(|li| li.sequence_number = 1100);
        assert!(client.is_ready(&hash));
        client.execute(&hash);
        // Now should no longer be ready (already executed)
        assert!(!client.is_ready(&hash));
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4006)")]
    fn test_execute_before_eta_rejected() {
        let (env, client, admin, _) = setup();
        let hash = make_hash(&env, 5);
        let target = Address::generate(&env);
        client.queue(&admin, &target, &hash, &1200u64);
        // Still at ledger 1000, eta is 1200
        client.execute(&hash);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4005)")]
    fn test_execute_unknown_hash_rejected() {
        let (env, client, _, _) = setup();
        // Use a hash that was never queued
        let hash = make_hash(&env, 99);
        client.execute(&hash);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4008)")]
    fn test_execute_twice_rejected() {
        let (env, client, admin, _) = setup();
        let hash = make_hash(&env, 6);
        let target = Address::generate(&env);
        client.queue(&admin, &target, &hash, &1100u64);
        env.ledger().with_mut(|li| li.sequence_number = 1100);
        client.execute(&hash);
        client.execute(&hash); // second time must fail
    }

    // ── Cancel ────────────────────────────────────────────────────────────────

    #[test]
    fn test_cancel_pending_action() {
        let (env, client, admin, _) = setup();
        let hash = make_hash(&env, 7);
        let target = Address::generate(&env);
        client.queue(&admin, &target, &hash, &1200u64);
        client.cancel(&admin, &hash);
        assert!(!client.is_ready(&hash));
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4007)")]
    fn test_execute_cancelled_action_rejected() {
        let (env, client, admin, _) = setup();
        let hash = make_hash(&env, 8);
        let target = Address::generate(&env);
        client.queue(&admin, &target, &hash, &1100u64);
        client.cancel(&admin, &hash);
        env.ledger().with_mut(|li| li.sequence_number = 1100);
        client.execute(&hash); // must fail with Cancelled
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4007)")]
    fn test_cancel_twice_rejected() {
        let (env, client, admin, _) = setup();
        let hash = make_hash(&env, 9);
        let target = Address::generate(&env);
        client.queue(&admin, &target, &hash, &1200u64);
        client.cancel(&admin, &hash);
        client.cancel(&admin, &hash); // second cancel must fail
    }

    // ── is_ready boundary ─────────────────────────────────────────────────────

    #[test]
    fn test_is_ready_at_exact_eta() {
        let (env, client, admin, _) = setup();
        let hash = make_hash(&env, 10);
        let target = Address::generate(&env);
        client.queue(&admin, &target, &hash, &1100u64);
        env.ledger().with_mut(|li| li.sequence_number = 1100);
        assert!(client.is_ready(&hash));
    }

    #[test]
    fn test_is_ready_one_before_eta() {
        let (env, client, admin, _) = setup();
        let hash = make_hash(&env, 11);
        let target = Address::generate(&env);
        client.queue(&admin, &target, &hash, &1100u64);
        env.ledger().with_mut(|li| li.sequence_number = 1099);
        assert!(!client.is_ready(&hash));
    }

    // ── Re-queue after cancelled ───────────────────────────────────────────────

    #[test]
    fn test_requeue_after_cancel_succeeds() {
        let (env, client, admin, _) = setup();
        let hash = make_hash(&env, 12);
        let target = Address::generate(&env);
        client.queue(&admin, &target, &hash, &1200u64);
        client.cancel(&admin, &hash);
        // Re-queue with same hash is allowed after cancellation
        client.queue(&admin, &target, &hash, &1300u64);
        assert!(!client.is_ready(&hash));
    }
}
