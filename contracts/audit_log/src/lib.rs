#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

pub use errors::Error;
pub use storage::AuditEntry;

/// An append-only, admin-gated audit log that persists structured records
/// in contract storage for on-chain queryability.
///
/// ## Purpose
///
/// Ledger events are ephemeral and fall outside their retention window.
/// This contract stores structured entries (actor, action, subject, ledger)
/// in persistent storage so they remain queryable independently of any
/// other contract's event stream.
///
/// ## Access control
///
/// * Only the `admin` set during `initialize` may call `authorize_writer`.
/// * Only authorized writers may call `record`.
/// * Read-only operations (`get`, `count`) are unrestricted.
/// * Entries are **immutable once written** — there is no update or delete
///   function exposed.
///
/// ## Entry IDs
///
/// IDs are sequential `u64` values starting at 0, assigned atomically at
/// record time.  `count()` returns the total number of entries written so far,
/// which equals the next ID that will be assigned.
#[contract]
pub struct AuditLog;

#[contractimpl]
impl AuditLog {
    /// One-time initialization that sets the admin address.
    ///
    /// Must be called exactly once.  The `admin` address is persisted and
    /// required to authorize every future `authorize_writer` call.
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

    /// Authorize `writer` to call [`record`].
    ///
    /// Only the admin may call this function.  Authorizing an already-authorized
    /// writer is a no-op.
    ///
    /// # Arguments
    /// * `admin`  – must match the admin set during `initialize`.
    /// * `writer` – the address to authorize.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract has not been initialized.
    /// * [`Error::Unauthorized`]   – `admin` does not match the stored admin.
    pub fn authorize_writer(env: Env, admin: Address, writer: Address) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let stored_admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        storage::authorize_writer(&env, &writer);
        events::emit_writer_authorized(&env, writer, admin);

        Ok(())
    }

    /// Append a new audit entry and return its sequential ID.
    ///
    /// IDs start at 0 and increment by 1 per call.  The entry is stored in
    /// persistent contract storage and is **immutable** — there is no update
    /// or delete path.
    ///
    /// # Arguments
    /// * `writer`  – must be an authorized writer (see `authorize_writer`).
    /// * `actor`   – the address that performed the audited action.
    /// * `action`  – a short symbol describing the action (≤ 9 bytes).
    /// * `subject` – the address the action was performed on or against.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`]    – contract has not been initialized.
    /// * [`Error::UnauthorizedWriter`] – `writer` has not been authorized.
    pub fn record(
        env: Env,
        writer: Address,
        actor: Address,
        action: Symbol,
        subject: Address,
    ) -> Result<u64, Error> {
        storage::extend_instance_ttl(&env);

        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }

        if !storage::is_authorized_writer(&env, &writer) {
            return Err(Error::UnauthorizedWriter);
        }

        writer.require_auth();

        let ledger = env.ledger().sequence();
        let id = storage::increment_counter(&env);

        let entry = AuditEntry {
            writer: writer.clone(),
            actor: actor.clone(),
            action: action.clone(),
            subject: subject.clone(),
            ledger,
        };

        storage::set_entry(&env, id, &entry);

        events::emit_entry_recorded(&env, id, writer, actor, action, subject, ledger);

        Ok(id)
    }

    /// Retrieve the audit entry with the given `id`, or `None` if it does not
    /// exist.
    ///
    /// The `id` must be less than `count()` to retrieve a valid entry.
    pub fn get(env: Env, id: u64) -> Option<AuditEntry> {
        storage::extend_instance_ttl(&env);
        storage::get_entry(&env, id)
    }

    /// Return the total number of entries recorded so far.
    ///
    /// This value equals the ID that will be assigned to the *next* entry.
    pub fn count(env: Env) -> u64 {
        storage::extend_instance_ttl(&env);
        storage::get_counter(&env)
    }

    /// Return the admin address, if the contract has been initialized.
    pub fn get_admin(env: Env) -> Option<Address> {
        storage::extend_instance_ttl(&env);
        storage::get_admin(&env)
    }
}
