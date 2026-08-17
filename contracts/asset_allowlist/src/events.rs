use soroban_sdk::{contracttype, symbol_short, Address, Env};

// ── Event payloads ───────────────────────────────────────────────────────────

/// Emitted when [`AssetAllowlist::initialize`] is called successfully.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Initialized {
    pub admin: Address,
}

/// Emitted when an asset is added to the allowlist.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetAllowed {
    pub asset: Address,
    pub admin: Address,
}

/// Emitted when an asset is removed from the allowlist.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetDisallowed {
    pub asset: Address,
    pub admin: Address,
}

// ── Emit helpers ─────────────────────────────────────────────────────────────

pub fn emit_initialized(env: &Env, admin: Address) {
    env.events()
        .publish((symbol_short!("init"),), Initialized { admin });
}

pub fn emit_asset_allowed(env: &Env, asset: Address, admin: Address) {
    env.events()
        .publish((symbol_short!("allowed"),), AssetAllowed { asset, admin });
}

pub fn emit_asset_disallowed(env: &Env, asset: Address, admin: Address) {
    env.events().publish(
        (symbol_short!("disallowd"),),
        AssetDisallowed { asset, admin },
    );
}
