use soroban_sdk::{
    contracttype,
    symbol_short,
    Address,
    Env,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractInitialized {
    pub admin: Address,
    pub window_ledgers: u32,
    pub max_calls: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LimitsUpdated {
    pub window_ledgers: u32,
    pub max_calls: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallRecorded {
    pub key: Address,
    pub calls: u32,
}

pub fn emit_initialized(
    env: &Env,
    admin: Address,
    window_ledgers: u32,
    max_calls: u32,
) {
    let event = ContractInitialized {
        admin,
        window_ledgers,
        max_calls,
    };

    env.events()
        .publish(
            (symbol_short!("init"),),
            event,
        );
}

pub fn emit_limits_updated(
    env: &Env,
    window_ledgers: u32,
    max_calls: u32,
) {
    let event = LimitsUpdated {
        window_ledgers,
        max_calls,
    };

    env.events()
        .publish(
            (symbol_short!("limits"),),
            event,
        );
}

pub fn emit_call_recorded(
    env: &Env,
    key: Address,
    calls: u32,
) {
    let event = CallRecorded {
        key,
        calls,
    };

    env.events()
        .publish(
            (symbol_short!("record"),),
            event,
        );
}