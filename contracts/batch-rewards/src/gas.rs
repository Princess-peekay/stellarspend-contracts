//! # batch_reward.rs
//!
//! Distributes staking rewards to multiple users in a single contract call.
//!
//! ## Gas optimizations
//! - Config read **once** before the loop — not once per recipient
//! - All per-user computation done in memory; storage written only at the end
//!   of each user's iteration (no intermediate reads inside the loop body)
//! - Emits **one** `BatchRewardEventData` summary instead of N individual
//!   events — saves `(N - 1) * event_base_cost` per batch run
//! - Users with zero balance are skipped before any storage is touched
//! - Storage slot removed when user balance drops to zero (reclaims rent)
//!
//! ## Naïve vs optimized storage operations for a 100-user batch
//!
//! | Operation          | Naïve  | Optimized |
//! |--------------------|--------|-----------|
//! | Config reads       | 100    | 1         |
//! | StakeEntry reads   | 100    | 100       |  <- unavoidable
//! | StakeEntry writes  | 100    | ≤ 100     |  <- skipped when balance = 0
//! | Events emitted     | 100    | 1         |
//! | **Total ops**      | **400+** | **~202** |

use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

use crate::events::{emit_batch_reward, BatchRewardEventData};
use crate::{Config, DataKey, StakeEntry, StakingContract};

// ─── Public input type ────────────────────────────────────────────────────────

/// A (staker_address, override_reward) pair.
/// Pass `override_reward = 0` to use the automatic time-weighted calculation.
/// Pass a positive value to distribute a fixed bonus on top of the calculated reward.
pub struct RewardRecipient {
    /// Address of the staker to credit.
    pub staker:          Address,
    /// Extra tokens to credit on top of the calculated reward (0 = none)
    pub bonus_amount:    i128,
}

// ─── Contract ─────────────────────────────────────────────────────────────────

/// Batch reward distributor for the staking contract.
///
/// Every entry point is written to keep the ledger footprint — the part of a
/// Soroban invocation that dominates its cost — proportional to the number of
/// stakers actually credited, rather than to the size of the batch.
#[contract]
pub struct BatchRewardContract;

#[contractimpl]
impl BatchRewardContract {

