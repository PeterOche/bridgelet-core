use super::*;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

fn setup() -> (Env, NotificationRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(NotificationRegistry, ());
    let client = NotificationRegistryClient::new(&env, &contract_id);
    (env, client)
}

fn endpoint_hash(env: &Env, byte: u8) -> BytesN<32> {
    BytesN::from_array(env, &[byte; 32])
}

#[test]
fn subscribe_adds_subscriber() {
    let (env, client) = setup();
    let subscriber = Address::generate(&env);
    let watched = Address::generate(&env);

    client.subscribe(&subscriber, &watched, &endpoint_hash(&env, 1));

    let subscribers = client.subscribers_of(&watched);
    assert_eq!(subscribers.len(), 1);
    assert_eq!(subscribers.get(0), Some(subscriber));
}

#[test]
fn subscribe_requires_subscriber_authorization() {
    let (env, client) = setup();
    let subscriber = Address::generate(&env);
    let watched = Address::generate(&env);

    client.subscribe(&subscriber, &watched, &endpoint_hash(&env, 1));

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, subscriber);
}

#[test]
fn duplicate_subscribe_updates_without_duplicate() {
    let (env, client) = setup();
    let subscriber = Address::generate(&env);
    let watched = Address::generate(&env);

    client.subscribe(&subscriber, &watched, &endpoint_hash(&env, 1));
    client.subscribe(&subscriber, &watched, &endpoint_hash(&env, 2));

    let subscribers = client.subscribers_of(&watched);
    assert_eq!(subscribers.len(), 1);
    assert_eq!(subscribers.get(0), Some(subscriber));
}

#[test]
fn subscribers_of_unknown_address_returns_empty_vec() {
    let (env, client) = setup();
    let watched = Address::generate(&env);

    assert!(client.subscribers_of(&watched).is_empty());
}

#[test]
fn subscribers_of_returns_all_subscribers_in_order() {
    let (env, client) = setup();
    let first = Address::generate(&env);
    let second = Address::generate(&env);
    let watched = Address::generate(&env);

    client.subscribe(&first, &watched, &endpoint_hash(&env, 1));
    client.subscribe(&second, &watched, &endpoint_hash(&env, 2));

    let subscribers = client.subscribers_of(&watched);
    assert_eq!(subscribers.len(), 2);
    assert_eq!(subscribers.get(0), Some(first));
    assert_eq!(subscribers.get(1), Some(second));
}

#[test]
fn subscriptions_are_isolated_by_watched_address() {
    let (env, client) = setup();
    let subscriber = Address::generate(&env);
    let first_watched = Address::generate(&env);
    let second_watched = Address::generate(&env);

    client.subscribe(&subscriber, &first_watched, &endpoint_hash(&env, 1));

    assert_eq!(client.subscribers_of(&first_watched).len(), 1);
    assert!(client.subscribers_of(&second_watched).is_empty());
}

#[test]
fn unsubscribe_removes_only_that_subscriber() {
    let (env, client) = setup();
    let first = Address::generate(&env);
    let second = Address::generate(&env);
    let watched = Address::generate(&env);

    client.subscribe(&first, &watched, &endpoint_hash(&env, 1));
    client.subscribe(&second, &watched, &endpoint_hash(&env, 2));
    client.unsubscribe(&first, &watched);

    let subscribers = client.subscribers_of(&watched);
    assert_eq!(subscribers.len(), 1);
    assert_eq!(subscribers.get(0), Some(second));
}

#[test]
fn unsubscribe_requires_subscriber_authorization() {
    let (env, client) = setup();
    let subscriber = Address::generate(&env);
    let watched = Address::generate(&env);

    client.subscribe(&subscriber, &watched, &endpoint_hash(&env, 1));
    client.unsubscribe(&subscriber, &watched);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, subscriber);
}

#[test]
fn unsubscribe_from_one_address_preserves_other_subscription() {
    let (env, client) = setup();
    let subscriber = Address::generate(&env);
    let first_watched = Address::generate(&env);
    let second_watched = Address::generate(&env);

    client.subscribe(&subscriber, &first_watched, &endpoint_hash(&env, 1));
    client.subscribe(&subscriber, &second_watched, &endpoint_hash(&env, 2));
    client.unsubscribe(&subscriber, &first_watched);

    assert!(client.subscribers_of(&first_watched).is_empty());
    assert_eq!(client.subscribers_of(&second_watched).len(), 1);
}

#[test]
fn unsubscribe_unknown_subscription_returns_error() {
    let (env, client) = setup();
    let subscriber = Address::generate(&env);
    let watched = Address::generate(&env);

    let error = client
        .try_unsubscribe(&subscriber, &watched)
        .unwrap_err()
        .unwrap();

    assert_eq!(error, Error::NotSubscribed);
}

#[test]
fn resubscribe_after_unsubscribe_works() {
    let (env, client) = setup();
    let subscriber = Address::generate(&env);
    let watched = Address::generate(&env);

    client.subscribe(&subscriber, &watched, &endpoint_hash(&env, 1));
    client.unsubscribe(&subscriber, &watched);
    client.subscribe(&subscriber, &watched, &endpoint_hash(&env, 2));

    let subscribers = client.subscribers_of(&watched);
    assert_eq!(subscribers.len(), 1);
    assert_eq!(subscribers.get(0), Some(subscriber));
}
