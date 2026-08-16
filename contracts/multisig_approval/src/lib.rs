#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Vec};

pub use errors::Error;
pub use storage::{DataKey, Proposal};

/// A general-purpose M-of-N multisig approval contract.
///
/// ## Usage pattern
/// 1. Deploy and call [`initialize`] with the initial signer list and threshold.
/// 2. Any signer may call [`propose`] with an opaque 32-byte `action_hash`
///    to create a new proposal.  The proposal ID is returned.
/// 3. Registered signers call [`approve`] with the proposal ID.  Each signer
///    may approve at most once per proposal.
/// 4. [`is_approved`] returns `true` once the approval count reaches the
///    threshold.  External contracts check this before performing privileged
///    actions.
/// 5. The admin may call [`set_signers`] to update the signer set and/or
///    threshold at any time (does not retroactively affect open proposals).
///
/// This contract is intentionally a *primitive*: it does **not** execute
/// external calls.  Callers are responsible for verifying approval status
/// before performing the underlying privileged action.
#[contract]
pub struct MultiSigApproval;

#[contractimpl]
impl MultiSigApproval {
    /// One-time initialization.
    ///
    /// # Errors
    /// * [`Error::AlreadyInitialized`] – called more than once.
    /// * [`Error::NoSigners`]          – empty signer list.
    /// * [`Error::ThresholdZero`]      – threshold is 0.
    /// * [`Error::ThresholdTooHigh`]   – threshold > signer count.
    pub fn initialize(env: Env, signers: Vec<Address>, threshold: u32) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }

        Self::validate_signers_and_threshold(&signers, threshold)?;

        // The first caller of initialize becomes the admin.
        // We require auth from the first signer as a proxy for the deployer.
        let admin = signers.get(0).unwrap();
        admin.require_auth();

        storage::set_admin(&env, &admin);
        storage::set_signers(&env, &signers);
        storage::set_threshold(&env, threshold);
        events::emit_initialized(&env, signers, threshold);

        Ok(())
    }

    /// Create a new proposal for the given `action_hash`.
    ///
    /// `proposer` must be a registered signer and must authorize the call.
    /// Returns the new proposal ID.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract not initialized.
    /// * [`Error::NotASigner`]     – proposer is not a registered signer.
    pub fn propose(env: Env, proposer: Address, action_hash: BytesN<32>) -> Result<u64, Error> {
        storage::extend_instance_ttl(&env);

        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }

        if !storage::is_signer(&env, &proposer) {
            return Err(Error::NotASigner);
        }
        proposer.require_auth();

        let proposal_id = storage::next_proposal_id(&env);
        let proposal = Proposal {
            action_hash: action_hash.clone(),
            approval_count: 0,
        };
        storage::set_proposal(&env, proposal_id, &proposal);
        events::emit_proposal_created(&env, proposal_id, action_hash, proposer);

        Ok(proposal_id)
    }

    /// Approve a proposal.
    ///
    /// `signer` must be a registered signer and must authorize the call.
    /// Duplicate approvals from the same signer are rejected.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`]  – contract not initialized.
    /// * [`Error::NotASigner`]      – signer not in the registered signer list.
    /// * [`Error::ProposalNotFound`]– proposal_id does not exist.
    /// * [`Error::AlreadyApproved`] – signer has already approved this proposal.
    pub fn approve(env: Env, signer: Address, proposal_id: u64) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }

        if !storage::is_signer(&env, &signer) {
            return Err(Error::NotASigner);
        }
        signer.require_auth();

        let mut proposal =
            storage::get_proposal(&env, proposal_id).ok_or(Error::ProposalNotFound)?;

        if storage::has_approved(&env, proposal_id, &signer) {
            return Err(Error::AlreadyApproved);
        }

        storage::set_approved(&env, proposal_id, &signer);
        proposal.approval_count += 1;
        storage::set_proposal(&env, proposal_id, &proposal);

        let threshold = storage::get_threshold(&env).unwrap_or(0);
        events::emit_proposal_approved(
            &env,
            proposal_id,
            signer,
            proposal.approval_count,
            threshold,
        );

        Ok(())
    }

    /// Returns `true` if the proposal's approval count has reached the
    /// configured threshold.
    pub fn is_approved(env: Env, proposal_id: u64) -> bool {
        storage::extend_instance_ttl(&env);

        let threshold = match storage::get_threshold(&env) {
            Some(t) => t,
            None => return false,
        };

        match storage::get_proposal(&env, proposal_id) {
            Some(p) => p.approval_count >= threshold,
            None => false,
        }
    }

    /// Update the signer set and/or threshold.
    ///
    /// Only the admin may call this function.  Does **not** retroactively
    /// affect open proposals.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`]  – contract not initialized.
    /// * [`Error::Unauthorized`]    – caller is not the admin.
    /// * [`Error::NoSigners`]       – empty signer list.
    /// * [`Error::ThresholdZero`]   – threshold is 0.
    /// * [`Error::ThresholdTooHigh`]– threshold > signer count.
    pub fn set_signers(
        env: Env,
        admin: Address,
        signers: Vec<Address>,
        threshold: u32,
    ) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let stored_admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;
        if stored_admin != admin {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        Self::validate_signers_and_threshold(&signers, threshold)?;

        storage::set_signers(&env, &signers);
        storage::set_threshold(&env, threshold);
        events::emit_signers_updated(&env, signers, threshold);

        Ok(())
    }

    /// Return the current signer list and threshold.
    pub fn get_signers(env: Env) -> Option<Vec<Address>> {
        storage::extend_instance_ttl(&env);
        storage::get_signers(&env)
    }

    /// Return the current threshold.
    pub fn get_threshold(env: Env) -> Option<u32> {
        storage::extend_instance_ttl(&env);
        storage::get_threshold(&env)
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    fn validate_signers_and_threshold(signers: &Vec<Address>, threshold: u32) -> Result<(), Error> {
        if signers.is_empty() {
            return Err(Error::NoSigners);
        }
        if threshold == 0 {
            return Err(Error::ThresholdZero);
        }
        if threshold > signers.len() {
            return Err(Error::ThresholdTooHigh);
        }
        Ok(())
    }
}
