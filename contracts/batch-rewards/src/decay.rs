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
/// # Reward formula
///
/// Rewards here are **time-weighted and undecayed**. Each staker's reward is
/// delegated to `StakingContract::compute_reward(balance, staked_at, now,
/// reward_rate)`, which accrues on the stake held since `staked_at` at the
/// configured `reward_rate`; the same stake held twice as long earns twice as
/// much. No decay factor, half-life or age discount is applied anywhere in
/// this module.
///
/// The only time-based reset is that crediting a staker sets `staked_at =
/// now`, so the reward clock restarts from the distribution and accrued time
/// is never counted twice.
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
    /// # Formula
    ///
    /// For each staker:
    ///
    /// ```text
    /// time_reward = compute_reward(balance, staked_at, now, reward_rate)   // 0 if balance == 0
    /// credited    = time_reward + bonus
    /// balance    += credited
    /// staked_at   = now                                                    // reward clock reset
    /// ```
    ///
    /// The accrual is linear in elapsed time and **undecayed** — no half-life
    /// or age discount is applied. Setting `staked_at = now` on credit is what
    /// stops the same elapsed time being paid for twice; it is a reset, not a
    /// decay.
    ///
    /// A staker is skipped without a write when they have neither balance nor
    /// bonus, or when `credited` comes out non-positive.
    ///
    /// # Panics
    ///
    /// Panics if `admin` has not authorized the call, if `stakers` and
    /// `bonus_amounts` differ in length, if `stakers` is empty, if the
    /// staking contract has not been initialised, or if `admin` is not the
    /// configured admin. Any of these aborts the entire batch — no staker is
    /// credited.
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
    /// # Formula
    ///
    /// Applies the same undecayed, time-weighted accrual as
    /// [`BatchRewardContract::distribute_rewards`] —
    /// `compute_reward(balance, staked_at, now, reward_rate)`, or `0` for a
    /// staker with no balance — evaluated at the current ledger timestamp.
    ///
    /// Bonuses are **not** included, so a preview is the accrual component
    /// only. The value grows with elapsed time, so a preview taken now
    /// understates what the same call would credit later.
    ///
    /// # Panics
    ///
    /// Panics if the staking contract has not been initialised. Requires no
    /// authorization and writes nothing.
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

//! # Batch Rewards Distribution Contract
#![no_std]

mod types;
mod validation;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, panic_with_error, token, Address, Env, Vec};

pub use crate::types::{
    BatchRewardResult, DataKey, RewardEvents, RewardRequest, RewardResult, MAX_BATCH_SIZE,
};
use crate::validation::{validate_address, validate_amount};

/// Error codes for the batch rewards contract.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum BatchRewardsError {
    /// Contract not initialized
    NotInitialized = 1,
    /// Caller is not authorized
    Unauthorized = 2,
    /// Invalid batch data
    InvalidBatch = 3,
    /// Batch is empty
    EmptyBatch = 4,
    /// Batch exceeds maximum size
    BatchTooLarge = 5,
    /// Invalid token contract
    InvalidToken = 6,
    /// Insufficient balance to distribute rewards
    InsufficientBalance = 7,
    /// Invalid reward amount
    InvalidAmount = 8,
}

impl From<BatchRewardsError> for soroban_sdk::Error {
    fn from(e: BatchRewardsError) -> Self {
        soroban_sdk::Error::from_contract_error(e as u32)
    }
}

/// Admin-operated batch payer for reward amounts decided off-chain.
///
/// # Reward formula
///
/// There is none, and that is the design: this contract transfers the exact
/// `amount` given in each [`RewardRequest`]. No accrual, no decay, no age
/// weighting — whatever computed the amounts did so before the call. It
/// validates, transfers, and records what happened.
///
/// Contrast [`BatchRewardContract`] above, which derives each amount from
/// stake and elapsed time.
#[contract]
pub struct BatchRewardsContract;

