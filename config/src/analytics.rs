use serde::{Deserialize, Serialize};

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
}
