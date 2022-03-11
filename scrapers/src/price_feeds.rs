use chrono::{DateTime, Utc};

use config::Configuration;
use diesel::PgConnection;
use log::error;

use solana_account_decoder::UiAccountEncoding;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcAccountInfoConfig};
use diesel::Connection;
use std::str::FromStr;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use spl_token::state::Mint;
use config::analytics::PriceFeed;
use std::collections::HashMap;

use std::sync::Arc;

pub fn scrape_price_feeds(
    rpc: &Arc<RpcClient>,
    conn: &PgConnection,
    price_feeds: &Arc<HashMap<Pubkey, PriceFeed>>,
    scraped_at: DateTime<Utc>,
) {

    let price_feeds_len = price_feeds.len();

    let mut price_accounts = Vec::with_capacity(price_feeds_len);
    let mut price_account_names = Vec::with_capacity(price_feeds_len);

    for price_feed in price_feeds.iter() {
        price_accounts.push(*price_feed.0);
        price_account_names.push(price_feed.1.clone());
    }
    let mut price_account_datas = match rpc.get_multiple_accounts_with_config(
        &price_accounts[..],
        RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64Zstd),
            data_slice: None,
            commitment: None,
        },
    ) {
        Ok(account_datas) => account_datas.value,
        Err(err) => {
            error!("failed to fetch multiple token accounts {:#?}", err);
            return;
        }
    };
    if price_accounts.len() != price_account_datas.len() {
        error!("mismatching number of token account datas and accounts");
    }

    let mut price_feed_updates = Vec::with_capacity(price_account_names.len());

    for (idx, price_account) in price_account_datas.iter_mut().enumerate() {
        if let Some(price_account) = price_account {
            let price_account = std::mem::take(price_account);
            let price = match crate::pyth::get_pyth_price(&price_account) {
                Ok(price) => price,
                Err(err) => {
                    error!(
                        "failed to fetch pyth price for {}({}): {:#?}",
                        &price_account_names[idx].name, price_accounts[idx], err
                    );
                    continue;
                }
            };
            let price = match f64::from_str(
                &price.to_string(),
            ) {
                Ok(price) => price,
                Err(err) => {
                    error!(
                        "failed to parse price to float {}({}): {:#?}", 
                        price_account_names[idx].name, price_accounts[idx], err
                    );
                    continue;
                }
            };
            price_feed_updates.push(db::client::NewPriceFeed {
                token_mint: price_account_names[idx].token_mint.clone(),
                price_account: price_account_names[idx].price_account.clone(),
                price,
                decimals: price_account_names[idx].token_decimals as i16,
                scraped_at,
            })
        } else {
            error!("price_account {}({}) is None", &price_account_names[idx].name, price_accounts[idx]);
            continue;
        }
    }
    // should we get rid of the transaction??
    match conn.transaction::<_, anyhow::Error, _>(|| {
        for price_feed_update in price_feed_updates.iter() {
            db::client::put_price_feed(
                conn,
                &price_feed_update.token_mint,
                &price_feed_update.price_account,
                price_feed_update.decimals,
                price_feed_update.price,
                price_feed_update.scraped_at,
            )?;
        }
        Ok(())
    }) {
        Ok(_) => (),
        Err(err) => {
            error!("failed to commit transaction {:#?}", err);
        }
    };
}