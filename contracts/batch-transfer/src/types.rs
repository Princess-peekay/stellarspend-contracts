use soroban_sdk::contracttype;

/// Stored configuration for this StellarSpend contract.
///
/// A single instance is persisted under the contract's storage key and is
/// written on initialization, then replaced on each administrative update.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Config {
    /// Contract administrator. Only this address may mutate the config.
    pub admin: soroban_sdk::Address,
    /// Current configured value. Always non-negative.
    pub value: i128,
}
