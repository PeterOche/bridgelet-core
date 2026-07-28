# wasm32v1-none Target

## What is wasm32v1-none?

`wasm32v1-none` is the **only compilation target officially supported by the Soroban runtime** for building Stellar smart contracts. It is a Soroban/Stellar-specific WebAssembly target that differs from the more generic `wasm32-unknown-unknown` target in critical ways:

- **Feature compatibility**: Unlike generic WASM targets that may enable modern WASM features like reference-types or multivalue (which Soroban's runtime rejects), `wasm32v1-none` exclusively emits WASM that conforms to the exact feature set the Soroban VM supports.
- **Soroban-native**: This target is maintained and recommended by Stellar for all Soroban contract development. Using it avoids common runtime errors like `HostError: Error(Wasmvm, InvalidAction) / "reference-types not enabled"` that can occur with newer Rust versions and generic targets.
- **Official tooling**: The standard `stellar contract build` command (from stellar-cli) automatically uses this target, as documented in the [soroban-sdk documentation](https://docs.rs/soroban-sdk/latest/soroban_sdk/).

## All References in This Repository

### CI Workflow Configurations

1. **`.github/workflows/test.yml`**
   - Build step uses `cargo build --target wasm32v1-none --release` for all three core contracts
   - Lines 77-83:
     ```bash
     cd contracts/ephemeral_account
     cargo build --target wasm32v1-none --release
     cd ../sweep_controller
     cargo build --target wasm32v1-none --release
     cd ../reserve_contract
     cargo build --target wasm32v1-none --release
     ```

2. **`.github/workflows/test.yml` - Artifact Upload Paths**
   - Uploads WASM artifacts from the wasm32v1-none build directories
   - Lines 88-92:
     ```yaml
     path: |
       contracts/ephemeral_account/target/wasm32v1-none/release/*.wasm
       contracts/sweep_controller/target/wasm32v1-none/release/*.wasm
       contracts/reserve_contract/target/wasm32v1-none/release/*.wasm
     ```

### Scripts

1. **`scripts/build.sh`**
   - Explicitly documents why wasm32v1-none is preferred over wasm32-unknown-unknown
   - Notes that `stellar contract build` (which uses wasm32v1-none) is the recommended build approach
   - Explains the historical context: previous use of wasm32-unknown-unknown caused compatibility issues with Rust 1.82+ due to unsupported WASM features

### Codebase References

The target string `wasm32v1-none` appears in test files and runbooks across the repository, including:
- `test_fixtures/mock_v2/src/test_upgrade.rs`
- `contracts/account_factory/src/multiple.rs`
- `contracts/account_factory/src/test.rs`
- `contracts/account_factory/tests/batch.rs`
- `contracts/account_factory/tests/e2e_pipeline.rs`
- Multiple runbooks in `bridgelet-audit/runbooks/` that reference WASM build paths and verification processes

## Note

This is purely a descriptive document. No changes to the repository's build configuration are proposed here. The wasm32v1-none target is the correct, supported target for Soroban contract development in this codebase.