use soroban_sdk::contracterror;

// Error codes for NotificationRegistry occupy the 11000-11099 range.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// No subscription exists for the `(watched_address, subscriber)` pair.
    NotSubscribed = 11000,
}
