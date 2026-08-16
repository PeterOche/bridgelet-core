use soroban_sdk::contracterror;

// Error codes for FeeSplitter occupy the 6000–6099 range.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Contract has already been initialized.
    AlreadyInitialized = 6000,

    /// Contract has not been initialized yet.
    NotInitialized = 6001,

    /// Caller is not the admin.
    Unauthorized = 6002,

    /// The payee list is empty.
    NoPayees = 6003,

    /// The shares_bps and payees arrays have different lengths.
    LengthMismatch = 6004,

    /// The sum of shares_bps does not equal exactly 10 000.
    SharesDoNotSum = 6005,

    /// An individual share value is 0 (all payees must receive something).
    ZeroShare = 6006,

    /// The split amount is not positive.
    InvalidAmount = 6007,
}
