# Glossary: `symbol_short!` 9-Character Limit
**Path:** `bridgelet-audit/glossary/symbol-short-limit.md`

## Definition
In Soroban smart contracts (Rust), the `symbol_short!` macro is used to create short `Symbol` values at compile time. However, it enforces a strict **9-character limit** for the string being converted into a symbol.

## Why the Limit?
Soroban's `Symbol` type packs characters directly into a 64-bit integer (using a custom 6-bit encoding for a subset of ASCII characters). 64 bits can store up to 10 characters (60 bits) plus some type tagging bits. `symbol_short!` guarantees that the conversion can happen zero-cost at compile time without allocating memory or utilizing dynamic environments, restricted to exactly 9 characters maximum.

## Implications for Bridgelet Core
- Key names in persistent storage (e.g., `DataKey::Admin` or `symbol_short!("Nonce")`) must be kept extremely brief.
- If a string exceeds 9 characters, developers must use `Symbol::new(&env, "longer_string")` which incurs a minor runtime cost and interacts with the host environment.
