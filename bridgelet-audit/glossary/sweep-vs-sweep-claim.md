# Glossary Entry: `sweep` vs `sweep_claim`

**Path:** `bridgelet-audit/glossary/sweep-vs-sweep-claim.md`  
**Component:** `EphemeralAccount` & `SweepController`  
**Related Functions:** `SweepController::execute_sweep()`, `SweepController::claim()`, `EphemeralAccount::sweep()`, `EphemeralAccount::sweep_claim()`

---

## Overview

Bridgelet Core's `EphemeralAccountContract` provides two entrypoints for executing a sweep:

1. **`sweep()`**: The off-chain-signed flow, invoked exclusively by `SweepController::execute_sweep()`
2. **`sweep_claim()`**: The native-auth claim flow, invoked exclusively by `SweepController::claim()`

Both entrypoints ultimately produce identical state transitions and reserve-reclaim behavior.

---

## Precondition Check Comparison

The table below lists all precondition checks performed by each entrypoint before allowing a sweep to execute. Most checks are shared, but they differ in their authorization mechanisms and input requirements.

| Precondition Check | `EphemeralAccount::sweep()` (off-chain flow) | `EphemeralAccount::sweep_claim()` (native-auth flow) |
| :--- | :---: | :---: |
| Contract must be initialized | ✅ | ✅ |
| Account status must not already be `Swept` | ✅ | ✅ |
| Account must have received at least one payment (`has_payment_received()`) | ✅ | ✅ |
| Account must not be expired (`!is_expired()`) | ✅ | ✅ |
| Caller must be the `authorized_controller` | ✅ | ✅ |
| Ed25519 signature verification (of `auth_signature`) | ✅ | ❌ (not required) |
| Requires `auth_signature` input parameter | ✅ | ❌ |

### Key Differences in Preconditions
- **Authorization method**: The `sweep()` entrypoint verifies an Ed25519 signature passed as `auth_signature`, while `sweep_claim()` relies on Soroban's native `require_auth()` enforced by the `SweepController`
- **Input requirements**: `sweep()` requires an additional `auth_signature` parameter that `sweep_claim()` does not need
- **Controller verification**: Both ensure the caller is the authorized controller, but `sweep_claim()` enforces this via `controller.require_auth()` immediately upon entry

---

## Intended Callers & Execution Flows

### 1. Off-Chain-Signed Flow (`sweep()` invoked by `SweepController::execute_sweep()`)
**Intended caller**: Only `SweepController::execute_sweep()` — this entrypoint should never be called directly.
- The off-chain Bridgelet signer produces an Ed25519 signature over `hash(destination || nonce || controller_id)`
- A relayer submits a transaction invoking `SweepController::execute_sweep(ephemeral_account, destination, auth_signature)`
- `SweepController` verifies the Ed25519 signature, increments its internal sweep nonce, and authorizes the cross-contract call
- `SweepController` calls `EphemeralAccount::sweep()`, which performs its own precondition checks including re-verifying the signature
- The account transitions to `Swept`, reserve is reclaimed, and tokens are transferred to the destination

### 2. Native-Auth Claim Flow (`sweep_claim()` invoked by `SweepController::claim()`)
**Intended caller**: Only `SweepController::claim()` — this entrypoint should never be called directly.
- The recipient signs a Soroban native authorization entry for `SweepController::claim(recipient, ephemeral_account)`
- A relayer submits the transaction and pays all gas fees
- `SweepController` first verifies `recipient.require_auth()` to confirm the recipient has authorized the claim
- `SweepController` validates the destination against any `authorized_destination` restrictions
- `SweepController` calls `EphemeralAccount::sweep_claim(recipient)`, which verifies the caller is the authorized controller
- The account transitions to `Swept`, reserve is reclaimed, and tokens are transferred to the recipient

---

## Shared Outcome (Identical State & Reserve Behavior)

Despite their different authorization flows and precondition check ordering, both entrypoints ultimately execute identical final logic:
1. Set account status to `AccountStatus::Swept`
2. Record the destination address the account was swept to
3. Generate a sweep ID from the current ledger sequence
4. Emit the `SweepExecutedMulti` event with all payments
5. Reclaim the account's reserve balance to the destination via `reclaim_reserve_to()`

Both flows result in the same final state for the ephemeral account and identical reserve-reclaim semantics.