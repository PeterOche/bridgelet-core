use soroban_sdk::contracterror;

// Error codes for NonceRegistry occupy the 7000–7099 range.
// See contracts/ephemeral_account/src/errors.rs for the full namespace map.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// The (signer, nonce) pair has already been consumed and cannot be reused.
    ///
    /// This is the core replay-protection error.  Callers must use a fresh
    /// nonce (e.g. one returned by `next_nonce`) for every new authorisation.
    NonceAlreadyConsumed = 7000,
}
