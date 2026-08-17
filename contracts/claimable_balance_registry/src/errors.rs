use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// `initialize` was called more than once.
    AlreadyInitialized = 1,

    /// The contract has not been initialized yet.
    NotInitialized = 2,

    /// The caller is not an authorized writer.
    Unauthorized = 3,

    /// The balance for this `(recovery_address, asset)` pair is zero; nothing
    /// to claim.
    NothingToClaim = 4,

    /// `amount` passed to `record` must be positive.
    InvalidAmount = 5,
}
