#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Symbol, Vec};

pub use errors::Error;

/// A minimal on-chain registry that tracks published WASM hashes and version
/// labels for named contracts.
///
/// ## Purpose
///
/// Provides a single source of truth for the WASM hash of each named contract
/// on the platform.  Consumers can call `current` for O(1) lookup of the
/// latest version, or `history` to retrieve all past publications in order.
///
/// ## Access control
///
/// Only the `admin` set during `initialize` may call `publish`.
/// Read-only operations (`current`, `history`) are unrestricted.
#[contract]
pub struct VersionRegistry;

#[contractimpl]
impl VersionRegistry {
    /// One-time initialization that sets the admin address.
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

    /// Record a new WASM hash for the named contract.
    ///
    /// The entry is appended to the history list.  `current` will immediately
    /// return the new hash and version label.
    ///
    /// # Arguments
    /// * `admin`     – must match the admin set during `initialize`.
    /// * `name`      – the contract name (e.g. `"ephemeral_account"`).
    /// * `wasm_hash` – the 32-byte SHA-256 hash of the deployed WASM.
    /// * `version`   – a human-readable version label (e.g. `"1.0.0"`).
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract has not been initialized.
    /// * [`Error::Unauthorized`]   – `admin` does not match the stored admin.
    /// * [`Error::InvalidWasmHash`] – the supplied hash is empty.
    pub fn publish(
        env: Env,
        admin: Address,
        name: Symbol,
        wasm_hash: BytesN<32>,
        version: Symbol,
    ) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let stored_admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        if wasm_hash == soroban_sdk::BytesN::from_array(&env, &[0u8; 32]) {
            return Err(Error::InvalidWasmHash);
        }

        let ledger = env.ledger().sequence();

        storage::set_current(&env, &name, &(wasm_hash.clone(), version.clone()));
        storage::append_history(&env, &name, &(wasm_hash.clone(), version.clone(), ledger));

        events::emit_version_published(&env, name, wasm_hash, version, ledger, admin);

        Ok(())
    }

    /// Return the most recently published WASM hash and version for `name`,
    /// or `None` if no version has been published.
    pub fn current(env: Env, name: Symbol) -> Option<(BytesN<32>, Symbol)> {
        storage::extend_instance_ttl(&env);
        storage::get_current(&env, &name)
    }

    /// Return all published versions for `name` in chronological order.
    ///
    /// Each entry is `(wasm_hash, version, ledger_sequence)`.
    pub fn history(env: Env, name: Symbol) -> Vec<(BytesN<32>, Symbol, u32)> {
        storage::extend_instance_ttl(&env);
        storage::get_history(&env, &name)
    }

    /// Return the admin address, if the contract has been initialized.
    pub fn get_admin(env: Env) -> Option<Address> {
        storage::extend_instance_ttl(&env);
        storage::get_admin(&env)
    }
}
