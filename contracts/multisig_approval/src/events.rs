use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env, Vec};

// ─── Event payloads ──────────────────────────────────────────────────────────

/// Emitted when [`MultiSigApproval::initialize`] succeeds.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractInitialized {
    pub signers: Vec<Address>,
    pub threshold: u32,
}

/// Emitted when a new proposal is created via [`MultiSigApproval::propose`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalCreated {
    pub proposal_id: u64,
    pub action_hash: BytesN<32>,
    pub proposer: Address,
}

/// Emitted when a signer approves a proposal via [`MultiSigApproval::approve`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalApproved {
    pub proposal_id: u64,
    pub signer: Address,
    pub approval_count: u32,
    pub threshold: u32,
}

/// Emitted when the signer set is updated via [`MultiSigApproval::set_signers`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignersUpdated {
    pub signers: Vec<Address>,
    pub threshold: u32,
}

// ─── Emit helpers ────────────────────────────────────────────────────────────

pub fn emit_initialized(env: &Env, signers: Vec<Address>, threshold: u32) {
    let event = ContractInitialized { signers, threshold };
    env.events().publish((symbol_short!("init"),), event);
}

pub fn emit_proposal_created(
    env: &Env,
    proposal_id: u64,
    action_hash: BytesN<32>,
    proposer: Address,
) {
    let event = ProposalCreated {
        proposal_id,
        action_hash,
        proposer,
    };
    env.events().publish((symbol_short!("proposed"),), event);
}

pub fn emit_proposal_approved(
    env: &Env,
    proposal_id: u64,
    signer: Address,
    approval_count: u32,
    threshold: u32,
) {
    let event = ProposalApproved {
        proposal_id,
        signer,
        approval_count,
        threshold,
    };
    env.events().publish((symbol_short!("approved"),), event);
}

pub fn emit_signers_updated(env: &Env, signers: Vec<Address>, threshold: u32) {
    let event = SignersUpdated { signers, threshold };
    env.events().publish((symbol_short!("signers"),), event);
}
