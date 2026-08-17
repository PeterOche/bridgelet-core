#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

pub use errors::Error;

fn require_admin_or_super_admin(env: &Env, caller: &Address) -> Result<(), Error> {
    caller.require_auth();
    let super_admin = storage::get_super_admin(env).ok_or(Error::NotInitialized)?;
    if *caller == super_admin {
        return Ok(());
    }
    if storage::has_role(env, &Symbol::new(env, "admin"), caller) {
        return Ok(());
    }
    Err(Error::Unauthorized)
}

#[contract]
pub struct AccessController;

#[contractimpl]
impl AccessController {
    /// Initialize the controller with a super admin. Can only be called once.
    pub fn initialize(env: Env, super_admin: Address) -> Result<(), Error> {
        if storage::get_super_admin(&env).is_some() {
            return Err(Error::AlreadyInitialized);
        }
        super_admin.require_auth();
        storage::set_super_admin(&env, &super_admin);
        storage::extend_instance_ttl(&env);
        Ok(())
    }

    /// Grant a role to an account. Caller must be the super admin or hold the "admin" role.
    pub fn grant_role(
        env: Env,
        caller: Address,
        role: Symbol,
        account: Address,
    ) -> Result<(), Error> {
        require_admin_or_super_admin(&env, &caller)?;
        storage::set_role(&env, &role, &account);
        events::emit_role_granted(&env, role, account, caller);
        Ok(())
    }

    /// Revoke a role from an account. Caller must be the super admin or hold the "admin" role.
    /// Cannot revoke the super admin's implicit super-admin status.
    pub fn revoke_role(
        env: Env,
        caller: Address,
        role: Symbol,
        account: Address,
    ) -> Result<(), Error> {
        require_admin_or_super_admin(&env, &caller)?;

        // The super admin always has super-admin privileges; this cannot be revoked.
        let super_admin = storage::get_super_admin(&env).ok_or(Error::NotInitialized)?;
        if role == Symbol::new(&env, "super_admin") && account == super_admin {
            return Err(Error::CannotRevokeSuperAdmin);
        }

        storage::remove_role(&env, &role, &account);
        events::emit_role_revoked(&env, role, account, caller);
        Ok(())
    }

    /// Check whether an account holds a given role. Returns false if the role was never
    /// granted (or was revoked).
    pub fn has_role(env: Env, role: Symbol, account: Address) -> bool {
        storage::has_role(&env, &role, &account)
    }

    /// Returns the super admin address.
    pub fn get_super_admin(env: Env) -> Option<Address> {
        storage::get_super_admin(&env)
    }
}
