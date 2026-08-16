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
};

pub use errors::Error;
pub use events::{
    CallRecorded,
    ContractInitialized,
    LimitsUpdated,
};
pub use storage::{
    DataKey,
    Limits,
    RateLimitState,
};

#[contract]
pub struct RateLimiter;

#[contractimpl]
impl RateLimiter {
    /// Initialize the rate limiter.
    ///
    /// The contract can only be initialized once.
    pub fn initialize(
        env: Env,
        admin: Address,
        window_ledgers: u32,
        max_calls: u32,
    ) -> Result<(), Error> {
        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }

        Self::validate_limits(
            window_ledgers,
            max_calls,
        )?;

        admin.require_auth();

        storage::set_admin(&env, &admin);

        let limits = Limits {
            window_ledgers,
            max_calls,
        };

        storage::set_limits(&env, &limits);

        events::emit_initialized(
            &env,
            admin,
            window_ledgers,
            max_calls,
        );

        Ok(())
    }

    /// Check whether a key is allowed to perform another operation.
    ///
    /// If allowed, the operation is recorded.
    ///
    /// Example with max_calls = 3:
    ///
    /// Call 1 -> allowed
    /// Call 2 -> allowed
    /// Call 3 -> allowed
    /// Call 4 -> rejected
    pub fn check_and_record(
        env: Env,
        key: Address,
    ) -> Result<(), Error> {
        let limits = storage::get_limits(&env)
            .ok_or(Error::NotInitialized)?;

        let current_ledger = env.ledger().sequence();

        let state = match storage::get_state(&env, &key) {
            None => RateLimitState {
                window_start: current_ledger,
                calls: 1,
            },

            Some(previous) => {
                let window_end = previous
                    .window_start
                    .saturating_add(limits.window_ledgers);

                // The current window has expired.
                if current_ledger >= window_end {
                    RateLimitState {
                        window_start: current_ledger,
                        calls: 1,
                    }
                } else {
                    // The limit has already been reached.
                    if previous.calls >= limits.max_calls {
                        return Err(Error::RateLimitExceeded);
                    }

                    RateLimitState {
                        window_start: previous.window_start,
                        calls: previous.calls + 1,
                    }
                }
            }
        };

        storage::set_state(
            &env,
            &key,
            &state,
        );

        events::emit_call_recorded(
            &env,
            key,
            state.calls,
        );

        Ok(())
    }

    /// Return the number of calls remaining for a key
    /// in the current rate-limit window.
    pub fn remaining(
        env: Env,
        key: Address,
    ) -> u32 {
        let limits = match storage::get_limits(&env) {
            Some(value) => value,
            None => return 0,
        };

        let current_ledger = env.ledger().sequence();

        let state = match storage::get_state(&env, &key) {
            Some(value) => value,
            None => return limits.max_calls,
        };

        let window_end = state
            .window_start
            .saturating_add(limits.window_ledgers);

        // The previous window has expired.
        if current_ledger >= window_end {
            return limits.max_calls;
        }

        limits
            .max_calls
            .saturating_sub(state.calls)
    }

    /// Update the rate limiter configuration.
    ///
    /// Only the configured admin can change the limits.
    pub fn set_limits(
        env: Env,
        admin: Address,
        window_ledgers: u32,
        max_calls: u32,
    ) -> Result<(), Error> {
        let stored_admin = storage::get_admin(&env)
            .ok_or(Error::NotInitialized)?;

        if stored_admin != admin {
            return Err(Error::Unauthorized);
        }

        Self::validate_limits(
            window_ledgers,
            max_calls,
        )?;

        admin.require_auth();

        let limits = Limits {
            window_ledgers,
            max_calls,
        };

        storage::set_limits(
            &env,
            &limits,
        );

        events::emit_limits_updated(
            &env,
            window_ledgers,
            max_calls,
        );

        Ok(())
    }
}

impl RateLimiter {
    fn validate_limits(
        window_ledgers: u32,
        max_calls: u32,
    ) -> Result<(), Error> {
        if window_ledgers == 0 || max_calls == 0 {
            return Err(Error::InvalidLimits);
        }

        Ok(())
    }
}