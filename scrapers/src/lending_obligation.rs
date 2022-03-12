use chrono::{DateTime, Utc};

use diesel::PgConnection;
use log::error;
use config::Configuration;
use config::analytics::PriceFeed;
use diesel::Connection;
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_filter::RpcFilterType;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcAccountInfoConfig};
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use tulipv2_sdk_common::lending::lending_obligation::{LendingObligation};
use std::sync::Arc;
use std::borrow::Borrow;
pub const LENDING_OBLIGATION_SIZE: usize = LendingObligation::LEN;

pub fn scrape_lending_obligations(
    config: &Arc<Configuration>,
    rpc: &Arc<RpcClient>,
    conn: &PgConnection,
    scraped_at: DateTime<Utc>,
)  {
    let accounts = match rpc.get_program_accounts_with_config(
        &config.programs.lending_id(),
        solana_client::rpc_config::RpcProgramAccountsConfig { 
            filters: Some(vec![
                RpcFilterType::DataSize(LENDING_OBLIGATION_SIZE as u64),
            ]),
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
        let lending_obligation_account = match LendingObligation::unpack_unchecked(account.1.data.borrow()) {
            Ok(acct) => acct,
            Err(err) => {
                error!("failed to unpack lending obligation {}: {:#?}", account.0, err);
                continue;
            }
        };
        let ltv = match lending_obligation_account.loan_to_value() {
            Ok(ltv) => ltv,
            Err(err) => {
                error!("failed to calculate ltv for {}: {:#?}", account.0, err);
                continue;
            }
        };
    }
}
