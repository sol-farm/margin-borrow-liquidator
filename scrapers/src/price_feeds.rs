use chrono::{DateTime, Utc};

use config::Configuration;
use diesel::PgConnection;
use log::error;

use config::analytics::PriceFeed;
use diesel::Connection;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcAccountInfoConfig};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use tulipv2_sdk_common::pyth;
use std::sync::Arc;

/// scrapes the given price accounts for the quoted price
/// caching the value into the database.
pub fn scrape_price_feeds(
    config: &Arc<Configuration>,
    rpc: &Arc<RpcClient>,
    conn: &PgConnection,
    price_accounts: &Arc<HashMap<Pubkey, String>>,
    scraped_at: DateTime<Utc>,
) {
    let price_feeds = config.analytics.price_feed_map();

    let mut price_feed_accounts = match config.get_price_feeds(
        rpc,
        &price_accounts,
    ) {
        Ok(price_feed_accounts) => price_feed_accounts,
        Err(err) => {
            error!("failed to retrieve price feed accounts {:#?}", err);
            return;
        }
    };
    price_feed_accounts
    .iter_mut()
    .for_each(|(price_key, price_account)| {
        let price_account = std::mem::take(price_account);
        let pyth_price = match pyth::parse_pyth_price(&price_account) {
            Ok(pyth_price) => pyth_price,
            Err(err) => {
                error!("failed to load parse pyth price {}: {:#?}", price_key, err);
                return;
            }
        };
        let price = match f64::from_str(&pyth_price.to_string()) {
            Ok(price) => price,
            Err(err) => {
                error!("failed to parse pyth to float {}: {:#?}", price_key, err);
                return;
            }
        };
        let price_feed = match price_feeds.get(price_key) {
            Some(price_feed) => price_feed,
            None => {
                error!("price_feeds for {} is None", price_key);
                return;
            }
        };
        if let Err(err) = db::client::put_price_feed(
            conn,
            &price_feed.token_mint,
            &price_feed.price_account,
            price_feed.token_decimals as i16,
            price,
            scraped_at,
        ) {
            error!("failed to put price feed update for {}: {:#?}", price_key, err);
        }
    });
}
