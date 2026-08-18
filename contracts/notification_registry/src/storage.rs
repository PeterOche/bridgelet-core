use soroban_sdk::{contracttype, Address, BytesN, Env, Vec};

/// Persistent keys used by the notification registry.
///
/// Each subscription is stored separately for O(1) membership checks and
/// endpoint updates. Subscriber lists are also split by watched address so one
/// global instance-storage value cannot grow with every registry subscription.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Maps `(watched_address, subscriber)` to its endpoint hash.
    Subscription(Address, Address),
    /// Ordered subscribers for a watched address.
    Subscribers(Address),
}

const INSTANCE_TTL_THRESHOLD: u32 = 100;
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400; // ~30 days
const PERSISTENT_TTL_THRESHOLD: u32 = 100;
const PERSISTENT_TTL_EXTEND_TO: u32 = 6_307_200; // ~1 year

pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}

fn subscription_key(watched_address: &Address, subscriber: &Address) -> DataKey {
    DataKey::Subscription(watched_address.clone(), subscriber.clone())
}

fn subscribers_key(watched_address: &Address) -> DataKey {
    DataKey::Subscribers(watched_address.clone())
}

pub fn has_subscription(env: &Env, subscriber: &Address, watched_address: &Address) -> bool {
    env.storage()
        .persistent()
        .has(&subscription_key(watched_address, subscriber))
}

pub fn set_subscription(
    env: &Env,
    subscriber: &Address,
    watched_address: &Address,
    endpoint_hash: &BytesN<32>,
) {
    let key = subscription_key(watched_address, subscriber);
    env.storage().persistent().set(&key, endpoint_hash);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

pub fn remove_subscription(env: &Env, subscriber: &Address, watched_address: &Address) {
    env.storage()
        .persistent()
        .remove(&subscription_key(watched_address, subscriber));
}

pub fn get_subscribers(env: &Env, watched_address: &Address) -> Vec<Address> {
    let key = subscribers_key(watched_address);
    let subscribers = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Vec::new(env));

    if env.storage().persistent().has(&key) {
        env.storage().persistent().extend_ttl(
            &key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );
    }

    subscribers
}

pub fn add_subscriber(env: &Env, watched_address: &Address, subscriber: &Address) {
    let mut subscribers = get_subscribers(env, watched_address);
    subscribers.push_back(subscriber.clone());
    set_subscribers(env, watched_address, &subscribers);
}

pub fn remove_subscriber(env: &Env, watched_address: &Address, subscriber: &Address) {
    let old = get_subscribers(env, watched_address);
    let mut updated = Vec::new(env);

    for address in old.iter() {
        if &address != subscriber {
            updated.push_back(address);
        }
    }

    set_subscribers(env, watched_address, &updated);
}

fn set_subscribers(env: &Env, watched_address: &Address, subscribers: &Vec<Address>) {
    let key = subscribers_key(watched_address);

    if subscribers.is_empty() {
        env.storage().persistent().remove(&key);
        return;
    }

    env.storage().persistent().set(&key, subscribers);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}
