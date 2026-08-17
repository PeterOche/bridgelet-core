#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env};

pub use errors::Error;
pub use storage::{ActionRecord, ActionStatus, DataKey};

/// A generic timelock contract that lets an admin queue actions with a
/// mandatory delay before they can be executed.
///
/// ## Usage pattern
/// 1. Deploy and call [`initialize`] once with an admin address and a
///    `min_delay` expressed in ledger sequence units.
/// 2. The admin calls [`queue`] with a target address, an opaque
///    `action_hash` (a 32-byte commitment to the off-chain action parameters),
///    and an `eta` (the earliest ledger sequence at which `execute` may run).
/// 3. After the current ledger sequence reaches `eta`, anyone may call
///    [`execute`] to mark the action as executed and emit the event.
/// 4. The admin may call [`cancel`] at any time before execution to abort.
///
/// This contract is intentionally generic: it does **not** call external
/// contracts.  The `target` field is recorded for informational / event
/// purposes only.  Callers are responsible for verifying approval status
/// before performing the underlying privileged action.
#[contract]
pub struct TimelockController;

#[contractimpl]
impl TimelockController {
    /// One-time initialization.
    ///
    /// # Errors
    /// * [`Error::AlreadyInitialized`] – called more than once.
    pub fn initialize(env: Env, admin: Address, min_delay: u64) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();

        storage::set_admin(&env, &admin);
        storage::set_min_delay(&env, min_delay);
        events::emit_initialized(&env, admin, min_delay);

        Ok(())
    }

    /// Queue an action for delayed execution.
    ///
    /// `eta` must be at least `current_ledger + min_delay`. The `admin`
    /// address must authorize the call.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`]  – contract not initialized.
    /// * [`Error::Unauthorized`]    – caller is not the admin.
    /// * [`Error::EtaTooEarly`]     – `eta < now + min_delay`.
    /// * [`Error::AlreadyQueued`]   – action with this hash already pending.
    pub fn queue(
        env: Env,
        admin: Address,
        target: Address,
        action_hash: BytesN<32>,
        eta: u64,
    ) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let stored_admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;
        if stored_admin != admin {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        let min_delay = storage::get_min_delay(&env).ok_or(Error::NotInitialized)?;
        let now = env.ledger().sequence() as u64;
        if eta < now + min_delay {
            return Err(Error::EtaTooEarly);
        }

        // Reject if an active (Pending) record already exists for this hash.
        if let Some(existing) = storage::get_action(&env, &action_hash) {
            if existing.status == ActionStatus::Pending {
                return Err(Error::AlreadyQueued);
            }
        }

        let record = ActionRecord {
            target: target.clone(),
            eta,
            status: ActionStatus::Pending,
        };
        storage::set_action(&env, &action_hash, &record);

        events::emit_action_queued(&env, action_hash, target, eta, admin);

        Ok(())
    }

    /// Mark a queued action as executed.
    ///
    /// The action must exist, be in `Pending` state, and the current ledger
    /// sequence must have reached `eta`.
    ///
    /// # Errors
    /// * [`Error::NotQueued`]      – action_hash not in queue.
    /// * [`Error::Cancelled`]      – action was previously cancelled.
    /// * [`Error::AlreadyExecuted`]– action was already executed.
    /// * [`Error::NotReady`]       – current ledger < eta.
    pub fn execute(env: Env, action_hash: BytesN<32>) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let mut record = storage::get_action(&env, &action_hash).ok_or(Error::NotQueued)?;

        match record.status {
            ActionStatus::Cancelled => return Err(Error::Cancelled),
            ActionStatus::Executed => return Err(Error::AlreadyExecuted),
            ActionStatus::Pending => {}
        }

        let now = env.ledger().sequence() as u64;
        if now < record.eta {
            return Err(Error::NotReady);
        }

        record.status = ActionStatus::Executed;
        storage::set_action(&env, &action_hash, &record);

        events::emit_action_executed(&env, action_hash);

        Ok(())
    }

    /// Cancel a pending action.
    ///
    /// Only the admin may cancel. The action must be in `Pending` state.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`]  – contract not initialized.
    /// * [`Error::Unauthorized`]    – caller is not the admin.
    /// * [`Error::NotQueued`]       – action_hash not in queue.
    /// * [`Error::AlreadyExecuted`] – action was already executed.
    /// * [`Error::Cancelled`]       – action was already cancelled.
    pub fn cancel(env: Env, admin: Address, action_hash: BytesN<32>) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let stored_admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;
        if stored_admin != admin {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        let mut record = storage::get_action(&env, &action_hash).ok_or(Error::NotQueued)?;

        match record.status {
            ActionStatus::Executed => return Err(Error::AlreadyExecuted),
            ActionStatus::Cancelled => return Err(Error::Cancelled),
            ActionStatus::Pending => {}
        }

        record.status = ActionStatus::Cancelled;
        storage::set_action(&env, &action_hash, &record);

        events::emit_action_cancelled(&env, action_hash, admin);

        Ok(())
    }

    /// Returns `true` if the action exists, is in `Pending` state, and the
    /// current ledger sequence has reached its `eta`.
    pub fn is_ready(env: Env, action_hash: BytesN<32>) -> bool {
        storage::extend_instance_ttl(&env);

        match storage::get_action(&env, &action_hash) {
            Some(record) => {
                record.status == ActionStatus::Pending
                    && env.ledger().sequence() as u64 >= record.eta
            }
            None => false,
        }
    }

    /// Returns the admin address, if initialized.
    pub fn get_admin(env: Env) -> Option<Address> {
        storage::extend_instance_ttl(&env);
        storage::get_admin(&env)
    }

    /// Returns the configured minimum delay.
    pub fn get_min_delay(env: Env) -> Option<u64> {
        storage::extend_instance_ttl(&env);
        storage::get_min_delay(&env)
    }
}
