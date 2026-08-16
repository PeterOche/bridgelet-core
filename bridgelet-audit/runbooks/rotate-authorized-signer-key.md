# Runbook: Rotating `SweepController`'s `authorized_signer` Key

**Audience:** operators / SREs / on-call engineers responsible for the Bridgelet Core deployment.

**Scope:** rotating the off-chain Ed25519 public key stored inside a deployed `SweepController` instance (`contracts/sweep_controller/src/lib.rs`), which `verify_sweep_auth()` checks every `execute_sweep()` signature against.

**When to use this runbook:** the off-chain Ed25519 private key used to sign sweep authorizations has been (or is suspected of being) compromised; scheduled periodic rotation; an operator with access to the signing key is leaving the team.

---

## Important: there is no `update_authorized_signer`

`SweepController` writes `authorized_signer` exactly once, inside `initialize()`:

```rust
if storage::get_authorized_signer(&env).is_some() {
    return Err(Error::AuthorizationFailed);
}
...
storage::set_authorized_signer(&env, &authorized_signer);
```

There is no other function in `contracts/sweep_controller/src/lib.rs` that writes `DataKey`'s signer slot, and `SweepController` does not expose an `upgrade()` entry point the way `EphemeralAccount` does. That means today there is no in-place path to change the signer on an already-deployed `SweepController` instance. Rotation requires deploying a fresh `SweepController` instance and calling `initialize()` on it with the new key.

---

## Prerequisites

- [ ] A new Ed25519 keypair has already been generated inside an HSM or secure enclave, per the key-management guidance in [`docs/security.md`](../../docs/security.md) ("Use a hardware security module (HSM) or secure enclave if possible"). The private key should never leave that boundary; only the 32-byte public key is needed for the steps below.
- [ ] You control the **deployer** key used for the original `SweepController` deployment (the identity that can install and deploy new WASM).
- [ ] You control the **creator** address that was passed to the original `initialize()` call, since `creator.require_auth()` gates initialization.
- [ ] You have decided whether this deployment uses locked mode (`authorized_destination` set) or flexible mode, and have the correct value on hand to pass to the new instance's `initialize()`.
- [ ] You have a disposable/testnet `EphemeralAccount` available to exercise the verification step below without touching production funds.

---

## What redeployment implies today

Because `EphemeralAccount` stores its own `authorized_controller` address at initialization time (`storage::set_authorized_controller`, read back via `get_authorized_controller()` in `sweep()`/`sweep_claim()`), rotating the signer by deploying a new `SweepController` only affects **new** `EphemeralAccount` instances created after the rotation and pointed at the new controller address.

Any `EphemeralAccount` that was already initialized against the **old** `SweepController` instance keeps trusting that old instance's `authorized_signer` for the lifetime of that account -- a new `SweepController` deployment does not retroactively repoint it. If the rotation is being done because the old signing key is compromised, redeploying `SweepController` alone does not protect funds already sitting in accounts wired to the old instance; those need a separate mitigation (e.g. the `authorized_destination` lock covered in [`emergency-destination-lock.md`](./emergency-destination-lock.md), or expediting `execute_sweep`/`claim` on them before an attacker can act).

---

## Steps

### 1. Build the WASM

No source changes are needed for a rotation -- you're redeploying the same artifact with new init arguments:

```bash
./scripts/build.sh
```

Confirm `target/wasm32v1-none/release/sweep_controller.wasm` exists.

### 2. Deploy the new instance

```bash
NEW_SWEEP_CONTROLLER_ID=$(stellar contract deploy \
    --wasm target/wasm32v1-none/release/sweep_controller.wasm \
    --source "$DEPLOYER_SECRET_KEY" \
    --network "$NETWORK" \
    --rpc-url "$SOROBAN_RPC_URL" \
    --network-passphrase "$NETWORK_PASSPHRASE")
echo "New SweepController instance: $NEW_SWEEP_CONTROLLER_ID"
```

### 3. Initialize with the new signer

Authorized by the **creator** key. Pass the new 32-byte Ed25519 public key (not the private key) as `authorized_signer`, and the same `authorized_destination` policy the old instance used (or `None` for flexible mode):

```bash
stellar contract invoke \
    --id "$NEW_SWEEP_CONTROLLER_ID" \
    --source "$CREATOR_SECRET_KEY" \
    --network "$NETWORK" \
    --rpc-url "$SOROBAN_RPC_URL" \
    --network-passphrase "$NETWORK_PASSPHRASE" \
    -- initialize \
    --creator "$CREATOR_ADDRESS" \
    --authorized_signer "$NEW_SIGNER_PUBLIC_KEY_HEX" \
    --authorized_destination "$AUTHORIZED_DESTINATION_OR_NONE"
```

