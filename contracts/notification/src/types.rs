use soroban_sdk::contracttype;

/// Stored configuration for the notification contract.
///
/// The notification contract is intended to manage user-facing alert
/// preferences and delivery settings for StellarSpend events (e.g.
/// spending limits, budget thresholds). This `Config` record is the
/// contract's admin-controlled configuration; today it tracks a single
/// generic value, ahead of that value being specialised into a real
/// notification-preference setting.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Config {
    /// The administrator address authorized to update `value` for this
    /// notification configuration.
    pub admin: soroban_sdk::Address,
    /// The current configuration value tracked by this notification
    /// contract (intended to represent a notification preference or
    /// delivery setting once notification-specific logic is
    /// implemented).
    pub value: i128,
}
