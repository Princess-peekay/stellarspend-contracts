use soroban_sdk::{contracttype, Address, Symbol};

/// Global contract configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Config {
    /// Contract administrator. Not required for any deposit/withdraw
    /// operation today; kept for parity with the rest of the StellarSpend
    /// contracts and as a hook for future admin-gated functionality.
    pub admin: Address,
}

/// Storage keys used by this contract.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum DataKey {
    Config,
    /// Balance for a given (user, asset) pair.
    Balance(Address, Symbol),
}
