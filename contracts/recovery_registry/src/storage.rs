use soroban_sdk::{contracttype, Address, Env, Vec};

// ─── Data structures ─────────────────────────────────────────────────────────

/// The guardian configuration for one registered account.
#[contracttype]
#[derive(Clone)]
pub struct GuardianSet {
    /// Ordered list of guardian addresses.
    pub guardians: Vec<Address>,
    /// Number of approvals required to authorise a recovery.
    pub threshold: u32,
}

// ─── Storage keys ─────────────────────────────────────────────────────────────

/// Key for the guardian set of a specific account.
#[contracttype]
#[derive(Clone)]
pub struct GuardianSetKey {
    pub account: Address,
}

/// Key for the approval count a specific (account, new_owner) pair has
/// accumulated.
#[contracttype]
#[derive(Clone)]
pub struct ApprovalCountKey {
    pub account: Address,
    pub new_owner: Address,
}

/// Key tracking whether a specific guardian has already approved a given
/// (account, new_owner) pair — prevents double-voting.
#[contracttype]
#[derive(Clone)]
pub struct ApprovalFlagKey {
    pub account: Address,
    pub new_owner: Address,
    pub guardian: Address,
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

// ─── Guardian set ─────────────────────────────────────────────────────────────

pub fn set_guardian_set(env: &Env, account: &Address, set: &GuardianSet) {
    env.storage()
        .instance()
        .set(&GuardianSetKey { account: account.clone() }, set);
}

pub fn get_guardian_set(env: &Env, account: &Address) -> Option<GuardianSet> {
    env.storage()
        .instance()
        .get(&GuardianSetKey { account: account.clone() })
}

pub fn has_guardian_set(env: &Env, account: &Address) -> bool {
    env.storage()
        .instance()
        .has(&GuardianSetKey { account: account.clone() })
}

// ─── Approval counts ──────────────────────────────────────────────────────────

pub fn get_approval_count(env: &Env, account: &Address, new_owner: &Address) -> u32 {
    env.storage()
        .instance()
        .get(&ApprovalCountKey {
            account: account.clone(),
            new_owner: new_owner.clone(),
        })
        .unwrap_or(0u32)
}

pub fn set_approval_count(env: &Env, account: &Address, new_owner: &Address, count: u32) {
    env.storage().instance().set(
        &ApprovalCountKey {
            account: account.clone(),
            new_owner: new_owner.clone(),
        },
        &count,
    );
}

// ─── Per-guardian approval flags ──────────────────────────────────────────────

pub fn has_guardian_approved(
    env: &Env,
    account: &Address,
    new_owner: &Address,
    guardian: &Address,
) -> bool {
    env.storage().instance().has(&ApprovalFlagKey {
        account: account.clone(),
        new_owner: new_owner.clone(),
        guardian: guardian.clone(),
    })
}

pub fn set_guardian_approved(
    env: &Env,
    account: &Address,
    new_owner: &Address,
    guardian: &Address,
) {
    env.storage().instance().set(
        &ApprovalFlagKey {
            account: account.clone(),
            new_owner: new_owner.clone(),
            guardian: guardian.clone(),
        },
        &true,
    );
}
