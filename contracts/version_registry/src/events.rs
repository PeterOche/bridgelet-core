use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env, Symbol};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Initialized {
    pub admin: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionPublished {
    pub name: Symbol,
    pub wasm_hash: BytesN<32>,
    pub version: Symbol,
    pub ledger: u32,
    pub admin: Address,
}

pub fn emit_initialized(env: &Env, admin: Address) {
    env.events()
        .publish((symbol_short!("init"),), Initialized { admin });
}

pub fn emit_version_published(
    env: &Env,
    name: Symbol,
    wasm_hash: BytesN<32>,
    version: Symbol,
    ledger: u32,
    admin: Address,
) {
    env.events().publish(
        (symbol_short!("publish"),),
        VersionPublished {
            name,
            wasm_hash,
            version,
            ledger,
            admin,
        },
    );
}
