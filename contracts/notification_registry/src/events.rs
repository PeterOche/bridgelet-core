use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env};

/// Emitted when a subscriber registers interest in a watched address.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Subscribed {
    pub subscriber: Address,
    pub watched_address: Address,
    pub endpoint_hash: BytesN<32>,
}

/// Emitted when an existing subscriber replaces its endpoint hash.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointUpdated {
    pub subscriber: Address,
    pub watched_address: Address,
    pub endpoint_hash: BytesN<32>,
}

/// Emitted when a subscriber stops watching an address.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Unsubscribed {
    pub subscriber: Address,
    pub watched_address: Address,
}

pub fn emit_subscribed(
    env: &Env,
    subscriber: Address,
    watched_address: Address,
    endpoint_hash: BytesN<32>,
) {
    env.events().publish(
        (symbol_short!("subscribe"),),
        Subscribed {
            subscriber,
            watched_address,
            endpoint_hash,
        },
    );
}

pub fn emit_endpoint_updated(
    env: &Env,
    subscriber: Address,
    watched_address: Address,
    endpoint_hash: BytesN<32>,
) {
    env.events().publish(
        (symbol_short!("updated"),),
        EndpointUpdated {
            subscriber,
            watched_address,
            endpoint_hash,
        },
    );
}

pub fn emit_unsubscribed(env: &Env, subscriber: Address, watched_address: Address) {
    env.events().publish(
        (symbol_short!("unsub"),),
        Unsubscribed {
            subscriber,
            watched_address,
        },
    );
}
