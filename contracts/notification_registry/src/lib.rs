#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Vec};

pub use errors::Error;

/// Interface exposed by the notification registry contract.
pub trait NotificationRegistryInterface {
    fn subscribe(
        env: Env,
        subscriber: Address,
        watched_address: Address,
        endpoint_hash: BytesN<32>,
    ) -> Result<(), Error>;

    fn unsubscribe(env: Env, subscriber: Address, watched_address: Address) -> Result<(), Error>;

    fn subscribers_of(env: Env, watched_address: Address) -> Vec<Address>;
}

/// An on-chain registry of off-chain event subscribers.
///
/// Each subscriber authenticates its own mutations. A subscription associates
/// the subscriber with an address whose events it watches and stores a hash of
/// the subscriber's off-chain endpoint without exposing that endpoint on-chain.
/// Read access is unrestricted so indexers can discover interested services.
///
/// Re-subscribing to the same address is an idempotent upsert: the endpoint hash
/// is replaced, while the subscriber appears only once in `subscribers_of`.
#[contract]
pub struct NotificationRegistry;

#[contractimpl]
impl NotificationRegistry {
    /// Subscribe to events associated with `watched_address`.
    ///
    /// `subscriber` must authorize the call. If the pair already exists, this
    /// updates `endpoint_hash` without adding a duplicate list entry.
    pub fn subscribe(
        env: Env,
        subscriber: Address,
        watched_address: Address,
        endpoint_hash: BytesN<32>,
    ) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);
        subscriber.require_auth();

        let already_subscribed = storage::has_subscription(&env, &subscriber, &watched_address);

        storage::set_subscription(&env, &subscriber, &watched_address, &endpoint_hash);

        if already_subscribed {
            events::emit_endpoint_updated(&env, subscriber, watched_address, endpoint_hash);
        } else {
            storage::add_subscriber(&env, &watched_address, &subscriber);
            events::emit_subscribed(&env, subscriber, watched_address, endpoint_hash);
        }

        Ok(())
    }

    /// Remove the authenticated subscriber's subscription to `watched_address`.
    ///
    /// # Errors
    /// * [`Error::NotSubscribed`] - the subscriber does not watch this address.
    pub fn unsubscribe(
        env: Env,
        subscriber: Address,
        watched_address: Address,
    ) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);
        subscriber.require_auth();

        if !storage::has_subscription(&env, &subscriber, &watched_address) {
            return Err(Error::NotSubscribed);
        }

        storage::remove_subscription(&env, &subscriber, &watched_address);
        storage::remove_subscriber(&env, &watched_address, &subscriber);
        events::emit_unsubscribed(&env, subscriber, watched_address);

        Ok(())
    }

    /// Return subscribers for `watched_address` in subscription order.
    ///
    /// Returns an empty vector when the address has no subscribers.
    pub fn subscribers_of(env: Env, watched_address: Address) -> Vec<Address> {
        storage::extend_instance_ttl(&env);
        storage::get_subscribers(&env, &watched_address)
    }
}

impl NotificationRegistryInterface for NotificationRegistry {
    fn subscribe(
        env: Env,
        subscriber: Address,
        watched_address: Address,
        endpoint_hash: BytesN<32>,
    ) -> Result<(), Error> {
        NotificationRegistry::subscribe(env, subscriber, watched_address, endpoint_hash)
    }

    fn unsubscribe(env: Env, subscriber: Address, watched_address: Address) -> Result<(), Error> {
        NotificationRegistry::unsubscribe(env, subscriber, watched_address)
    }

    fn subscribers_of(env: Env, watched_address: Address) -> Vec<Address> {
        NotificationRegistry::subscribers_of(env, watched_address)
    }
}
