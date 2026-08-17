use soroban_sdk::{contracttype, Address, Env};

// ─── Storage keys ────────────────────────────────────────────────────────────

/// Top-level keys in instance storage.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Admin address set at initialization.
    Admin,
    /// Monotonically increasing counter; the next ID to assign.
    NextId,
    /// Ordered list of pending request IDs (u64 values packed as i64).
    /// We keep a separate list to preserve FIFO order without scanning all entries.
    PendingIds,
}

/// Per-entry key: maps a request ID to its (account, destination) pair.
#[contracttype]
#[derive(Clone)]
pub struct EntryKey {
    pub id: u64,
}

/// A single sweep request stored in the queue.
#[contracttype]
#[derive(Clone)]
pub struct QueueEntry {
    pub account: Address,
    pub destination: Address,
}

// ─── TTL ─────────────────────────────────────────────────────────────────────

const INSTANCE_TTL_THRESHOLD: u32 = 100;
/// ~30 days at ~5 s per ledger.
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400;

pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}

// ─── Admin ───────────────────────────────────────────────────────────────────

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Admin)
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

// ─── Counter ──────────────────────────────────────────────────────────────────

pub fn get_next_id(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::NextId)
        .unwrap_or(0u64)
}

pub fn set_next_id(env: &Env, id: u64) {
    env.storage().instance().set(&DataKey::NextId, &id);
}

// ─── Pending ID list ─────────────────────────────────────────────────────────

pub fn get_pending_ids(env: &Env) -> soroban_sdk::Vec<u64> {
    env.storage()
        .instance()
        .get(&DataKey::PendingIds)
        .unwrap_or_else(|| soroban_sdk::Vec::new(env))
}

pub fn set_pending_ids(env: &Env, ids: &soroban_sdk::Vec<u64>) {
    env.storage().instance().set(&DataKey::PendingIds, ids);
}

// ─── Individual entries ───────────────────────────────────────────────────────

pub fn set_entry(env: &Env, id: u64, entry: &QueueEntry) {
    env.storage()
        .instance()
        .set(&EntryKey { id }, entry);
}

pub fn get_entry(env: &Env, id: u64) -> Option<QueueEntry> {
    env.storage().instance().get(&EntryKey { id })
}

pub fn remove_entry(env: &Env, id: u64) {
    env.storage().instance().remove(&EntryKey { id });
}
