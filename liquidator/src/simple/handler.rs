use super::*;
use crate::MIN_LTV;
use db::models::Obligation;
use solana_sdk::{program_pack::Pack, transaction::Transaction};
use std::{cmp::Ordering, str::FromStr};
use tulipv2_sdk_common::lending::{
    lending_obligation::{pseudo_refresh_lending_obligation, LendingObligation},
    reserve::Reserve,
};
use tulipv2_sdk_common::math::decimal::Decimal;
impl SimpleLiquidator {
    pub fn handle_liquidation_check(self: &Arc<Self>, obligation: &Obligation) -> Result<()> {
        let payer = self.cfg.payer_signer(None)?;
        let payer_pubkey = payer.pubkey();
        let obligation_key = obligation.account;
        let obligation_account_data = self.rpc.get_account_data(&obligation_key)?;
        let mut obligation_account =
            LendingObligation::unpack_unchecked(&obligation_account_data[..])?;

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
        let mut reserve_accounts =
            Vec::with_capacity(liquidity_reserves.len() + borrow_reserves.len());
        reserve_accounts.extend_from_slice(&liquidity_reserves[..]);
        reserve_accounts.extend_from_slice(&borrow_reserves[..]);
        reserve_accounts.sort_unstable();
        reserve_accounts.dedup();

        // fetch all reserve accounts in bulk
        let mut reserve_account_infos = match self.rpc.get_multiple_accounts_with_config(
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
                    obligation.account,
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

        // check the current ltv to see if we need to liquidate
        let ltv = obligation_account.loan_to_value()?;
        match ltv.cmp(&MIN_LTV) {
            Ordering::Greater | Ordering::Equal => {
                info!(
                    "found obligation({}) with ltv {}, liquidating...",
                    obligation_key, ltv,
                );
            }
            _ => return Ok(()),
        }

        // fetch the obligation's borrow and deposits information
        // sorting based on market value
        let mut obligation_borrows = obligation_account.borrows.clone();
        let mut obligation_deposits = obligation_account.deposits.clone();
        obligation_borrows.sort_unstable_by_key(|borrow| borrow.market_value);
        obligation_deposits.sort_unstable_by_key(|deposit| deposit.market_value);

        for (borrow, deposit) in obligation_borrows.iter().zip(obligation_deposits) {
            let borrow_reserve = match reserves.get(&borrow.borrow_reserve) {
                Some(borrow_reserve) => borrow_reserve,
                None => {
                    // todo(bonedaddy): is this the right way to handle this?
                    error!("failed to find borrow_reserve {}", borrow.borrow_reserve);
                    continue;
                }
            };

            let deposit_reserve = match reserves.get(&deposit.deposit_reserve) {
                Some(deposit_reserve) => deposit_reserve,
                None => {
                    // todo(bonedaddy): is this the right way to handle this?
                    error!("failed to find deposit_reserve {}", deposit.deposit_reserve);
                    continue;
                }
            };

            let source_liquidity_token_account =
                spl_associated_token_account::get_associated_token_address(
                    &payer_pubkey,
                    &borrow_reserve.liquidity.mint_pubkey,
                );

            // use the available balance as the amount to repay if less than borrowed value,
            // otherwise if greater use u64::MAX
            let liquidity_balance = match self
                .rpc
                .get_token_account_balance(&source_liquidity_token_account)
            {
                Ok(balance) => match u64::from_str(&balance.amount) {
                    Ok(amount) => Decimal::from(amount),
                    Err(err) => {
                        error!("failed to fetch token account balance {:#?}", err);
                        continue;
                    }
                },
                Err(err) => {
                    error!("failed to fetch token account balance {:#?}", err);
                    continue;
                }
            };
            let amount_to_repay = match borrow.borrowed_amount_wads.cmp(&liquidity_balance) {
                Ordering::Greater => u64::MAX,
                Ordering::Equal | Ordering::Less => match liquidity_balance.try_floor_u64() {
                    Ok(balance) => balance,
                    Err(err) => {
                        error!("failed to floor liquidity_balance {:#?}", err);
                        continue;
                    }
                },
            };

            // 1 ix for refreshing each reserve (2)
            // 1 ix for refreshing obligation (3)
            // 1 ix for liquidation (4)
            let mut liq_instructions = Vec::with_capacity(4);

            // if deposit & borrow are same, no need for multiple reserve refresh sintructions
            if borrow.borrow_reserve.eq(&deposit.deposit_reserve) {
                liq_instructions.push(crate::instructions::new_refresh_reserve_ix(
                    borrow.borrow_reserve,
                    borrow_reserve.liquidity.oracle_pubkey,
                ));
            } else {
                liq_instructions.push(crate::instructions::new_refresh_reserve_ix(
                    borrow.borrow_reserve,
                    borrow_reserve.liquidity.oracle_pubkey,
                ));
                liq_instructions.push(crate::instructions::new_refresh_reserve_ix(
                    deposit.deposit_reserve,
                    deposit_reserve.liquidity.oracle_pubkey,
                ));
            }
            liq_instructions.push(crate::instructions::new_refresh_lending_obligation_ix(
                obligation_key,
                &liquidity_reserves[..],
                &borrow_reserves[..],
            ));
            liq_instructions.push(crate::instructions::new_liquidate_lending_obligation_ix(
                spl_associated_token_account::get_associated_token_address(
                    &payer_pubkey,
                    &borrow_reserve.liquidity.mint_pubkey,
                ),
                spl_associated_token_account::get_associated_token_address(
                    &payer_pubkey,
                    &deposit_reserve.liquidity.mint_pubkey,
                ),
                borrow.borrow_reserve,
                borrow_reserve.liquidity.supply_pubkey,
                deposit.deposit_reserve,
                deposit_reserve.liquidity.supply_pubkey,
                obligation_key,
                borrow_reserve.lending_market,
                Pubkey::find_program_address(
                    &[borrow_reserve.lending_market.as_ref()],
                    &crate::instructions::LENDING_PROGRAM_ID,
                )
                .0,
                payer_pubkey,
                amount_to_repay,
            ));
            let mut tx = Transaction::new_with_payer(&liq_instructions[..], Some(&payer_pubkey));
            let blockhash = self.rpc.get_latest_blockhash()?;
            tx.sign(&vec![&*payer], blockhash);
            info!(
                "sending liquidation obligation {} tx. deposit_reserve {}, borrow_reserve {}",
                obligation_key, deposit.deposit_reserve, borrow.borrow_reserve
            );
            match self.rpc.send_and_confirm_transaction(&tx) {
                Ok(sig) => info!(
                    "sent liquidation obligation {} tx {}. deposit_reserve {}, borrow_reserve {}",
                    obligation_key, sig, deposit.deposit_reserve, borrow.borrow_reserve
                ),
                Err(err) => error!(
                    "failed to send liquidate obligation {} tx {:#?}. deposit_reserve {}, borrow_reserve {}",
                    obligation_key, err, deposit.deposit_reserve, borrow.borrow_reserve,
                ),
            }
        }
        Ok(())
    }
}
