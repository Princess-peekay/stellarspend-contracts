//! Tiered rate-curve utilities for StellarSpend contracts.
//!
//! Provides a generic mechanism for applying step-function fee or reward
//! rates based on configurable threshold tiers. Used by budget, savings,
//! and rewards contracts to compute amounts without floating-point arithmetic.
use soroban_sdk::contracttype;

/// A single tier boundary: any value >= `threshold` (and below the next
/// tier's threshold, if one exists) uses `rate_bps`.
/// Tiers MUST be passed to calculate_tiered_rate sorted ascending by
/// threshold; the first tier should normally have threshold = 0.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tier {
    pub threshold: i128,
    pub rate_bps: u32, // basis points, e.g. 250 = 2.50%
}

/// Applies a stepped (tiered) rate to `value` and returns the resulting amount.
///
/// The function walks `tiers` in order (assumed sorted ascending by `threshold`)
/// and selects the **highest** tier whose `threshold` is ≤ `value`. The selected
/// tier's `rate_bps` (basis points, where 10 000 bps = 100 %) is then applied:
///
/// ```text
/// result = value * rate_bps / 10_000
/// ```
///
/// Integer division is used throughout; sub-unit remainders are truncated.
///
/// # Arguments
/// * `value` — The input amount in the contract's base unit (e.g. stroops).
/// * `tiers` — A slice of [`Tier`] values sorted ascending by `threshold`.
///
/// # Returns
/// The computed amount, or `0` when `tiers` is empty (rather than panicking —
/// callers that require at least one tier should validate that themselves with
/// a clearer domain-specific error message).
///
/// # Examples
/// ```
/// let tiers = [Tier { threshold: 0, rate_bps: 100 }]; // 1 %
/// assert_eq!(calculate_tiered_rate(500, &tiers), 5);
/// ```
pub fn calculate_tiered_rate(value: i128, tiers: &[Tier]) -> i128 {
    if tiers.is_empty() {
        return 0;
    }

    // Find the highest threshold that is <= value.
    let mut applicable_rate_bps: u32 = tiers[0].rate_bps;
    for tier in tiers {
        if value >= tier.threshold {
            applicable_rate_bps = tier.rate_bps;
        } else {
            break; // tiers assumed sorted ascending
        }
    }

    (value * applicable_rate_bps as i128) / 10_000
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    fn sample_tiers() -> Vec<Tier> {
        vec![
            Tier {
                threshold: 0,
                rate_bps: 100,
            }, // 1.00% from 0
            Tier {
                threshold: 1_000,
                rate_bps: 50,
            }, // 0.50% from 1000
            Tier {
                threshold: 10_000,
                rate_bps: 25,
            }, // 0.25% from 10000
        ]
    }

    #[test]
    fn empty_tier_list_returns_zero() {
        assert_eq!(calculate_tiered_rate(5_000, &[]), 0);
    }

    #[test]
    fn value_in_first_tier() {
        let tiers = sample_tiers();
        // 500 * 1.00% = 5
        assert_eq!(calculate_tiered_rate(500, &tiers), 5);
    }

    #[test]
    fn value_exactly_at_second_tier_boundary_uses_second_tier() {
        let tiers = sample_tiers();
        // value == 1000 should use the 0.50% tier, not the 1.00% tier
        // 1000 * 0.50% = 5
        assert_eq!(calculate_tiered_rate(1_000, &tiers), 5);
    }

    #[test]
    fn value_just_below_second_tier_boundary_uses_first_tier() {
        let tiers = sample_tiers();
        // 999 * 1.00% = 9 (integer division)
        assert_eq!(calculate_tiered_rate(999, &tiers), 9);
    }

    #[test]
    fn value_above_highest_tier_uses_highest_tier() {
        let tiers = sample_tiers();
        // 50_000 * 0.25% = 125
        assert_eq!(calculate_tiered_rate(50_000, &tiers), 125);
    }

    #[test]
    fn zero_value_returns_zero() {
        let tiers = sample_tiers();
        assert_eq!(calculate_tiered_rate(0, &tiers), 0);
    }
}
