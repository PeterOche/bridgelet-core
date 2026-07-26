# Audit: No Hardcoded Stellar Addresses in Contract Code (Issue #48)

## Objective

Confirm that no Stellar account addresses (`G...`), contract addresses (`C...`),
or secret keys (`S...`) are hardcoded into the deployable contract code. Every
address must be supplied as a function parameter or read from contract storage,
never baked into the compiled WASM.

## Scope

All workspace contract crates whose code ships in a deployed WASM:

- `contracts/ephemeral_account`
- `contracts/sweep_controller`
- `contracts/account_factory`
- `contracts/reserve_contract`
- `contracts/shared`

Test modules (`#[cfg(test)]`, `src/test.rs`, `tests/`) are **out of WASM scope**
— they are compiled only for `cargo test`, not into the deployed contract — but
were reviewed anyway for good hygiene.

## Method

Ran the following searches across `contracts/**/*.rs` (excluding `target/`):

```bash
# Stellar strkeys are a 1-char version prefix + 55 base32 chars.
grep -rnE '\b[GSC][A-Z2-7]{55}\b'  contracts --include=*.rs   # any strkey literal
grep -rnE '"[GSC][A-Z2-7]{55}"'    contracts --include=*.rs   # strkey string literal
grep -rnE 'from_string|from_str|Address::from' contracts --include=*.rs
```

Rationale:

- `G[A-Z2-7]{55}` / `C[A-Z2-7]{55}` / `S[A-Z2-7]{55}` matches the canonical
  strkey encoding of ed25519 public keys, contract IDs, and secret seeds.
- `Address::from_string` / `from_str` would be the idiomatic way to turn such a
  literal into an on-chain `Address`, so its absence is corroborating evidence.

## Findings

**No hardcoded addresses or secret keys were found in any contract crate.**

- Zero `G.../C.../S...` strkey literals in `contracts/**/*.rs` (WASM or test).
- Zero `Address::from_string` / `from_str` call sites in contract source.
- Every address is either a function parameter or read from storage, for example:
  - `EphemeralAccountContract::initialize(creator, …, recovery_address, authorized_controller, admin)` — all addresses are parameters.
  - `SweepController` reads the authorized signer, authorized destination, and creator from `instance()` storage via its `DataKey` variants; the destination is a call parameter.
  - `AccountFactory::batch_initialize(creator, requests)` — the creator is a parameter and each `recovery_address` comes from the request.
- Test modules construct addresses with `soroban_sdk::testutils::Address::generate(&env)` (≈250 call sites) and use a randomly generated ed25519 public key for the sweep signer fixture — no real network addresses are embedded, and none of this code is compiled into a deployed WASM.

## Conclusion

The contract code satisfies the requirement: no Stellar addresses or secret keys
are baked into the WASM. All addresses flow in as parameters or are read from
contract storage.

## Keeping it clean

To prevent regressions, reviewers should reject any contract-source change that
introduces a `G.../C.../S...` strkey literal or an `Address::from_string` call
on a literal. The `grep` commands above can be run locally or wired into CI as a
lightweight guard.
