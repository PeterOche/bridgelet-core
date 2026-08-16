# Glossary Entry: Sweep Nonce

**Path:** `bridgelet-audit/glossary/sweep-nonce.md`  
**Component:** `SweepController`  
**Related Functions:** `construct_sweep_message()`, `verify_sweep_auth()`, `execute_sweep()`, `claim()`, `get_nonce()`, `update_authorized_destination()`

---

## Overview

The **Sweep Nonce** is a 64-bit unsigned integer (`u64`) stored in the `SweepController` contract state. It serves as the core replay-protection counter for off-chain Ed25519 authorization signatures. 

By folding the current on-chain sweep nonce into the cryptographic message digest before hashing and signing, Bridgelet Core ensures that every signed sweep payload is bound to a single transaction execution and cannot be replayed.

---

## How `construct_sweep_message()` Folds the Sweep Nonce

When `SweepController::execute_sweep()` is called, the contract invokes `verify_sweep_auth()`, which internally calls `construct_sweep_message()`. 

The function constructs the 32-byte SHA-256 digest using the following procedure:

1. **Read On-Chain Nonce**: Calls `storage::get_sweep_nonce(env)` to retrieve the current stored `u64` value.
2. **Serialize Destination Address**: Serializes `destination.to_xdr(env)`.
3. **Encode Nonce in Big-Endian Format**: Converts the 64-bit unsigned integer into an 8-byte big-endian byte sequence:
   ```rust
   message.push_back(((nonce >> 56) & 0xFF) as u8);
   message.push_back(((nonce >> 48) & 0xFF) as u8);
   message.push_back(((nonce >> 40) & 0xFF) as u8);
   message.push_back(((nonce >> 32) & 0xFF) as u8);
   message.push_back(((nonce >> 24) & 0xFF) as u8);
   message.push_back(((nonce >> 16) & 0xFF) as u8);
   message.push_back(((nonce >> 8) & 0xFF) as u8);
   message.push_back((nonce & 0xFF) as u8);
   ```
4. **Serialize Controller Address**: Appends `contract_id.to_xdr(env)` (the `SweepController` contract address).
5. **Hash Digest**: Computes `SHA256(destination_xdr || nonce_be_u64 || controller_id_xdr)` and returns the resulting `BytesN<32>` payload for Ed25519 verification.

---

## Code Path Nonce Mutations (`execute_sweep` vs `claim`)

Not all sweep pathways interact with the sweep nonce in the same manner:

### 1. `execute_sweep()` (Increments Nonce)
- **Flow**: `execute_sweep()` verifies the signature against `construct_sweep_message()`. Upon successful verification, it explicitly calls `authorization::increment_nonce(env)`.
- **State Mutation**: Increments `nonce = nonce + 1` in instance storage **before** initiating token transfers.
- **Replay Protection**: The incremented nonce immediately invalidates the signature used in this call, preventing any attacker from replaying the transaction payload.

### 2. `claim()` (Does NOT Increment Nonce)
- **Flow**: `claim()` uses Soroban native authorization (`recipient.require_auth()`) instead of an Ed25519 signature payload.
- **State Mutation**: `claim()` invokes `EphemeralAccount::sweep_claim()` directly and **does not** call `authorization::increment_nonce()`.
- **Side Effect**: Because `claim()` leaves `nonce` at its existing value (e.g. `0` if all sweeps were executed via `claim()`), `update_authorized_destination()` (which checks `nonce > 0` to lock updates) remains unlocked.

---

## Operational Rule: Read `get_nonce()` Live From Chain

> **CRITICAL FOR OFF-CHAIN SIGNERS**:  
> Off-chain applications, SDKs, and HSM services **must read `SweepController::get_nonce()` live from the Stellar ledger** immediately before constructing and signing the sweep message.

### Why Local Off-Chain Nonce Tracking Fails:
- If off-chain services maintain an internal/cached nonce counter, any out-of-order transaction submission, failed transaction, or concurrent sweep will desynchronize the local counter from the true on-chain nonce.
- Constructing a message with a stale or mismatched nonce causes `construct_sweep_message()` on-chain to hash a different byte sequence than the off-chain signer, resulting in signature verification failure (`Error::SignatureVerificationFailed`).
- Always query `SweepControllerClient::get_nonce()` via RPC prior to signing.
