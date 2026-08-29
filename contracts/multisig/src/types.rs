use soroban_sdk::contracttype;

/// Stored configuration for the multisig contract.
///
/// The multisig contract is intended to coordinate multi-party approval
/// workflows, where an action only takes effect once enough authorized
/// signers have approved it. This `Config` record is the contract's
/// admin-controlled configuration; today it tracks a single generic
/// value, ahead of that value being specialised into a real signer
/// threshold / approval-policy setting.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Config {
    /// The administrator address authorized to update `value` for this
    /// multisig configuration.
    pub admin: soroban_sdk::Address,
    /// The current configuration value tracked by this multisig
    /// contract (intended to represent a signer threshold or approval
    /// policy setting once multisig-specific logic is implemented).
    pub value: i128,
}

