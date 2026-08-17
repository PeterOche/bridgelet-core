use soroban_sdk::{contracttype, Address, BytesN, Env, Symbol, Vec};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Current(Symbol),
    History(Symbol),
}

// ── TTL constants ────────────────────────────────────────────────────────────

const INSTANCE_TTL_THRESHOLD: u32 = 100;
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400; // ~30 days

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

// ── Current-version helpers ──────────────────────────────────────────────────

pub fn get_current(env: &Env, name: &Symbol) -> Option<(BytesN<32>, Symbol)> {
    env.storage()
        .instance()
        .get(&DataKey::Current(name.clone()))
}

pub fn set_current(env: &Env, name: &Symbol, version: &(BytesN<32>, Symbol)) {
    env.storage()
        .instance()
        .set(&DataKey::Current(name.clone()), version);
}

// ── History helpers ──────────────────────────────────────────────────────────

pub fn get_history(env: &Env, name: &Symbol) -> Vec<(BytesN<32>, Symbol, u32)> {
    env.storage()
        .instance()
        .get(&DataKey::History(name.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn append_history(env: &Env, name: &Symbol, entry: &(BytesN<32>, Symbol, u32)) {
    let mut history = get_history(env, name);
    history.push_back(entry.clone());
    env.storage()
        .instance()
        .set(&DataKey::History(name.clone()), &history);
}
