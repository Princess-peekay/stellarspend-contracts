use soroban_sdk::contracttype;

/// Stored configuration for the governance contract.
///
/// The governance contract is intended to manage community/protocol
/// decision-making, such as proposal creation and voting on protocol
/// parameters. This `Config` record is the contract's admin-controlled
/// configuration; today it tracks a single generic value, ahead of that
/// value being specialised into a real governance setting (e.g. a
/// voting quorum or proposal threshold).
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Config {
    /// The administrator address authorized to update `value` for this
    /// governance configuration.
    pub admin: soroban_sdk::Address,
    /// The current configuration value tracked by this governance
    /// contract (intended to represent a voting quorum or proposal
    /// threshold once governance-specific logic is implemented).
    pub value: i128,
}
