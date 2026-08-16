use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env};

// ─── Event payloads ──────────────────────────────────────────────────────────

/// Emitted when [`TimelockController::initialize`] succeeds.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractInitialized {
    pub admin: Address,
    pub min_delay: u64,
}

/// Emitted when an action is queued via [`TimelockController::queue`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionQueued {
    pub action_hash: BytesN<32>,
    pub target: Address,
    pub eta: u64,
    pub queued_by: Address,
}

/// Emitted when an action is executed via [`TimelockController::execute`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionExecuted {
    pub action_hash: BytesN<32>,
}

/// Emitted when an action is cancelled via [`TimelockController::cancel`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionCancelled {
    pub action_hash: BytesN<32>,
    pub cancelled_by: Address,
}

// ─── Emit helpers ────────────────────────────────────────────────────────────

pub fn emit_initialized(env: &Env, admin: Address, min_delay: u64) {
    let event = ContractInitialized { admin, min_delay };
    env.events().publish((symbol_short!("init"),), event);
}

pub fn emit_action_queued(
    env: &Env,
    action_hash: BytesN<32>,
    target: Address,
    eta: u64,
    queued_by: Address,
) {
    let event = ActionQueued {
        action_hash,
        target,
        eta,
        queued_by,
    };
    env.events().publish((symbol_short!("queued"),), event);
}

pub fn emit_action_executed(env: &Env, action_hash: BytesN<32>) {
    let event = ActionExecuted { action_hash };
    env.events().publish((symbol_short!("executed"),), event);
}

pub fn emit_action_cancelled(env: &Env, action_hash: BytesN<32>, cancelled_by: Address) {
    let event = ActionCancelled {
        action_hash,
        cancelled_by,
    };
    env.events().publish((symbol_short!("canceld"),), event);
}
