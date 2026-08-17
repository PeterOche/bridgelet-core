#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env};

pub use errors::Error;

/// An additive read/write registry that tracks claimable balances left by
/// expired ephemeral accounts.
///
/// ## Motivation
///
/// When an `EphemeralAccount` expires it emits an `AccountExpired` event
/// containing the `recovery_address`, `total_amount`, and `reserve_amount`.
/// An authorized off-chain indexer (or relayer) reads those events and writes
/// the resulting claimable amounts into this registry.  Recovery addresses or
/// downstream tooling can then call `balance_of` to discover what they are
/// owed, and `claim` to zero that balance out and receive confirmation of the
/// amount — without re-scanning raw ledger history.
///
/// ## Access control
///
/// * Only the configured `writer` (set at `initialize` time, changeable by the
///   `admin`) may call `record`.
/// * Any address may call `claim` — but only the `recovery_address` itself can
///   produce a useful result, since it receives `require_auth()`.
/// * Read-only operations (`balance_of`) are unrestricted.
///
/// ## Design notes
///
/// * Balances are *additive*: multiple `record` calls for the same
///   `(recovery_address, asset)` accumulate.
/// * `claim` zeroes the recorded balance atomically and returns the claimed
///   amount.  Calling `claim` a second time with no intervening `record`
///   returns `Err(Error::NothingToClaim)`.
/// * This contract does **not** perform token transfers — it is a pure
///   accounting registry.  The actual transfer of funds is expected to happen
///   out-of-band (e.g. by the same relayer that calls `record`).
#[contract]
pub struct ClaimableBalanceRegistry;

#[contractimpl]
impl ClaimableBalanceRegistry {
    /// One-time initialization.
    ///
    /// Sets the `admin` (who may later call `set_writer`) and the initial
    /// authorized `writer` (the only address permitted to call `record`).
    ///
    /// # Errors
    /// * [`Error::AlreadyInitialized`] – called more than once.
    pub fn initialize(env: Env, admin: Address, writer: Address) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();

        storage::set_admin(&env, &admin);
        storage::set_writer(&env, &writer);
        events::emit_initialized(&env, admin, writer);

        Ok(())
    }

    /// Replace the authorized writer.
    ///
    /// Only the `admin` may call this.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract has not been initialized.
    /// * [`Error::Unauthorized`]   – caller is not the stored admin.
    pub fn set_writer(env: Env, admin: Address, new_writer: Address) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let stored_admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        storage::set_writer(&env, &new_writer);

        Ok(())
    }

    /// Record a claimable balance on behalf of `recovery_address` for `asset`.
    ///
    /// Only the authorized `writer` may call this function.  Multiple calls
    /// for the same `(recovery_address, asset)` pair accumulate (the new
    /// `amount` is added to whatever was already recorded).
    ///
    /// # Arguments
    /// * `writer`           – must match the configured writer address.
    /// * `recovery_address` – the address that may later `claim` this balance.
    /// * `asset`            – the token contract address.
    /// * `amount`           – the positive amount to credit; must be > 0.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract has not been initialized.
    /// * [`Error::Unauthorized`]   – `writer` does not match the stored writer.
    /// * [`Error::InvalidAmount`]  – `amount` is zero or negative.
    pub fn record(
        env: Env,
        writer: Address,
        recovery_address: Address,
        asset: Address,
        amount: i128,
    ) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let stored_writer = storage::get_writer(&env).ok_or(Error::NotInitialized)?;
        if writer != stored_writer {
            return Err(Error::Unauthorized);
        }
        writer.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let current = storage::get_balance(&env, &recovery_address, &asset);
        let new_total = current + amount;
        storage::set_balance(&env, &recovery_address, &asset, new_total);

        events::emit_recorded(&env, recovery_address, asset, amount, new_total);

        Ok(())
    }

    /// Claim the recorded balance for `(recovery_address, asset)`.
    ///
    /// The `recovery_address` must authorize this call.  The stored balance is
    /// zeroed atomically, and the previously-recorded amount is returned.
    ///
    /// # Errors
    /// * [`Error::NothingToClaim`] – the balance is zero (either never recorded
    ///   or already claimed).
    pub fn claim(env: Env, recovery_address: Address, asset: Address) -> Result<i128, Error> {
        storage::extend_instance_ttl(&env);

        recovery_address.require_auth();

        let amount = storage::get_balance(&env, &recovery_address, &asset);
        if amount == 0 {
            return Err(Error::NothingToClaim);
        }

        // Zero out before returning — prevents double-claim.
        storage::set_balance(&env, &recovery_address, &asset, 0);

        events::emit_claimed(&env, recovery_address, asset, amount);

        Ok(amount)
    }

    /// Return the currently-recorded claimable balance for
    /// `(recovery_address, asset)`.  Returns 0 if nothing has been recorded.
    pub fn balance_of(env: Env, recovery_address: Address, asset: Address) -> i128 {
        storage::extend_instance_ttl(&env);
        storage::get_balance(&env, &recovery_address, &asset)
    }
}
