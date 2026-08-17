use soroban_sdk::contracterror;

// Error codes for AuditLog occupy the 9100–9199 range.
// See contracts/ephemeral_account/src/errors.rs for the full namespace map.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// [`AuditLog::initialize`] was called more than once.
    AlreadyInitialized = 9100,

    /// A state-changing operation was attempted before the contract was
    /// initialized.
    NotInitialized = 9101,

    /// The caller is not the admin set during [`AuditLog::initialize`].
    Unauthorized = 9102,

    /// `record` was called by an address that has not been authorized as a
    /// writer by the admin.
    UnauthorizedWriter = 9103,

    /// The requested entry ID does not exist.
    EntryNotFound = 9104,
}
