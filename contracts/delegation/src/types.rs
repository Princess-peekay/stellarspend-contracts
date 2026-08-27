use soroban_sdk::contracttype;

/// Stored configuration for the delegation contract.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Config {
    /// Contract administrator.
    pub admin: soroban_sdk::Address,
    /// Current configured value.
    pub value: i128,
}
