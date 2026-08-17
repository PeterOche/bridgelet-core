use soroban_sdk::{contracttype, Address, Env};

// ─── Storage keys ─────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Stores the admin address.
    Admin,
    /// Stores the authorized writer address.
    Writer,
    /// Stores the claimable balance for `(recovery_address, asset)`.
    Balance(Address, Address),
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

// ─── Writer ───────────────────────────────────────────────────────────────────

pub fn set_writer(env: &Env, writer: &Address) {
    env.storage().instance().set(&DataKey::Writer, writer);
}

pub fn get_writer(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Writer)
}

// ─── Balances ─────────────────────────────────────────────────────────────────

/// Returns the currently recorded claimable balance, or 0 if none.
pub fn get_balance(env: &Env, recovery_address: &Address, asset: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::Balance(recovery_address.clone(), asset.clone()))
        .unwrap_or(0i128)
}

/// Overwrites the balance for a `(recovery_address, asset)` pair.
pub fn set_balance(env: &Env, recovery_address: &Address, asset: &Address, amount: i128) {
    env.storage().persistent().set(
        &DataKey::Balance(recovery_address.clone(), asset.clone()),
        &amount,
    );
}
