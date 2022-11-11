use anyhow::{anyhow, Result};

use config::Configuration;

use solana_account_decoder::UiAccountEncoding;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_client::rpc_filter::RpcFilterType;
use solana_sdk::account::Account;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::system_program;

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tulipv2_sdk_common::lending::lending_obligation::{
    pseudo_refresh_lending_obligation, LendingObligation,
};
use tulipv2_sdk_common::lending::reserve::Reserve;
pub const LENDING_OBLIGATION_SIZE: usize = LendingObligation::LEN;

/// fetches all lending obligations, and refreshes the lending obligation state locally.
pub async fn scrape_lending_obligations(
    config: &Arc<Configuration>,
    rpc: &Arc<RpcClient>,
    reserve_account_map: HashMap<Pubkey, Reserve>,
) -> Result<Vec<(Pubkey, Account, f64)>> {
    //let reserve_account_map = match config.get_reserve_infos(rpc, reserve_map) {
    //    Ok(reserve_account_map) => reserve_account_map,
    //    Err(err) => {
    //        return Err(anyhow!("failed to load reserve accounts {:#?}", err));
    //    }
    //};

    let accounts = match rpc
        .get_program_accounts_with_config(
            &config.programs.lending_id(),
            solana_client::rpc_config::RpcProgramAccountsConfig {
                filters: Some(vec![RpcFilterType::DataSize(
                    LENDING_OBLIGATION_SIZE as u64,
                )]),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .await
    {
        Ok(accounts) => accounts,
        Err(err) => {
            return Err(anyhow!("failed to retrieve lending obligations {:#?}", err));
        }
    };
    Ok(accounts
        .into_iter()
        .filter_map(|(account_address, account_info)| {
            if let Ok(mut lending_obligation) = LendingObligation::unpack(&account_info.data[..]) {
                if lending_obligation.owner.ne(&system_program::id()) {
                    if let Err(err) = pseudo_refresh_lending_obligation(
                        &mut lending_obligation,
                        &reserve_account_map,
                    ) {
                        log::error!(
                            "failed to refresh lending obligation {} account_address {:#?}",
                            account_address,
                            err
                        );
                        // todo: should we return Some here even if the pseudo refresh failed?
                        return None;
                    }
                    let ltv = match lending_obligation.loan_to_value() {
                        Ok(ltv) => match f64::from_str(&ltv.to_string()) {
                            Ok(ltv) => ltv,
                            Err(err) => {
                                log::error!(
                                    "failed to parse ltv for {}: {:#?}",
                                    account_address,
                                    err
                                );
                                return None;
                            }
                        },
                        Err(err) => {
                            log::error!("failed to parse ltv for {}: {:#?}", account_address, err);
                            return None;
                        }
                    };
                    return Some((account_address, account_info, ltv));
                }
            }
            None
        })
        .collect::<Vec<_>>())
}
