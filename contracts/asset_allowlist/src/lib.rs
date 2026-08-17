#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

pub use errors::Error;

/// An admin-gated allowlist of approved asset addresses that ephemeral
/// accounts may receive.
///
/// ## Purpose
///
/// `SweepController` handles multi-asset transfers, but there is no shared
/// source of truth for which assets are supported/approved on the platform.
/// This contract provides that registry as an independent, queryable primitive.
/// Any contract can call `is_allowed` to verify an asset before acting on it.
///
/// This contract is intentionally standalone — wiring it into existing
/// contracts is a separate concern tracked in follow-up issues.
///
/// ## Access control
///
/// Only the `admin` set during `initialize` may call `allow` or `disallow`.
/// Read-only operations (`is_allowed`, `list`) are unrestricted.
///
/// ## Storage design
///
/// * `is_allowed` is an O(1) persistent-storage lookup on `Allowed(asset)`.
/// * `list` returns the full `Vec<Address>` stored in instance storage; it is
///   O(n) and intended for off-chain indexing/display, not on-chain hot paths.
#[contract]
pub struct AssetAllowlist;

#[contractimpl]
impl AssetAllowlist {
    /// One-time initialization that sets the admin address.
    ///
    /// Must be called exactly once.  The `admin` address is persisted and
    /// required to authorize every future `allow` / `disallow` call.
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

    /// Add `asset` to the allowlist.
    ///
    /// If `asset` is already allowed, the call is a no-op — it is **not** an
    /// error.  This satisfies the acceptance criterion: duplicate `allow` calls
    /// for an already-allowed asset are idempotent.
    ///
    /// # Arguments
    /// * `admin` – must match the admin set during `initialize`.
    /// * `asset` – the asset contract address to allow.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract has not been initialized.
    /// * [`Error::Unauthorized`]   – `admin` does not match the stored admin.
    pub fn allow(env: Env, admin: Address, asset: Address) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let stored_admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        // Idempotent: if already allowed, skip storage writes and event.
        if storage::is_allowed(&env, &asset) {
            return Ok(());
        }

        storage::set_allowed(&env, &asset);
        storage::list_add(&env, &asset);
        events::emit_asset_allowed(&env, asset, admin);

        Ok(())
    }

    /// Remove `asset` from the allowlist.
    ///
    /// If the asset is not currently on the allowlist the call is a no-op
    /// (idempotent).
    ///
    /// # Arguments
    /// * `admin` – must match the admin set during `initialize`.
    /// * `asset` – the asset contract address to disallow.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract has not been initialized.
    /// * [`Error::Unauthorized`]   – `admin` does not match the stored admin.
    pub fn disallow(env: Env, admin: Address, asset: Address) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let stored_admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        storage::remove_allowed(&env, &asset);
        storage::list_remove(&env, &asset);
        events::emit_asset_disallowed(&env, asset, admin);

        Ok(())
    }

    /// Return `true` if `asset` is currently on the allowlist.
    ///
    /// This is an O(1) persistent-storage lookup, safe to call from other
    /// contracts in hot execution paths.
    pub fn is_allowed(env: Env, asset: Address) -> bool {
        storage::extend_instance_ttl(&env);
        storage::is_allowed(&env, &asset)
    }

    /// Return all currently-allowed asset addresses.
    ///
    /// The list is maintained in insertion order.  Assets that were disallowed
    /// are not included.  This is O(n) and intended for off-chain display and
    /// indexing, not on-chain hot paths.
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
