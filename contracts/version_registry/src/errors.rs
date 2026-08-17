use soroban_sdk::contracterror;

// Error codes for VersionRegistry occupy the 9000–9099 range.
// See contracts/ephemeral_account/src/errors.rs for the full namespace map.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// [`VersionRegistry::initialize`] was called more than once.
    AlreadyInitialized = 9000,

    /// A state-changing operation was attempted before the contract was
    /// initialized.
    NotInitialized = 9001,

    /// The caller is not the admin set during [`VersionRegistry::initialize`].
    Unauthorized = 9002,

    /// The supplied `wasm_hash` does not match a known contract WASM.
    InvalidWasmHash = 9003,
}
