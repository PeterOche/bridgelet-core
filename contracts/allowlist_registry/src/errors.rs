use soroban_sdk::contracterror;

// Error codes for AllowlistRegistry occupy the 8000–8099 range.
// See contracts/ephemeral_account/src/errors.rs for the full namespace map.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// [`AllowlistRegistry::initialize`] was called more than once.
    AlreadyInitialized = 8000,

    /// A state-changing operation was attempted before the contract was
    /// initialized.
    NotInitialized = 8001,

    /// The caller is not the admin set during [`AllowlistRegistry::initialize`].
    Unauthorized = 8002,
}
