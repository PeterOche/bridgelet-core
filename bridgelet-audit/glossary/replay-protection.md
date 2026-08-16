# Glossary Entry: Replay Protection

**Path:** `bridgelet-audit/glossary/replay-protection.md`
**Component:** `SweepController`
**Related Functions:** `construct_sweep_message()`, `verify_sweep_auth()`, `execute_sweep()`, `claim()`

---

## Overview

**Replay protection** is any mechanism that prevents a valid, previously-authorized message or signature from being reused to repeat an action the signer did not intend to authorize twice.

---

## The General Problem

An off-chain signature (or any signed authorization) proves that a specific signer approved a specific action at the time of signing. On its own, it does not prove the action has not already been executed. If a system verifies a signature purely against a fixed message (for example, "send funds to address X"), an attacker who observes that signature on-chain can resubmit the exact same signed payload and have it accepted again, since nothing about the message changes between the first and second submission. This is the general replay problem: a valid signature is reusable unless the system ties it to some piece of state that changes after first use.

---

## The `execute_sweep` Mechanism: Nonce-in-Message-Hash

`SweepController` closes this gap for the `execute_sweep()` path by folding a per-contract nonce into the signed message itself, rather than checking the nonce as a separate step.

1. `construct_sweep_message()` builds the digest signers must sign by hashing the destination address, the current sweep nonce, and the controller's own contract ID together (see `sha256-message-construction.md` for the exact byte layout).
2. `verify_sweep_auth()` rebuilds this same digest using the nonce currently stored on-chain and checks the Ed25519 signature against it.
3. On a successful `execute_sweep()` call, the contract increments the stored nonce before completing the transfer.

Because the nonce is baked into the hashed message rather than checked separately, a signature that was valid for nonce `N` no longer matches the message the contract reconstructs once the nonce has advanced to `N + 1`. Resubmitting the same signed payload fails verification rather than succeeding a second time. See `sweep-nonce.md` for the full state-transition detail of the nonce itself.

---

## Distinct from `claim()`

This nonce-based mechanism is specific to the `execute_sweep()` path. The `claim()` function protects against replay differently: it relies on Soroban's native `require_auth()` on the recipient address rather than an Ed25519 signature over a custom hashed message, and it does not read or increment the `SweepController` sweep nonce at all. The two functions should not be assumed to share a replay-protection mechanism; see `sweep-vs-sweep-claim.md` for a full side-by-side comparison.