A repeat call against this same instance will fail with `Error::AuthorizationFailed` (code 3) -- that's expected and confirms the one-time-init guard is working, not a rotation failure.

### 4. Verify the new signer against a real, current nonce

Do not assume the new key works just because deployment succeeded. `SweepController` has no public getter for the raw `authorized_signer` bytes, so the only way to confirm the new key is wired up correctly is to exercise the actual signature path against the instance's real on-chain nonce.

**4a. Read the current nonce from the new instance:**

```bash
stellar contract invoke \
    --id "$NEW_SWEEP_CONTROLLER_ID" \
    --network "$NETWORK" --rpc-url "$SOROBAN_RPC_URL" \
    --network-passphrase "$NETWORK_PASSPHRASE" \
    -- get_nonce
# Expected immediately after initialize(): 0
```

Use this value, not a locally-tracked guess -- `verify_sweep_auth()` always checks against the contract's current on-chain nonce (see `get_nonce()`'s doc comment in `lib.rs`).

**4b. Off-chain, build the exact `construct_sweep_message()` payload** -- SHA-256 over `destination.to_xdr() || nonce (big-endian u64) || contract_id.to_xdr()`, using the value from 4a and `$NEW_SWEEP_CONTROLLER_ID` as `contract_id` -- and sign it with the **new** private key inside the HSM/enclave. See [`sha256-message-construction.md`](../glossary/sha256-message-construction.md) for the exact byte layout.

**4c. Simulate `execute_sweep`** against the disposable/testnet `EphemeralAccount` (do not submit for real unless you intend to actually sweep it) to confirm the signature verifies:

```bash
stellar contract invoke \
    --id "$NEW_SWEEP_CONTROLLER_ID" \
    --source "$ANY_FEE_SOURCE" \
    --network "$NETWORK" \
    --rpc-url "$SOROBAN_RPC_URL" \
    --network-passphrase "$NETWORK_PASSPHRASE" \
    --sim-only \
    -- execute_sweep \
    --ephemeral_account "$DISPOSABLE_EPHEMERAL_ACCOUNT_ID" \
    --destination "$TEST_DESTINATION" \
    --auth_signature "$SIGNATURE_FROM_4B"
```

If simulation fails with `Error::AuthorizedSignerNotSet` (code 10), the new instance wasn't initialized correctly -- go back to step 3. If it fails on signature verification, the message bytes in 4b likely don't match `construct_sweep_message()`'s layout exactly, or the nonce used no longer matches the instance's current nonce (re-read `get_nonce()` and rebuild). Do not treat the new signer as trustworthy until this simulation succeeds.

### 5. Coordinate the cutover

- Point new `EphemeralAccount` deployments at `$NEW_SWEEP_CONTROLLER_ID` as `authorized_controller` going forward.
- Update `deployments/<network>.json` and any CI/config references (e.g. in `scripts/deploy-testnet.sh`).
- Notify SDK integrators and off-chain signing services of the new contract ID and new public key.
- Mark the old `SweepController` instance ID as deprecated in operational docs; do not assume it stops functioning -- it will keep verifying signatures from the old key until every `EphemeralAccount` still pointed at it has been swept or expired.

### 6. Retire the old signer

Once cutover is confirmed:

- Revoke HSM/KMS access for the old private key.
- If the rotation was triggered by a compromise, follow [`security-disclosure-triage.md`](./security-disclosure-triage.md).

---

## Cross-references

- [`docs/security.md`](../../docs/security.md) -- key-management guidance and the broader authorization model.
- [`emergency-destination-lock.md`](./emergency-destination-lock.md) -- the analogous procedure for locking down `authorized_destination` on a compromise.
- [`reserve-contract-admin-key-rotation.md`](./reserve-contract-admin-key-rotation.md) -- the same redeploy-and-reinitialize pattern applied to `ReserveContract`'s admin address.
- [`../glossary/sweep-nonce.md`](../glossary/sweep-nonce.md) and [`../glossary/sha256-message-construction.md`](../glossary/sha256-message-construction.md) -- detail on the nonce and message-hash mechanics referenced in step 4.
- [`contracts/sweep_controller/src/lib.rs`](../../contracts/sweep_controller/src/lib.rs) -- the contract source. Re-read before any code-level changes.
