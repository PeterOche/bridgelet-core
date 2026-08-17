use soroban_sdk::{contracttype, Address, Env, Symbol, Vec};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Guardians,
    Threshold,
    Paused(Symbol),
    PauseApproval(Symbol, Address),
    UnpauseApproval(Symbol, Address),
}

pub fn get_guardians(env: &Env) -> Option<Vec<Address>> {
    env.storage().instance().get(&DataKey::Guardians)
}

pub fn set_guardians(env: &Env, guardians: &Vec<Address>) {
    env.storage()
        .instance()
        .set(&DataKey::Guardians, guardians);
}

pub fn get_threshold(env: &Env) -> Option<u32> {
    env.storage().instance().get(&DataKey::Threshold)
}

pub fn set_threshold(env: &Env, threshold: u32) {
    env.storage()
        .instance()
        .set(&DataKey::Threshold, &threshold);
}

pub fn is_paused(env: &Env, scope: &Symbol) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::Paused(scope.clone()))
        .unwrap_or(false)
}

pub fn set_paused(env: &Env, scope: &Symbol, paused: bool) {
    env.storage()
        .persistent()
        .set(&DataKey::Paused(scope.clone()), &paused);
}

pub fn has_pause_approval(
    env: &Env,
    scope: &Symbol,
    guardian: &Address,
) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::PauseApproval(
            scope.clone(),
            guardian.clone(),
        ))
}

pub fn set_pause_approval(
    env: &Env,
    scope: &Symbol,
    guardian: &Address,
) {
    env.storage()
        .persistent()
        .set(
            &DataKey::PauseApproval(scope.clone(), guardian.clone()),
            &true,
        );
}

pub fn remove_pause_approval(
    env: &Env,
    scope: &Symbol,
    guardian: &Address,
) {
    env.storage()
        .persistent()
        .remove(&DataKey::PauseApproval(
            scope.clone(),
            guardian.clone(),
        ));
}

pub fn has_unpause_approval(
    env: &Env,
    scope: &Symbol,
    guardian: &Address,
) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::UnpauseApproval(
            scope.clone(),
            guardian.clone(),
        ))
}

pub fn set_unpause_approval(
    env: &Env,
    scope: &Symbol,
    guardian: &Address,
) {
    env.storage()
        .persistent()
        .set(
            &DataKey::UnpauseApproval(scope.clone(), guardian.clone()),
            &true,
        );
}

pub fn remove_unpause_approval(
    env: &Env,
    scope: &Symbol,
    guardian: &Address,
) {
    env.storage()
        .persistent()
        .remove(&DataKey::UnpauseApproval(
            scope.clone(),
            guardian.clone(),
        ));
}