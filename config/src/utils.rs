use super::*;
use std::{collections::HashMap, sync::Arc, borrow::Borrow};
use anchor_lang::prelude::*;
use anchor_client::solana_client::rpc_client::RpcClient;
use solana_account_decoder::UiAccountEncoding;
use tulipv2_sdk_common::lending::reserve::Reserve;
use anyhow::{Result, anyhow};
use log::error;
use solana_sdk::program_pack::Pack;

impl Configuration {
    /// from a HashMap of reserve accounts, retrieve all reserve account data
    /// and return a ReserveInfos struct
    pub fn get_reserve_infos(
        &self,
        rpc: &Arc<RpcClient>,
        account_key_map: &HashMap<Pubkey, String>,
    ) -> Result<HashMap<Pubkey, Reserve>> {
        let account_keys: Vec<Pubkey> = account_key_map
        .into_iter()
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
            return Err(
                anyhow!(
                    "mismatched reserve_accounts_len({}) and accounts_keys_len({})",
                    reserve_accounts.len(), account_keys.len(),
                )
            );
        }

        let mut reserve_map = HashMap::with_capacity(account_keys.len());
        reserve_accounts
            .into_iter()
            .zip(account_keys)
            .for_each(|(reserve_account, reserve_key)| {
            let reserve_account = match reserve_account {
                Some(account) => account,
                None => {
                    error!("reserve {} is None", reserve_key);
                    return;
                }
            };
            let reserve = match Reserve::unpack_unchecked(
                reserve_account.data.borrow()
            ) {
                Ok(reserve) => reserve,
                Err(err) => {
                    error!("failed to unpack reserve {}: {:#?}", reserve_key, err);
                    return;
                }
            };

            reserve_map.insert(reserve_key, reserve);
        });

        Ok(reserve_map)
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
    fn test_get_reserve_infos() {
        let cfg = Configuration {
            rpc_endpoints: RPCs { 
                failover_endpoints: vec![], 
                primary_endpoint: RPCEndpoint { 
                    http_url: "https://ssc-dao.genesysgo.net".to_string(), 
                    ws_url: "".to_string()
                },
            },
            analytics: Analytics {
                reserves: vec![
                    analytics::Reserve { 
                        name: "tulip".to_string(), 
                        account: "DdFHZu9n41MuH2dNJMgXpnnmuefxDMdUbCp4iizPfz1o".to_string()
                    },
                    analytics::Reserve {
                        name: "usdc".to_string(),
                        account: "FTkSmGsJ3ZqDSHdcnY7ejN1pWV3Ej7i88MYpZyyaqgGt".to_string()
                    }
                ],
                ..Default::default()
            },
            ..Default::default()
        };
        let rpc = Arc::new(cfg.get_rpc_client(false, None));
        let reserve_map = cfg.analytics.get_reserve_map();
        let reserve_account_map = cfg.get_reserve_infos(&rpc, &reserve_map).unwrap();
        assert_eq!(reserve_map.len(), reserve_account_map.len());
    }
}