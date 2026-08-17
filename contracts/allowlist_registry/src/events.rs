use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

// ── Event payloads ───────────────────────────────────────────────────────────

/// Emitted when [`AllowlistRegistry::initialize`] is called successfully.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Initialized {
    pub admin: Address,
}

/// Emitted when an address is added to the allowlist.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddressAllowed {
    pub address: Address,
    pub label: Symbol,
    pub admin: Address,
}

/// Emitted when an address is removed from the allowlist.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddressRemoved {
    pub address: Address,
    pub admin: Address,
}

// ── Emit helpers ─────────────────────────────────────────────────────────────

pub fn emit_initialized(env: &Env, admin: Address) {
    env.events()
        .publish((symbol_short!("init"),), Initialized { admin });
}

pub fn emit_address_allowed(env: &Env, address: Address, label: Symbol, admin: Address) {
    env.events().publish(
        (symbol_short!("allowed"),),
        AddressAllowed {
            address,
            label,
            admin,
        },
    );
}

pub fn emit_address_removed(env: &Env, address: Address, admin: Address) {
    env.events().publish(
        (symbol_short!("removed"),),
        AddressRemoved { address, admin },
    );
}
