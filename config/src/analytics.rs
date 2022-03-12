use anchor_lang::solana_program::pubkey::Pubkey;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr, sync::Arc};
#[remain::sorted]
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
/// provides configuration options for the analytics backend
pub struct Analytics {
    /// the maximum number of concurrent obligation refreshes
    pub obligation_refresh_concurrency: u64,
    /// information for scraping pricing related data
    pub price_feeds: Vec<PriceFeed>,
    pub reserves: Vec<Reserve>,
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
    pub quote_decimals: i16,
}


#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Reserve {
    /// name of the reserve
    pub name: String,
    /// reserve account address
    pub account: String,
}


impl Analytics {
    /// returns a HashMap of price_account -> price_feed_name
    pub fn price_account_map(&self) -> HashMap<Pubkey, String> {
        let mut feed_map = HashMap::with_capacity(self.price_feeds.len());
        for price_feed in self.price_feeds.iter() {
            feed_map.insert(price_feed.price_account(), price_feed.name.clone());
        }
        feed_map
    }
    /// returns a HashMap of price_account -> price_feed
    pub fn price_feed_map(&self) -> HashMap<Pubkey, PriceFeed> {
        let mut feed_map = HashMap::with_capacity(self.price_feeds.len());
        for price_feed in self.price_feeds.iter() {
            feed_map.insert(price_feed.price_account(), price_feed.clone());
        }
        feed_map
    }
    
    /// returns a HashMap of reserve_name -> reserve_account
    pub fn reserve_map(&self) -> HashMap<Pubkey, String> {
        let mut reserve_map = HashMap::with_capacity(self.reserves.len());
        for reserve in self.reserves.iter() {
            reserve_map.insert(
                reserve.account(),
                reserve.name.clone(),
            );
        }
        reserve_map
    }
    /// returns a PriceFeed object by searching
    /// for the price account
    pub fn price_feed_by_account(
        &self,
        account: &Pubkey
    ) -> Result<PriceFeed> {
        let account = account.to_string();
        for price_feed in self.price_feeds.iter() {
            if price_feed.price_account.eq(&account) {
                return Ok(price_feed.clone())
            }
        }
        Err(anyhow!("failed to find price feed for {}", account))
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

impl Reserve {
    pub fn account(&self) -> Pubkey {
        if self.account.is_empty() {
            Pubkey::default()
        } else {
            Pubkey::from_str(self.account.as_str()).unwrap()
        }
    }
}
