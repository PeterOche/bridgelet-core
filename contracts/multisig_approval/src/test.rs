#[cfg(test)]
mod test {
    extern crate std;

    use crate::{MultiSigApproval, MultiSigApprovalClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        Address, BytesN, Env, Vec,
    };

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

    fn make_hash(env: &Env, seed: u8) -> BytesN<32> {
        BytesN::from_array(env, &[seed; 32])
    }

    /// Deploy and initialize a multisig with `n` signers and the given threshold.
    fn setup_n_of_m(
        n: u32,
        threshold: u32,
    ) -> (
        Env,
        MultiSigApprovalClient<'static>,
        std::vec::Vec<Address>,
        Address,
    ) {
        let env = create_env();
        env.mock_all_auths();
        let contract_id = env.register(MultiSigApproval, ());
        let client = MultiSigApprovalClient::new(&env, &contract_id);

        let mut signers_vec = std::vec::Vec::new();
        for _ in 0..n {
            signers_vec.push(Address::generate(&env));
        }

        let mut soroban_signers = Vec::new(&env);
        for s in &signers_vec {
            soroban_signers.push_back(s.clone());
        }

        client.initialize(&soroban_signers, &threshold);
        (env, client, signers_vec, contract_id)
    }

    // ── Initialization ────────────────────────────────────────────────────────

