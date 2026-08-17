#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, Vec};

pub use errors::Error;

/// An admin-gated allowlist of approved recovery/sweep destination addresses.
///
/// ## Purpose
///
/// `EphemeralAccount` and `SweepController` each accept a recovery/destination
/// address as a caller-supplied argument with no shared source of truth.  This
/// registry gives operators a single place to maintain approved addresses across
/// the whole platform.  Any contract can optionally call `is_allowed` to verify
/// a destination before acting on it.
///
/// This contract is intentionally standalone — wiring it into existing contracts
/// is a separate concern tracked in follow-up issues.
///
/// ## Access control
///
/// Only the `admin` set during `initialize` may call `add` or `remove`.
/// Read-only operations (`is_allowed`, `list`) are unrestricted.
///
/// ## Storage design
///
/// * `is_allowed` is an O(1) persistent-storage lookup on `Allowed(address)`.
/// * `list` returns the full `Vec<Address>` stored in instance storage; it is
///   O(n) and intended for off-chain indexing/display, not on-chain hot paths.
#[contract]
pub struct AllowlistRegistry;

#[contractimpl]
impl AllowlistRegistry {
    /// One-time initialization that sets the admin address.
    ///
    /// Must be called exactly once.  The `admin` address is persisted and
    /// required to authorize every future `add` / `remove` call.
    ///
    /// # Errors
    /// * [`Error::AlreadyInitialized`] – called more than once.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();

        storage::set_admin(&env, &admin);
        events::emit_initialized(&env, admin);

        Ok(())
    }

    /// Add `address` to the allowlist with an optional human-readable `label`.
    ///
    /// If `address` was previously removed and is being re-added, the call
    /// succeeds and the address is marked allowed again.  The operation is
    /// idempotent if the address is already present.
    ///
    /// # Arguments
    /// * `admin`   – must match the admin set during `initialize`.
    /// * `address` – the address to allow.
    /// * `label`   – a short descriptive name for off-chain auditability.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract has not been initialized.
    /// * [`Error::Unauthorized`]   – `admin` does not match the stored admin.
    pub fn add(env: Env, admin: Address, address: Address, label: Symbol) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let stored_admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        storage::set_allowed(&env, &address);
        storage::list_add(&env, &address);
        events::emit_address_allowed(&env, address, label, admin);

        Ok(())
    }

    /// Remove `address` from the allowlist.
    ///
    /// If the address is not currently on the allowlist the call is a no-op
    /// (idempotent).
    ///
    /// # Arguments
    /// * `admin`   – must match the admin set during `initialize`.
    /// * `address` – the address to remove.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract has not been initialized.
    /// * [`Error::Unauthorized`]   – `admin` does not match the stored admin.
    pub fn remove(env: Env, admin: Address, address: Address) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let stored_admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        storage::remove_allowed(&env, &address);
        storage::list_remove(&env, &address);
        events::emit_address_removed(&env, address, admin);

        Ok(())
    }

    /// Return `true` if `address` is currently on the allowlist.
    ///
    /// This is an O(1) persistent-storage lookup.  Safe to call from other
    /// contracts in hot execution paths.
    pub fn is_allowed(env: Env, address: Address) -> bool {
        storage::extend_instance_ttl(&env);
        storage::is_allowed(&env, &address)
    }

    /// Return all currently-allowed addresses.
    ///
    /// The list is maintained in insertion order.  Addresses that were
    /// removed are not included.  This is O(n) and intended for off-chain
    /// display and indexing, not on-chain hot paths.
    pub fn list(env: Env) -> Vec<Address> {
        storage::extend_instance_ttl(&env);
        storage::get_list(&env)
    }

    /// Return the admin address, if the contract has been initialized.
    pub fn get_admin(env: Env) -> Option<Address> {
        storage::extend_instance_ttl(&env);
        storage::get_admin(&env)
    }
}
