use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

/// Emitted on first initialization.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleInitialized {
    pub admin: Address,
    pub attestor: Address,
}

/// Emitted when an attestation is written (or overwritten) for an address.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attested {
    pub attestor: Address,
    pub address: Address,
    pub status: Symbol,
    pub expiry_ledger: u32,
}

/// Emitted when the authorized attestor is rotated.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestorUpdated {
    pub admin: Address,
    pub old_attestor: Address,
    pub new_attestor: Address,
}

pub fn emit_initialized(env: &Env, admin: Address, attestor: Address) {
    let event = OracleInitialized { admin, attestor };
    env.events().publish((symbol_short!("init"),), event);
}

pub fn emit_attested(
    env: &Env,
    attestor: Address,
    address: Address,
    status: Symbol,
    expiry_ledger: u32,
) {
    let event = Attested {
        attestor,
        address,
        status,
        expiry_ledger,
    };
    env.events().publish((symbol_short!("attested"),), event);
}

pub fn emit_attestor_updated(
    env: &Env,
    admin: Address,
    old_attestor: Address,
    new_attestor: Address,
) {
    let event = AttestorUpdated {
        admin,
        old_attestor,
        new_attestor,
    };
    env.events().publish((symbol_short!("atst_upd"),), event);
}
