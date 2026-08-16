use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// The contract has already been initialized.
    AlreadyInitialized = 1,

    /// A state-changing operation was attempted before the contract was initialized.
    NotInitialized = 2,

    /// The caller is not the admin.
    Unauthorized = 3,

    /// The supplied `max` argument to `peek_batch` is zero.
    InvalidBatchSize = 4,
}
