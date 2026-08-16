use crate::{
    Error,
    RateLimiter,
    RateLimiterClient,
};

use soroban_sdk::{
    testutils::{
        Address as _,
        Ledger,
    },
    Address,
    Env,
};

fn setup(
    env: &Env,
    window_ledgers: u32,
    max_calls: u32,
) -> (RateLimiterClient<'_>, Address, Address) {
    env.mock_all_auths();

    let admin = Address::generate(env);
    let key = Address::generate(env);

    let contract_id = env.register(RateLimiter, ());

    let client = RateLimiterClient::new(
        env,
        &contract_id,
    );

    client.initialize(
        &admin,
        &window_ledgers,
        &max_calls,
    );

    (client, admin, key)
}

#[test]
fn test_initialize_and_remaining() {
    let env = Env::default();

    let (client, _admin, key) = setup(
        &env,
        10,
        3,
    );

    assert_eq!(
        client.remaining(&key),
        3
    );
}

#[test]
fn test_allows_exactly_max_calls() {
    let env = Env::default();

    let (client, _admin, key) = setup(
        &env,
        10,
        3,
    );

    // Call 1: allowed.
    client.check_and_record(&key);

    assert_eq!(
        client.remaining(&key),
        2
    );

    // Call 2: allowed.
    client.check_and_record(&key);

    assert_eq!(
        client.remaining(&key),
        1
    );

    // Call 3: exactly at the configured limit, still allowed.
    client.check_and_record(&key);

    assert_eq!(
        client.remaining(&key),
        0
    );
}

#[test]
fn test_rejects_one_past_limit() {
    let env = Env::default();

    let (client, _admin, key) = setup(
        &env,
        10,
        3,
    );

    // Consume the complete allowance.
    client.check_and_record(&key);
    client.check_and_record(&key);
    client.check_and_record(&key);

    assert_eq!(
        client.remaining(&key),
        0
    );

    // The fourth call must fail.
    let result = client.try_check_and_record(&key);

    assert_eq!(
        result,
        Err(Ok(Error::RateLimitExceeded))
    );

    // The rejected call must not change the count.
    assert_eq!(
        client.remaining(&key),
        0
    );
}

#[test]
fn test_window_resets_after_boundary() {
    let env = Env::default();

    let (client, _admin, key) = setup(
        &env,
        10,
        2,
    );

    let start_ledger = env.ledger().sequence();

    // First two calls consume the window.
    client.check_and_record(&key);
    client.check_and_record(&key);

    assert_eq!(
        client.remaining(&key),
        0
    );

    // Move to the exact window boundary.
    env.ledger().set_sequence_number(
        start_ledger + 10,
    );

    // The previous window has expired.
    assert_eq!(
        client.remaining(&key),
        2
    );

    // A new call should now be allowed.
    client.check_and_record(&key);

    assert_eq!(
        client.remaining(&key),
        1
    );
}

#[test]
fn test_different_keys_have_independent_limits() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let key_a = Address::generate(&env);
    let key_b = Address::generate(&env);

    let contract_id = env.register(RateLimiter, ());

    let client = RateLimiterClient::new(
        &env,
        &contract_id,
    );

    client.initialize(
        &admin,
        &10,
        &2,
    );

    // Consume key A's entire allowance.
    client.check_and_record(&key_a);
    client.check_and_record(&key_a);

    assert_eq!(
        client.remaining(&key_a),
        0
    );

    // Key B has its own independent allowance.
    assert_eq!(
        client.remaining(&key_b),
        2
    );

    client.check_and_record(&key_b);

    assert_eq!(
        client.remaining(&key_b),
        1
    );

    assert_eq!(
        client.remaining(&key_a),
        0
    );
}

#[test]
fn test_remaining_returns_full_limit_for_new_key() {
    let env = Env::default();

    let (client, _admin, key) = setup(
        &env,
        20,
        5,
    );

    assert_eq!(
        client.remaining(&key),
        5
    );
}

#[test]
fn test_invalid_limits_are_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);

    let contract_id = env.register(RateLimiter, ());

    let client = RateLimiterClient::new(
        &env,
        &contract_id,
    );

    let result = client.try_initialize(
        &admin,
        &0,
        &5,
    );

    assert_eq!(
        result,
        Err(Ok(Error::InvalidLimits))
    );
}

#[test]
fn test_max_calls_zero_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);

    let contract_id = env.register(RateLimiter, ());

    let client = RateLimiterClient::new(
        &env,
        &contract_id,
    );

    let result = client.try_initialize(
        &admin,
        &10,
        &0,
    );

    assert_eq!(
        result,
        Err(Ok(Error::InvalidLimits))
    );
}

#[test]
fn test_initialize_only_once() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);

    let contract_id = env.register(RateLimiter, ());

    let client = RateLimiterClient::new(
        &env,
        &contract_id,
    );

    client.initialize(
        &admin,
        &10,
        &3,
    );

    let result = client.try_initialize(
        &admin,
        &10,
        &3,
    );

    assert_eq!(
        result,
        Err(Ok(Error::AlreadyInitialized))
    );
}

#[test]
fn test_set_limits_updates_configuration() {
    let env = Env::default();

    let (client, admin, key) = setup(
        &env,
        10,
        3,
    );

    assert_eq!(
        client.remaining(&key),
        3
    );

    client.set_limits(
        &admin,
        &20,
        &5,
    );

    assert_eq!(
        client.remaining(&key),
        5
    );
}

#[test]
fn test_unauthorized_set_limits_is_rejected() {
    let env = Env::default();

    let (client, _admin, _key) = setup(
        &env,
        10,
        3,
    );

    let unauthorized = Address::generate(&env);

    let result = client.try_set_limits(
        &unauthorized,
        &20,
        &5,
    );

    assert_eq!(
        result,
        Err(Ok(Error::Unauthorized))
    );
}