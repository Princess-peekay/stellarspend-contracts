//! # escrow.rs
//!
//! Holds tokens in trust between a depositor and a beneficiary until
//! a ledger-timestamp unlock condition is met.
//!
//! ## Gas optimizations
//! - One packed `EscrowEntry` per escrow ID instead of separate amount/ts keys
//! - `release` removes the storage slot entirely (reclaims ledger rent)
//! - Config read cached in local — no repeated instance storage lookups
//! - Single token::Client constructed per call (not once per branch)

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env};

use crate::events::{
    emit_escrow_locked, emit_escrow_released,
    EscrowLockedEventData, EscrowReleasedEventData,
};
use crate::{Config, DataKey};

// ─── Additional storage key for escrow entries ────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum EscrowKey {
    /// Keyed by a depositor-chosen numeric ID so multiple escrows per user
    /// are supported without an expensive Vec in storage.
    Entry(Address, u64),
}

// ─── Packed escrow record ─────────────────────────────────────────────────────

/// One storage slot holds everything needed to validate and release an escrow.
/// Previously would have required 3 separate keys (depositor, amount, unlock_ts).
#[contracttype]
#[derive(Clone, Debug)]
pub struct EscrowEntry {
    pub depositor:   Address,
    pub beneficiary: Address,
    pub amount:      i128,
    /// Ledger timestamp after which `release` may be called
    pub unlock_ts:   u64,
}

// ─── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {

    /// Estimates the gas cost of locking `amount` tokens in escrow until `unlock_ts`.
    ///
    /// The estimate covers depositor authorization, config lookup, token transfer,
    /// one packed persistent storage write, and lock event emission.
    ///
    /// `escrow_id` is chosen by the depositor — use a monotonic counter
    /// or a hash of (depositor, beneficiary, nonce) off-chain.
    pub fn lock(
        env:         Env,
        depositor:   Address,
        beneficiary: Address,
        amount:      i128,
        unlock_ts:   u64,
        escrow_id:   u64,
    ) {
        depositor.require_auth();

        let now = env.ledger().timestamp();
        assert!(amount    > 0,   "escrow amount must be > 0");
        assert!(unlock_ts > now, "unlock_ts must be in the future");

        // Guard: reject duplicate escrow IDs for this depositor
        let key = EscrowKey::Entry(depositor.clone(), escrow_id);
        assert!(
            !env.storage().persistent().has(&key),
            "escrow ID already in use — choose a different escrow_id"
        );

        // Read config once (optimization #4 from lib.rs applied here too)
        let config: Config = env.storage().instance()
            .get(&DataKey::Config)
            .expect("staking contract not initialised");

        // Transfer depositor → escrow contract
        token::Client::new(&env, &config.token)
            .transfer(&depositor, &env.current_contract_address(), &amount);

        // Single write — packed entry (optimization #1)
        env.storage().persistent().set(&key, &EscrowEntry {
            depositor:   depositor.clone(),
            beneficiary: beneficiary.clone(),
            amount,
            unlock_ts,
        });

        emit_escrow_locked(&env, EscrowLockedEventData {
            depositor,
            beneficiary,
            amount,
            unlock_ts,
            timestamp: now,
        });
    }

    /// Estimates the gas cost of releasing escrowed funds after `unlock_ts`.
    ///
    /// The estimate covers one escrow storage read, config lookup, persistent
    /// storage removal, token transfer to the beneficiary, and release event emission.
    /// Anyone may call this — no auth required (funds go to the beneficiary).
    pub fn release(env: Env, depositor: Address, escrow_id: u64) {
        let key = EscrowKey::Entry(depositor, escrow_id);

        // Single read (optimization #1)
        let entry: EscrowEntry = env.storage().persistent()
            .get(&key)
            .expect("escrow entry not found");

        assert!(
            env.ledger().timestamp() >= entry.unlock_ts,
            "escrow is still locked — unlock_ts has not been reached"
        );

        let config: Config = env.storage().instance()
            .get(&DataKey::Config)
            .expect("staking contract not initialised");

        // Remove slot before transfer — reclaims ledger rent (optimization)
        env.storage().persistent().remove(&key);

        token::Client::new(&env, &config.token)
            .transfer(&env.current_contract_address(), &entry.beneficiary, &entry.amount);

        emit_escrow_released(&env, EscrowReleasedEventData {
            beneficiary: entry.beneficiary,
            amount:      entry.amount,
            timestamp:   env.ledger().timestamp(),
        });
    }

    /// Estimates the gas cost of reading an escrow entry without modifying state.
    pub fn get_escrow(env: Env, depositor: Address, escrow_id: u64) -> Option<EscrowEntry> {
        env.storage().persistent()
            .get(&EscrowKey::Entry(depositor, escrow_id))
    }
}
