use crate::models::*;
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;

/// this is really the time at which this value is first accessed
/// it's used as an optimization for the ::Default handlers defined below
pub static CURRENT_TIME: Lazy<DateTime<Utc>> = Lazy::new(Utc::now);

impl Default for Obligation {
    fn default() -> Self {
        Self {
            id: 0,
            ltv: 0_f64,
            account: String::default(),
            scraped_at: *CURRENT_TIME,
        }
    }
}

impl Default for PriceFeed {
    fn default() -> Self {
        Self {
            id: 0,
            token_mint: String::default(),
            price_account: String::default(),
            decimals: 0,
            price: 0_f64,
            scraped_at: *CURRENT_TIME,
        }
    }
}