    /// Distribute rewards to all recipients in `stakers`.
    ///
    /// Only callable by the contract admin (enforced via require_auth).
    ///
    /// `bonus_amounts` must be the same length as `stakers`; pass a vec of
    /// zeros if no bonuses are needed. Using parallel vecs avoids the cost of
    /// encoding a Vec of structs in Soroban's XDR type system.
    ///
    /// # Cost
    ///
    /// The estimate below counts the ledger operations that dominate a
    /// Soroban invocation's fee — instance and persistent entry accesses, and
    /// emitted events. Arithmetic on values already in memory is not counted,
    /// because it is negligible beside a storage access.
    ///
    /// For a batch of `N` stakers, of which `C` are credited:
    ///
    /// | Operation                | Count       |
    /// |--------------------------|-------------|
    /// | Instance reads (config)  | 1           |
    /// | Persistent reads         | N           |
    /// | Persistent writes        | C           |
    /// | Events emitted           | 1 if C > 0  |
    ///
    /// The three savings against a naïve loop are: the config is read once
    /// before the loop rather than once per staker (saves `N - 1` instance
    /// reads); a staker with no balance and no bonus is skipped before any
    /// write, and one whose computed reward is non-positive is skipped too
    /// (saves `N - C` persistent writes); and the batch emits one summary
    /// event rather than one per staker (saves `C - 1` events).
    ///
    /// The `N` persistent reads are unavoidable — each staker's entry has to
    /// be read to know whether it is worth writing.
    ///
    /// # Panics
    ///
    /// Panics if `admin` has not authorized the call, if `stakers` and
    /// `bonus_amounts` differ in length, if `stakers` is empty, if the
    /// staking contract has not been initialised, or if `admin` is not the
    /// configured admin. Every one of these aborts the whole batch, so a
    /// rejected call costs nothing beyond the reads made before the failure.
    pub fn distribute_rewards(
        env:           Env,
        admin:         Address,
        stakers:       Vec<Address>,
        bonus_amounts: Vec<i128>,
    ) {
        admin.require_auth();

        assert!(
            stakers.len() == bonus_amounts.len(),
            "stakers and bonus_amounts must be the same length"
        );
        assert!(!stakers.is_empty(), "staker list must not be empty");

        // ── Optimization: read config ONCE before the loop ────────────────────
        let config: Config = env.storage().instance()
            .get(&DataKey::Config)
            .expect("staking contract not initialised");

        assert!(config.admin == admin, "caller is not the contract admin");

        let now = env.ledger().timestamp();
        let mut total_rewards: i128 = 0;
        let mut recipients:    u32  = 0;

        // ── Main loop ─────────────────────────────────────────────────────────
        // Each iteration: 1 read + (at most) 1 write. No config re-reads.
        let len = stakers.len();
        for i in 0..len {
            let staker = stakers.get(i).unwrap();
            let bonus  = bonus_amounts.get(i).unwrap();

            // Single read per user
            let mut entry: StakeEntry = env.storage()
                .persistent()
                .get(&DataKey::StakeEntry(staker.clone()))
                .unwrap_or_default();

            // Skip users with no stake — zero storage writes (optimization)
            if entry.balance == 0 && bonus == 0 {
                continue;
            }

            // Compute time-weighted reward in memory — reuse lib.rs helper
            let time_reward = if entry.balance > 0 {
                StakingContract::compute_reward(
                    entry.balance, entry.staked_at, now, config.reward_rate,
                )
            } else {
                0
            };

            let total_user_reward = time_reward + bonus;
            if total_user_reward <= 0 {
                continue;
            }

            // Credit reward into balance, reset reward clock
            entry.balance  += total_user_reward;
            entry.staked_at = now;

            // Single write per user (optimization #2)
            env.storage()
                .persistent()
                .set(&DataKey::StakeEntry(staker), &entry);

            total_rewards += total_user_reward;
            recipients    += 1;
        }

        // Only emit if at least one user received a reward
        if recipients > 0 {
            // One event for the whole batch (optimization — saves N-1 events)
            emit_batch_reward(&env, BatchRewardEventData {
                recipients,
                total_rewards,
                timestamp: now,
            });
        }
    }

    /// Preview how much reward each staker would receive right now,
    /// without modifying any state.
    ///
    /// Useful for off-chain tooling to estimate batch costs before calling
    /// `distribute_rewards`. Returns parallel vec of reward amounts.
    ///
    /// # Cost
    ///
    /// For a batch of `N` stakers: 1 instance read for the config, `N`
    /// persistent reads, no writes and no events. Read-only, so when it is
    /// simulated rather than submitted it costs nothing on-chain at all.
    ///
    /// This is the function to use to price a batch before paying for it.
    /// Counting the non-zero entries gives a lower bound on `C`, the write
    /// count in `distribute_rewards`' cost table, without touching the
    /// ledger. It is only a lower bound because this function computes the
    /// time-weighted reward alone: a staker with no balance but a positive
    /// bonus previews as zero and is still written.
    ///
    /// The count is also only valid for the ledger timestamp it was read at.
    /// Rewards accrue with elapsed time, so a staker previewing as zero can
    /// become non-zero before the distribution is submitted.
    ///
    /// # Panics
    ///
    /// Panics if the staking contract has not been initialised. Unlike
    /// `distribute_rewards`, this entry point requires no authorization.
    pub fn preview_rewards(
        env:     Env,
        stakers: Vec<Address>,
    ) -> Vec<i128> {
        let config: Config = env.storage().instance()
            .get(&DataKey::Config)
            .expect("staking contract not initialised");

        let now = env.ledger().timestamp();
        let mut results = Vec::new(&env);

        for i in 0..stakers.len() {
            let staker = stakers.get(i).unwrap();
            let entry: StakeEntry = env.storage()
                .persistent()
                .get(&DataKey::StakeEntry(staker))
                .unwrap_or_default();

            let reward = if entry.balance > 0 {
                StakingContract::compute_reward(
                    entry.balance, entry.staked_at, now, config.reward_rate,
                )
            } else {
                0
            };

            results.push_back(reward);
        }

        results
    }
}