    #[test]
    fn test_initialize_stores_config() {
        let (env, client, signers, _) = setup_n_of_m(3, 2);
        assert_eq!(client.get_threshold(), Some(2u32));
        let stored = client.get_signers().unwrap();
        assert_eq!(stored.len(), 3);
        let _ = env;
        let _ = signers;
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #5000)")]
    fn test_double_initialize_panics() {
        let (env, client, signers, _) = setup_n_of_m(2, 1);
        let mut new_signers = Vec::new(&env);
        for s in &signers {
            new_signers.push_back(s.clone());
        }
        client.initialize(&new_signers, &1u32);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #5003)")]
    fn test_threshold_exceeds_signers_rejected() {
        let env = create_env();
        env.mock_all_auths();
        let contract_id = env.register(MultiSigApproval, ());
        let client = MultiSigApprovalClient::new(&env, &contract_id);
        let mut signers = Vec::new(&env);
        signers.push_back(Address::generate(&env));
        signers.push_back(Address::generate(&env));
        // threshold = 3, but only 2 signers
        client.initialize(&signers, &3u32);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #5004)")]
    fn test_threshold_zero_rejected() {
        let env = create_env();
        env.mock_all_auths();
        let contract_id = env.register(MultiSigApproval, ());
        let client = MultiSigApprovalClient::new(&env, &contract_id);
        let mut signers = Vec::new(&env);
        signers.push_back(Address::generate(&env));
        client.initialize(&signers, &0u32);
    }

    // ── 2-of-3 flow ───────────────────────────────────────────────────────────

    #[test]
    fn test_2_of_3_not_approved_after_one() {
        let (env, client, signers, _) = setup_n_of_m(3, 2);
        let hash = make_hash(&env, 1);
        let proposal_id = client.propose(&signers[0], &hash);
        client.approve(&signers[0], &proposal_id);
        assert!(!client.is_approved(&proposal_id));
    }

    #[test]
    fn test_2_of_3_approved_after_two() {
        let (env, client, signers, _) = setup_n_of_m(3, 2);
        let hash = make_hash(&env, 2);
        let proposal_id = client.propose(&signers[0], &hash);
        client.approve(&signers[0], &proposal_id);
        client.approve(&signers[1], &proposal_id);
        assert!(client.is_approved(&proposal_id));
    }

    #[test]
    fn test_3_of_3_requires_all_signers() {
        let (env, client, signers, _) = setup_n_of_m(3, 3);
        let hash = make_hash(&env, 3);
        let proposal_id = client.propose(&signers[0], &hash);
        client.approve(&signers[0], &proposal_id);
        assert!(!client.is_approved(&proposal_id));
        client.approve(&signers[1], &proposal_id);
        assert!(!client.is_approved(&proposal_id));
        client.approve(&signers[2], &proposal_id);
        assert!(client.is_approved(&proposal_id));
    }

    // ── 3-of-5 flow ───────────────────────────────────────────────────────────

    #[test]
    fn test_3_of_5_approved_after_threshold() {
        let (env, client, signers, _) = setup_n_of_m(5, 3);
        let hash = make_hash(&env, 4);
        let proposal_id = client.propose(&signers[0], &hash);
        client.approve(&signers[0], &proposal_id);
        client.approve(&signers[1], &proposal_id);
        assert!(!client.is_approved(&proposal_id));
        client.approve(&signers[2], &proposal_id);
        assert!(client.is_approved(&proposal_id));
        // Additional approvals don't break anything
        client.approve(&signers[3], &proposal_id);
        assert!(client.is_approved(&proposal_id));
    }

    // ── Duplicate approvals ───────────────────────────────────────────────────

    #[test]
    #[should_panic(expected = "Error(Contract, #5006)")]
    fn test_duplicate_approval_rejected() {
        let (env, client, signers, _) = setup_n_of_m(3, 2);
        let hash = make_hash(&env, 5);
        let proposal_id = client.propose(&signers[0], &hash);
        client.approve(&signers[0], &proposal_id);
        client.approve(&signers[0], &proposal_id); // duplicate
    }

    // ── Non-signer cannot approve ─────────────────────────────────────────────

    #[test]
    #[should_panic(expected = "Error(Contract, #5002)")]
    fn test_non_signer_cannot_approve() {
        let (env, client, signers, _) = setup_n_of_m(2, 1);
        let hash = make_hash(&env, 6);
        let proposal_id = client.propose(&signers[0], &hash);
        let outsider = Address::generate(&env);
        client.approve(&outsider, &proposal_id);
    }

    // ── set_signers ───────────────────────────────────────────────────────────

    #[test]
    fn test_set_signers_updates_config() {
        let (env, client, signers, _) = setup_n_of_m(3, 2);
        let admin = signers[0].clone();
        let new_signer = Address::generate(&env);
        let mut new_list = Vec::new(&env);
        new_list.push_back(admin.clone());
        new_list.push_back(new_signer);
        client.set_signers(&admin, &new_list, &1u32);
        assert_eq!(client.get_threshold(), Some(1u32));
        assert_eq!(client.get_signers().unwrap().len(), 2);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #5007)")]
    fn test_non_admin_cannot_set_signers() {
        let (env, client, signers, _) = setup_n_of_m(3, 2);
        let imposter = Address::generate(&env);
        let mut new_list = Vec::new(&env);
        new_list.push_back(signers[0].clone());
        client.set_signers(&imposter, &new_list, &1u32);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #5003)")]
    fn test_set_signers_threshold_too_high_rejected() {
        let (env, client, signers, _) = setup_n_of_m(3, 2);
        let admin = signers[0].clone();
        let mut new_list = Vec::new(&env);
        new_list.push_back(admin.clone());
        // threshold = 5 but only 1 signer
        client.set_signers(&admin, &new_list, &5u32);
    }

    // ── Proposal not found ────────────────────────────────────────────────────

    #[test]
    #[should_panic(expected = "Error(Contract, #5005)")]
    fn test_approve_nonexistent_proposal_fails() {
        let (_, client, signers, _) = setup_n_of_m(2, 1);
        client.approve(&signers[0], &9999u64);
    }

    // ── is_approved for unknown proposal ─────────────────────────────────────

    #[test]
    fn test_is_approved_unknown_proposal_returns_false() {
        let (_, client, _, _) = setup_n_of_m(2, 1);
        assert!(!client.is_approved(&9999u64));
    }
}
