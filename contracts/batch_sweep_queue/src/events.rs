use soroban_sdk::{contracttype, symbol_short, Address, Env};

/// Emitted once when the contract is initialized.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueueInitialized {
    pub admin: Address,
}

/// Emitted whenever a new sweep request is enqueued.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestEnqueued {
    pub id: u64,
    pub account: Address,
    pub destination: Address,
}

/// Emitted when one or more requests are marked as processed (removed).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestsProcessed {
    pub count: u32,
}

pub fn emit_initialized(env: &Env, admin: Address) {
    let event = QueueInitialized { admin };
    env.events().publish((symbol_short!("init"),), event);
}

pub fn emit_enqueued(env: &Env, id: u64, account: Address, destination: Address) {
    let event = RequestEnqueued {
        id,
        account,
        destination,
    };
    env.events().publish((symbol_short!("enqueue"),), event);
}

pub fn emit_processed(env: &Env, count: u32) {
    let event = RequestsProcessed { count };
    env.events().publish((symbol_short!("processed"),), event);
}
