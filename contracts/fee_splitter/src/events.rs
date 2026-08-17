use soroban_sdk::{contracttype, symbol_short, Address, Env, Vec};

// ─── Event payloads ──────────────────────────────────────────────────────────

/// Emitted when [`FeeSplitter::initialize`] succeeds.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractInitialized {
    pub admin: Address,
    pub payee_count: u32,
}

/// Emitted once per `split` call for each payee that received funds.
///
/// Emitting individual per-payee events makes it straightforward to reconcile
/// individual receipts in indexers without parsing the total amount or shares.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitExecuted {
    pub asset: Address,
    pub payee: Address,
    pub amount: i128,
    pub share_bps: u32,
}

/// Emitted when the payee configuration is updated via
/// [`FeeSplitter::set_payees`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayeesUpdated {
    pub payees: Vec<Address>,
    pub shares_bps: Vec<u32>,
}

// ─── Emit helpers ────────────────────────────────────────────────────────────

pub fn emit_initialized(env: &Env, admin: Address, payee_count: u32) {
    let event = ContractInitialized { admin, payee_count };
    env.events().publish((symbol_short!("init"),), event);
}

pub fn emit_split_executed(
    env: &Env,
    asset: Address,
    payee: Address,
    amount: i128,
    share_bps: u32,
) {
    let event = SplitExecuted {
        asset,
        payee,
        amount,
        share_bps,
    };
    env.events().publish((symbol_short!("split"),), event);
}

pub fn emit_payees_updated(env: &Env, payees: Vec<Address>, shares_bps: Vec<u32>) {
    let event = PayeesUpdated { payees, shares_bps };
    env.events().publish((symbol_short!("payees"),), event);
}
