# Operational Runbook: Diagnosing a Failed `execute_sweep` or `claim` Call

**Path:** `bridgelet-audit/runbooks/diagnose-failed-sweep.md`  
**Component:** `SweepController`, `EphemeralAccount`, SEP-41 Tokens  
**Target Invocations:** `SweepController::execute_sweep()`, `SweepController::claim()`

---

## Purpose & Scope

This runbook provides a structured triage checklist and diagnostic tree for operators, relayers, and developers investigating failed sweep transactions on Bridgelet Core.

Sweep operations can fail due to account state invalidity, signature verification mismatch, parameter locking violations, or SEP-41 token transfer issues. Follow the steps below in order to systematically isolate and resolve the root cause.

---

## Step-by-Step Triage Checklist

```
                      [ Sweep Transaction Failed ]
                                   │
                                   ▼
                       Step 1: Read can_sweep()
                                 /   \
                         [False]       [True]
                           /             \
            Account State Issue           Step 2: Check Call Type
        (Expired/AlreadySwept/         /                         \
           No Payment)           [execute_sweep]               [claim]
                                       │                          │
                             Step 3: Signature &          Step 4: Token Trustlines
                              Nonce Verification          & Balance Checks
```

---

### Step 1: Rule Out Account-State Issues via `can_sweep()`

Before inspecting cryptographic signatures or token contract state, verify the readiness of the `EphemeralAccount`.

#### Action
Call `SweepController::can_sweep(env, ephemeral_account_address)` or query state via `EphemeralAccount::get_info()`.

#### Evaluation
- **If `can_sweep()` returns `false`**, inspect `EphemeralAccount::get_info()` to determine the exact state failure:
  1. **`AccountStatus::Active` (0)**: No payment has been recorded yet (`payment_received == false`). Sweeping requires at least one recorded payment (`Error::AccountNotReady` / `Error::NoPaymentReceived`).
  2. **`AccountStatus::Swept` (2)**: The account has already been swept. Re-sweeping is blocked (`Error::AlreadySwept`).
  3. **`AccountStatus::Expired` (3)**: The account passed its `expiry_ledger` and was expired or recovered. Funds can no longer be swept (`Error::AccountExpired`).
  4. **Ledger Sequence Check**: Check if `current_ledger >= expiry_ledger`. If so, `is_expired()` returns `true`, blocking sweep calls.

---

### Step 2: Triage Cryptographic & Signature Failures (`execute_sweep` Branch)

If `can_sweep()` is `true` but `execute_sweep()` fails or reverts during execution, the failure is typically caused by signature verification or parameter mismatch.

#### Diagnostic Checklist for `execute_sweep`:

1. **Verify On-Chain Nonce Alignment**:
   - Query `SweepController::get_nonce()`.
   - **Check**: Did the off-chain signer construct the message using the exact current on-chain nonce? If the off-chain service used a cached or stale nonce, signature verification fails with `Error::SignatureVerificationFailed`.
   - **Resolution**: Re-read `get_nonce()` live from the contract and regenerate the signature payload.

2. **Verify SHA-256 Message Layout**:
   - The message payload signed off-chain **must** match:
     $$\text{message} = \text{SHA256}(\text{destination.to\_xdr()} \mathbin{\Vert} \text{nonce}_{\text{be\_u64}} \mathbin{\Vert} \text{controller\_id.to\_xdr()})$$
   - **Check**: Ensure no extra bytes (e.g. timestamps or legacy headers) were included. `destination` and `controller_id` must be valid Soroban XDR byte encodings.

3. **Verify `authorized_signer` Public Key**:
   - Inspect `storage::get_authorized_signer()`.
   - Confirm the Ed25519 private key used to sign matches the 32-byte public key registered during `SweepController::initialize()`.

4. **Verify Destination Lock (`authorized_destination`)**:
   - If `SweepController` was initialized in **Locked Mode** (`authorized_destination = Some(locked_addr)`), passing any `destination != locked_addr` returns `Error::UnauthorizedDestination`.

---

### Step 3: Triage Authorization & Token Transfer Issues (`claim` Branch)

The gas-free `claim()` function bypasses Ed25519 signatures and relies on Soroban native auth (`recipient.require_auth()`). If `can_sweep()` is `true` but `claim()` fails:

#### Diagnostic Checklist for `claim`:

1. **Verify Outer Transaction Authorization**:
   - Confirm the transaction submitted by the relayer includes a valid Soroban auth entry signed by `recipient`.
   - If `recipient` did not sign the `claim(recipient, ephemeral_account)` contract invocation, Soroban auth fails.

2. **Verify Destination Lock (`authorized_destination`)**:
   - In locked mode, `claim(recipient, ephemeral_account)` requires `recipient == authorized_destination`. If `recipient` differs, `claim()` returns `Error::UnauthorizedDestination`.

3. **Check Token Trustlines & Asset Balances**:
   - Iterate over each payment asset in `EphemeralAccount::get_info().payments`.
   - For each `payment.asset` (SEP-41 token contract):
     a. **Trustline Check**: Does the `recipient` address have an established trustline / active balance entry for the asset? On Stellar, non-native tokens require a trustline to receive funds.
     b. **Token Contract Status**: Is the SEP-41 token contract active and un-paused?
     c. **Balance Verification**: Does the `EphemeralAccount` hold a liquid token balance $\ge \text{payment.amount}$ inside the SEP-41 token contract? If recorded payment amounts exceed the actual token balance held by the account, `token.transfer()` will fail with `Error::TransferFailed`.

---

## Common Error Code Reference

| Error Variant | Code | Root Cause | Primary Fix |
| :--- | :--- | :--- | :--- |
| `AccountNotReady` | 5 | Payment count is zero or total amount is 0. | Ensure `record_payment()` was called prior to sweep. |
| `AccountExpired` | 6 | Current ledger $\ge$ `expiry_ledger`. | Process via `expire()` or `recover()` instead of sweep. |
| `AccountAlreadySwept` | 7 | Account status is already `Swept`. | Do not retry sweep; inspect prior `SweepExecutedMulti` events. |
| `SignatureVerificationFailed` | 9 | Nonce desync or incorrect message payload layout. | Re-read `get_nonce()` live and re-sign SHA-256 message payload. |
| `AuthorizedSignerNotSet` | 10 | `SweepController` was not initialized. | Invoke `SweepController::initialize()` with valid public key. |
| `UnauthorizedDestination` | 13 | Destination does not match locked address. | Pass the registered `authorized_destination` address. |
| `TransferFailed` | 2 | Underlying SEP-41 `token.transfer()` call failed. | Check recipient trustlines and account token balances. |
