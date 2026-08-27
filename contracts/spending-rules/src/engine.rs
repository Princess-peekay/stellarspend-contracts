//! Core evaluation engine for the spending-rules composition contract.
//!
//! The engine orchestrates cross-contract calls to spending-limits,
//! spending-categories, and zk-verifier to determine whether a proposed
//! transaction should be accepted or rejected.

use soroban_sdk::{Address, Bytes, Env, IntoVal, Symbol};

use crate::storage;
use crate::types::Error;

/// Evaluates a proposed transaction against all active spending rules for the
/// given user and category.
///
/// # Cross-contract calls
///
/// 1. **zk-verifier** — If `rule.zk_required_above != i128::MAX` and `amount > rule.zk_required_above`,
///    the `zk_proof` must be `Some(proof)` and it is verified via `zk_verifier.verify_spending_proof(user, proof)`.
///    A missing or invalid proof returns `Error::ZkProofRequired` or `Error::ZkProofInvalid`.
///
/// 2. **spending-categories** — The user's current weekly spend in this category is fetched via
///    `spending_categories.get_category_total(user, category, "weekly")`. If `current_spent + amount > rule.weekly_limit`,
///    `Error::CategoryLimitExceeded` is returned.
///
/// 3. **spending-limits** — The general per-asset spending cap is checked via
///    `spending_limits.check_limit(user, asset, amount)`. If this returns `false`, the transaction
///    is rejected. (The `asset` is read from the contract's configured asset; defaults to "XLM".)
///
/// # Returns
///
/// `Ok(())` if the transaction passes all checks; an appropriate `Error` otherwise.
pub fn evaluate(
    env: &Env,
    user: &Address,
    category: &Symbol,
    amount: i128,
    zk_proof: Option<Bytes>,
) -> Result<(), Error> {
    if amount <= 0 {
        return Err(Error::InvalidAmount);
    }

    // ── 1. Load the rule for this (user, category) ─────────────────────────
    let rule = storage::read_rule(env, user, category).ok_or(Error::RuleNotFound)?;

    // ── 2. ZK proof check ──────────────────────────────────────────────────
    if amount > rule.zk_required_above {
        // ZK proof is required for payments above the threshold.
        let proof = zk_proof.ok_or(Error::ZkProofRequired)?;
        verify_zk_proof(env, user, &proof)?;
    }

    // ── 3. Weekly category spend check ──────────────────────────────────────
    let current_spent = get_weekly_category_spend(env, user, category)?;
    let projected = current_spent
        .checked_add(amount)
        .ok_or(Error::CategoryLimitExceeded)?;
    if projected > rule.weekly_limit {
        return Err(Error::CategoryLimitExceeded);
    }

    // ── 4. General spending-limits check ────────────────────────────────────
    // Check the per-user asset spending cap. This is a read-only check that
    // does not modify state — it only verifies whether recording this spend
    // would exceed the user's configured limit for the asset.
    let limits_addr =
        storage::read_spending_limits_address(env).ok_or(Error::CrossContractFailed)?;
    let asset = Symbol::new(env, "XLM");
    let fn_name = Symbol::new(env, "check_limit");
    let args = soroban_sdk::vec![
        env,
        user.clone().into_val(env),
        asset.into_val(env),
        amount.into_val(env),
    ];
    // invoke_contract panics if the target contract/function doesn't exist.
    // This is the correct behavior: a missing dependency is an unrecoverable
    // configuration error. All three target contracts return plain values
    // (not Result), so no inner error handling is needed.
    let within_limit: bool = env.invoke_contract(&limits_addr, &fn_name, args);

    if !within_limit {
        return Err(Error::CategoryLimitExceeded);
    }

    Ok(())
}

// ─── Cross-contract helpers ──────────────────────────────────────────────────

/// Verifies a ZK proof via the zk-verifier contract.
fn verify_zk_proof(env: &Env, user: &Address, proof: &Bytes) -> Result<(), Error> {
    let verifier_addr = storage::read_zk_verifier_address(env).ok_or(Error::CrossContractFailed)?;
    let fn_name = Symbol::new(env, "verify_spending_proof");
    let args = soroban_sdk::vec![env, user.clone().into_val(env), proof.clone().into_val(env),];
    let valid: bool = env.invoke_contract(&verifier_addr, &fn_name, args);
    if valid {
        Ok(())
    } else {
        Err(Error::ZkProofInvalid)
    }
}

/// Fetches the user's current weekly spend in the given category from the
/// spending-categories contract.
fn get_weekly_category_spend(env: &Env, user: &Address, category: &Symbol) -> Result<i128, Error> {
    let categories_addr =
        storage::read_spending_categories_address(env).ok_or(Error::CrossContractFailed)?;
    let fn_name = Symbol::new(env, "get_category_total");
    let weekly = Symbol::new(env, "weekly");
    let args = soroban_sdk::vec![
        env,
        user.clone().into_val(env),
        category.clone().into_val(env),
        weekly.into_val(env),
    ];
    Ok(env.invoke_contract(&categories_addr, &fn_name, args))
}
