use anchor_lang::solana_program::pubkey::Pubkey;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};
#[remain::sorted]
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
/// provides configuration options for the analytics backend
pub struct Analytics {
    /// the maximum number of concurrent obligation refreshes
    pub obligation_refresh_concurrency: u64,
    /// information for scraping pricing related data
    pub price_feeds: Vec<PriceFeed>,
    /// how often we start the analytics scraper work loop
    pub scrape_interval: u64,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
/// information for an asset specific price feed, such as a token (RAY)
/// or an lp token (RAY-USDC)
pub struct PriceFeed {
    /// the name of the price feed
    pub name: String,
    /// the price account which stores pricing information
    pub price_account: String,
    /// the number of decimals in the token mint
    pub token_decimals: u8,
    /// the token mint for which this price feed tracks a price for
    pub token_mint: String,
    /// number of decimals in the quote token used
    /// by the price feed
    pub quote_decimals: u8,
}

impl Analytics {
    /// returns a HashMap of price_account -> name
    pub fn price_feed_map(&self) -> Result<HashMap<Pubkey, PriceFeed>> {
        let mut feed_map = HashMap::with_capacity(self.price_feeds.len());
        for price_feed in self.price_feeds.iter() {
            feed_map.insert(price_feed.price_account(), price_feed.clone());
        }
        Ok(feed_map)
    }
}

impl PriceFeed {
    pub fn price_account(&self) -> Pubkey {
        if self.price_account.is_empty() {
            Pubkey::default()
        } else {
            Pubkey::from_str(self.price_account.as_str()).unwrap()
        }
    }
    pub fn token_mint(&self) -> Pubkey {
        if self.token_mint.is_empty() {
            Pubkey::default()
        } else {
            Pubkey::from_str(self.token_mint.as_str()).unwrap()
        }
    }
}
