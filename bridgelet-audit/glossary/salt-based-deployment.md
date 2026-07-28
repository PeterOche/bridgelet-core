# Salt-Based Deployment

## What is a Salt in batch_initialize's deploy_v2 Calls?

In the context of `AccountFactory::batch_initialize` and Soroban's `deploy_v2`, a **salt** is a 32-byte value used as input to deterministic contract address derivation. When deploying contracts via Soroban's deployer API, the resulting contract address is computed as:
```
contract_address = H(factory_address, salt, wasm_hash, init_args)
```
This deterministic derivation mechanism is analogous to Ethereum's CREATE2 deployment style, which enables precomputation of contract addresses before they are deployed.

## Salt Derivation in the Current Implementation

The current implementation in the AccountFactory contract derives a unique salt for each request in a batch by combining two values:
1. **A monotonic factory-wide batch nonce**: A `u64` counter stored in the factory's instance storage that increments by exactly 1 for every invocation of `batch_initialize`
2. **The request's position (index) in the batch**: A `u32` representing the 0-based index of the request within the current batch's requests vector

### Salt Layout (32 bytes, big-endian):
```
[0..8]   nonce   — monotonically increases each call to batch_initialize
[8..28]  zeros   — reserved for future use
[28..32] index   — per-request position inside the current batch
```

The code that constructs this salt (from [multiple.rs](file:///c:/Users/n-ishaq/Desktop/wave7/bridgelet-core/contracts/account_factory/src/multiple.rs#L131-L134)):
```rust
let mut salt_bytes = [0u8; 32];
salt_bytes[0..8].copy_from_slice(&nonce.to_be_bytes());
salt_bytes[28..32].copy_from_slice(&(index as u32).to_be_bytes());
let salt = BytesN::from_array(&env, &salt_bytes);
```

## Required Uniqueness Properties

For the deployment system to work safely across the contract's entire lifetime, salts must satisfy these critical uniqueness properties:

1. **Global uniqueness across all deployments**: No two ephemeral account deployments from the same factory can ever use the same salt. This guarantees no address collisions, which would cause `deploy_v2` to fail or interact with an existing contract incorrectly.

2. **Batch isolation**: Separate invocations of `batch_initialize` must produce completely disjoint sets of addresses, even if they contain the same number of requests at the same indices. The monotonic batch nonce ensures this — since the nonce always increases, the first 8 bytes of the salt will never repeat across different batch calls.

3. **Within-batch uniqueness**: Within a single batch call, every request must have a unique index. Since the loop iterates with `enumerate()`, each index is guaranteed to be unique within the batch, ensuring the last 4 bytes of the salt are distinct.

4. **Overflow prevention**: The batch nonce is a `u64`, which can never realistically overflow (it would require 1.8e19 batch calls to exhaust the space). The workspace enables `overflow-checks = true` in release builds, so any hypothetical overflow would panic rather than silently wrap around and produce colliding salts.

## Relationship to CREATE2-Style Deployment

This salt-based deterministic deployment follows the same principles as **CREATE2-style deployment** (a common pattern in smart contract ecosystems for deterministic address generation). Like CREATE2, Soroban's `deploy_v2` with a salt allows:
- Precomputing contract addresses before deployment
- Guaranteeing address uniqueness when salts are unique
- Avoiding reliance on transaction order or nonce-based address derivation

While this repository does not currently have a standalone `create2-style-deployment.md` glossary entry, the salt mechanism used here implements the core concepts of that pattern within the Soroban runtime's specific implementation.