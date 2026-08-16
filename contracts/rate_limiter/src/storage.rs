use soroban_sdk::{
    contracttype,
    Address,
    Env,
};

#[contracttype]
#[derive(Clone)]
pub struct Limits {
    pub window_ledgers: u32,
    pub max_calls: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct RateLimitState {
    pub window_start: u32,
    pub calls: u32,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Limits,
    State(Address),
}

pub fn has_admin(env: &Env) -> bool {
    env.storage()
        .instance()
        .has(&DataKey::Admin)
}

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
}

pub fn set_admin(
    env: &Env,
    admin: &Address,
) {
    env.storage()
        .instance()
        .set(
            &DataKey::Admin,
            admin,
        );
}

pub fn get_limits(
    env: &Env,
) -> Option<Limits> {
    env.storage()
        .instance()
        .get(&DataKey::Limits)
}

pub fn set_limits(
    env: &Env,
    limits: &Limits,
) {
    env.storage()
        .instance()
        .set(
            &DataKey::Limits,
            limits,
        );
}

pub fn get_state(
    env: &Env,
    key: &Address,
) -> Option<RateLimitState> {
    env.storage()
        .persistent()
        .get(&DataKey::State(key.clone()))
}

pub fn set_state(
    env: &Env,
    key: &Address,
    state: &RateLimitState,
) {
    env.storage()
        .persistent()
        .set(
            &DataKey::State(key.clone()),
            state,
        );
}