#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, token, Address, Env, Vec};

pub use errors::Error;
pub use storage::{DataKey, PayeeEntry};

const TOTAL_BPS: u32 = 10_000;

/// A general-purpose fee-splitting contract.
///
/// ## Usage pattern
/// 1. Deploy and call [`initialize`] with the admin, payee addresses, and
///    their respective shares expressed in basis points (1 bps = 0.01%).
///    The shares must sum to exactly 10 000 (= 100%).
/// 2. Callers fund the splitter by calling [`split`] with an asset and amount.
///    Each payee receives their pro-rata portion atomically.
/// 3. The admin may update the payee configuration at any time via
///    [`set_payees`] without redeploying the contract.
///
/// ## Rounding
/// Each payee receives `floor(amount * share_bps / 10_000)` tokens.
/// Any remainder from rounding is credited to the **last** payee in the list
/// to guarantee the exact `amount` is always transferred in full.
#[contract]
pub struct FeeSplitter;

#[contractimpl]
impl FeeSplitter {
    /// One-time initialization.
    ///
    /// `payees` and `shares_bps` must have the same length.
    /// `shares_bps` must sum to exactly 10 000.
    ///
    /// # Errors
    /// * [`Error::AlreadyInitialized`] – called more than once.
    /// * [`Error::NoPayees`]           – empty payee list.
    /// * [`Error::LengthMismatch`]     – payees and shares_bps differ in length.
    /// * [`Error::ZeroShare`]          – at least one share is 0.
    /// * [`Error::SharesDoNotSum`]     – shares don't sum to 10 000.
    pub fn initialize(
        env: Env,
        admin: Address,
        payees: Vec<Address>,
        shares_bps: Vec<u32>,
    ) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();

        let entries = Self::build_and_validate_payees(&env, &payees, &shares_bps)?;
        let payee_count = entries.len();

        storage::set_admin(&env, &admin);
        storage::set_payees(&env, &entries);
        events::emit_initialized(&env, admin, payee_count);

        Ok(())
    }

    /// Atomically split `amount` of `asset` from `from` across all configured
    /// payees.
    ///
    /// `from` must have authorized this contract to move `amount` tokens on
    /// their behalf (standard SEP-41 allowance).
    ///
    /// Rounding remainder goes to the last payee.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract not initialized.
    /// * [`Error::InvalidAmount`]  – amount is zero or negative.
    pub fn split(env: Env, from: Address, asset: Address, amount: i128) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        from.require_auth();

        let entries = storage::get_payees(&env).ok_or(Error::NotInitialized)?;
        let token_client = token::Client::new(&env, &asset);

        let total_entries = entries.len();
        let mut distributed: i128 = 0;

        for (i, entry) in entries.iter().enumerate() {
            let payee_amount = if i as u32 == total_entries - 1 {
                // Last payee gets the remainder to absorb rounding dust.
                amount - distributed
            } else {
                (amount * entry.share_bps as i128) / TOTAL_BPS as i128
            };

            token_client.transfer(&from, &entry.address, &payee_amount);
            events::emit_split_executed(
                &env,
                asset.clone(),
                entry.address.clone(),
                payee_amount,
                entry.share_bps,
            );
            distributed += payee_amount;
        }

        Ok(())
    }

    /// Update the payee configuration.
    ///
    /// Only the admin may call this function. Does **not** affect in-flight
    /// `split` calls (those are atomic).
    ///
    /// # Errors
    /// * [`Error::NotInitialized`]  – contract not initialized.
    /// * [`Error::Unauthorized`]    – caller is not the admin.
    /// * [`Error::NoPayees`]        – empty payee list.
    /// * [`Error::LengthMismatch`]  – payees and shares_bps differ in length.
    /// * [`Error::ZeroShare`]       – at least one share is 0.
    /// * [`Error::SharesDoNotSum`]  – shares don't sum to 10 000.
    pub fn set_payees(
        env: Env,
        admin: Address,
        payees: Vec<Address>,
        shares_bps: Vec<u32>,
    ) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let stored_admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;
        if stored_admin != admin {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        let entries = Self::build_and_validate_payees(&env, &payees, &shares_bps)?;
        storage::set_payees(&env, &entries);
        events::emit_payees_updated(&env, payees, shares_bps);

        Ok(())
    }

    /// Return the current payee list as `(Address, share_bps)` tuples.
    pub fn get_payees(env: Env) -> Vec<(Address, u32)> {
        storage::extend_instance_ttl(&env);

        let mut result = Vec::new(&env);
        if let Some(entries) = storage::get_payees(&env) {
            for entry in entries.iter() {
                result.push_back((entry.address.clone(), entry.share_bps));
            }
        }
        result
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    fn build_and_validate_payees(
        env: &Env,
        payees: &Vec<Address>,
        shares_bps: &Vec<u32>,
    ) -> Result<Vec<PayeeEntry>, Error> {
        if payees.is_empty() {
            return Err(Error::NoPayees);
        }
        if payees.len() != shares_bps.len() {
            return Err(Error::LengthMismatch);
        }

        let mut total: u32 = 0;
        let mut entries = Vec::new(env);

        for (address, share) in payees.iter().zip(shares_bps.iter()) {
            if share == 0 {
                return Err(Error::ZeroShare);
            }
            total = total.saturating_add(share);
            entries.push_back(PayeeEntry {
                address,
                share_bps: share,
            });
        }

        if total != TOTAL_BPS {
            return Err(Error::SharesDoNotSum);
        }

        Ok(entries)
    }
}
