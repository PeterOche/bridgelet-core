#[cfg(test)]
mod test {
    extern crate std;

    use crate::{FeeSplitter, FeeSplitterClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token,
        Address, Env, Vec,
    };

    // ── Token test helpers ────────────────────────────────────────────────────

    fn create_token<'a>(env: &'a Env, admin: &'a Address) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
        let contract_address = env.register_stellar_asset_contract_v2(admin.clone());
        let client = token::Client::new(env, &contract_address.address());
        let admin_client = token::StellarAssetClient::new(env, &contract_address.address());
        (contract_address.address(), client, admin_client)
    }

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

    /// Deploy + initialize a FeeSplitter and fund `sender` with tokens.
    fn setup_2_payee(
        share_a: u32,
        share_b: u32,
    ) -> (
        Env,
        FeeSplitterClient<'static>,
        Address, // sender
        Address, // payee_a
        Address, // payee_b
        Address, // asset
        Address, // contract_id
    ) {
        let env = create_env();
        env.mock_all_auths();

        let contract_id = env.register(FeeSplitter, ());
        let client = FeeSplitterClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let payee_a = Address::generate(&env);
        let payee_b = Address::generate(&env);
        let sender = Address::generate(&env);
        let token_admin = Address::generate(&env);

        let (asset, token_client, token_admin_client) = create_token(&env, &token_admin);

        // Mint 10 000 tokens to sender
        token_admin_client.mint(&sender, &10_000i128);

        let mut payees = Vec::new(&env);
        payees.push_back(payee_a.clone());
        payees.push_back(payee_b.clone());

        let mut shares = Vec::new(&env);
        shares.push_back(share_a);
        shares.push_back(share_b);

        client.initialize(&admin, &payees, &shares);

        let _ = token_client;
        (env, client, sender, payee_a, payee_b, asset, contract_id)
    }

    // ── Initialization ────────────────────────────────────────────────────────

    #[test]
    fn test_initialize_stores_payees() {
        let (_, client, _, payee_a, payee_b, _, _) = setup_2_payee(5_000, 5_000);
        let stored = client.get_payees();
        assert_eq!(stored.len(), 2);
        assert_eq!(stored.get(0).unwrap(), (payee_a, 5_000u32));
        assert_eq!(stored.get(1).unwrap(), (payee_b, 5_000u32));
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #6000)")]
    fn test_double_initialize_panics() {
        let (env, client, _, payee_a, payee_b, _, _) = setup_2_payee(5_000, 5_000);
        let admin = Address::generate(&env);
        let mut payees = Vec::new(&env);
        payees.push_back(payee_a);
        payees.push_back(payee_b);
        let mut shares = Vec::new(&env);
        shares.push_back(5_000u32);
        shares.push_back(5_000u32);
        client.initialize(&admin, &payees, &shares);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #6005)")]
    fn test_shares_not_summing_to_10000_rejected() {
        let env = create_env();
        env.mock_all_auths();
        let contract_id = env.register(FeeSplitter, ());
        let client = FeeSplitterClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let mut payees = Vec::new(&env);
        payees.push_back(Address::generate(&env));
        payees.push_back(Address::generate(&env));
        let mut shares = Vec::new(&env);
        shares.push_back(4_000u32);
        shares.push_back(4_000u32); // sum = 8000, not 10000
        client.initialize(&admin, &payees, &shares);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #6004)")]
    fn test_length_mismatch_rejected() {
        let env = create_env();
        env.mock_all_auths();
        let contract_id = env.register(FeeSplitter, ());
        let client = FeeSplitterClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let mut payees = Vec::new(&env);
        payees.push_back(Address::generate(&env));
        payees.push_back(Address::generate(&env));
        let mut shares = Vec::new(&env);
        shares.push_back(10_000u32); // 2 payees, 1 share
        client.initialize(&admin, &payees, &shares);
    }

    // ── 2-payee split ─────────────────────────────────────────────────────────

    #[test]
    fn test_2_payee_equal_split() {
        let (env, client, sender, payee_a, payee_b, asset, _) =
            setup_2_payee(5_000, 5_000);

        client.split(&sender, &asset, &1_000i128);

        let token = token::Client::new(&env, &asset);
        assert_eq!(token.balance(&payee_a), 500);
        assert_eq!(token.balance(&payee_b), 500);
        assert_eq!(token.balance(&sender), 9_000);
    }

    #[test]
    fn test_2_payee_70_30_split() {
        let (env, client, sender, payee_a, payee_b, asset, _) =
            setup_2_payee(7_000, 3_000);

        client.split(&sender, &asset, &1_000i128);

        let token = token::Client::new(&env, &asset);
        assert_eq!(token.balance(&payee_a), 700);
        assert_eq!(token.balance(&payee_b), 300);
    }

    // ── 5-payee split ─────────────────────────────────────────────────────────

    fn setup_5_payee() -> (
        Env,
        FeeSplitterClient<'static>,
        Address,               // sender
        std::vec::Vec<Address>,// payees
        Address,               // asset
    ) {
        let env = create_env();
        env.mock_all_auths();
        let contract_id = env.register(FeeSplitter, ());
        let client = FeeSplitterClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let (asset, _, token_admin_client) = create_token(&env, &token_admin);
        token_admin_client.mint(&sender, &10_000i128);

        let mut payee_addrs = std::vec::Vec::new();
        let mut soroban_payees = Vec::new(&env);
        let mut soroban_shares = Vec::new(&env);

        // 2000 each = 10000 bps total
        for _ in 0..5 {
            let p = Address::generate(&env);
            payee_addrs.push(p.clone());
            soroban_payees.push_back(p);
            soroban_shares.push_back(2_000u32);
        }

        client.initialize(&admin, &soroban_payees, &soroban_shares);
        (env, client, sender, payee_addrs, asset)
    }

    #[test]
    fn test_5_payee_equal_split() {
        let (env, client, sender, payees, asset) = setup_5_payee();
        client.split(&sender, &asset, &5_000i128);
        let token = token::Client::new(&env, &asset);
        for p in &payees {
            assert_eq!(token.balance(p), 1_000);
        }
        assert_eq!(token.balance(&sender), 5_000);
    }

    // ── Rounding remainder goes to last payee ─────────────────────────────────

    #[test]
    fn test_rounding_remainder_to_last_payee() {
        // 3 payees: 33.33% each → bps: 3333 + 3333 + 3334 = 10000
        let env = create_env();
        env.mock_all_auths();
        let contract_id = env.register(FeeSplitter, ());
        let client = FeeSplitterClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let (asset, _, token_admin_client) = create_token(&env, &token_admin);
        token_admin_client.mint(&sender, &10_000i128);

        let p0 = Address::generate(&env);
        let p1 = Address::generate(&env);
        let p2 = Address::generate(&env);

        let mut payees = Vec::new(&env);
        payees.push_back(p0.clone());
        payees.push_back(p1.clone());
        payees.push_back(p2.clone());

        let mut shares = Vec::new(&env);
        shares.push_back(3_333u32);
        shares.push_back(3_333u32);
        shares.push_back(3_334u32);

        client.initialize(&admin, &payees, &shares);

        // Split 10 tokens: p0 gets 3, p1 gets 3, p2 gets 4 (remainder)
        client.split(&sender, &asset, &10i128);

        let token = token::Client::new(&env, &asset);
        // floor(10 * 3333 / 10000) = floor(3.333) = 3
        assert_eq!(token.balance(&p0), 3);
        assert_eq!(token.balance(&p1), 3);
        // Last payee absorbs remainder: 10 - 3 - 3 = 4
        assert_eq!(token.balance(&p2), 4);
        assert_eq!(token.balance(&sender), 10_000 - 10);
    }

    // ── set_payees ────────────────────────────────────────────────────────────

    #[test]
    fn test_set_payees_updates_config() {
        let env = create_env();
        env.mock_all_auths();
        let contract_id = env.register(FeeSplitter, ());
        let client = FeeSplitterClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let pa = Address::generate(&env);
        let pb = Address::generate(&env);

        let mut payees = Vec::new(&env);
        payees.push_back(pa.clone());
        payees.push_back(pb.clone());
        let mut shares = Vec::new(&env);
        shares.push_back(5_000u32);
        shares.push_back(5_000u32);
        client.initialize(&admin, &payees, &shares);

        // Update to a single payee with 100%
        let new_payee = Address::generate(&env);
        let mut np = Vec::new(&env);
        np.push_back(new_payee.clone());
        let mut ns = Vec::new(&env);
        ns.push_back(10_000u32);
        client.set_payees(&admin, &np, &ns);

        let stored = client.get_payees();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored.get(0).unwrap(), (new_payee, 10_000u32));
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #6002)")]
    fn test_non_admin_cannot_set_payees() {
        let (env, client, _, _, _, _, _) = setup_2_payee(5_000, 5_000);
        let imposter = Address::generate(&env);
        let mut np = Vec::new(&env);
        np.push_back(Address::generate(&env));
        let mut ns = Vec::new(&env);
        ns.push_back(10_000u32);
        client.set_payees(&imposter, &np, &ns);
    }

    // ── Invalid amount ────────────────────────────────────────────────────────

    #[test]
    #[should_panic(expected = "Error(Contract, #6007)")]
    fn test_split_zero_amount_rejected() {
        let (_, client, sender, _, _, asset, _) = setup_2_payee(5_000, 5_000);
        client.split(&sender, &asset, &0i128);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #6007)")]
    fn test_split_negative_amount_rejected() {
        let (_, client, sender, _, _, asset, _) = setup_2_payee(5_000, 5_000);
        client.split(&sender, &asset, &-100i128);
    }
}
