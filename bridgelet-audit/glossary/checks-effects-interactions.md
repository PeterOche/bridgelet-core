# Glossary Entry: Checks-Effects-Interactions (CEI)

**Path:** `bridgelet-audit/glossary/checks-effects-interactions.md`  
**Component:** `EphemeralAccount`  
**Related Functions:** `EphemeralAccount::sweep()`, `EphemeralAccount::sweep_claim()`

---

## Overview

The Checks-Effects-Interactions (CEI) pattern is a fundamental security best practice in smart contract development that prevents reentrancy attacks and other unexpected behaviors by enforcing a strict order of operations.

## General CEI Pattern Definition

The pattern requires all smart contract functions to execute operations in three sequential phases:

1. **Checks** (Validate): First perform all input validation, authorization checks, and precondition verifications. Revert immediately if any check fails.
2. **Effects** (Mutate state): After all checks pass, update the contract's own state before making any external calls.
3. **Interactions** (Make external calls): Only after state is fully updated, interact with external contracts or send Ether/tokens.

This ordering prevents reentrancy attacks because by the time an external call is made, all internal state changes have already been completed, preventing an attacker from re-entering the function with the old state still intact.

## Application in `EphemeralAccountContract::sweep()`

The `sweep()` function in `EphemeralAccountContract` correctly implements the CEI pattern. Here's how the ordering is applied:

### 1. Checks Phase (All preconditions first)
```rust
// Check initialized
if !storage::is_initialized(&env) {
    return Err(Error::NotInitialized);
}
// Check not already swept
if storage::get_status(&env) == AccountStatus::Swept {
    return Err(Error::AlreadySwept);
}
// Check payment received
if !storage::has_payment_received(&env) {
    return Err(Error::NoPaymentReceived);
}
// Check not expired
if Self::is_expired(env.clone()) {
    return Err(Error::AccountExpired);
}
// Verify authorization signature
Self::verify_sweep_authorization(&env, &destination, &auth_signature)?;
```

### 2. Effects Phase (State mutation before external calls)
```rust
// Update account state to mark as swept - this happens BEFORE any external calls
storage::set_status(env, AccountStatus::Swept);
storage::set_swept_to(env, &destination);
let sweep_id = env.ledger().sequence() as u64;
storage::set_last_sweep_id(env, sweep_id);
```

### 3. Interactions Phase (External interactions last)
```rust
// Emit events (internal to the contract but part of finalization)
events::emit_sweep_executed_multi(env, destination.clone(), &payments_vec);
// Execute reserve reclaim which may involve external calls to transfer funds
Self::reclaim_reserve_to(env, &destination, sweep_id)?;
```

The critical security measure here is that the account's status is set to `Swept` **before** any external calls are made to transfer funds or interact with other contracts. This prevents any reentrant calls from attempting to sweep the same account twice, as the `AlreadySwept` check would fail on any subsequent invocation.

### Same Pattern Applied to `sweep_claim()`
The `sweep_claim()` entrypoint follows the exact same CEI ordering, ensuring consistent security across both sweep pathways.

## Canonical Source
For the full reentrancy risk analysis and complete reasoning behind this implementation, see:
[docs/reentrancy-analysis.md](file:///c:/Users/hp/Desktop/wave7/bridgelet-core/docs/reentrancy-analysis.md)