use chrono::{DateTime, Utc};

use config::Configuration;
use diesel::PgConnection;
use log::error;

use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_filter::RpcFilterType;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcAccountInfoConfig};
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tulipv2_sdk_common::lending::lending_obligation::{
    pseudo_refresh_lending_obligation, LendingObligation,
};
pub const LENDING_OBLIGATION_SIZE: usize = LendingObligation::LEN;

pub fn scrape_lending_obligations(
    config: &Arc<Configuration>,
    rpc: &Arc<RpcClient>,
    conn: &PgConnection,
    reserve_map: &Arc<HashMap<Pubkey, String>>,
    scraped_at: DateTime<Utc>,
) {
    let reserve_account_map = match config.get_reserve_infos(rpc, reserve_map) {
        Ok(reserve_account_map) => reserve_account_map,
        Err(err) => {
            error!("failed to load reserve accounts {:#?}", err);
            return;
        }
    };

    let accounts = match rpc.get_program_accounts_with_config(
        &config.programs.lending_id(),
        solana_client::rpc_config::RpcProgramAccountsConfig {
            filters: Some(vec![RpcFilterType::DataSize(
                LENDING_OBLIGATION_SIZE as u64,
            )]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64Zstd),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(accounts) => accounts,
        Err(err) => {
            error!("failed to retrieve lending obligations {:#?}", err);
            return;
        }
    };
    // todo(bonedaddy): should we use transactions?
    for account in accounts.iter() {
        let account_data = account.1.data.borrow();
        let mut lending_obligation = match LendingObligation::unpack_unchecked(account_data) {
            Ok(acct) => acct,
            Err(err) => {
                error!(
                    "failed to unpack lending obligation {}: {:#?}",
                    account.0, err
                );
                continue;
            }
        };
        match pseudo_refresh_lending_obligation(&mut lending_obligation, &reserve_account_map) {
            Ok(_) => (),
            Err(err) => {
                error!(
                    "failed to pseudo refresh obligation {}: {:#?}",
                    account.0, err
                );
            }
        }
        let ltv = match lending_obligation.loan_to_value() {
            Ok(ltv) => ltv,
            Err(err) => {
                error!("failed to calculate ltv for {}: {:#?}", account.0, err);
                continue;
            }
        };
        let ltv = match f64::from_str(&ltv.to_string()) {
            Ok(ltv) => ltv,
            Err(err) => {
                error!("failed to parse ltv to string {}: {:#?}", account.0, err);
                continue;
            }
        };
        match db::client::put_obligation(conn, ltv, &account.0.to_string(), scraped_at) {
            Ok(_) => (),
            Err(err) => {
                error!("failed to put obligation update {}: {:#?}", account.0, err);
            }
        }
    }
}
