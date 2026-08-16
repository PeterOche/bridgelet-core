use soroban_sdk::{contracttype, symbol_short, Address, Env, Vec};

/// Emitted when a guardian set is registered for an account.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuardianSetRegistered {
    pub account: Address,
    pub guardians: Vec<Address>,
    pub threshold: u32,
}

/// Emitted each time a guardian approves a recovery proposal.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryApproved {
    pub account: Address,
    pub new_owner: Address,
    pub guardian: Address,
    pub approvals_so_far: u32,
    pub threshold: u32,
}

/// Emitted when approval count reaches the threshold (recovery is ready).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryReady {
    pub account: Address,
    pub new_owner: Address,
}

pub fn emit_registered(env: &Env, account: Address, guardians: Vec<Address>, threshold: u32) {
    let event = GuardianSetRegistered {
        account,
        guardians,
        threshold,
    };
    env.events().publish((symbol_short!("register"),), event);
}

pub fn emit_approved(
    env: &Env,
    account: Address,
    new_owner: Address,
    guardian: Address,
    approvals_so_far: u32,
    threshold: u32,
) {
    let event = RecoveryApproved {
        account,
        new_owner,
        guardian,
        approvals_so_far,
        threshold,
    };
    env.events().publish((symbol_short!("approved"),), event);
}

pub fn emit_ready(env: &Env, account: Address, new_owner: Address) {
    let event = RecoveryReady { account, new_owner };
    env.events().publish((symbol_short!("ready"),), event);
}
