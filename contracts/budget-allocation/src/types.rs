use soroban_sdk::contracttype;

/// Stores the administrator and current value for the budget allocation contract.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Config {
    /// Contract administrator.
    pub admin: soroban_sdk::Address,
    /// Current configured value.
    pub value: i128,
}
