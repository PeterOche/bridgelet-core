#![no_std]

pub mod errors;
pub mod passphrase;
pub mod storage_keys;
mod types;

#[cfg(test)]
pub mod test_utils;

pub mod interfaces;

// Re-export the interface traits at the crate root so consumers can import
// them as `bridgelet_shared::EphemeralAccountInterface` /
// `bridgelet_shared::SweepControllerInterface` instead of having to know
// about the private `interfaces` submodule.
pub use interfaces::{EphemeralAccountInterface, SweepControllerInterface};

pub use errors::SharedError;
pub use storage_keys::StorageKey;
pub use types::{
    AccountInfo, AccountInitRequest, AccountInitResult, AccountStatus, AssetBalance,
    ContractVersion, Payment, SweepPayload,
};
