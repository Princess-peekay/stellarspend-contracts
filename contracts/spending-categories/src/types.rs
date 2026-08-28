use soroban_sdk::{contracttype, Address, Env, Symbol};

/// Reporting period granularity for category totals. Mirrors
/// spending-limits' Period type (duplicated locally rather than shared
/// cross-crate — these are independently deployable contracts, see
/// docs/ARCHITECTURE.md).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum Period {
    Daily,
    Weekly,
    Monthly,
}

impl Period {
    /// Period length in seconds.
    pub fn seconds(&self) -> u64 {
        match self {
            Period::Daily => 86_400,
            Period::Weekly => 604_800,
            Period::Monthly => 2_592_000, // 30 days
        }
    }

    /// Parses a period from its canonical Symbol representation.
    pub fn from_symbol(env: &Env, symbol: &Symbol) -> Option<Period> {
        if *symbol == Symbol::new(env, "daily") {
            Some(Period::Daily)
        } else if *symbol == Symbol::new(env, "weekly") {
            Some(Period::Weekly)
        } else if *symbol == Symbol::new(env, "monthly") {
            Some(Period::Monthly)
        } else {
            None
        }
    }

    /// The index of the period bucket the current ledger timestamp falls
    /// into, used to key each period's accumulated spend independently.
    pub fn index(&self, env: &Env) -> u64 {
        env.ledger().timestamp() / self.seconds()
    }

    /// All granularities record_category_spend maintains simultaneously, so
    /// get_category_total(user, category, period) can answer any of them
    /// without record_category_spend needing to know in advance which
    /// granularity a future query will ask for.
    pub fn all() -> [Period; 3] {
        [Period::Daily, Period::Weekly, Period::Monthly]
    }
}

/// A transaction's category assignment. `owner` is whoever called
/// `set_category` — the identity that later spend recordings and
/// per-category totals are attributed to.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct CategoryAssignment {
    pub owner: Address,
    pub category: Symbol,
}

/// Storage keys for this contract.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// tx_id -> CategoryAssignment
    Assignment(u64),
    /// (owner, category, period_kind, period_index) -> i128 accumulated spend
    CategoryTotal(Address, Symbol, Period, u64),
}
