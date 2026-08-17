use soroban_sdk::{contracttype, symbol_short, Address, Env};

/// Emitted when an authorized writer records a claimable balance.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BalanceRecorded {
    pub recovery_address: Address,
    pub asset: Address,
    pub amount: i128,
    pub new_total: i128,
}

/// Emitted when a recovery address successfully claims its balance.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BalanceClaimed {
    pub recovery_address: Address,
    pub asset: Address,
    pub amount: i128,
}

/// Emitted on first initialization.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Initialized {
    pub admin: Address,
    pub writer: Address,
}

pub fn emit_initialized(env: &Env, admin: Address, writer: Address) {
    let event = Initialized { admin, writer };
    env.events().publish((symbol_short!("init"),), event);
}

pub fn emit_recorded(
    env: &Env,
    recovery_address: Address,
    asset: Address,
    amount: i128,
    new_total: i128,
) {
    let event = BalanceRecorded {
        recovery_address,
        asset,
        amount,
        new_total,
    };
    env.events().publish((symbol_short!("recorded"),), event);
}

pub fn emit_claimed(env: &Env, recovery_address: Address, asset: Address, amount: i128) {
    let event = BalanceClaimed {
        recovery_address,
        asset,
        amount,
    };
    env.events().publish((symbol_short!("claimed"),), event);
}
