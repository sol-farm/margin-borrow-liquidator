use crate::models::*;
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use solana_sdk::{system_instruction, system_program};

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

