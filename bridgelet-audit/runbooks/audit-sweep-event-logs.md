# Runbook: Auditing On-Chain Event Logs After a Sweep

**Audience:** auditors / compliance reviewers / on-call engineers producing or reviewing sweep-incident reports.

**Scope:** reconstructing the full on-chain history of a single sweep from emitted event logs alone (without relying on contract storage reads). Covers events from both `EphemeralAccount` (`contracts/ephemeral_account/src/events.rs`) and `SweepController` (`contracts/sweep_controller/src/lib.rs`).

**When to use this runbook:** investigating a sweep incident after the fact; producing evidence packets for an external audit; reconciling what happened against off-chain SDK/relayer logs when the two disagree.

**Tracking issue:** [#287](https://github.com/bridgelet-org/bridgelet-core/issues/287)

---

## Purpose

A single sweep touches **two** contracts — `EphemeralAccount` (state machine + reserve bookkeeping) and `SweepController` (signature verification + token transfers) — and emits events on **both** ledgers. To reconstruct what happened from event logs alone, an auditor needs:

1. A complete inventory of every event the two contracts emit during a sweep.
2. A reliable method to correlate the events emitted on the two contracts as belonging to the same logical sweep.
3. Known gaps in event coverage so the auditor knows what can and *cannot* be conclusively proven from the event log alone.

This runbook covers all three.

---

## Why event-only reconstruction is hard here

- Events from `EphemeralAccount` and `SweepController` are published to **two different contract addresses**, so the Soroban event indexer's per-contract filter will not natively correlate them. Correlation is the auditor's job.
- `EphemeralAccount` carries a `sweep_id: u64` field on its `ReserveReclaimed` event; `SweepController`'s `SweepCompleted` event does **not** carry a matching `sweep_id` field. Correlation therefore relies on `(ephemeral_account, destination, ledger_sequence, nonce)` rather than a single shared identifier.
- Several state transitions produce **no event at all** today. The gaps are catalogued in step 4 below — do not fill them with inferred data.

---

## Step 1 — Collect every event published by the two contracts

Query the ledger for events over the relevant ledger range, indexed by **both** contract addresses. Event topics are `contract-id-scoped`, so a query restricted to the `SweepController` instance will not return `EphemeralAccount`-side events, and vice versa.

```bash
# Pseudocode — adapt to your SDK/RPC client of choice.
# CONTRACT_IDS = ephemeral_account_or_account_factory_address, sweep_controller_address
for CONTRACT in "${CONTRACT_IDS[@]}"; do
  soroban rpc events \
    --contract-id  "$CONTRACT" \
    --start-ledger <FIRST_LEDGER> \
    --end-ledger   <LATEST_LEDGER> \
    --limit        10000 \
    --output json
done
```

Save the raw response per contract before any client-side filtering — the topics listed below are what you'll filter against.

---

## Step 2 — Catalog the events you should expect from a single sweep

### `EphemeralAccount` events (`contracts/ephemeral_account/src/events.rs`)

| Event struct (Rust) | Topic symbol | When emitted | Fields |
|---|---|---|---|
| `AccountCreated` | `created` | inside `initialize(...)` | `creator: Address`, `expiry_ledger: u32` |
| `PaymentReceived` | `payment` | inside `record_payment(...)`, **only when this call is the first payment on the account** (`payment_count == 0`) | `amount: i128`, `asset: Address` |
| `MultiPaymentReceived` | `multi_pay` | inside `record_payment(...)`, **only when this call is not the first payment** (`payment_count > 0`) | `asset: Address`, `amount: i128` |
| `SweepExecutedMulti` | `swept_mul` | inside `sweep(...)` / `sweep_claim(...)` *before* reserve reclaim | `destination: Address`, `payments: Vec<Payment>` |
| `ReserveReclaimed` | `reserve` | inside `reclaim_reserve_to(...)` after `sweep*` flips status to `Swept` | `destination: Address`, `amount: i128`, `sweep_id: u64`, `fully_reclaimed: bool`, `remaining_reserve: i128` |
| `AccountExpired` | `expired` | inside `expire(...)` / `recover(...)` once `expiry_ledger` is past | `recovery_address: Address`, `amount_returned: i128`, `reserve_amount: i128` |

> **Note on `PaymentReceived` vs `MultiPaymentReceived`:** `record_payment(...)` emits **exactly one** of these two events per call, not both:
> - The first-ever payment on an account (when `storage::get_total_payments(...) == 0`) creates a single `PaymentReceived` event.
> - Each subsequent payment on the same account (`payment_count > 0`) produces a single `MultiPaymentReceived` event.
>
> Concretely: an account that recorded 3 distinct assets over its lifetime will show **one** `PaymentReceived` (for the first asset recorded) and **two** `MultiPaymentReceived` events (for the second and third assets). The number of `MultiPaymentReceived` events is therefore `payment_count - 1`. The authoritative per-payment list is the `Payment` records returned by `account.get_info()` (`payment_count == AccountInfo.payment_count`) — do not infer `payment_count` from raw event counts without accounting for this 1-vs-(N-1) asymmetry.

### `SweepController` events (`contracts/sweep_controller/src/lib.rs`)

| Event struct (Rust) | Topic symbol | When emitted | Fields |
|---|---|---|---|
| `SweepCompleted` | `sweep` | at the end of `execute_sweep` (after `transfers::execute_transfers` succeeds) **and** at the end of `claim` (after `account_client.sweep_claim` returns) | `ephemeral_account: Address`, `destination: Address`, `amount: i128` |
| `DestinationAuthorized` | `dest_auth` | inside `initialize(...)` when `authorized_destination = Some(addr)` | `destination: Address` |
| `DestinationUpdated` | `dest_upd` | inside `update_authorized_destination(...)` after the nonce check passes | `old_destination: Option<Address>`, `new_destination: Address` |

> **Important:** `SweepController` emits **no event if `execute_sweep` aborts during token transfer**. The whole transaction fails atomically and no `SweepCompleted` is published — even though the corresponding `EphemeralAccount::sweep` *did* flip its status to `Swept` (reentrancy guard) and *will* still publish `SweepExecutedMulti` and `ReserveReclaimed` ahead of the failed transfer. See step 4 for how to recognize this.

---

## Step 3 — Correlate the two contract streams into a single sweep

`SweepController` does **not** emit its own `sweep_id`. To tie `EphemeralAccount`'s `ReserveReclaimed` to `SweepController`'s `SweepCompleted`, use a composite key built from fields that both events carry:

| Field | Where it appears | Tie-breaker if both events fail? |
|---|---|---|
| `ephemeral_account` (or `destination`) | `SweepCompleted.ephemeral_account`; `ReserveReclaimed.destination` (when `reserve reclaim destination == sweep destination`) | Not unique on its own |
| `destination` | `SweepCompleted.destination`; `ReserveReclaimed.destination` | Not unique on its own |
| `ledger_sequence` of the event publication | Both | Use a tight window (default: same ledger or `±1` ledger) |
| `EphemeralAccount`'s `ReserveReclaimed.sweep_id` | `ReserveReclaimed.sweep_id` only | Each `ReserveReclaimed.sweep_id` is set to `env.ledger().sequence() as u64` at the moment **that specific event** was emitted (`ephemeral_account/src/lib.rs::sweep` / `sweep_claim` / `finalize_expiry`), so per-event values differ across multiple reclaims on the same account (and tend to be monotonically increasing since ledger sequence always grows). The on-account storage `last_sweep_id` overwrites each time and only retains the value from the most recent reclaim. The per-event on-ledger `sweep_id` field is therefore a per-event ledger-sequence stamp, not a per-account counter — do not promote it to a global counter, and do not use the on-account `last_sweep_id` storage value alone as proof of multiple historical reclaims (it always holds just the latest). |
| `SweepController`'s sweep nonce | Set internally via `authorization::increment_nonce(...)` after `execute_sweep` verifies the Ed25519 signature; **not emitted in any event**. Read it via `get_nonce()` *before* and *after* the sweep, or rely on the diff. | Authoritative, but requires a state-read call, not pure event data. |

### Recommended correlation rule

For a sweep that may have succeeded:

1. Find every `SweepCompleted` with `ephemeral_account == A` and `destination == D`. (There will normally be exactly one for a given `A`.)
2. From the same ledger range, find the `ReserveReclaimed` event on `A`'s contract with **the same `destination == D`** and within `±1` ledger of the `SweepCompleted`.
3. If both events exist with matching `(A, D)` and adjacent ledgers, that is one logical sweep. Use `A`'s `ReserveReclaimed.sweep_id` only as a per-account reclaim counter; do not promote it to a global sweep counter.
4. If a `SweepExecutedMulti` precedes the `ReserveReclaimed` on `A` and precedes the `SweepCompleted` on `SweepController` by the same ledger (or `±1`), that further confirms the sweep.
5. To prove a global monotonic counter across the controller (e.g. for replay-protection evidence), take the `SweepController`'s `get_nonce()` value before and after the sweep and compute the diff. This requires a state read, not a pure event reconstruction.

### What "success" looks like in event form

For a single successful `execute_sweep` you should see (in ledger order):

```
1. AccountCreated               (A's contract, previously)         → proves A existed
2. PaymentReceived              (A's contract, very first payment) → first payment recorded
3. MultiPaymentReceived × (M-1) (A's contract, subsequent payments) → M-1 additional assets
4. SweepExecutedMulti           (A's contract, at sweep)           → A flipped to Swept
5. ReserveReclaimed             (A's contract, immediately after 4)→ reserve bookkeeping
6. SweepCompleted               (SweepController contract, at sweep) → transfers succeeded
```

where `M` = `AccountInfo.payment_count` (the total number of distinct assets recorded). Note the **1 / (M-1)**: there will be exactly **one** `PaymentReceived` and **M-1** `MultiPaymentReceived` events for an account with `M` recorded payments, never `M` of each. `recover()`/`expire()` will instead terminate with `AccountExpired` instead of `SweepExecutedMulti` + `ReserveReclaimed`.

If you see #4 and #5 but not #6, the sweep **flipped A's status and reclaimed reserve but the token transfer failed** — see the next step.

---

## Step 4 — Document known gaps in event coverage

These transitions either emit no event today or emit only partial information. Spot them on the timeline and note them explicitly in the audit report rather than guessing:

| Gap | What is missing from events today | Implication for the auditor |
|---|---|---|
| **Per-transfer success/failure on a multi-asset sweep** | `transfers::execute_transfers(...)` iterates `TokenClient::transfer` in a single pass with no per-call instrumentation. If the 3rd transfer fails, no event indicates which asset or how much. | A failed sweep aborts atomically; the auditor sees only "sweep attempted, did not complete". Combine with off-chain SDK logs or replay the sweep in simulation mode against a discardable test account. |
| **`SweepController::claim()` does not iterate `execute_transfers`** | Per `contracts/sweep_controller/src/lib.rs::claim`, the `claim` path calls `validate_destination`, reads `AccountInfo`, calls `authorize_claim` (which delegates to `EphemeralAccount::sweep_claim` for status-flip and event emission), then emits `SweepCompleted` on the controller. It does **not** call `transfers::execute_transfers(...)`. So `SweepCompleted` from a `claim` call today is not, by itself, evidence that `TokenClient::transfer` was actually invoked by the controller for that sweep. Use this gap section rather than asserting "transfers succeeded" purely from the `SweepCompleted` event in a mempool/forensic context. |
| **`execute_sweep` increment happens before transfer call** | `authorization::increment_nonce(...)` (`contracts/sweep_controller/src/authorization.rs`) runs immediately after `verify_sweep_auth(...)` in `SweepController::sweep_account(..., increment_nonce=true)`, **before** `account_client.sweep(...)` and `transfers::execute_transfers(...)`. If the transfer then fails, the nonce has already incremented but no `SweepCompleted` was emitted. | A `SweepCompleted` followed by the same ephemeral account being swept again will fail signature verification because the off-chain signing service used the pre-failure nonce. The next signed message must be rebuilt with the post-failure nonce. |
| **`EphemeralAccount::sweep` status flip happens before `TokenClient::transfer` is invoked from the controller** | `EphemeralAccount::sweep` calls `storage::set_status(&env, AccountStatus::Swept)` first, then emits `SweepExecutedMulti`, then `reclaim_reserve_to`; *after* that, on the controller side, `transfers::execute_transfers(...)` is invoked. If the token transfer fails, the account's on-chain status is already `Swept` and `ReserveReclaimed` has been emitted, even though no token moved. | A failed `TransferFailed` response from `SweepController` does not mean the on-account `Swept` status will roll back — it won't. Cross-reference with `EphemeralAccount::get_info().status` to confirm a successful sweep. |
| **`ResolvedDestinationLock` / `claim()` skipped destination check on flexible mode** | In flexible mode (`authorized_destination = None`), `claim()` does not call `validate_destination` (there is nothing to validate against), so frontrunning on `claim(malicious_recipient, A)` against an in-mempool `claim(legitimate_recipient, A)` produces identical-shape events. | Cannot reconstruct frontrun attempts from events alone. Cross-reference with Horizon mempool observations. |
| **Per-payment `timestamp`** | `Payment.timestamp: u64` is stored in `AccountInfo.payments` but is **not** echoed in the `SweepExecutedMulti.payments` reconstruction on some code paths and never appears in any event field. | If precise per-payment timing matters, read `AccountInfo` directly rather than reconstructing from events. |
| **Authorization source (signature vs claim path)** | No event distinguishes a sweep that went through `execute_sweep(...)` (Ed25519 verified) from one that went through `claim(...)` (Soroban auth entry from `recipient.require_auth`). | Both emit identical-shape `SweepCompleted` events. To disambiguate, take `SweepController::get_nonce()` before and after; only `execute_sweep` increments it. |
| **Errors surfaced to the caller** | `AccountFactory::batch_initialize` hardcodes `error: None` on per-account init failure (see `account_factory/src/lib.rs`). | The off-chain SDK only sees `success: false` for those accounts without the underlying error. This affects audit-quality reporting for batch deployments, not sweeps directly. |

---

## Step 5 — Write the audit packet

When producing an external audit response or incident postmortem, structure the packet so a third party can independently reproduce the reconstruction:

1. **Ledger range** queried per contract, with the exact `start-ledger` / `end-ledger` values.
2. **Raw event responses** per contract, unfiltered (so the auditor can re-filter against step 2's catalogue).
3. **Correlation table** mapping `ReserveReclaimed(sweep_id=N, destination=D, ephemeral_account=A)` to `SweepCompleted(ephemeral_account=A, destination=D)`, including the ledger sequence of each.
4. **Pre/post `get_nonce()` snapshot** for the relevant `SweepController` instance.
5. **Explicit "not reconstructible" notes** for every gap in step 4 that applied (e.g. "transaction-mode source of this sweep could not be determined from events alone because nonce delta was 0").
6. **Cross-references** to off-chain sources that fill the gap (relayer logs, Horizon mempool snapshots, SDK audit logs).

---

## Limitations

- **Pure event reconstruction is lossy by design today.** Some gaps in step 4 will always require a supporting `get_nonce()` or storage read. Treat this runbook as a way to *bound* that loss, not eliminate it.
- **Multi-network ambiguity.** Events are scoped to a single `(contract_id, ledger_sequence)`. On a network with re-org risk or fork depth > 1, multiple events for the same logical sweep can exist on different chains; collapse them on `(ephemeral_account, destination)` exactly as described in step 3, then disambiguate by network passphrase.
- **No PII redaction layer.** Event topics are public on-chain. If your report includes raw events, sanitize `destination` / `creator` for any field a compliance team has flagged.

---

## Cross-references

- [`contracts/ephemeral_account/src/events.rs`](../../contracts/ephemeral_account/src/events.rs) — event struct definitions and emission call sites for `EphemeralAccount`.
- [`contracts/sweep_controller/src/lib.rs`](../../contracts/sweep_controller/src/lib.rs) — event struct definitions and emission call sites for `SweepController`.
- [`contracts/sweep_controller/src/authorization.rs`](../../contracts/sweep_controller/src/authorization.rs) — where `increment_nonce` is called (only on the `execute_sweep` path).
- [`emergency-destination-lock.md`](./emergency-destination-lock.md) — context for the `DestinationUpdated` / `DestinationAuthorized` events.
- [`post-incident-ttl-extension-sweep.md`](./post-incident-ttl-extension-sweep.md) — if event history cannot be queried because a contract's TTL expired, run that runbook first.
- [`../glossary/sweep-nonce.md`](../glossary/sweep-nonce.md) and [`../glossary/sweep-vs-sweep-claim.md`](../glossary/sweep-vs-sweep-claim.md) — backgrounder on what each path does to the nonce and corresponding events.
