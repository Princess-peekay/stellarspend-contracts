//! Shared event-emission helpers for StellarSpend contracts.
//!
//! Wraps the Soroban [`Env::events`] API with a single consistent call
//! signature so every contract emits events in the same way. Using a
//! shared helper prevents each contract from duplicating boilerplate and
//! makes it easy to evolve the event schema in one place.
use soroban_sdk::{Env, Symbol, Val};

/// Emits a standard StellarSpend contract event.
///
/// Events are published to the Soroban event ledger under a single-element
/// topic tuple `(name,)` with `data` as the event body. Downstream indexers
/// and dApps can filter by `name` to track specific contract actions.
///
/// # Arguments
/// * `env`  — The current contract execution environment.
/// * `name` — A short [`Symbol`] identifying the event type (e.g.
///   `symbol_short!("transfer")`, `symbol_short!("deposit")`).
/// * `data` — An arbitrary [`Val`] payload attached to the event. Use
///   `soroban_sdk::vec!` or a [`contracttype`]-annotated struct converted
///   with `env.to_val()` for structured payloads.
///
/// # Examples
/// ```
/// emit_event(&env, symbol_short!("deposit"), amount.into_val(&env));
/// ```
pub fn emit_event(env: &Env, name: Symbol, data: Val) {
    env.events().publish((name,), data)
}
