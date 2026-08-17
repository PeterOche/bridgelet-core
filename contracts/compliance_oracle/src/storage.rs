use soroban_sdk::{contracttype, Address, Env, Symbol};

// ─── Data structures ─────────────────────────────────────────────────────────

/// A single compliance attestation stored for one address.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attestation {
    /// A short status symbol, e.g. `Symbol::new(env, "CLEAR")` or `"BLOCKED"`.
    pub status: Symbol,
    /// The ledger sequence number after which this attestation is considered
    /// stale and `status` returns `None`.
    pub expiry_ledger: u32,
}

// ─── Storage keys ─────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// The admin address (may rotate the attestor).
    Admin,
    /// The currently authorized attestor address.
    Attestor,
    /// Attestation record keyed by the attested address.
    Attestation(Address),
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

// ─── Attestor ────────────────────────────────────────────────────────────────

pub fn set_attestor(env: &Env, attestor: &Address) {
    env.storage().instance().set(&DataKey::Attestor, attestor);
}

pub fn get_attestor(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Attestor)
}

// ─── Attestations ─────────────────────────────────────────────────────────────

pub fn set_attestation(env: &Env, address: &Address, attestation: &Attestation) {
    env.storage()
        .persistent()
        .set(&DataKey::Attestation(address.clone()), attestation);
}

pub fn get_attestation(env: &Env, address: &Address) -> Option<Attestation> {
    env.storage()
        .persistent()
        .get(&DataKey::Attestation(address.clone()))
}
