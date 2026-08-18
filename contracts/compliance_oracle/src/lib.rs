#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

pub use errors::Error;

/// An adapter contract that stores compliance attestations written by an
/// authorized off-chain compliance service (e.g. sanctions screening results)
/// and exposes a simple status lookup that other contracts or off-chain clients
/// may consult.
///
/// ## Motivation
///
/// Nothing in the current Bridgelet contract set represents compliance status
/// for destination addresses.  This standalone oracle-adapter holds
/// attestations pushed by an authorized `attestor` and exposes a simple
/// `status(address)` call that any contract could optionally consult before
/// allowing a sweep destination.
///
/// ## Access control
///
/// * Only the configured `attestor` may call `attest`.
/// * Only the `admin` may rotate the `attestor` via `set_attestor`.
/// * `status` is unrestricted.
///
/// ## Expiry
///
/// Every attestation carries an `expiry_ledger`.  Once the current ledger
/// sequence passes `expiry_ledger`, `status` returns `None` — stale
/// attestations are treated as absent.  The `attestor` must re-attest to
/// refresh a status.
#[contract]
pub struct ComplianceOracle;

#[contractimpl]
impl ComplianceOracle {
    /// One-time initialization.
    ///
    /// Sets the `admin` (who may rotate the attestor) and the initial
    /// `attestor` (the only address permitted to call `attest`).
    ///
    /// # Errors
    /// * [`Error::AlreadyInitialized`] – called more than once.
    pub fn initialize(env: Env, admin: Address, attestor: Address) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();

        storage::set_admin(&env, &admin);
        storage::set_attestor(&env, &attestor);
        events::emit_initialized(&env, admin, attestor);

        Ok(())
    }

    /// Write (or overwrite) a compliance attestation for `address`.
    ///
    /// Only the currently authorized `attestor` may call this.  A new call
    /// for the same address replaces any prior attestation.
    ///
    /// # Arguments
    /// * `attestor`      – must match the stored attestor address.
    /// * `address`       – the address being attested.
    /// * `status`        – a short status symbol (e.g. `"CLEAR"`, `"BLOCKED"`).
    /// * `expiry_ledger` – the ledger sequence number after which the
    ///   attestation expires; must be strictly greater than the current ledger.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract has not been initialized.
    /// * [`Error::Unauthorized`]   – `attestor` does not match the stored attestor.
    /// * [`Error::InvalidExpiry`]  – `expiry_ledger` ≤ current ledger sequence.
    pub fn attest(
        env: Env,
        attestor: Address,
        address: Address,
        status: Symbol,
        expiry_ledger: u32,
    ) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let stored_attestor = storage::get_attestor(&env).ok_or(Error::NotInitialized)?;
        if attestor != stored_attestor {
            return Err(Error::Unauthorized);
        }
        attestor.require_auth();

        // An attestation that expires at or before the current ledger is
        // immediately stale and therefore useless.
        if expiry_ledger <= env.ledger().sequence() {
            return Err(Error::InvalidExpiry);
        }

        let attestation = storage::Attestation {
            status: status.clone(),
            expiry_ledger,
        };
        storage::set_attestation(&env, &address, &attestation);

        events::emit_attested(&env, attestor, address, status, expiry_ledger);

        Ok(())
    }

    /// Look up the compliance status for `address`.
    ///
    /// Returns `Some((status, expiry_ledger))` if a non-expired attestation
    /// exists, or `None` if:
    /// * no attestation has ever been written for this address, **or**
    /// * the most recent attestation has passed its `expiry_ledger`.
    pub fn status(env: Env, address: Address) -> Option<(Symbol, u32)> {
        storage::extend_instance_ttl(&env);

        let attestation = storage::get_attestation(&env, &address)?;

        // Treat stale attestations as absent.
        if env.ledger().sequence() > attestation.expiry_ledger {
            return None;
        }

        Some((attestation.status, attestation.expiry_ledger))
    }

    /// Rotate the authorized attestor.
    ///
    /// Only the `admin` may call this.
    ///
    /// # Arguments
    /// * `admin`       – must match the stored admin address.
    /// * `new_attestor` – the replacement attestor address.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract has not been initialized.
    /// * [`Error::Unauthorized`]   – `admin` does not match the stored admin.
    pub fn set_attestor(env: Env, admin: Address, new_attestor: Address) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let stored_admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        let old_attestor = storage::get_attestor(&env).ok_or(Error::NotInitialized)?;
        storage::set_attestor(&env, &new_attestor);

        events::emit_attestor_updated(&env, admin, old_attestor, new_attestor);

        Ok(())
    }
}
