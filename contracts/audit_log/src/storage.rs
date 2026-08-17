use soroban_sdk::{contracttype, Address, Env, Symbol};

/// An immutable audit entry stored on-chain.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEntry {
    /// The address that called `record` (authorized writer).
    pub writer: Address,
    /// The address that performed the audited action.
    pub actor: Address,
    /// Short symbol describing the action.
    pub action: Symbol,
    /// The address the action was performed on or against.
    pub subject: Address,
    /// Ledger sequence number at the time of recording.
    pub ledger: u32,
}

/// Storage keys used by the audit log contract.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Admin address.  Set once during `initialize`.
    Admin,

    /// Monotonically increasing counter; the next entry will receive this ID.
    /// Starts at 0.
    Counter,

    /// Marker that `writer` is authorized to call `record`.
    Writer(Address),

    /// The audit entry with the given sequential ID.
    Entry(u64),
}

// ── TTL constants ────────────────────────────────────────────────────────────

const INSTANCE_TTL_THRESHOLD: u32 = 100;
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400; // ~30 days

const PERSISTENT_TTL_THRESHOLD: u32 = 100;
const PERSISTENT_TTL_EXTEND_TO: u32 = 6_307_200; // ~1 year

// ── Instance TTL ─────────────────────────────────────────────────────────────

pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}

// ── Admin helpers ────────────────────────────────────────────────────────────

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Admin)
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

// ── Writer helpers ───────────────────────────────────────────────────────────

/// Mark `writer` as authorized to call `record`.
pub fn authorize_writer(env: &Env, writer: &Address) {
    let key = DataKey::Writer(writer.clone());
    env.storage().persistent().set(&key, &true);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

/// Return `true` if `writer` has been authorized.
pub fn is_authorized_writer(env: &Env, writer: &Address) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Writer(writer.clone()))
}

// ── Counter helpers ───────────────────────────────────────────────────────────

/// Return the current counter value (= ID of the *next* entry to be written).
pub fn get_counter(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::Counter)
        .unwrap_or(0u64)
}

/// Increment the counter and return the ID that was just consumed.
pub fn increment_counter(env: &Env) -> u64 {
    let id = get_counter(env);
    env.storage().instance().set(&DataKey::Counter, &(id + 1));
    id
}

// ── Entry helpers ─────────────────────────────────────────────────────────────

/// Persist an audit entry under the given `id`.
pub fn set_entry(env: &Env, id: u64, entry: &AuditEntry) {
    let key = DataKey::Entry(id);
    env.storage().persistent().set(&key, entry);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

/// Retrieve an audit entry by `id`, or `None` if it does not exist.
pub fn get_entry(env: &Env, id: u64) -> Option<AuditEntry> {
    env.storage().persistent().get(&DataKey::Entry(id))
}
