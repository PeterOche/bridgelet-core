use soroban_sdk::{contracttype, Address, BytesN, Env, Vec};

/// A proposal record stored on-chain.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Proposal {
    /// Opaque commitment to the off-chain action.
    pub action_hash: BytesN<32>,
    /// Number of distinct signer approvals collected so far.
    pub approval_count: u32,
}

/// Storage keys used by the multisig contract.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Admin address (set on first initialize; required for set_signers).
    Admin,
    /// Current signer list.
    Signers,
    /// Current approval threshold.
    Threshold,
    /// Auto-incrementing proposal counter used to generate IDs.
    NextProposalId,
    /// Per-proposal record. Key: proposal_id (u64).
    Proposal(u64),
    /// Approval flag per (proposal_id, signer). Key: (u64, Address).
    Approved(u64, Address),
}

const INSTANCE_TTL_THRESHOLD: u32 = 100;
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400;
const PERSISTENT_TTL_THRESHOLD: u32 = 100;
const PERSISTENT_TTL_EXTEND_TO: u32 = 518_400;

pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}

fn extend_persistent<K>(env: &Env, key: &K)
where
    K: soroban_sdk::IntoVal<Env, soroban_sdk::Val>
        + soroban_sdk::TryFromVal<Env, soroban_sdk::Val>
        + Clone,
{
    env.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
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

// ─── Signers ─────────────────────────────────────────────────────────────────

pub fn set_signers(env: &Env, signers: &Vec<Address>) {
    env.storage().instance().set(&DataKey::Signers, signers);
}

pub fn get_signers(env: &Env) -> Option<Vec<Address>> {
    env.storage().instance().get(&DataKey::Signers)
}

pub fn is_signer(env: &Env, address: &Address) -> bool {
    if let Some(signers) = get_signers(env) {
        signers.contains(address)
    } else {
        false
    }
}

// ─── Threshold ───────────────────────────────────────────────────────────────

pub fn set_threshold(env: &Env, threshold: u32) {
    env.storage()
        .instance()
        .set(&DataKey::Threshold, &threshold);
}

pub fn get_threshold(env: &Env) -> Option<u32> {
    env.storage().instance().get(&DataKey::Threshold)
}

// ─── Proposal counter ────────────────────────────────────────────────────────

pub fn next_proposal_id(env: &Env) -> u64 {
    let current: u64 = env
        .storage()
        .instance()
        .get(&DataKey::NextProposalId)
        .unwrap_or(0u64);
    let next = current + 1;
    env.storage()
        .instance()
        .set(&DataKey::NextProposalId, &next);
    current
}

// ─── Proposals (persistent) ──────────────────────────────────────────────────

pub fn set_proposal(env: &Env, proposal_id: u64, proposal: &Proposal) {
    let key = DataKey::Proposal(proposal_id);
    env.storage().persistent().set(&key, proposal);
    extend_persistent(env, &key);
}

pub fn get_proposal(env: &Env, proposal_id: u64) -> Option<Proposal> {
    let key = DataKey::Proposal(proposal_id);
    let result = env.storage().persistent().get::<DataKey, Proposal>(&key);
    if result.is_some() {
        extend_persistent(env, &key);
    }
    result
}

// ─── Approval flags (persistent) ─────────────────────────────────────────────

pub fn set_approved(env: &Env, proposal_id: u64, signer: &Address) {
    let key = DataKey::Approved(proposal_id, signer.clone());
    env.storage().persistent().set(&key, &true);
    extend_persistent(env, &key);
}

pub fn has_approved(env: &Env, proposal_id: u64, signer: &Address) -> bool {
    let key = DataKey::Approved(proposal_id, signer.clone());
    env.storage()
        .persistent()
        .get::<DataKey, bool>(&key)
        .unwrap_or(false)
}
