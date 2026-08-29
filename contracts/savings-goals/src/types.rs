use soroban_sdk::{contracttype, Address, Symbol};

/// Global contract configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Config {
    /// Contract administrator. Not required for any per-user goal
    /// operation today; kept for parity with the rest of the StellarSpend
    /// contracts and as a hook for future admin-gated functionality.
    pub admin: Address,
    /// Auto-incrementing counter used to allocate goal ids.
    pub last_goal_id: u64,
}

/// Lifecycle status of a goal's automated contribution schedule (e.g. a
/// round-up rule). A newly created goal starts `Active`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ScheduleStatus {
    Active,
    Paused,
    Cancelled,
}

/// A user's savings goal.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Goal {
    /// Auto-allocated unique identifier for this goal.
    pub goal_id: u64,
    /// Address of the goal owner; only this address may contribute or
    /// modify the goal.
    pub user: Address,
    /// Human-readable label for the goal, stored as a Soroban `Symbol`.
    pub name: Symbol,
    /// Target amount the user wants to save, in the smallest unit of
    /// `asset`. Must be strictly positive.
    pub target: i128,
    /// Total amount contributed so far. Incremented on each successful
    /// `contribute` call.
    pub current_amount: i128,
    /// Ticker symbol identifying the asset being saved (e.g. `USDC`).
    pub asset: Symbol,
    /// Unix timestamp after which the goal deadline has passed.
    pub deadline: u64,
    /// Ledger timestamp at the time the goal was created.
    pub created_at: u64,
    /// Set to `true` the first time `current_amount` reaches or exceeds
    /// `target`. A `milestone` event is emitted at that moment.
    pub is_complete: bool,
    /// Whether the round-up contribution rule is active for this goal.
    pub round_up_enabled: bool,
    /// The unit to which transaction amounts are rounded up when the
    /// round-up rule is enabled. Zero when the rule is disabled.
    pub round_up_nearest_unit: i128,
    /// Current lifecycle state of the automated contribution schedule.
    pub schedule_status: ScheduleStatus,
}

/// A single contribution recorded against a goal.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Contribution {
    /// Auto-allocated unique identifier for this contribution within the
    /// goal's contribution history.
    pub contribution_id: u64,
    /// Id of the goal this contribution is applied to.
    pub goal_id: u64,
    /// Address of the contributor.
    pub user: Address,
    /// Amount contributed, in the smallest unit of the goal's asset.
    pub amount: i128,
    /// Ledger timestamp at the time the contribution was recorded.
    pub timestamp: u64,
}

/// Storage keys used by this contract.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum DataKey {
    Config,
    Goal(u64),
    UserGoals(Address),
    GoalContributions(u64),
    LastContribId(u64),
}
