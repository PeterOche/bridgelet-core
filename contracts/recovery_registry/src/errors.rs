use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// `register` was called for an account that already has a guardian set.
    AlreadyRegistered = 1,

    /// The requested account has not been registered yet.
    NotRegistered = 2,

    /// The supplied threshold is zero or greater than the number of guardians.
    InvalidThreshold = 3,

    /// The guardian list is empty.
    NoGuardians = 4,

    /// The caller is not one of the registered guardians for this account.
    NotAGuardian = 5,

    /// The guardian has already approved this (account, new_owner) pair.
    AlreadyApproved = 6,
}
