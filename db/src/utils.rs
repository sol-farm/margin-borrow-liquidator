use crate::models::*;
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use solana_sdk::system_program;

/// this is really the time at which this value is first accessed
/// it's used as an optimization for the ::Default handlers defined below
pub static CURRENT_TIME: Lazy<DateTime<Utc>> = Lazy::new(Utc::now);

impl Default for Obligation {
    fn default() -> Self {
        Self {
            ltv: 0_f64,
            account: system_program::id(),
            scraped_at: *CURRENT_TIME,
        }
    }
}

impl Default for PriceFeed {
    fn default() -> Self {
        Self {
            token_mint: system_program::id(),
            price_account: system_program::id(),
            decimals: 0,
            price: 0_f64,
            scraped_at: *CURRENT_TIME,
        }
    }
}

use std::cmp::Ordering;
pub fn cmp_ltvs(a: &Obligation, b: &Obligation) -> Ordering {
    cmp_f64(&a.ltv, &b.ltv)
}
/// used to compare f64s, suitable for use in a `sort_by` call.
/// this is taken from https://users.rust-lang.org/t/sorting-vector-of-vectors-of-f64/16264
/// however it has been modified to put NaN values at the end
pub fn cmp_f64(a: &f64, b: &f64) -> Ordering {
    if a.is_nan() {
        return Ordering::Less;
    }
    if b.is_nan() {
        return Ordering::Less;
    }
    if a < b {
        Ordering::Less
    } else if a > b {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_sort_f64() {
        let mut items = vec![
            4.5,
            11.5,
            -7.3,
            14.0,
            f64::NAN,
            18.7,
            11.5,
            f64::NAN,
            1.3,
            -2.1,
            33.7,
        ];
        items.sort_unstable_by(cmp_f64);
        assert!(items[0].is_nan());
        assert!(items[1].is_nan());
        assert!(items[items.len() - 1] == 33.7);
        assert!(items[items.len() - 2] == 18.7);
    }
}
