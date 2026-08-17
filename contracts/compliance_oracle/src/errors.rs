use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// `initialize` was called more than once.
    AlreadyInitialized = 1,

    /// The contract has not been initialized yet.
    NotInitialized = 2,

    /// The caller is not the authorized attestor.
    Unauthorized = 3,

    /// The supplied `expiry_ledger` is not strictly greater than the current
    /// ledger sequence, which would produce an immediately-stale attestation.
    InvalidExpiry = 4,
}
