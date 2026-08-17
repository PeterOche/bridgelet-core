#![no_std]

mod errors;
mod events;
mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{
    contract,
    contractimpl,
    Address,
    Env,
    Symbol,
    Vec,
};

pub use errors::Error;

#[contract]
pub struct PauseGuardian;

#[contractimpl]
impl PauseGuardian {
    pub fn initialize(
        env: Env,
        guardians: Vec<Address>,
        threshold: u32,
    ) -> Result<(), Error> {
        if storage::get_guardians(&env).is_some() {
            return Err(Error::AlreadyInitialized);
        }

        if guardians.is_empty() {
            return Err(Error::InvalidGuardians);
        }

        if threshold == 0 {
            return Err(Error::InvalidThreshold);
        }

        if threshold > guardians.len() {
            return Err(Error::InvalidThreshold);
        }

        let first_guardian = guardians
            .get(0)
            .ok_or(Error::InvalidGuardians)?;

        first_guardian.require_auth();

        storage::set_guardians(&env, &guardians);
        storage::set_threshold(&env, threshold);

        Ok(())
    }

    pub fn pause(
        env: Env,
        guardian: Address,
        scope: Symbol,
    ) -> Result<(), Error> {
        Self::require_guardian(&env, &guardian)?;

        if storage::is_paused(&env, &scope) {
            return Err(Error::AlreadyPaused);
        }

        guardian.require_auth();

        if storage::has_pause_approval(
            &env,
            &scope,
            &guardian,
        ) {
            return Ok(());
        }

        storage::set_pause_approval(
            &env,
            &scope,
            &guardian,
        );

        let approvals = Self::count_pause_approvals(
            &env,
            &scope,
        );

        let threshold = storage::get_threshold(&env)
            .ok_or(Error::NotInitialized)?;

        if approvals >= threshold {
            storage::set_paused(
                &env,
                &scope,
                true,
            );

            Self::clear_pause_approvals(
                &env,
                &scope,
            );

            events::emit_paused(
                &env,
                scope,
                guardian,
            );
        }

        Ok(())
    }

    pub fn unpause(
        env: Env,
        guardian: Address,
        scope: Symbol,
    ) -> Result<(), Error> {
        Self::require_guardian(&env, &guardian)?;

        if !storage::is_paused(&env, &scope) {
            return Err(Error::AlreadyUnpaused);
        }

        guardian.require_auth();

        if storage::has_unpause_approval(
            &env,
            &scope,
            &guardian,
        ) {
            return Ok(());
        }

        storage::set_unpause_approval(
            &env,
            &scope,
            &guardian,
        );

        let approvals = Self::count_unpause_approvals(
            &env,
            &scope,
        );

        let threshold = storage::get_threshold(&env)
            .ok_or(Error::NotInitialized)?;

        if approvals >= threshold {
            storage::set_paused(
                &env,
                &scope,
                false,
            );

            Self::clear_unpause_approvals(
                &env,
                &scope,
            );

            events::emit_unpaused(
                &env,
                scope,
                guardian,
            );
        }

        Ok(())
    }

    pub fn is_paused(
        env: Env,
        scope: Symbol,
    ) -> bool {
        storage::is_paused(&env, &scope)
    }
}

impl PauseGuardian {
    fn require_guardian(
        env: &Env,
        guardian: &Address,
    ) -> Result<(), Error> {
        let guardians = storage::get_guardians(env)
            .ok_or(Error::NotInitialized)?;

        for configured_guardian in guardians.iter() {
            if configured_guardian == *guardian {
                return Ok(());
            }
        }

        Err(Error::Unauthorized)
    }

    fn count_pause_approvals(
        env: &Env,
        scope: &Symbol,
    ) -> u32 {
        let guardians = match storage::get_guardians(env) {
            Some(value) => value,
            None => return 0,
        };

        let mut count = 0;

        for guardian in guardians.iter() {
            if storage::has_pause_approval(
                env,
                scope,
                &guardian,
            ) {
                count += 1;
            }
        }

        count
    }

    fn count_unpause_approvals(
        env: &Env,
        scope: &Symbol,
    ) -> u32 {
        let guardians = match storage::get_guardians(env) {
            Some(value) => value,
            None => return 0,
        };

        let mut count = 0;

        for guardian in guardians.iter() {
            if storage::has_unpause_approval(
                env,
                scope,
                &guardian,
            ) {
                count += 1;
            }
        }

        count
    }

    fn clear_pause_approvals(
        env: &Env,
        scope: &Symbol,
    ) {
        if let Some(guardians) =
            storage::get_guardians(env)
        {
            for guardian in guardians.iter() {
                storage::remove_pause_approval(
                    env,
                    scope,
                    &guardian,
                );
            }
        }
    }

    fn clear_unpause_approvals(
        env: &Env,
        scope: &Symbol,
    ) {
        if let Some(guardians) =
            storage::get_guardians(env)
        {
            for guardian in guardians.iter() {
                storage::remove_unpause_approval(
                    env,
                    scope,
                    &guardian,
                );
            }
        }
    }
}