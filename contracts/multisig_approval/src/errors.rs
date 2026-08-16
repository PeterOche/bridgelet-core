use soroban_sdk::contracterror;

// Error codes for MultiSigApproval occupy the 5000–5099 range.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Contract has already been initialized.
    AlreadyInitialized = 5000,

    /// Contract has not been initialized yet.
    NotInitialized = 5001,

    /// Caller is not a registered signer.
    NotASigner = 5002,

    /// Threshold exceeds the number of registered signers.
    ThresholdTooHigh = 5003,

    /// Threshold must be at least 1.
    ThresholdZero = 5004,

    /// The proposal ID does not exist.
    ProposalNotFound = 5005,

    /// This signer has already approved this proposal.
    AlreadyApproved = 5006,

    /// Caller is not the admin.
    Unauthorized = 5007,

    /// Must provide at least one signer.
    NoSigners = 5008,
}
