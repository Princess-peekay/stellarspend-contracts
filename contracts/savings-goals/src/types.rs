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
    pub goal_id: u64,
    pub user: Address,
    pub name: Symbol,
    pub target: i128,
    pub current_amount: i128,
    pub asset: Symbol,
    pub deadline: u64,
    pub created_at: u64,
    pub is_complete: bool,
    pub round_up_enabled: bool,
    pub round_up_nearest_unit: i128,
    pub schedule_status: ScheduleStatus,
}

/// A single contribution recorded against a goal.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Contribution {
    pub contribution_id: u64,
    pub goal_id: u64,
    pub user: Address,
    pub amount: i128,
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
