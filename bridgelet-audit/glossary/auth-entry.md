# Glossary: Soroban Auth Entry
**Path:** `bridgelet-audit/glossary/auth-entry.md`

## Definition
A `Soroban Auth Entry` is the fundamental mechanism the Soroban network uses to cryptographically verify that an invoker (User, Contract, or Ed25519 keypair) has authorized a specific contract function call with specific arguments.

## `authorize_as_current_contract`
In cross-contract calls, a contract often needs to act on its own behalf (e.g., transferring tokens it owns). The method `env.auth().authorize_as_current_contract(args)` allows the current executing contract to explicitly insert an authorization entry into the authorization tree for a downstream call.

## Implications for Bridgelet Core
- When the `SweepController` needs to pull tokens from a user's wallet, it relies on the user providing their own auth entry (either via native wallet signing or cross-contract `require_auth()`).
- When the `AccountFactory` deploys or sweeps an ephemeral account, the factory itself might need to `authorize_as_current_contract()` to prove to the token contract that it is the legitimate owner of the funds being transferred out of the factory's pool.
