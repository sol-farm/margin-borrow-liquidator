use chrono::{DateTime, Utc};

use config::Configuration;
use db::models::PriceFeed;
use db::LiquidatorDb;
use log::error;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tulipv2_sdk_common::pyth;

/// scrapes the given price accounts for the quoted price
/// caching the value into the database.
pub async fn scrape_price_feeds(
    config: &Arc<Configuration>,
    rpc: &Arc<RpcClient>,
    db: &Arc<LiquidatorDb>,
    price_accounts: &Arc<HashMap<Pubkey, String>>,
    scraped_at: DateTime<Utc>,
) {
    let price_feeds = config.analytics.price_feed_map();

    let mut price_feed_accounts = match config.get_price_feeds(rpc, price_accounts).await {
        Ok(price_feed_accounts) => price_feed_accounts,
        Err(err) => {
            error!("failed to retrieve price feed accounts {:#?}", err);
            return;
        }
    };
    price_feed_accounts
        .iter_mut()
        .for_each(|(price_key, price_account)| {
            // take the value, replacing it with a default Price object
            // which will help to free up a small bit of memory related to this
            // price_account once each closure completes
            //
            // this is also more cost effective than cloning each price account
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
            if let Err(err) = db.insert_price_feeds(&[PriceFeed {
                token_mint: price_feed.token_mint(),
                price_account: price_feed.price_account(),
                scraped_at,
                price,
                decimals: price_feed.token_decimals as i16,
            }]) {
                log::error!("failed to update price feed {:#?}", err);
            }
        });
}

#[cfg(test)]
mod test {
    use super::*;
    use bonerjams_config::database::DbOpts;
    use config::analytics::{Analytics, PriceFeed, Reserve};
    use config::rpcs::{RPCEndpoint, RPCs};
    #[test]
    #[allow(unused_must_use)]
    fn test_scrape_price_feeds() {
        {
            let cfg = Configuration {
                rpc_endpoints: RPCs {
                    failover_endpoints: vec![],
                    primary_endpoint: RPCEndpoint {
                        http_url: "https://solana-api.projectserum.com".to_string(),
                        ws_url: "".to_string(),
                    },
                },
                analytics: Analytics {
                    reserves: vec![
                        Reserve {
                            name: "tulip".to_string(),
                            account: "DdFHZu9n41MuH2dNJMgXpnnmuefxDMdUbCp4iizPfz1o".to_string(),
                        },
                        Reserve {
                            name: "usdc".to_string(),
                            account: "FTkSmGsJ3ZqDSHdcnY7ejN1pWV3Ej7i88MYpZyyaqgGt".to_string(),
                        },
                    ],
                    price_feeds: vec![PriceFeed {
                        name: "tulip".to_string(),
                        price_account: "5RHxy1NbUR15y34uktDbN1a2SWbhgHwkCZ75yK2RJ1FC".to_string(),
                        token_decimals: 6,
                        token_mint: "TuLipcqtGVXP9XR62wM8WWCm6a9vhLs7T1uoWBk6FDs".to_string(),
                        quote_decimals: -6,
                    }],
                    ..Default::default()
                },
                sled_db: bonerjams_config::Configuration {
                    db: DbOpts {
                        path: "scrapers_price_feed_test.db".to_string(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            };
            let rpc = Arc::new(cfg.get_rpc_client(false, None));
            let price_accounts = Arc::new(cfg.analytics.price_account_map());
            let scraped_at = Utc::now();
            let db = Arc::new(db::LiquidatorDb::new(cfg.clone()).unwrap());
            scrape_price_feeds(&Arc::new(cfg), &rpc, &db, &price_accounts, scraped_at);

            let price_feeds = db.list_price_feeds().unwrap();

            assert_eq!(price_feeds.len(), 1);
            assert_eq!(
                price_feeds[0].token_mint.to_string(),
                "TuLipcqtGVXP9XR62wM8WWCm6a9vhLs7T1uoWBk6FDs"
            );
            if price_feeds[0].price <= 0_f64 {
                panic!("fuck");
            }
        }

        std::fs::remove_dir_all("scrapers_price_feed_test.db").unwrap();
    }
}
