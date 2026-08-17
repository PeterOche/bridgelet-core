use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidGuardians = 3,
    InvalidThreshold = 4,
    Unauthorized = 5,
    AlreadyPaused = 6,
    AlreadyUnpaused = 7,
}