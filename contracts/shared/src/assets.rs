//! Shared asset-allowlist helpers for StellarSpend contracts.
//!
//! Centralises the list of assets accepted across all StellarSpend contract
//! operations. Keeping the allowlist in one place ensures every contract
//! rejects unsupported assets in a consistent, maintainable way.
use soroban_sdk::Symbol;

/// Returns `true` if `asset` is a supported StellarSpend asset symbol.
///
/// # Supported assets
/// | Symbol | Description |
/// |--------|-------------|
/// | `XLM`  | Stellar Lumens — native Stellar asset |
/// | `USDC` | USD Coin — Circle stablecoin on Stellar |
/// | `EURC` | Euro Coin — Circle euro stablecoin on Stellar |
///
/// Any symbol not in this list returns `false`. Contracts should call this
/// before accepting a deposit, transfer, or budget operation to ensure only
/// whitelisted assets are processed.
///
/// # Examples
/// ```
/// let env = Env::default();
/// assert!(is_supported_asset(Symbol::new(&env, "XLM")));
/// assert!(!is_supported_asset(Symbol::new(&env, "BTC")));
/// ```
pub fn is_supported_asset(asset: Symbol) -> bool {
    asset == Symbol::new(&soroban_sdk::Env::default(), "XLM")
        || asset == Symbol::new(&soroban_sdk::Env::default(), "USDC")
        || asset == Symbol::new(&soroban_sdk::Env::default(), "EURC")
}
