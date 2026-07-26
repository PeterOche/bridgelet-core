#[cfg(test)]
mod test {
    extern crate std;

    use std::println;

    use crate::{
        storage, AccountStatus, EphemeralAccountContract, EphemeralAccountContractClient, Error,
        ReserveReclaimed,
    };
    use soroban_sdk::{
        testutils::{Address as _, Events as _, Ledger as _},
        Address, BytesN, Env, IntoVal, InvokeError, TryFromVal,
    };

    const BASE_RESERVE_STROOPS: i128 = 1_000_000_000;

    fn latest_reserve_event(client: &EphemeralAccountContractClient) -> ReserveReclaimed {
        client
            .get_last_reserve_event()
            .expect("reserve event was not emitted")
    }

    #[test]
    fn test_initialize() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &controller,
            &BytesN::from_array(&env, &[0u8; 32]),
            &Address::generate(&env),
        );

        assert_eq!(client.get_status(), AccountStatus::Active);
        assert!(!client.is_expired());
        assert_eq!(client.get_reserve_remaining(), BASE_RESERVE_STROOPS);
        assert_eq!(client.get_reserve_available(), BASE_RESERVE_STROOPS);
        assert!(!client.is_reserve_reclaimed());
    }

    #[test]
    fn test_record_payment() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &controller,
            &BytesN::from_array(&env, &[0u8; 32]),
            &Address::generate(&env),
        );
        client.record_payment(&100, &asset);

        assert_eq!(client.get_status(), AccountStatus::PaymentReceived);
    }

    #[test]
    fn test_multiple_payments() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let asset1 = Address::generate(&env);
        let asset2 = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &controller,
            &BytesN::from_array(&env, &[0u8; 32]),
            &Address::generate(&env),
        );

        client.record_payment(&100, &asset1);
        let info = client.get_info();
        assert_eq!(info.payment_count, 1);

        client.record_payment(&50, &asset2);
        let info = client.get_info();
        assert_eq!(info.payment_count, 2);

        assert_eq!(client.get_status(), AccountStatus::PaymentReceived);
    }

    #[test]
    fn test_sweep_single_asset() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let asset = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &controller,
            &BytesN::from_array(&env, &[0u8; 32]),
            &Address::generate(&env),
        );
        client.record_payment(&100, &asset);


        client.sweep_claim(&destination);

        assert_eq!(client.get_status(), AccountStatus::Swept);
        assert_eq!(client.get_reserve_remaining(), 0);
        assert!(client.is_reserve_reclaimed());

        let reserve_event = latest_reserve_event(&client);
        assert_eq!(reserve_event.destination, destination);
        assert_eq!(reserve_event.amount, BASE_RESERVE_STROOPS);
        assert_eq!(reserve_event.remaining_reserve, 0);
        assert!(reserve_event.fully_reclaimed);
        assert_eq!(reserve_event.sweep_id, env.ledger().sequence() as u64);
        assert_eq!(client.get_reserve_reclaim_event_count(), 1);
    }

    #[test]
    fn test_duplicate_asset_returns_expected_error_code() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &controller,
            &BytesN::from_array(&env, &[0u8; 32]),
            &Address::generate(&env),
        );
        client.record_payment(&100, &asset);
        let result = client.try_record_payment(&50, &asset);

        assert!(matches!(result, Err(Ok(Error::DuplicateAsset))));
    }

    #[test]
    fn test_too_many_assets_returns_expected_error_code() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &controller,
            &BytesN::from_array(&env, &[0u8; 32]),
            &Address::generate(&env),
        );

        for i in 0..10 {
            let asset = Address::generate(&env);
            client.record_payment(&(100 + i as i128), &asset);
        }

        let asset = Address::generate(&env);
        let result = client.try_record_payment(&200, &asset);

        assert!(matches!(result, Err(Ok(Error::TooManyPayments))));
    }

    #[test]
    fn test_record_payment_returns_not_initialized_error() {
        let env = Env::default();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let asset = Address::generate(&env);
        let result = client.try_record_payment(&100, &asset);

        assert!(matches!(result, Err(Ok(Error::NotInitialized))));
    }

    #[test]
    fn test_record_payment_returns_invalid_amount_error() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
            &BytesN::from_array(&env, &[0u8; 32]),
            &Address::generate(&env),
        );

        assert!(matches!(result, Err(Ok(Error::InvalidExpiry))));
    }

    #[test]
    fn test_expire_returns_not_expired_error() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
            &BytesN::from_array(&env, &[0u8; 32]),
            &Address::generate(&env),
        );

        let result = client.try_sweep_claim(&destination);

        assert!(matches!(result, Err(Ok(Error::NoPaymentReceived))));
    }

    #[test]
    fn test_sweep_returns_account_expired_error() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let asset = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
            &BytesN::from_array(&env, &[0u8; 32]),
            &Address::generate(&env),
        );
        client.record_payment(&100, &asset);


        client.sweep_claim(&destination);
        let replay_result = client.try_sweep_claim(&destination);

        assert!(matches!(replay_result, Err(Ok(Error::AlreadySwept))));
    }

    #[test]
    fn test_sweep_accepts_placeholder_authorization_and_succeeds() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let asset = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
            &BytesN::from_array(&env, &[0u8; 32]),
            &Address::generate(&env),
        );
        client.initialize(
            &creator,
            &(expiry_ledger + 1),
            &recovery,
            &Address::generate(&env),
            &BytesN::from_array(&env, &[0u8; 32]),
            &Address::generate(&env),
        );
        client.record_payment(&100, &asset);
        client.record_payment(&50, &asset);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1010)")]
    fn test_sweep_after_expiry_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let asset = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
            &BytesN::from_array(&env, &[0u8; 32]),
            &Address::generate(&env),
        );
        client.record_payment(&100, &asset);

        env.ledger().set_sequence_number(expiry_ledger);
        client.expire();

        let info = client.get_info();
        assert_eq!(info.status, AccountStatus::Expired);
        assert_eq!(info.swept_to, Some(recovery));
        assert_eq!(client.get_reserve_remaining(), 0);
        assert!(client.is_reserve_reclaimed());
        assert_eq!(client.get_reserve_reclaim_event_count(), 1);
    }

    #[test]
    fn test_initialize_requires_creator_authorization() {
        let env = Env::default();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        let result = client.try_initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
            &BytesN::from_array(&env, &[0u8; 32]),
}
