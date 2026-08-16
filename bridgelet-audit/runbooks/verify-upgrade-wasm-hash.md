# Operational Runbook: Verifying WASM Hash Before Invoking `upgrade()`

**Path:** `bridgelet-audit/runbooks/verify-upgrade-wasm-hash.md`  
**Component:** `EphemeralAccount`  
**Target Invocations:** `EphemeralAccountContract::upgrade(new_wasm_hash)`

---

## Purpose & Overview

The `EphemeralAccount` contract implements a WASM migration mechanism via `upgrade(env, new_wasm_hash)`. This allows the registered `admin` address to update the executable code of deployed ephemeral accounts to a new WASM hash previously uploaded to the Stellar network.

Because WASM upgrades execute directly on live contract storage, providing an incorrect, incompatible, or buggy WASM hash can permanently brick the contract. This runbook establishes mandatory pre-flight checks and verification protocols before executing an upgrade.

---

## Technical Mechanism & On-Chain Behavior

In `contracts/ephemeral_account/src/lib.rs`:

```rust
pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
    if !storage::is_initialized(&env) {
        return Err(Error::NotInitialized);
    }

    let admin = storage::get_admin(&env).ok_or(Error::NotUpgradeAdmin)?;
    admin.require_auth();

    env.deployer().update_current_contract_wasm(new_wasm_hash);
    Ok(())
}
```

### Critical Security Note on `upgrade()` Behavior:
> **NO ON-CHAIN WASM VALIDATION**:  
> The `upgrade()` function accepts a 32-byte `new_wasm_hash` and invokes `update_current_contract_wasm()`. The Soroban host and contract perform **zero validation** on-chain regarding the contents, interface compatibility, state layout, or functional correctness of the targeted WASM bytecode. If the specified hash exists on-chain, the contract's code reference is immediately updated.

---

## Pre-Flight Verification Checklist

Before executing `upgrade()`, admins must complete all four pre-flight checks:

### Check 1: Reproducible Build & Hash Verification
1. **Clean Workspace Build**: Compile the WASM bytecode inside a deterministic container environment (e.g. `stellar/contract-builder` or standard cargo workspace release target).
   ```bash
   cargo build --target wasm32v1-none --release -p ephemeral_account
   ```
2. **Compute WASM SHA-256 Digest**:
   ```bash
   sha256sum target/wasm32v1-none/release/ephemeral_account.wasm
   ```
3. **Compare Against Published Release Hash**: Cross-reference the computed hex hash against the published release artifact in the official repository / release notes.

---

### Check 2: Confirm On-Chain Upload Status
Verify that the target WASM bytecode has already been uploaded to the ledger (installed) and that its hash matches `new_wasm_hash`.

```bash
# Verify the WASM bytecode exists on ledger
stellar contract install \
    --wasm target/wasm32v1-none/release/ephemeral_account.wasm \
    --source <ADMIN_SECRET> \
    --network testnet
```
*If installed, this returns the 32-byte hex WASM hash without re-uploading.*

---

### Check 3: Storage Schema Compatibility Audit
Ensure the new WASM code maintains exact memory layout and DataKey compatibility with existing contract storage:
- **Enum Discriminants**: Confirm all `DataKey` and `AccountStatus` enum discriminants remain unchanged.
- **Type Layouts**: Ensure `Payment`, `AccountInfo`, and reserve tracking fields retain structural alignment.
- **Migration Logic**: If new storage keys were added, confirm the new WASM includes default-fallback handling for uninitialized keys on existing accounts.

---

### Check 4: Sandbox & Integration Test Verification
Before executing on mainnet or active testnet accounts, run integration tests against a local Soroban sandbox or test instance:
1. Deploy original `ephemeral_account.wasm`.
2. Initialize and deposit funds.
3. Call `upgrade(new_wasm_hash)`.
4. Confirm subsequent read and write calls (`get_info()`, `sweep()`, `reclaim_reserve()`) succeed without state corruption or panics.

---

## Rollback & Downgrade Considerations

> **WARNING: NO AUTOMATED DOWNGRADE OR ROLLBACK PATH**

1. **Permanent Code Swap**: Executing `upgrade()` immediately replaces the active WASM code reference for the contract instance.
2. **Bricking Risk**:
   - If the new WASM bytecode contains a panic, invalid instruction, or broken storage deserialization, subsequent calls to the contract will revert.
   - **If the new WASM breaks or omits the `upgrade()` function itself**, the contract can **never be upgraded again** and becomes permanently immutable in a broken state.
3. **Emergency Downgrade Recovery Procedure**:
   - If an upgraded WASM contains a non-fatal bug but retains a functioning `upgrade()` interface, the admin can perform a "downgrade" by calling `upgrade(previous_known_good_wasm_hash)`.
   - **Mandatory Requirement**: Always maintain the exact 32-byte hash of the previous WASM release on file to enable rapid emergency rollback if needed.
