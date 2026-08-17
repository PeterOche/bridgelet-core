use soroban_sdk::{contracttype, Address, Env, Vec};

/// A single payee entry with their basis-point share.
#[contracttype]
#[derive(Clone, Debug)]
pub struct PayeeEntry {
    pub address: Address,
    pub share_bps: u32,
}

/// Storage keys used by the fee splitter.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Admin address (set once at initialization).
    Admin,
    /// List of payee entries (address + share_bps).
    Payees,
}

const INSTANCE_TTL_THRESHOLD: u32 = 100;
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

// ─── Payees ───────────────────────────────────────────────────────────────────

pub fn set_payees(env: &Env, payees: &Vec<PayeeEntry>) {
    env.storage().instance().set(&DataKey::Payees, payees);
}

pub fn get_payees(env: &Env) -> Option<Vec<PayeeEntry>> {
    env.storage().instance().get(&DataKey::Payees)
}
