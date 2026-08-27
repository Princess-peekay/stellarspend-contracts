use soroban_sdk::{contracttype, Address, Symbol};

/// Global contract configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Config {
    /// Contract administrator. Not required for any per-user budget
    /// operation today; kept for parity with the rest of the StellarSpend
    /// contracts and as a hook for future admin-gated functionality.
    pub admin: Address,
    /// Auto-incrementing counter used to allocate budget ids.
    pub last_budget_id: u64,
}

/// A user's budget for a spending category over a time window.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Budget {
    pub budget_id: u64,
    pub user: Address,
    pub name: Symbol,
    pub amount: i128,
    pub category: Symbol,
    pub asset: Symbol,
    pub start_date: u64,
    pub end_date: u64,
    pub created_at: u64,
}

/// Storage keys used by this contract.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum DataKey {
    Config,
    Budget(u64),
    UserBudgets(Address),
}
