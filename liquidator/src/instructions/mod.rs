use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use anchor_lang::solana_program::sysvar;
use solana_sdk::instruction::Instruction;
use static_pubkey::static_pubkey;

pub const LENDING_PROGRAM_ID: Pubkey =
    static_pubkey!("4bcFeLv4nydFrsZqV5CgwCVrPhkQKsXtzfy2KyMz7ozM");

/// returns a new instruction used to refresh the given lending obligation
pub fn new_refresh_lending_obligation_ix(
    obligation: Pubkey,
    collateral_deposit_reserves: &[Pubkey],
    liquidity_borrow_reserves: &[Pubkey],
) -> Instruction {
    let mut collateral_deposit_reserves: Vec<AccountMeta> = collateral_deposit_reserves
        .iter()
        .map(|deposit| AccountMeta::new_readonly(*deposit, false))
        .collect();
    let mut liquidity_borrow_reserves: Vec<AccountMeta> = liquidity_borrow_reserves
        .iter()
        .map(|borrow| AccountMeta::new_readonly(*borrow, false))
        .collect();

    let mut accounts = vec![
        AccountMeta::new(obligation, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];
    accounts.append(&mut collateral_deposit_reserves);
    accounts.append(&mut liquidity_borrow_reserves);

    Instruction {
        program_id: LENDING_PROGRAM_ID,
        accounts,
        data: vec![24],
    }
}

/// returns a new instruction used to refresh the given reserve
pub fn new_refresh_reserve_ix(
    reserve_account: Pubkey,
    reserve_liquidity_oracle: Pubkey,
) -> Instruction {
    Instruction {
        program_id: LENDING_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(reserve_account, false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(reserve_liquidity_oracle, false),
        ],
        data: vec![3],
    }
}

/// returns a new instruction used to liquidity a positions unhealthy collateral
/// repay reserve is the funds being returned that were loaned, while the withdraw reserve
/// is the reserve for the collateral which was deposited.
///
/// the amount specified is the amount of borrowed liquidity to repay with
/// u64::MAX indicating to repay 100% of the borrowed amount. you may only close out
/// up to 20% of a given positions unhealthy debt, which is automatically enforced onchain
pub fn new_liquidate_lending_obligation_ix(
    source_liquidity_token_account: Pubkey,
    destination_collateral_token_account: Pubkey,
    repay_reserve_account: Pubkey,
    repay_reserve_liquidity_supply_token_account: Pubkey,
    withdraw_reserve_account: Pubkey,
    withdraw_reserve_collateral_token_account: Pubkey,
    obligation: Pubkey,
    lending_market: Pubkey,
    derived_lending_market_authority: Pubkey,
    // the main signer / caller
    authority: Pubkey,
    amount: u64,
) -> Instruction {
    let mut data = vec![29];
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: LENDING_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(source_liquidity_token_account, false),
            AccountMeta::new(destination_collateral_token_account, false),
            AccountMeta::new(repay_reserve_account, false),
            AccountMeta::new(repay_reserve_liquidity_supply_token_account, false),
            AccountMeta::new_readonly(withdraw_reserve_account, false),
            AccountMeta::new(withdraw_reserve_collateral_token_account, false),
            AccountMeta::new(obligation, false),
            AccountMeta::new_readonly(lending_market, false),
            AccountMeta::new_readonly(derived_lending_market_authority, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    }
}
