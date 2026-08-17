use soroban_sdk::{contracttype, Address, Env, Vec};

/// Storage keys used by the allowlist registry.
///
/// ## Design: two-tier storage
///
/// 1. `Allowed(address)` — a **persistent** boolean flag per address.
///    O(1) lookup for `is_allowed`, no list scan required.
///
/// 2. `AllowedList` — an **instance** `Vec<Address>` that tracks the full
///    set of allowed addresses for `list()`.  Updated on every `add`/`remove`.
///
/// This separation ensures that `is_allowed` queries are cheap even as the
/// list grows, satisfying the acceptance criterion for O(1) lookups.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Admin address.  Set once during `initialize`.
    Admin,

    /// Marker that `address` is on the allowlist.
    /// Key present → allowed.  Key absent → not allowed.
    Allowed(Address),

    /// Ordered list of all currently-allowed addresses.
    /// Used exclusively by `list()`.
    AllowedList,
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

// ── Allowed-flag helpers ─────────────────────────────────────────────────────

/// Return `true` if `address` is currently on the allowlist.
pub fn is_allowed(env: &Env, address: &Address) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Allowed(address.clone()))
}

/// Add `address` to the allowlist (persistent flag).
pub fn set_allowed(env: &Env, address: &Address) {
    let key = DataKey::Allowed(address.clone());
    env.storage().persistent().set(&key, &true);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

/// Remove `address` from the allowlist (delete the persistent flag).
pub fn remove_allowed(env: &Env, address: &Address) {
    env.storage()
        .persistent()
        .remove(&DataKey::Allowed(address.clone()));
}

// ── AllowedList helpers ──────────────────────────────────────────────────────

/// Return the full list of allowed addresses.
pub fn get_list(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::AllowedList)
        .unwrap_or_else(|| Vec::new(env))
}

/// Append `address` to the list (no-op if already present).
pub fn list_add(env: &Env, address: &Address) {
    let mut list = get_list(env);
    // Guard: skip if already in the list (can happen on re-add after remove).
    for item in list.iter() {
        if &item == address {
            return;
        }
    }
    list.push_back(address.clone());
    env.storage().instance().set(&DataKey::AllowedList, &list);
}

/// Remove `address` from the list.
pub fn list_remove(env: &Env, address: &Address) {
    let old = get_list(env);
    let mut new_list: Vec<Address> = Vec::new(env);
    for item in old.iter() {
        if &item != address {
            new_list.push_back(item);
        }
    }
    env.storage()
        .instance()
        .set(&DataKey::AllowedList, &new_list);
}
