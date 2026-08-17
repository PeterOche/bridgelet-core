use crate::{
    Error,
    PauseGuardian,
    PauseGuardianClient,
};

use soroban_sdk::{
    symbol_short,
    testutils::{
        Address as _,
    },
    Address,
    Env,
    Vec,
};

fn setup(
    env: &Env,
    threshold: u32,
) -> (
    PauseGuardianClient<'_>,
    Address,
    Address,
    Address,
) {
    env.mock_all_auths();

    let guardian_a = Address::generate(env);
    let guardian_b = Address::generate(env);
    let guardian_c = Address::generate(env);

    let mut guardians = Vec::new(env);
    guardians.push_back(guardian_a.clone());
    guardians.push_back(guardian_b.clone());
    guardians.push_back(guardian_c);

    let contract_id =
        env.register(PauseGuardian, ());

    let client =
        PauseGuardianClient::new(
            env,
            &contract_id,
        );

    client.initialize(
        &guardians,
        &threshold,
    );

    (
        client,
        guardian_a,
        guardian_b,
        Address::generate(env),
    )
}

#[test]
fn test_initialize() {
    let env = Env::default();

    let (
        client,
        _guardian_a,
        _guardian_b,
        scope_owner,
    ) = setup(&env, 2);

    let scope = symbol_short!("sweeps");

    assert_eq!(
        client.is_paused(&scope),
        false
    );

    let _ = scope_owner;
}

#[test]
fn test_one_guardian_is_not_enough() {
    let env = Env::default();

    let (
        client,
        guardian_a,
        _guardian_b,
        _,
    ) = setup(&env, 2);

    let scope = symbol_short!("sweeps");

    client.pause(
        &guardian_a,
        &scope,
    );

    assert_eq!(
        client.is_paused(&scope),
        false
    );
}

#[test]
fn test_threshold_pauses_scope() {
    let env = Env::default();

    let (
        client,
        guardian_a,
        guardian_b,
        _,
    ) = setup(&env, 2);

    let scope = symbol_short!("sweeps");

    client.pause(
        &guardian_a,
        &scope,
    );

    assert_eq!(
        client.is_paused(&scope),
        false
    );

    client.pause(
        &guardian_b,
        &scope,
    );

    assert_eq!(
        client.is_paused(&scope),
        true
    );
}

#[test]
fn test_scopes_are_isolated() {
    let env = Env::default();

    let (
        client,
        guardian_a,
        guardian_b,
        _,
    ) = setup(&env, 2);

    let sweeps = symbol_short!("sweeps");
    let global = symbol_short!("global");

    client.pause(
        &guardian_a,
        &sweeps,
    );

    client.pause(
        &guardian_b,
        &sweeps,
    );

    assert_eq!(
        client.is_paused(&sweeps),
        true
    );

    assert_eq!(
        client.is_paused(&global),
        false
    );
}

#[test]
fn test_unpause_requires_threshold() {
    let env = Env::default();

    let (
        client,
        guardian_a,
        guardian_b,
        _,
    ) = setup(&env, 2);

    let scope = symbol_short!("sweeps");

    client.pause(
        &guardian_a,
        &scope,
    );

    client.pause(
        &guardian_b,
        &scope,
    );

    assert_eq!(
        client.is_paused(&scope),
        true
    );

    client.unpause(
        &guardian_a,
        &scope,
    );

    assert_eq!(
        client.is_paused(&scope),
        true
    );

    client.unpause(
        &guardian_b,
        &scope,
    );

    assert_eq!(
        client.is_paused(&scope),
        false
    );
}

#[test]
fn test_unauthorized_guardian_is_rejected() {
    let env = Env::default();

    let (
        client,
        _guardian_a,
        _guardian_b,
        _,
    ) = setup(&env, 2);

    let unauthorized =
        Address::generate(&env);

    let scope = symbol_short!("sweeps");

    let result = client.try_pause(
        &unauthorized,
        &scope,
    );

    assert_eq!(
        result,
        Err(Ok(Error::Unauthorized))
    );
}

#[test]
fn test_invalid_threshold_is_rejected() {
    let env = Env::default();

    env.mock_all_auths();

    let guardian =
        Address::generate(&env);

    let mut guardians = Vec::new(&env);
    guardians.push_back(guardian);

    let contract_id =
        env.register(PauseGuardian, ());

    let client =
        PauseGuardianClient::new(
            &env,
            &contract_id,
        );

    let result = client.try_initialize(
        &guardians,
        &0,
    );

    assert_eq!(
        result,
        Err(Ok(Error::InvalidThreshold))
    );
}