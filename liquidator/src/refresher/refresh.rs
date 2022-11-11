//! standalone refresh helper function

use anyhow::{anyhow, Result};
use db::models::Obligation as DbObligation;
use log::error;
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_client::RpcClient;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::{collections::HashMap, sync::Arc};
use tulipv2_sdk_common::lending::{
    lending_obligation::{pseudo_refresh_lending_obligation, LendingObligation},
    reserve::Reserve,
};

/// given a database Obligation record, automatically fetch all accounts
/// required to preform a pseudo refresh, returning the refreshed oblgiation
pub fn handle_pseudo_obligation_refresh(
    rpc: &Arc<RpcClient>,
    db_obligation: &DbObligation,
) -> Result<LendingObligation> {
    let obligation_key = db_obligation.account;
    let obligation_account_data = rpc.get_account_data(&obligation_key)?;
    let mut obligation_account = LendingObligation::unpack_unchecked(&obligation_account_data[..])?;

    // contains all reserves used as collateral
    let liquidity_reserves: Vec<Pubkey> = obligation_account
        .deposits
        .iter()
        .map(|deposit| deposit.deposit_reserve)
        .collect();
    // contains all reserves which were used to borrow liquidity
    let borrow_reserves: Vec<Pubkey> = obligation_account
        .borrows
        .iter()
        .map(|borrow| borrow.borrow_reserve)
        .collect();

    // contains all reserves, deduped
    let mut reserve_accounts = Vec::with_capacity(liquidity_reserves.len() + borrow_reserves.len());
    reserve_accounts.extend_from_slice(&liquidity_reserves[..]);
    reserve_accounts.extend_from_slice(&borrow_reserves[..]);
    reserve_accounts.sort_unstable();
    reserve_accounts.dedup();

    // fetch all reserve accounts in bulk
    let mut reserve_account_infos = match rpc.get_multiple_accounts_with_config(
        &reserve_accounts[..],
        solana_client::rpc_config::RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64Zstd),
            ..Default::default()
        },
    ) {
        Ok(response) => response.value,
        Err(err) => {
            return Err(anyhow!(
                "failed to fetch reserves for {}: {:#?}",
                db_obligation.account,
                err
            ))
        }
    };

    // create a hashmap indexing all reserves by their reserve account address
    let mut reserves = HashMap::with_capacity(reserve_accounts.len());
    for (idx, reserve_info) in reserve_account_infos.iter_mut().enumerate() {
        if let Some(reserve_info) = std::mem::take(reserve_info) {
            match Reserve::unpack_unchecked(&reserve_info.data[..]) {
                Ok(reserve_info) => {
                    reserves.insert(reserve_accounts[idx], reserve_info);
                }
                Err(err) => {
                    error!("failed to unpack reserve {:#?}", err);
                    continue;
                }
            }
        } else {
            error!("found None reserve");
            continue;
        }
    }

    // perform a pseudo reserve refresh to estiamte the current ltv
    if let Err(err) = pseudo_refresh_lending_obligation(&mut obligation_account, &reserves) {
        return Err(anyhow!(
            "pseudo refresh failed for {}: {:#?}",
            obligation_key,
            err
        ));
    };

    Ok(obligation_account)
}
