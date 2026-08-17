#![no_std]

mod errors;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env};

pub use errors::Error;

/// A reusable on-chain nonce registry for replay protection.
///
/// ## Purpose
///
/// Any Soroban contract that accepts off-chain-signed authorisations needs a
/// way to prevent the same signature from being submitted twice.  Rather than
/// each contract implementing its own nonce tracking, `NonceRegistry` provides
/// a shared, auditable primitive: callers mark a `(signer, nonce)` pair as
/// consumed and the registry guarantees it can never be consumed again.
///
/// ## Interface
///
/// | Method        | Description                                                   |
/// |---------------|---------------------------------------------------------------|
/// | `consume`     | Mark `(signer, nonce)` as used; errors if already consumed.  |
/// | `is_consumed` | Read-only check — returns `true` if the pair was consumed.   |
/// | `next_nonce`  | Suggest the next unused nonce for a signer (monotonic hint). |
///
/// ## Storage tradeoffs
///
/// Consumed entries are stored in **persistent** storage with a ~1-year TTL
/// (~6 307 200 ledgers).  This prevents replays for the expected lifetime of
/// any off-chain authorisation while keeping storage costs bounded — entries
/// are eventually archivable once the TTL lapses.
///
/// The `next_nonce` counter is also persistent so the hint remains accurate
/// across long periods of inactivity.
///
/// ## Caller responsibility
///
/// `consume` does **not** call `signer.require_auth()`.  The caller (typically
/// another contract's sweep or claim flow) is responsible for verifying the
/// signer's Ed25519 signature before invoking `consume`.  This keeps the
/// registry stateless with respect to authentication.
#[contract]
pub struct NonceRegistry;

#[contractimpl]
impl NonceRegistry {
    /// Mark `(signer, nonce)` as consumed.
    ///
    /// Succeeds exactly once for any given pair.  Subsequent calls with the
    /// same arguments return [`Error::NonceAlreadyConsumed`].
    ///
    /// The internal `next_nonce` counter for `signer` is advanced past the
    /// consumed nonce, keeping the hint strictly monotonic.
    ///
    /// # Errors
    /// * [`Error::NonceAlreadyConsumed`] – this `(signer, nonce)` was already used.
    pub fn consume(env: Env, signer: Address, nonce: u64) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        if storage::is_nonce_consumed(&env, &signer, nonce) {
            return Err(Error::NonceAlreadyConsumed);
        }

        storage::mark_nonce_consumed(&env, &signer, nonce);
        storage::advance_next_nonce(&env, &signer, nonce);

        Ok(())
    }

    /// Return `true` if `(signer, nonce)` has already been consumed.
    ///
    /// This is a pure read; it does not modify any state.
    pub fn is_consumed(env: Env, signer: Address, nonce: u64) -> bool {
        storage::extend_instance_ttl(&env);
        storage::is_nonce_consumed(&env, &signer, nonce)
    }

    /// Return the next suggested nonce for `signer`.
    ///
    /// The value is strictly increasing: every time `consume` is called
    /// for a nonce ≥ the current suggestion, the counter advances past it.
    /// Callers may use any nonce value they like — this is a hint, not an
    /// enforcement mechanism.
    ///
    /// Returns `0` if `signer` has never consumed a nonce.
    pub fn next_nonce(env: Env, signer: Address) -> u64 {
        storage::extend_instance_ttl(&env);
        storage::get_next_nonce(&env, &signer)
    }
}