#[contractimpl]
impl BatchRewardsContract {
    /// Initializes the contract with an admin address.
    ///
    /// Records `admin` and zeroes the three running totals reported by
    /// [`BatchRewardsContract::get_total_batches`],
    /// [`BatchRewardsContract::get_total_rewards_processed`] and
    /// [`BatchRewardsContract::get_total_volume_distributed`].
    ///
    /// # Panics
    ///
    /// Panics if the contract has already been initialized. Note that this
    /// entry point does **not** require authorization, so whoever calls it
    /// first becomes admin.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Contract already initialized");
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TotalBatches, &0u64);
        env.storage()
            .instance()
            .set(&DataKey::TotalRewardsProcessed, &0u64);
        env.storage()
            .instance()
            .set(&DataKey::TotalVolumeDistributed, &0i128);
    }

    /// Gets the contract admin.
    ///
    /// # Panics
    ///
    /// Panics if the contract has not been initialized.
    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized")
    }

    /// Gets the total number of reward batches processed.
    ///
    /// This doubles as the id of the most recent batch: batch ids are this
    /// counter plus one, assigned at the start of each distribution.
    ///
    /// Returns `0` on an uninitialized contract rather than panicking.
    pub fn get_total_batches(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::TotalBatches)
            .unwrap_or(0)
    }

    /// Gets the total number of rewards processed.
    ///
    /// Counts every request **submitted**, including those that failed
    /// validation or whose transfer was rejected. Compare against
    /// [`BatchRewardsContract::get_total_volume_distributed`], which counts
    /// only what actually moved.
    ///
    /// Returns `0` on an uninitialized contract rather than panicking.
    pub fn get_total_rewards_processed(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::TotalRewardsProcessed)
            .unwrap_or(0)
    }

    /// Gets the total volume of rewards distributed.
    ///
    /// Sums the amounts of successful transfers only; failed requests
    /// contribute nothing.
    ///
    /// Returns `0` on an uninitialized contract rather than panicking.
    pub fn get_total_volume_distributed(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalVolumeDistributed)
            .unwrap_or(0)
    }

    /// Sets a new admin address.
    ///
    /// Transfers administration in one step and emits an `admin` event with
    /// the new address. There is no two-step accept, so an unusable
    /// `new_admin` leaves the contract without a working administrator.
    ///
    /// # Panics
    ///
    /// Panics if `caller` has not authorized the call, if the contract has
    /// not been initialized, or with
    /// [`BatchRewardsError::Unauthorized`] if `caller` is not the current
    /// admin.
    pub fn set_admin(env: Env, caller: Address, new_admin: Address) {
        caller.require_auth();
        Self::require_admin(&env, &caller);

        env.storage().instance().set(&DataKey::Admin, &new_admin);
        let topics = (soroban_sdk::symbol_short!("admin"),);
        env.events().publish(topics, (&new_admin,));
    }

    /// Distributes rewards to multiple recipients in a batch operation.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment
    /// * `caller` - The address initiating the batch rewards
    /// * `token` - The token contract address (e.g., XLM)
    /// * `rewards` - Vector of reward requests containing recipient and amount
    ///
    /// # Returns
    /// A `BatchRewardResult` containing the results of the distribution
    ///
    /// # Formula
    ///
    /// None is applied. Each recipient receives exactly `reward.amount` as
    /// submitted — no decay, no time weighting, no proportional scaling. If
    /// amounts should fall off with age or stake, that has to happen before
    /// the call.
    ///
    /// # Partial failure
    ///
    /// Individual requests fail independently: an invalid amount, an invalid
    /// recipient, or a rejected transfer records a
    /// [`RewardResult::Failure`] with an error code, emits a failure event,
    /// and the batch continues. So a returned `BatchRewardResult` can report
    /// successes and failures together, and the caller must read `failed`
    /// rather than assume the whole batch landed.
    ///
    /// The balance check before the loop compares the caller's balance
    /// against the sum of **all** requested amounts, including ones that will
    /// later fail validation — so it is conservative, and a batch can be
    /// rejected for insufficient balance even though the amounts that would
    /// actually transfer do fit.
    ///
    /// # Panics
    ///
    /// Unlike a per-request failure, these abort the whole batch: no
    /// authorization from `caller`, a caller who is not the admin, an empty
    /// batch ([`BatchRewardsError::EmptyBatch`]), more than
    /// [`MAX_BATCH_SIZE`] requests ([`BatchRewardsError::BatchTooLarge`]), or
    /// a caller balance below the total requested
    /// ([`BatchRewardsError::InsufficientBalance`]).
    pub fn distribute_rewards(
        env: Env,
        caller: Address,
        token: Address,
        rewards: Vec<RewardRequest>,
    ) -> BatchRewardResult {
        // Verify authorization
        caller.require_auth();
        Self::require_admin(&env, &caller);

        // Validate batch size
        let request_count = rewards.len();
        if request_count == 0 {
            panic_with_error!(&env, BatchRewardsError::EmptyBatch);
        }
        if request_count > MAX_BATCH_SIZE {
            panic_with_error!(&env, BatchRewardsError::BatchTooLarge);
        }

        // Get batch ID and increment
        let batch_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TotalBatches)
            .unwrap_or(0)
            + 1;

        // Emit batch started event
        RewardEvents::batch_started(&env, batch_id, request_count);

        // Initialize result vectors
        let mut results: Vec<RewardResult> = Vec::new(&env);
        let mut successful_count: u32 = 0;
        let mut failed_count: u32 = 0;
        let mut total_distributed: i128 = 0;

        // Create token client
        let token_client = token::Client::new(&env, &token);

        // Get initial balance to ensure sufficient funds
        let available_balance = token_client.balance(&caller);
        let total_required: i128 = rewards
            .iter()
            .fold(0i128, |sum, reward| sum + reward.amount);

        if available_balance < total_required {
            panic_with_error!(&env, BatchRewardsError::InsufficientBalance);
        }

        // Process each reward request
        for reward in rewards.iter() {
            // Validate reward amount
            if let Err(_) = validate_amount(reward.amount) {
                failed_count += 1;
                let error_code = BatchRewardsError::InvalidAmount as u32;
                results.push_back(RewardResult::Failure(
                    reward.recipient.clone(),
                    reward.amount,
                    error_code,
                ));
                RewardEvents::reward_failure(
                    &env,
                    batch_id,
                    &reward.recipient,
                    reward.amount,
                    error_code,
                );
                continue;
            }

            // Validate recipient address
            if let Err(_) = validate_address(&env, &reward.recipient) {
                failed_count += 1;
                let error_code = BatchRewardsError::InvalidBatch as u32;
                results.push_back(RewardResult::Failure(
                    reward.recipient.clone(),
                    reward.amount,
                    error_code,
                ));
                RewardEvents::reward_failure(
                    &env,
                    batch_id,
                    &reward.recipient,
                    reward.amount,
                    error_code,
                );
                continue;
            }

            // Attempt to transfer the reward
            match token_client.try_transfer(&caller, &reward.recipient, &reward.amount) {
                Ok(_) => {
                    successful_count += 1;
                    total_distributed += reward.amount;
                    results.push_back(RewardResult::Success(
                        reward.recipient.clone(),
                        reward.amount,
                    ));
                    RewardEvents::reward_success(&env, batch_id, &reward.recipient, reward.amount);
                }
                Err(_) => {
                    failed_count += 1;
                    let error_code = BatchRewardsError::InvalidToken as u32;
                    results.push_back(RewardResult::Failure(
                        reward.recipient.clone(),
                        reward.amount,
                        error_code,
                    ));
                    RewardEvents::reward_failure(
                        &env,
                        batch_id,
                        &reward.recipient,
                        reward.amount,
                        error_code,
                    );
                }
            }
        }

        // Update statistics
        env.storage()
            .instance()
            .set(&DataKey::TotalBatches, &batch_id);

        let total_processed: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TotalRewardsProcessed)
            .unwrap_or(0)
            + request_count as u64;
        env.storage()
            .instance()
            .set(&DataKey::TotalRewardsProcessed, &total_processed);

        let total_volume: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalVolumeDistributed)
            .unwrap_or(0)
            + total_distributed;
        env.storage()
            .instance()
            .set(&DataKey::TotalVolumeDistributed, &total_volume);

        // Emit batch completed event
        RewardEvents::batch_completed(
            &env,
            batch_id,
            successful_count,
            failed_count,
            total_distributed,
        );

        BatchRewardResult {
            total_requests: request_count as u32,
            successful: successful_count,
            failed: failed_count,
            total_distributed,
            results,
        }
    }

    /// Internal helper to verify that the caller is the admin.
    fn require_admin(env: &Env, caller: &Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");

        if admin != *caller {
            panic_with_error!(env, BatchRewardsError::Unauthorized);
        }
    }
}
