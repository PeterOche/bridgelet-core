# Operational Runbook: Restoring an Archived (TTL-Expired) Contract Instance

**Path:** `bridgelet-audit/runbooks/restore-archived-instance-storage.md`  
**Component:** Soroban State Management (`EphemeralAccount`, `SweepController`, `ReserveContract`, `AccountFactory`)  
**Operation:** Soroban `RestoreFootprintOp`

---

## Purpose & Overview

In Soroban, all contract instances, WASM bytecode entries, and contract storage entries are subject to Time-To-Live (TTL) storage expiration. If a contract instance or its storage entries are not periodically extended via rent bumps (`BumpFootprintExpirationOp`), their TTL drops to zero, transitioning the entry into an **archived state**.

Once a contract instance is archived, regular transaction invocations (e.g. `sweep()`, `get_info()`, `can_sweep()`) will fail with storage footprint errors (`HostError: Error(Storage, ExceededRentLimit)` or missing key errors). 

This runbook details the conceptual framework and step-by-step operational procedure for executing a Soroban `RestoreFootprintOp` transaction to restore an archived contract instance to active status.

---

## High-Risk Components & Exposure Profile

While all four contracts in Bridgelet Core utilize Soroban instance storage, their exposure profiles differ:

| Contract | Exposure Level | Primary Cause |
| :--- | :--- | :--- |
| **`EphemeralAccount`** | **CRITICAL (Highest Exposure)** | Deployed dynamically per transfer/payment. Accounts waiting for long-expiry ledgers or delayed payments may sit idle past default TTL without automated rent extensions. |
| **`SweepController`** | Moderate | Shared controller contract instance. Frequently invoked during sweeps, naturally bumping TTL, but can expire if unused during low-volume periods. |
| **`ReserveContract`** | Moderate | Standalone config store. Rarely updated after initial setup, making it prone to TTL expiration if unmonitored. |
| **`AccountFactory`** | Low / Moderate | Shared deployment factory. Active during batch creation, but instance state can expire if no deployments occur within TTL window. |

---

## Conceptual Architecture: The `RestoreFootprintOp` Pattern

Soroban separates storage restoration into a specialized transaction structure:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Stellar RPC / Horizon                           │
│   1. Simulate invocation to obtain Read-Only footprint for archived key │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                   RestoreFootprintOp Transaction                       │
│   2. Package footprint keys into RestoreFootprintOp                    │
│   3. Submit transaction to restore archived entry back to active state  │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                     Post-Restoration Verification                      │
│   4. Execute read-only contract call (e.g. get_info()) to confirm      │
└────────────────────────────────────────────────────────────────────────┘
```

1. **Footprint Assembly**: Archived keys are specified in the transaction's `readOnly` footprint ledger bounds.
2. **Resource Payment**: The fee-payer submits `RestoreFootprintOp` and pays the required state restoration fee (rent restoration stroops).
3. **Re-Activation**: The Soroban host un-archives the storage entry and resets its TTL to the minimum network live threshold.

---

## Step-by-Step Restoration Procedure

### Step 1: Identify the Archived Instance & Fetch Footprint
When an invocation fails due to archived state, simulate the transaction using the Soroban RPC endpoint or Stellar CLI to extract the required footprint keys.

```bash
# Example: Simulate read invocation to capture footprint for an archived EphemeralAccount
stellar contract read \
    --id <EPHEMERAL_ACCOUNT_CONTRACT_ID> \
    --network testnet \
    -- \
    get_info
```

*RPC Response will indicate storage eviction / archived key presence in the footprint requirement.*

### Step 2: Execute `RestoreFootprintOp` via CLI / SDK

Submit a footprint restoration transaction covering the archived contract instance and WASM code keys.

#### Using Stellar CLI:
```bash
stellar contract restore \
    --id <EPHEMERAL_ACCOUNT_CONTRACT_ID> \
    --source <FEE_PAYER_SECRET> \
    --network testnet
```

#### Using JS/TS SDK:
```typescript
import { Operation, TransactionBuilder, Horizon } from '@stellar/stellar-sdk';

// Construct RestoreFootprint transaction using footprint from simulation
const restoreOp = Operation.restoreFootprint({});
const tx = new TransactionBuilder(sourceAccount, { fee: '100000', networkPassphrase })
  .addOperation(restoreOp)
  .setTimeout(30)
  .build();

tx.preflightUploadAndRestore(server);
```

---

## Step 3: Post-Restoration Verification

After the `RestoreFootprintOp` transaction confirms on-chain, verify that the contract instance has successfully returned to active status.

### Verification Matrix by Contract:

| Target Contract | Verification Command | Expected Successful Response |
| :--- | :--- | :--- |
| **`EphemeralAccount`** | Call `get_info()` or `get_status()` | Returns `AccountInfo` struct with valid status (`Active`/`PaymentReceived`). |
| **`SweepController`** | Call `get_nonce()` | Returns current `u64` sweep nonce without storage error. |
| **`ReserveContract`** | Call `has_base_reserve()` | Returns `true` or `false` boolean value. |
| **`AccountFactory`** | Simulate `batch_initialize()` query | Responds without footprint eviction error. |

#### Verification Execution Example:
```bash
# Confirm EphemeralAccount is readable again
stellar contract invoke \
    --id <EPHEMERAL_ACCOUNT_CONTRACT_ID> \
    --network testnet \
    --source <READ_ONLY_ACCOUNT> \
    -- \
    get_status
```

---

## Post-Restoration Best Practice: Extend TTL Immediately

Once restored, execute a `BumpFootprintExpirationOp` to extend the contract instance's TTL for a substantial ledger duration (e.g. 500,000 ledgers) to prevent immediate re-archiving.

```bash
stellar contract extend \
    --id <EPHEMERAL_ACCOUNT_CONTRACT_ID> \
    --ledgers-to-extend 500000 \
    --source <FEE_PAYER_SECRET> \
    --network testnet
```
