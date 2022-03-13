use super::*;
use anchor_client::solana_client::rpc_client::RpcClient;
use anchor_lang::prelude::*;
use anyhow::{anyhow, Result};
use log::error;
use solana_account_decoder::UiAccountEncoding;
use solana_sdk::program_pack::Pack;
use std::{borrow::Borrow, collections::HashMap, sync::Arc};
use tulipv2_sdk_common::{
    lending::reserve::Reserve,
    pyth::{load as pyth_load, Price},
};

impl Configuration {
    pub fn get_reserve_infos(
        &self,
        rpc: &Arc<RpcClient>,
        account_key_map: &Arc<HashMap<Pubkey, String>>,
    ) -> Result<HashMap<Pubkey, Reserve>> {
        let account_keys: Vec<Pubkey> = account_key_map
            .iter()
            .map(|account_key| *account_key.0)
            .collect();

        let reserve_accounts = match rpc.get_multiple_accounts_with_config(
            &account_keys[..],
            solana_client::rpc_config::RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64Zstd),
                ..Default::default()
            },
        ) {
            Ok(accounts) => accounts.value,
            Err(err) => {
                return Err(anyhow!("failed to retrieve reserve accounts {:#?}", err));
            }
        };

        if reserve_accounts.len() != account_keys.len() {
            return Err(anyhow!(
                "mismatched reserve_accounts_len({}) and accounts_keys_len({})",
                reserve_accounts.len(),
                account_keys.len(),
            ));
        }

        let mut reserve_map = HashMap::with_capacity(account_keys.len());
        reserve_accounts.into_iter().zip(account_keys).for_each(
            |(reserve_account, reserve_key)| {
                let reserve_account = match reserve_account {
                    Some(account) => account,
                    None => {
                        error!("reserve {} is None", reserve_key);
                        return;
                    }
                };
                let reserve = match Reserve::unpack_unchecked(reserve_account.data.borrow()) {
                    Ok(reserve) => reserve,
                    Err(err) => {
                        error!("failed to unpack reserve {}: {:#?}", reserve_key, err);
                        return;
                    }
                };

                reserve_map.insert(reserve_key, reserve);
            },
        );

        Ok(reserve_map)
    }
    pub fn get_price_feeds(
        &self,
        rpc: &Arc<RpcClient>,
        account_key_map: &Arc<HashMap<Pubkey, String>>,
    ) -> Result<HashMap<Pubkey, Price>> {
        let account_keys: Vec<Pubkey> = account_key_map
            .iter()
            .map(|account_key| *account_key.0)
            .collect();

        let price_feed_accounts = match rpc.get_multiple_accounts_with_config(
            &account_keys[..],
            solana_client::rpc_config::RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64Zstd),
                ..Default::default()
            },
        ) {
            Ok(accounts) => accounts.value,
            Err(err) => {
                return Err(anyhow!("failed to retrieve reserve accounts {:#?}", err));
            }
        };

        if price_feed_accounts.len() != account_keys.len() {
            return Err(anyhow!(
                "mismatched price_feed_accounts({}) and accounts_keys_len({})",
                price_feed_accounts.len(),
                account_keys.len(),
            ));
        }

        let mut price_feed_map = HashMap::with_capacity(account_keys.len());
        price_feed_accounts.into_iter().zip(account_keys).for_each(
            |(price_feed_account, price_feed_key)| {
                let price_feed_account = match price_feed_account {
                    Some(account) => account,
                    None => {
                        error!("price_feed {} is None", price_feed_key);
                        return;
                    }
                };
                let price_feed = match pyth_load::<Price>(price_feed_account.data.borrow()) {
                    Ok(price_feed) => price_feed,
                    Err(err) => {
                        error!("failed to load price feed {}: {:#?}", price_feed_key, err);
                        return;
                    }
                };
                price_feed_map.insert(price_feed_key, *price_feed);
            },
        );

        Ok(price_feed_map)
    }
}

pub fn generate_random_number(min: i64, max: i64) -> i64 {
    use rand::prelude::*;
    let mut rng = rand::thread_rng();
    rng.gen_range(min, max)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_reserve_info_price_feed_helpers() {
        let cfg = Configuration {
            rpc_endpoints: RPCs {
                failover_endpoints: vec![],
                primary_endpoint: RPCEndpoint {
                    http_url: "https://ssc-dao.genesysgo.net".to_string(),
                    ws_url: "".to_string(),
                },
            },
            analytics: Analytics {
                reserves: vec![
                    analytics::Reserve {
                        name: "tulip".to_string(),
                        account: "DdFHZu9n41MuH2dNJMgXpnnmuefxDMdUbCp4iizPfz1o".to_string(),
                    },
                    analytics::Reserve {
                        name: "usdc".to_string(),
                        account: "FTkSmGsJ3ZqDSHdcnY7ejN1pWV3Ej7i88MYpZyyaqgGt".to_string(),
                    },
                ],
                price_feeds: vec![analytics::PriceFeed {
                    name: "tulip".to_string(),
                    price_account: "5RHxy1NbUR15y34uktDbN1a2SWbhgHwkCZ75yK2RJ1FC".to_string(),
                    token_decimals: 6,
                    token_mint: "TuLipcqtGVXP9XR62wM8WWCm6a9vhLs7T1uoWBk6FDs".to_string(),
                    quote_decimals: -6,
                }],
                ..Default::default()
            },
            ..Default::default()
        };
        let rpc = Arc::new(cfg.get_rpc_client(false, None));
        let reserve_map = Arc::new(cfg.analytics.reserve_map());
        let price_feed_map = Arc::new(cfg.analytics.price_account_map());
        let reserve_account_map = cfg.get_reserve_infos(&rpc, &reserve_map).unwrap();
        let price_feed_account_map = cfg.get_price_feeds(&rpc, &price_feed_map).unwrap();
        assert_eq!(reserve_map.len(), reserve_account_map.len());
        assert_eq!(price_feed_account_map.len(), price_feed_map.len());
    }
}
