use soroban_sdk::contracttype;

/// Stored configuration for the rewards contract.
///
/// The rewards contract is intended to track and distribute
/// loyalty/incentive rewards to StellarSpend users (e.g. cashback,
/// points, or streak bonuses). This `Config` record is the contract's
/// admin-controlled configuration; today it tracks a single generic
/// value, ahead of that value being specialised into a real reward
/// rate or pool setting.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Config {
    /// The administrator address authorized to update `value` for this
    /// rewards configuration.
    pub admin: soroban_sdk::Address,
    /// The current configuration value tracked by this rewards
    /// contract (intended to represent a reward rate or pool setting
    /// once rewards-specific logic is implemented).
    pub value: i128,
}
