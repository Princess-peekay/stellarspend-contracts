use soroban_sdk::{contracterror, contracttype, Address, Symbol};

/// Per-user, per-category spending rule.
///
/// Each rule defines a weekly spending cap and a ZK-proof threshold for a
/// specific spending category. When a transaction is evaluated, the engine
/// cross-contracts into spending-limits, spending-categories, and zk-verifier
/// to determine whether the proposed payment should be accepted.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Rule {
    /// The spending category this rule applies to (e.g. "Groceries", "Travel").
    pub category: Symbol,
    /// Maximum amount allowed per week in this category (in base units, e.g. stroops for XLM).
    pub weekly_limit: i128,
    /// Amount above which a ZK proof is required (in base units).
    /// Payments at or below this threshold are accepted without a proof.
    /// Set to `i128::MAX` to effectively disable the ZK requirement.
    pub zk_required_above: i128,
}

/// Typed errors for the spending_rules contract.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Contract has already been initialized.
    AlreadyInitialized = 1,
    /// Caller is not the administrator.
    Unauthorized = 2,
    /// Amount validation failed.
    InvalidAmount = 3,
    /// No rule exists for the given (user, category) pair.
    RuleNotFound = 4,
    /// A payment above the ZK-required threshold was submitted without a proof.
    ZkProofRequired = 5,
    /// The submitted ZK proof failed cryptographic verification.
    ZkProofInvalid = 6,
    /// The payment would push the user's weekly category spend over the cap.
    CategoryLimitExceeded = 7,
    /// A cross-contract call failed unexpectedly.
    CrossContractFailed = 8,
}

/// Persistent storage keys for per-user, per-category rules.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// (user, category) -> Rule
    Rule(Address, Symbol),
}

/// Contract-wide configuration stored in instance storage.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Config {
    /// Contract administrator.
    pub admin: Address,
}
