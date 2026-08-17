use soroban_sdk::contracterror;

// Error codes for AssetAllowlist occupy the 9000–9099 range.
// See contracts/ephemeral_account/src/errors.rs for the full namespace map.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// [`AssetAllowlist::initialize`] was called more than once.
    AlreadyInitialized = 9000,

    /// A state-changing operation was attempted before the contract was
    /// initialized.
    NotInitialized = 9001,

    /// The caller is not the admin set during [`AssetAllowlist::initialize`].
    Unauthorized = 9002,
}
