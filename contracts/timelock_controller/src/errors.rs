use soroban_sdk::contracterror;

// Error codes for TimelockController occupy the 4000–4099 range.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Contract has already been initialized.
    AlreadyInitialized = 4000,

    /// Contract has not been initialized yet.
    NotInitialized = 4001,

    /// Caller is not the admin.
    Unauthorized = 4002,

    /// The provided ETA is less than `now + min_delay`.
    EtaTooEarly = 4003,

    /// The action hash has already been queued and not yet executed/cancelled.
    AlreadyQueued = 4004,

    /// The action hash is not in the queue.
    NotQueued = 4005,

    /// `execute` was called before the ETA has been reached.
    NotReady = 4006,

    /// The action has been cancelled and cannot be executed.
    Cancelled = 4007,

    /// The action has already been executed.
    AlreadyExecuted = 4008,
}
