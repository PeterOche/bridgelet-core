#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

pub use errors::Error;
pub use storage::QueueEntry;

/// A queue contract that accumulates pending sweep requests from many
/// different ephemeral accounts so an external relayer or settlement
/// contract can drain them in one batch.
///
/// ## Access control
/// - `enqueue` is permissionless — any caller may submit a request.
/// - `mark_processed` is admin-only; only the admin set during
///   `initialize` may remove entries.
/// - `peek_batch` and `queue_length` are read-only and require no auth.
///
/// ## Ordering
/// Requests are stored in insertion order (FIFO). `peek_batch` always
/// returns the oldest `max` entries first.
#[contract]
pub struct BatchSweepQueue;

#[contractimpl]
impl BatchSweepQueue {
    /// One-time initialization. Sets the admin address that is authorized
    /// to call `mark_processed`.
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

    /// Add a new sweep request to the back of the queue.
    ///
    /// Returns the assigned request ID (monotonically increasing, starting
    /// at 0). The ID can be used later with `mark_processed` to remove
    /// exactly this request.
    ///
    /// This function is permissionless — any caller may enqueue a request.
    ///
    /// # Arguments
    /// * `account`     – the ephemeral account address to sweep from.
    /// * `destination` – the address that should receive the swept funds.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract has not been initialized yet.
    pub fn enqueue(env: Env, account: Address, destination: Address) -> Result<u64, Error> {
        storage::extend_instance_ttl(&env);

        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }

        let id = storage::get_next_id(&env);
        storage::set_next_id(&env, id + 1);

        let entry = QueueEntry {
            account: account.clone(),
            destination: destination.clone(),
        };
        storage::set_entry(&env, id, &entry);

        let mut pending = storage::get_pending_ids(&env);
        pending.push_back(id);
        storage::set_pending_ids(&env, &pending);

        events::emit_enqueued(&env, id, account, destination);

        Ok(id)
    }

    /// Return up to `max` pending requests from the front of the queue
    /// without removing them.
    ///
    /// Returns a `Vec` of `(id, account, destination)` tuples in FIFO
    /// order. If fewer than `max` requests are pending, returns all of them.
    ///
    /// This function is **read-only** — it does not mutate any queue state.
    ///
    /// # Arguments
    /// * `max` – maximum number of entries to return. Must be > 0.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract has not been initialized yet.
    /// * [`Error::InvalidBatchSize`] – `max` is zero.
    pub fn peek_batch(env: Env, max: u32) -> Result<Vec<(u64, Address, Address)>, Error> {
        storage::extend_instance_ttl(&env);

        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }
        if max == 0 {
            return Err(Error::InvalidBatchSize);
        }

        let pending = storage::get_pending_ids(&env);
        let mut result: Vec<(u64, Address, Address)> = Vec::new(&env);
        let limit = (max as usize).min(pending.len() as usize);

        for i in 0..limit {
            let id = pending.get(i as u32).unwrap();
            if let Some(entry) = storage::get_entry(&env, id) {
                result.push_back((id, entry.account, entry.destination));
            }
        }

        Ok(result)
    }

    /// Remove a specific set of request IDs from the queue, marking them
    /// as processed.
    ///
    /// Only entries whose IDs are listed in `ids` are removed — the rest
    /// of the queue is unaffected. IDs that do not exist in the queue are
    /// silently ignored (idempotent).
    ///
    /// Requires admin authorization.
    ///
    /// # Arguments
    /// * `admin` – must match the admin set at initialization.
    /// * `ids`   – the exact set of request IDs to remove.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract has not been initialized.
    /// * [`Error::Unauthorized`]   – caller is not the admin.
    pub fn mark_processed(env: Env, admin: Address, ids: Vec<u64>) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let stored_admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        // Build a set of IDs to remove for O(n) scan of pending list.
        // Soroban SDK has no HashSet, so we use a sorted Vec and binary search.
        let mut sorted_ids = ids.clone();
        // Simple insertion sort — batches are expected to be small.
        let n = sorted_ids.len();
        for i in 1..n {
            let mut j = i;
            while j > 0 && sorted_ids.get(j - 1).unwrap() > sorted_ids.get(j).unwrap() {
                let a = sorted_ids.get(j - 1).unwrap();
                let b = sorted_ids.get(j).unwrap();
                sorted_ids.set(j - 1, b);
                sorted_ids.set(j, a);
                j -= 1;
            }
        }

        let pending = storage::get_pending_ids(&env);
        let mut new_pending: Vec<u64> = Vec::new(&env);
        let mut removed: u32 = 0;

        for id in pending.iter() {
            if contains_sorted(&sorted_ids, id) {
                storage::remove_entry(&env, id);
                removed += 1;
            } else {
                new_pending.push_back(id);
            }
        }

        storage::set_pending_ids(&env, &new_pending);
        events::emit_processed(&env, removed);

        Ok(())
    }

    /// Return the number of requests currently in the queue.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract has not been initialized.
    pub fn queue_length(env: Env) -> Result<u32, Error> {
        storage::extend_instance_ttl(&env);

        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }

        Ok(storage::get_pending_ids(&env).len())
    }

    /// Return the admin address, if the contract has been initialized.
    pub fn get_admin(env: Env) -> Option<Address> {
        storage::extend_instance_ttl(&env);
        storage::get_admin(&env)
    }
}

/// Binary search on a sorted `Vec<u64>`.  Returns true if `val` is present.
fn contains_sorted(sorted: &Vec<u64>, val: u64) -> bool {
    let len = sorted.len();
    if len == 0 {
        return false;
    }
    let mut lo: u32 = 0;
    let mut hi: u32 = len - 1;
    loop {
        let mid = lo + (hi - lo) / 2;
        let mid_val = sorted.get(mid).unwrap();
        if mid_val == val {
            return true;
        } else if mid_val < val {
            if mid == hi {
                return false;
            }
            lo = mid + 1;
        } else {
            if mid == 0 {
                return false;
            }
            hi = mid - 1;
        }
        if lo > hi {
            return false;
        }
    }
}
