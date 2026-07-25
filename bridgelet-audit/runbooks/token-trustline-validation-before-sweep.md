# Runbook: Validating Recipient Trustlines Before Sweeping Non-Native Assets

**Audience:** SDK integrators / relayer operators / on-call engineers submitting `SweepController::execute_sweep` or `SweepController::claim` transactions.

**Scope:** caller-side pre-flight validation that every record-payment asset on an `EphemeralAccount` can be received by the proposed sweep destination. Documents why this step is required (the contract does no pre-check itself today), how to do it, and how to interpret a sweep transaction failure caused by a missing or un-authorized trustline.

**When to use this runbook:** every `execute_sweep` and every `claim` whose `payments.len() > 0` includes any non-native (i.e. classic Stellar asset) token. Native XLM does not require a trustline and is exempt.

**Tracking issue:** [#291](https://github.com/bridgelet-org/bridgelet-core/issues/291)

---

## Purpose

`SweepController::transfers::execute_transfers(...)` iterates every `Payment` in `AccountInfo.payments` and calls `TokenClient::new(env, &payment.asset).transfer(from, destination, &payment.amount)` in a single pass. There is **no pre-flight trustline check** inside that function, and a Soroban `transfer` to an address that lacks a corresponding trustline (or whose trustline is not `authorized`) fails atomically — taking the whole sweep transaction down with it.

Net effect if you skip pre-validation:

- The `EphemeralAccount`'s status has already been flipped to `Swept` (reentrancy guard, set in `sweep(...)` *before* the transfer tail).
- Yet no `SweepCompleted` event was emitted from `SweepController`, because the transfer raised first.
- The `ReserveReclaimed` event has already been emitted on `EphemeralAccount`.
- The signed-and-broadcast nonce has *already been incremented* by `authorization::increment_nonce(...)` because that runs immediately after `verify_sweep_auth(...)`, before any state mutation downstream.

So a missing-trustline failure leaves the account in **`Swept` status with funds not actually transferred**, with the broadcast nonce now unusable for replay, and with no `SweepCompleted` event for the off-chain ledger. Recovery is messy. **Pre-validate.**

This runbook describes the caller-side mitigation.

---

## Prerequisites

- [ ] A live Soroban RPC endpoint (testnet or mainnet) reachable from the relayer.
- [ ] A Horizon endpoint (or equivalent) capable of serving account/trustline detail for the proposed `destination` address.
- [ ] The full `EphemeralAccount::get_info()` payload in hand — specifically the `payments: Vec<Payment>` field, which gives you the per-asset list to validate against (see [`bridgelet_shared::Payment`](../../contracts/shared/src/types.rs)).
- [ ] A clear policy decision for one ambiguous case: what to do when an asset appears in the ephemeral account's payments but the destination's trustline status is unknown (timeout, partial response, etc.). Default: **abort the sweep and surface the asset list to a human**. Do not proceed silently.
- [ ] Awareness that the asset order returned by `account.get_info().payments` is the order that `transfers::execute_transfers` will iterate them in. If you batch-validate in JSON rather than directly against the contract, preserve that order for any retry.

---

## Step 1 — Pull the destination's trustline state

For each distinct `payment.asset` except native XLM, query the destination's trustline row. The required information per asset:

| Field | Why it matters |
|---|---|
| Trustline exists | If it does not, `transfer` will fail with `TrustLineMissing` semantics. |
| Trustline balance + limit | Confirms there is headroom to receive the incoming amount (currently the contract does not check this either). |
| Authorization status (`AUTH_REQUIRED` flag on issuer) | If the issuer is `AUTH_REQUIRED` and the destination does not have an entry with `authorized = true`, `transfer` will fail with `AuthRequired`. |
| Authorization revocation (`AUTH_REVOCABLE`) | Revocable trustlines can be flipped out of authorized state at any time — even between your pre-check and submit. Treat as a soft warning. |

### Using Horizon

```bash
curl -s "https://horizon-testnet.stellar.org/accounts/${DESTINATION_ADDRESS}" \
  | jq '.balances[] | select(.asset_type != "native") | {asset_code, asset_issuer, balance, limit, authorized_flags: .flags}'
```

### Using Soroban RPC (if Horizon mirrors are slow / unavailable)

There is no general-purpose "list-trustlines" RPC on the Soroban host today. In practice rely on Horizon or your own indexer for this view; Soroban RPC alone is insufficient.

---

## Step 2 — Decide whether to proceed, per asset

Apply this matrix against each non-native payment:

| Trustline exists? | Authorized? | Headroom for amount? | Action |
|---|---|---|---|
| Yes | Yes | Yes | **Proceed.** |
| Yes | Yes | No | **Abort** with a human-readable "destination at trustline capacity" message. Sweep will fail at transfer time otherwise. |
| Yes | No (`AUTH_REQUIRED` not satisfied) | n/a | **Abort** with "destination not authorized for asset". |
| No | n/a | n/a | **Abort** with "destination missing trustline for asset". |
| Unknown (timeout / API error / inconsistent response) | n/a | n/a | **Abort**. Do not infer. |

Native XLM (`payment.asset == stellar_native_asset_address`) is **not** in the table — it requires no trustline, no headroom check from the contract side, and transfers cleanly. Skip it.

---

## Step 3 — Build the abort / proceed decision

Aggregate the per-asset outcomes:

- **All assets `Proceed`** → submit `execute_sweep` (or `claim`) as normal.
- **Any asset `Abort`** → do **not** submit the sweep. Surface the per-asset failures to the operator / SDK caller, and handle the time-sensitive parts of the situation (see step 5).

Note: `execute_sweep` is atomic — a per-transfer failure inside the call aborts the whole transaction. There is no "partial sweep" success state where the first asset moved and the second one didn't. Treat a proceed decision as irrevocable.

---

## Step 4 — Submit and observe

If you proceeded, submit as usual — but watch for the exact failure modes listed in step 5 below rather than treating any failure as generic.

```bash
stellar contract invoke \
  --id "$SWEEP_CONTROLLER_ID" \
  --source "$RELAYER_SECRET_KEY" \
  --network "$NETWORK" \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- execute_sweep \
  --ephemeral_account "$EPHEMERAL_ACCOUNT_ADDRESS" \
  --destination "$DESTINATION_ADDRESS" \
  --auth_signature "$HEX_SIGNATURE"
```

> **Path applicability — `execute_sweep` vs `claim`:** This runbook's validation logic applies directly to the `execute_sweep` path, which is where `transfers::execute_transfers(...)` is actually iterated (see `contracts/sweep_controller/src/lib.rs::sweep_account`, which is only called from `execute_sweep`, not from `claim`). The `claim` path in `lib.rs::claim` does **not** call `transfers::execute_transfers(...)`; it only authorizes `EphemeralAccount::sweep_claim` and emits `SweepCompleted` on the controller. As a result, trustline issues for non-XLM assets on the `claim` path do **not** produce a clean SEP-41 transfer failure mode — they will surface in a different way (the controller currently has no caller-side equivalent of the trustline gate for `claim`). Practically: **run this validation for `execute_sweep`; for `claim`, treat the destination's overall ability-to-receive-any-asset as an additional, separate check before submission** and prefer locked-mode `SweepController` deployments when recipients can be pre-validated. See [`audit-sweep-event-logs.md`](./audit-sweep-event-logs.md) step 4 for the formalized gap.

---

## Step 5 — Interpreting a failed sweep as a trustline-specific cause

When a sweep attempt fails, three signals coincide in the failure path:

1. The transaction result reports `TransferFailed` from `SweepController` (`errors.rs` discriminant `2`) — or, equivalently, a host-level `HostError` from the underlying SEP-41 `TokenClient::transfer` host function.
2. On-chain, **the `EphemeralAccount`'s status is now `Swept`** (set by the reentrancy guard inside `EphemeralAccount::sweep` before the transfer tail). Reads of `account.get_info().status` will reflect this even though no actual transfer completed.
3. **`SweepController`'s sweep nonce has already incremented** — because `authorization::increment_nonce(...)` runs right after `verify_sweep_auth(...)` succeeds, before any downstream call.

To conclude "this was a missing-trustline failure" rather than "this was a different transfer error":

- Re-run step 1 against the destination. If **every non-native asset in `AccountInfo.payments` is `No` for `Trustline exists?` or `No` for `Authorized?`**, the failure mode is consistent with a trustline problem.
- If some assets have valid trustlines and some don't, the iteration order in `execute_transfers` means failure happens on the **first non-trivial trustline/balance violation** in the list. You cannot assume "the rest would have worked" — the validation must be repeated in order.
- Cross-reference with Horizon transaction detail: a Horizon-side `op_no_trust`, `op_not_authorized`, or `op_line_full` on the destination for the matching asset is direct evidence of a trustline-specific failure.

If the validation in step 2 said "Proceed" but the sweep still failed on a trustline condition, the most likely cause is a **race condition** between your pre-check window and submit:

- The destination's `AUTHORIZED` flag was revoked between your Horizon read and the submit (issuer has `AUTH_REVOCABLE`).
- The trustline was closed or its limit reduced.
- The destination account itself was merged or deleted.

Re-run step 1 immediately after the failure, confirm the destination's current state, and decide whether to retry against the **new** sweep nonce (`SweepController::get_nonce()` will reflect the increment from the failed attempt).

---

## Step 6 — Recovery for confirmed trustline failures

If the destination's user can fix the trustline (create / re-authorize / raise the limit), the recovery path is:

1. Tell them precisely which asset + issuer failed (the on-chain evidence is in `AccountInfo.payments`; the failure mode from step 5 narrows it to one).
2. Wait for them to create the trustline. Do **not** submit another `execute_sweep` until they confirm; otherwise you risk a second nonce increment and the same `Swept`-without-transfer state again.
3. Re-run steps 1–3 *before* retrying. The new build of the signed message must use the post-failure nonce from `get_nonce()` — see [`../glossary/sha256-message-construction.md`](../glossary/sha256-message-construction.md) and [`../glossary/sweep-nonce.md`](../glossary/sweep-nonce.md) for the exact byte layout.
4. Submit. Expect a `SweepCompleted` event from `SweepController` this time.

If the destination cannot/will not fix the trustline, the funds remain stranded on the `EphemeralAccount` (status `Swept`, no `SweepCompleted`, on-chain `ReserveReclaimed` already emitted). The recovery path for that situation is outside this runbook's scope and should be escalated.

---

## Limitations and caveats

- **The contract still does no pre-flight check.** This is a caller-side mitigation, not an in-contract guarantee. Every caller that submits a sweep must implement this runbook (or its equivalent). If a future audit changes the contract to perform the check, this runbook should be re-validated against that change.
- **Validation is observational, not authoritative.** A `Proceed` here is a best-effort prediction; the trustline state can change between check and submit. Treat horizon results as advisory and structure your submit logic to consume a precise failure instead of guessing.
- **Headroom is a soft check.** The contract does not currently bound transfers against the destination's existing balance + limit. If you skip the headroom row of the matrix above, a `Proceed` that hits an existing full trustline will fail in flight the same way as an outright missing trustline.
- **Issuers with `AUTH_REVOCABLE` can flip authorized status unilaterally.** Capture a timestamp on the pre-check; if more than a few seconds elapse before submit, re-check.
- **`claim()` path is not covered by this runbook.** `SweepController::claim` does not iterate `transfers::execute_transfers(...)` today (see [`sweep_controller/src/lib.rs::claim`](../../contracts/sweep_controller/src/lib.rs) — only `execute_sweep` → `sweep_account(..., increment_nonce = true)` does), so there is no SEP-41 `TokenClient::transfer` to fail on a missing trustline along that path. Conversely, this means caller-side trustline validation does not protect a `claim` submission against trustline-related loss today — that protection simply does not exist on the controller side for `claim`. Restrict the runbook's applicability accordingly, and treat `claim` flows as a separate problem class.

---

## Cross-references

- [`contracts/sweep_controller/src/transfers.rs`](../../contracts/sweep_controller/src/transfers.rs) — the function this runbook is mitigating against (`execute_transfers`). No pre-flight check exists there today.
- [`contracts/sweep_controller/src/errors.rs`](../../contracts/sweep_controller/src/errors.rs) — `TransferFailed` (code 2) is the SOROBAN-level failure code you see in the step 5 outcome; trustline-specific codes come from the SEP-41 host layer.
- [`contracts/ephemeral_account/src/lib.rs`](../../contracts/ephemeral_account/src/lib.rs) — `sweep()` and `sweep_claim()` set `AccountStatus::Swept` *before* the transfer tail runs (reentrancy guard), which is why step 5's outcome 2 happens even on transfer failure.
- [`contracts/sweep_controller/src/authorization.rs`](../../contracts/sweep_controller/src/authorization.rs) — `increment_nonce(...)` placement (immediately after `verify_sweep_auth(...)` in `sweep_account` with `increment_nonce = true`) is why step 5's outcome 3 also happens even on transfer failure.
- [`audit-sweep-event-logs.md`](./audit-sweep-event-logs.md) — for full event reconstruction when an incident like this happens; especially step 4's "transfer failed but events were partially emitted" gap documentation.
- [`emergency-destination-lock.md`](./emergency-destination-lock.md) — alternative recovery if the destination is itself compromised and you need to redirect to a known-good address.
- [`../glossary/sweep-nonce.md`](../glossary/sweep-nonce.md) and [`../glossary/sha256-message-construction.md`](../glossary/sha256-message-construction.md) — required reading before retrying after a failed sweep.
