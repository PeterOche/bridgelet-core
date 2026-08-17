use soroban_sdk::{symbol_short, Address, Env, Symbol};

pub fn emit_paused(
    env: &Env,
    scope: Symbol,
    guardian: Address,
) {
    env.events().publish(
        (symbol_short!("paused"),),
        (scope, guardian),
    );
}

pub fn emit_unpaused(
    env: &Env,
    scope: Symbol,
    guardian: Address,
) {
    env.events().publish(
        (symbol_short!("unpaused"),),
        (scope, guardian),
    );
}