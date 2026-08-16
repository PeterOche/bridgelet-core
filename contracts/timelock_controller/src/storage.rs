use soroban_sdk::{contracttype, Address, BytesN, Env};

/// Status of a queued action.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionStatus {
    /// Action is queued and waiting for ETA.
    Pending,
    /// Action has been executed.
    Executed,
    /// Action has been cancelled.
    Cancelled,
}

/// Full record for a queued action.
#[contracttype]
#[derive(Clone, Debug)]
pub struct ActionRecord {
    pub target: Address,
    pub eta: u64,
    pub status: ActionStatus,
}

/// Storage keys used by the timelock contract.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Admin address (set once at initialization).
    Admin,
    /// Minimum delay (in ledgers) required between queue and execute.
    MinDelay,
    /// Per action-hash record. Key: BytesN<32>.
    Action(BytesN<32>),
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

// ─── Min delay ───────────────────────────────────────────────────────────────

pub fn set_min_delay(env: &Env, delay: u64) {
    env.storage().instance().set(&DataKey::MinDelay, &delay);
}

pub fn get_min_delay(env: &Env) -> Option<u64> {
    env.storage().instance().get(&DataKey::MinDelay)
}

// ─── Action records (persistent storage, keyed per action hash) ───────────────

const ACTION_TTL_THRESHOLD: u32 = 100;
const ACTION_TTL_EXTEND_TO: u32 = 518_400;

pub fn set_action(env: &Env, action_hash: &BytesN<32>, record: &ActionRecord) {
    env.storage()
        .persistent()
        .set(&DataKey::Action(action_hash.clone()), record);
    env.storage().persistent().extend_ttl(
        &DataKey::Action(action_hash.clone()),
        ACTION_TTL_THRESHOLD,
        ACTION_TTL_EXTEND_TO,
    );
}

pub fn get_action(env: &Env, action_hash: &BytesN<32>) -> Option<ActionRecord> {
    let key = DataKey::Action(action_hash.clone());
    if let Some(record) = env
        .storage()
        .persistent()
        .get::<DataKey, ActionRecord>(&key)
    {
        env.storage()
            .persistent()
            .extend_ttl(&key, ACTION_TTL_THRESHOLD, ACTION_TTL_EXTEND_TO);
        Some(record)
    } else {
        None
    }
}
