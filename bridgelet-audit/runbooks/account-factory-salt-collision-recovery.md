# Runbook: Diagnosing a failed `batch_initialize` call

**Contract:** `AccountFactory` (`contracts/account_factory/src/lib.rs`)
**Symptom:** `batch_initialize` returned a results vector in which one or more
`AccountInitResult` entries have `success: false`.

> This is an **operational diagnostic guide only**. It explains how to work out
> *why* a batch entry failed. It does not propose or apply any contract fix.

---

## 1. Background: what `batch_initialize` returns

`batch_initialize(creator, requests)` iterates over `requests` and, for each
entry (by `index`):

1. Derives a 32-byte **salt** from the entry's position in the batch:
   `salt = [0u8; 32]` with the big-endian `u32` of `index` written into the
   **last 4 bytes** (`salt[28..32]`). So index `0` → `00…00`, index `1` →
   `00…0001`, index `2` → `00…0002`, etc.
2. Deploys a new ephemeral-account contract at the deterministic address
   `deployer().with_current_contract(salt).deploy_v2(wasm_hash, ())`.
3. Calls `try_initialize(...)` on the freshly deployed account and records:
   - `AccountInitResult { account_address, success: true,  error: None }` on `Ok`
   - `AccountInitResult { account_address, success: false, error: None }` on `Err`

### Key limitation for diagnosis

**`AccountInitResult.error` is currently always `None`**, on both the success
and failure paths (the failure arm carries the comment *"In a real
implementation, we'd serialize errors"*). This means the return value tells you
**that** an entry failed and **at which `account_address`**, but never **why**.
Root-causing therefore requires **external inspection**: the transaction's
diagnostic events, an RPC `simulateTransaction` re-run, or your indexer/log
pipeline — not the on-chain return value.

---

## 2. First response: identify the failed entries

1. Capture the full `Vec<AccountInitResult>` returned by the call.
2. Record, for every entry with `success: false`, its **batch index** (position
   in the vector) and its `account_address`.
3. Note whether failures are:
   - **all entries** — points at a batch-wide cause (e.g. factory not
     initialized, `creator` auth, or every request sharing a bad field), or
   - **a subset** — points at per-request causes (e.g. a specific
     `expiry_ledger`, or a salt/index that collides with a prior batch).

---

## 3. Recover the "why" from external sources

Because `error` is `None`, pull the real error from one of:

- **Diagnostic events / transaction meta.** Re-fetch the transaction result and
  read its diagnostic events; the inner `initialize` trap surfaces the
  `ephemeral_account` `Error` code there (e.g. `InvalidExpiry`,
  `AlreadyInitialized`). Map the numeric contract error code back to the variant
  in `contracts/ephemeral_account/src/errors.rs`.
- **`simulateTransaction`.** Re-submit the same `batch_initialize` args against
  the current ledger via simulation; the simulated result includes the error and
  events without spending fees.
- **Your logs / indexer.** If you emit structured logs around the call, correlate
  by the `account_address` values recorded in step 2.

### Common per-entry causes to check

| Observed | Likely `ephemeral_account` error | Check |
| --- | --- | --- |
| One request's `expiry_ledger` ≤ current ledger | `InvalidExpiry` | Compare each request's `expiry_ledger` to the ledger at execution time. |
| A reused batch position across two calls | `AlreadyInitialized` / deploy conflict | See salt-collision analysis below. |
| Every entry fails identically | batch-wide (factory `wasm_hash` unset, `creator` auth, RPC state) | Confirm `initialize(wasm_hash)` was called and `creator` signed. |

---

## 4. Checking whether a salt / address collision is the cause

The salt is derived **only from the in-batch index**, not from the creator or a
nonce. Within a single `batch_initialize` call the indices `0..n` are unique, so
no intra-batch collision occurs. **Across separate calls to the same factory,
however, the salts repeat** (every batch starts again at index `0`), so the
second batch's index `k` targets the **same deterministic address** as the first
batch's index `k`. That previously-deployed address is the collision to look for.

### 4.1 Compute the expected salt for an index

```
index = k                      # the batch position you are investigating
salt  = 32 bytes, all zero
salt[28..32] = big-endian u32(k)
```

Examples: `k = 0` → `0x00…00`; `k = 5` →
`0x0000000000000000000000000000000000000000000000000000000000000005`.

### 4.2 Compute the expected deterministic contract address

The address is what `deployer().with_current_contract(salt).deploy_v2(...)`
produces: a deterministic function of **(factory contract address, salt)**. To
reproduce it off-chain without guessing the hashing details, use the SDK/CLI
rather than hand-deriving it:

- **Stellar CLI:** `stellar contract id asset`/deployer tooling, or a tiny
  Soroban test that calls
  `env.deployer().with_address(factory_address, salt).deployed_address()` and
  prints the result. Feed it the **factory's** contract address and the salt
  from 4.1.
- **From the batch result:** the `account_address` in the failing
  `AccountInitResult` **is** this deterministic address — you can cross-check the
  computed value against it.

### 4.3 Decide whether it is a collision

For the computed address:

1. **Query the ledger** for a contract instance at that address (e.g.
   `stellar contract info` / an RPC `getLedgerEntries` on the contract's instance
   key).
2. If a contract **already exists** there from an earlier batch:
   - A re-deploy to that address cannot succeed, and any attempt to
     `initialize` an already-initialized account yields `AlreadyInitialized`.
     This is a **salt/index collision** — the same batch position was used by a
     prior call against the same factory.
3. If **no contract exists** at that address, a salt collision is **not** the
   cause; return to step 3 and treat it as a per-request error (most commonly
   `InvalidExpiry`).

---

## 5. Diagnostic checklist

- [ ] Captured the full results vector; listed failing indices + addresses.
- [ ] Determined failure spread (all vs. subset).
- [ ] Retrieved the real error via diagnostic events or `simulateTransaction`.
- [ ] Mapped the numeric error to an `ephemeral_account` `Error` variant.
- [ ] For suspected collisions: computed salt (4.1) and address (4.2), then
      queried the ledger (4.3) to confirm whether that address is already
      occupied by a prior batch.
- [ ] Recorded the concluded root cause (per-request error vs. cross-batch salt
      collision) for the incident write-up.

---

*Scope note: this runbook is a standalone operational guide. It intentionally
does not modify `contracts/`, `docs/`, `scripts/`, or `tools/`, and does not
propose the underlying code changes (such as populating `AccountInitResult.error`
or salting per creator) that would reduce the need for external inspection.*
