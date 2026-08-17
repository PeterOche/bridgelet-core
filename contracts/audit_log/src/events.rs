use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

// ── Event payloads ───────────────────────────────────────────────────────────

/// Emitted when [`AuditLog::initialize`] is called successfully.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Initialized {
    pub admin: Address,
}

/// Emitted when a writer is authorized.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriterAuthorized {
    pub writer: Address,
    pub admin: Address,
}

/// Emitted when an audit entry is recorded.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntryRecorded {
    /// Sequential entry ID assigned at record time.
    pub id: u64,
    /// The address that called `record`.
    pub writer: Address,
    /// The address that performed the action being audited.
    pub actor: Address,
    /// Short symbol describing the action (≤ 9 bytes for `symbol_short!` compat).
    pub action: Symbol,
    /// The address the action was performed on or against.
    pub subject: Address,
    /// The ledger sequence at which the entry was recorded.
    pub ledger: u32,
}

// ── Emit helpers ─────────────────────────────────────────────────────────────

pub fn emit_initialized(env: &Env, admin: Address) {
    env.events()
        .publish((symbol_short!("init"),), Initialized { admin });
}

pub fn emit_writer_authorized(env: &Env, writer: Address, admin: Address) {
    env.events().publish(
        (symbol_short!("writer"),),
        WriterAuthorized { writer, admin },
    );
}

pub fn emit_entry_recorded(
    env: &Env,
    id: u64,
    writer: Address,
    actor: Address,
    action: Symbol,
    subject: Address,
    ledger: u32,
) {
    env.events().publish(
        (symbol_short!("recorded"),),
        EntryRecorded {
            id,
            writer,
            actor,
            action,
            subject,
            ledger,
        },
    );
}
