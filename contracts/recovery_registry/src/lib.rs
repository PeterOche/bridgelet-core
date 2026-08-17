#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

pub use errors::Error;
pub use storage::GuardianSet;

/// An opt-in, on-chain guardian-quorum recovery registry for ephemeral
/// accounts.
///
/// ## Motivation
///
/// `EphemeralAccount` supports a single, fixed `recovery_address` set at
/// `initialize` time.  For integrators that want a *social recovery* model —
/// where N-of-M guardians must agree before a new owner is accepted — this
/// independent registry provides that capability without requiring any
/// change to `EphemeralAccount`'s existing interface.
///
/// ## How it works
///
/// 1. An account registers a guardian set: a list of `Address` values and a
///    `threshold` (the number of approvals needed).
/// 2. Any guardian calls `approve_recovery` for a specific `(account,
///    new_owner)` pair.  Each guardian may vote at most once per pair.
/// 3. `recovery_ready` returns `true` once the threshold is met for that
///    pair.  The caller is then responsible for acting on the result (e.g.
///    calling `EphemeralAccount::expire` with `new_owner` as the recovery
///    address, or any other flow that suits the integration).
///
/// ## Constraints
/// - `threshold` must be ≥ 1 and ≤ the number of guardians.
/// - A guardian cannot approve the same `(account, new_owner)` pair twice.
/// - Once registered, a guardian set is immutable (re-registration is
///   rejected with [`Error::AlreadyRegistered`]).
#[contract]
pub struct RecoveryRegistry;

#[contractimpl]
impl RecoveryRegistry {
    /// Register a guardian set for `account`.
    ///
    /// Can only be called once per account.  The caller does **not** need to
    /// be the account itself — any address can register a guardian set for any
    /// account address, enabling delegation (e.g. a factory contract registers
    /// sets on behalf of accounts it deploys).
    ///
    /// # Arguments
    /// * `account`   – the account address being protected.
    /// * `guardians` – non-empty list of guardian addresses.
    /// * `threshold` – number of approvals required (1 ≤ threshold ≤ len(guardians)).
    ///
    /// # Errors
    /// * [`Error::AlreadyRegistered`] – a guardian set already exists for `account`.
    /// * [`Error::NoGuardians`]       – `guardians` is empty.
    /// * [`Error::InvalidThreshold`]  – `threshold` is zero or > len(guardians).
    pub fn register(
        env: Env,
        account: Address,
        guardians: Vec<Address>,
        threshold: u32,
    ) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        if storage::has_guardian_set(&env, &account) {
            return Err(Error::AlreadyRegistered);
        }

        let guardian_count = guardians.len();
        if guardian_count == 0 {
            return Err(Error::NoGuardians);
        }
        if threshold == 0 || threshold > guardian_count {
            return Err(Error::InvalidThreshold);
        }

        let set = GuardianSet {
            guardians: guardians.clone(),
            threshold,
        };
        storage::set_guardian_set(&env, &account, &set);
        events::emit_registered(&env, account, guardians, threshold);

        Ok(())
    }

    /// Record a guardian's approval for transferring `account` to `new_owner`.
    ///
    /// The `guardian` must be one of the registered guardians for `account`.
    /// Each guardian may approve a given `(account, new_owner)` pair at most
    /// once.  The `guardian` address must authorize this call.
    ///
    /// # Arguments
    /// * `guardian`  – the approving guardian (must match a registered guardian).
    /// * `account`   – the account being recovered.
    /// * `new_owner` – the proposed new owner address.
    ///
    /// # Errors
    /// * [`Error::NotRegistered`]   – `account` has no guardian set.
    /// * [`Error::NotAGuardian`]    – `guardian` is not in the registered set.
    /// * [`Error::AlreadyApproved`] – `guardian` already approved this pair.
    pub fn approve_recovery(
        env: Env,
        guardian: Address,
        account: Address,
        new_owner: Address,
    ) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let set = storage::get_guardian_set(&env, &account)
            .ok_or(Error::NotRegistered)?;

        guardian.require_auth();

        // Verify guardian is in the registered set.
        if !vec_contains(&set.guardians, &guardian) {
            return Err(Error::NotAGuardian);
        }

        // Prevent double-voting.
        if storage::has_guardian_approved(&env, &account, &new_owner, &guardian) {
            return Err(Error::AlreadyApproved);
        }

        storage::set_guardian_approved(&env, &account, &new_owner, &guardian);

        let new_count = storage::get_approval_count(&env, &account, &new_owner) + 1;
        storage::set_approval_count(&env, &account, &new_owner, new_count);

        events::emit_approved(
            &env,
            account.clone(),
            new_owner.clone(),
            guardian,
            new_count,
            set.threshold,
        );

        if new_count >= set.threshold {
            events::emit_ready(&env, account, new_owner);
        }

        Ok(())
    }

    /// Returns `true` if enough guardians have approved the transfer of
    /// `account` to `new_owner` (i.e. approval count ≥ threshold).
    ///
    /// # Arguments
    /// * `account`   – the account being recovered.
    /// * `new_owner` – the proposed new owner.
    ///
    /// Returns `false` if `account` is not registered.
    pub fn recovery_ready(env: Env, account: Address, new_owner: Address) -> bool {
        storage::extend_instance_ttl(&env);

        let set = match storage::get_guardian_set(&env, &account) {
            Some(s) => s,
            None => return false,
        };

        let count = storage::get_approval_count(&env, &account, &new_owner);
        count >= set.threshold
    }

    /// Return the registered guardian set for `account`, if any.
    pub fn get_guardian_set(env: Env, account: Address) -> Option<GuardianSet> {
        storage::extend_instance_ttl(&env);
        storage::get_guardian_set(&env, &account)
    }

    /// Return the current approval count for a specific `(account, new_owner)`
    /// pair.
    pub fn approval_count(env: Env, account: Address, new_owner: Address) -> u32 {
        storage::extend_instance_ttl(&env);
        storage::get_approval_count(&env, &account, &new_owner)
    }
}

/// Returns true if `needle` is present in `haystack`.
fn vec_contains(haystack: &Vec<Address>, needle: &Address) -> bool {
    for item in haystack.iter() {
        if &item == needle {
            return true;
        }
    }
    false
}
