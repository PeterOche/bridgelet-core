use soroban_sdk::{contracttype, Address, Env};

/// Storage keys used by the nonce registry.
///
/// `Consumed(signer, nonce)` uses **persistent** storage so consumed nonces
/// survive ledger archival.  If a consumed entry were evicted it could be
/// replayed — persistent TTL prevents that without growing instance storage.
///
/// `NextNonce(signer)` also uses persistent storage so the monotonic counter
/// survives across extended periods of inactivity.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Marks that `(signer, nonce)` has been consumed.
    ///
    /// Key present → consumed.  Key absent → available.
    /// Value stored is a `bool` (always `true`) as a lightweight sentinel.
    Consumed(Address, u64),

    /// The next suggested nonce for `signer`.
    ///
    /// Strictly increasing; advanced every time `consume` succeeds for
    /// this signer with a nonce ≥ the current value.
    NextNonce(Address),
}

// ── TTL constants ────────────────────────────────────────────────────────────

/// Minimum remaining ledgers before we extend the instance TTL.
const INSTANCE_TTL_THRESHOLD: u32 = 100;

/// Target ledger lifetime for instance storage (~30 days at ~5 s/ledger).
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400;

/// Minimum remaining ledgers before we extend a persistent entry's TTL.
const PERSISTENT_TTL_THRESHOLD: u32 = 100;

/// Target ledger lifetime for persistent entries (~1 year at ~5 s/ledger).
///
/// Consumed nonces must be kept long enough that no valid off-chain signature
/// can be presented after the entry expires.  One year is a conservative
/// upper bound for the lifetime of any off-chain authorisation.
const PERSISTENT_TTL_EXTEND_TO: u32 = 6_307_200; // ≈ 1 year

// ── Instance TTL ─────────────────────────────────────────────────────────────

/// Extend the instance storage TTL.  Called from every public entry-point.
pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}

// ── Consumed-nonce helpers ───────────────────────────────────────────────────

/// Return `true` if the `(signer, nonce)` pair has been consumed.
pub fn is_nonce_consumed(env: &Env, signer: &Address, nonce: u64) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Consumed(signer.clone(), nonce))
}

/// Mark `(signer, nonce)` as consumed and extend its persistent TTL.
pub fn mark_nonce_consumed(env: &Env, signer: &Address, nonce: u64) {
    let key = DataKey::Consumed(signer.clone(), nonce);
    env.storage().persistent().set(&key, &true);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

// ── Next-nonce helpers ───────────────────────────────────────────────────────

/// Return the current `next_nonce` suggestion for `signer` (defaults to `0`).
pub fn get_next_nonce(env: &Env, signer: &Address) -> u64 {
    env.storage()
        .persistent()
        .get(&DataKey::NextNonce(signer.clone()))
        .unwrap_or(0u64)
}

/// Advance the `next_nonce` counter for `signer` past `consumed_nonce`.
///
/// If the current suggestion is already > `consumed_nonce` nothing changes.
/// Otherwise the counter is set to `consumed_nonce + 1`.
pub fn advance_next_nonce(env: &Env, signer: &Address, consumed_nonce: u64) {
    let key = DataKey::NextNonce(signer.clone());
    let current: u64 = env.storage().persistent().get(&key).unwrap_or(0u64);
    let new_next = consumed_nonce.saturating_add(1);
    if new_next > current {
        env.storage().persistent().set(&key, &new_next);
        env.storage().persistent().extend_ttl(
            &key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );
    }
}
