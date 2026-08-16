# Postmortem: Account Factory Salt Collision

## Deterministic Address Derivation
Deterministic address derivation (like `CREATE2` in Ethereum) allows a factory contract to deploy a new contract at an address that can be computed in advance. The formula generally relies on:
1. The address of the deploying factory.
2. A unique `salt` (a 32-byte value).
3. The bytecode of the contract being deployed.

If all three inputs are identical, the resulting address will be identical. A factory will fail if it attempts to deploy a contract to an address that already contains a deployed contract. Therefore, the `salt` is typically used to ensure that each deployed contract gets a unique address, even if the factory and the bytecode are identical.

## The Bug: Index-Based Salts Colliding
In the `AccountFactory` implementation, multiple accounts could be deployed via a batch initialization function (`batch_initialize`). 

**The Flaw:**
The salt was generated using an index based on the loop iteration within the batch (e.g., `salt = hash(index)`).

**A Worked Example:**
1. A caller invokes `batch_initialize` to deploy 3 accounts. The loop uses `index = 0, 1, 2`. 
   - `index 0` deploys to Address A.
   - `index 1` deploys to Address B.
   - `index 2` deploys to Address C.
2. Later, a caller (either the same or different) invokes `batch_initialize` to deploy 2 more accounts. 
   - The loop starts again at `index 0`.
   - The factory tries to deploy the new account with `salt = hash(0)`.
   - The computed address is Address A, which is already occupied by the first deployment.
   - The transaction reverts due to a collision at `index 0`.

Because the salt space was local to the function call rather than globally unique (like a nonce or an external unique identifier), separate calls overlapped in their salt generation.

## Lessons Learned and General Guidance
When designing smart contracts that perform programmatic deployments, consider the following best practices for salt design:

1. **Avoid Loop-Only Salts:** Never rely solely on a loop index to generate a salt unless the factory guarantees that the loop will only ever be executed once.
2. **Incorporate Global State:** Use a globally incrementing `nonce` within the factory state or require the caller to pass in a unique `salt`. If using a nonce, increment it after every deployment (e.g., `salt = hash(caller, nonce++)`).
3. **Bind to the Caller:** Incorporate the `msg.sender` into the salt if deployments should be uniquely scoped to a specific user (e.g., `salt = hash(msg.sender, user_provided_salt)`).
4. **Collision Resistance:** Treat the salt as a globally unique identifier. Ensure the entropy is sufficient to avoid collisions across different batches and callers over the entire lifetime of the factory.
