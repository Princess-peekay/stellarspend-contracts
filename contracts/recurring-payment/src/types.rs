use soroban_sdk::contracttype;

/// Stored configuration for the recurring-payment contract.
///
/// The recurring-payment contract is intended to manage scheduled,
/// repeating payments between StellarSpend users (e.g. subscriptions or
/// standing orders). This `Config` record is the contract's
/// admin-controlled configuration; today it tracks a single generic
/// value, ahead of that value being specialised into a real payment
/// interval or amount setting.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Config {
    /// The administrator address authorized to update `value` for this
    /// recurring-payment configuration.
    pub admin: soroban_sdk::Address,
    /// The current configuration value tracked by this
    /// recurring-payment contract (intended to represent a payment
    /// interval or amount setting once recurring-payment-specific
    /// logic is implemented).
    pub value: i128,
}

