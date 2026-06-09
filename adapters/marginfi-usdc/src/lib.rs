use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

declare_id!("MAdPtMaRGInFiUSDCXXXXXXXXXXXXXXXXXXXXXXXXXXX");

pub const MARGINFI_PROGRAM_ID: Pubkey = pubkey!("MFv2hWf31Z9kbCa1snEPdcgX6CAm7cDpSvtVkGX3H5a");
pub const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const MARGINFI_USDC_BANK: Pubkey = pubkey!("2s37akK2eyBbp8DZgCm7RtsaEz8eJP3Nxd4urLHQv7yB");
pub const SHARE_VALUE_SCALE: u128 = 1_000_000_000_000;

#[program]
pub mod marginfi_usdc_adapter {
    use super::*;

    pub fn adapter_deposit(ctx: Context<MarginFiDeposit>, amount: u64, min_shares_out: u64) -> Result<u64> {
        let clock = Clock::get()?;
        let asset_share_value = read_marginfi_asset_share_value(&ctx.accounts.usdc_bank)?;

        let shares_out = (amount as u128)
            .checked_mul(SHARE_VALUE_SCALE)
            .ok_or(AdapterError::MathOverflow)?
            .checked_div(asset_share_value)
            .ok_or(AdapterError::MathOverflow)? as u64;

        require!(shares_out >= min_shares_out, AdapterError::SlippageExceeded);

        invoke_marginfi_deposit(&ctx, amount)?;

        let mut return_data = shares_out.to_le_bytes().to_vec();
        return_data.extend_from_slice(&asset_share_value.to_le_bytes());
        anchor_lang::solana_program::program::set_return_data(&return_data);

        emit!(MarginFiDepositEvent {
            amount_in: amount,
            shares_out,
            asset_share_value,
            slot: clock.slot,
        });

        msg!("MarginFi deposit: {} USDC -> {} shares", amount, shares_out);
        Ok(shares_out)
    }

    pub fn adapter_withdraw(ctx: Context<MarginFiWithdraw>, shares: u64, min_amount_out: u64) -> Result<u64> {
        let clock = Clock::get()?;
        let asset_share_value = read_marginfi_asset_share_value(&ctx.accounts.usdc_bank)?;

        let amount_out = (shares as u128)
            .checked_mul(asset_share_value)
            .ok_or(AdapterError::MathOverflow)?
            .checked_div(SHARE_VALUE_SCALE)
            .ok_or(AdapterError::MathOverflow)? as u64;

        require!(amount_out >= min_amount_out, AdapterError::SlippageExceeded);

        invoke_marginfi_withdraw(&ctx, shares, amount_out)?;

        let mut return_data = amount_out.to_le_bytes().to_vec();
        return_data.extend_from_slice(&asset_share_value.to_le_bytes());
        anchor_lang::solana_program::program::set_return_data(&return_data);

        emit!(MarginFiWithdrawEvent {
            shares_in: shares,
            amount_out,
            asset_share_value,
            slot: clock.slot,
        });

        msg!("MarginFi withdraw: {} shares -> {} USDC", shares, amount_out);
        Ok(amount_out)
    }

    pub fn adapter_current_value(ctx: Context<MarginFiCurrentValue>, shares: u64) -> Result<u64> {
        let asset_share_value = read_marginfi_asset_share_value(&ctx.accounts.usdc_bank)?;

        let value = (shares as u128)
            .checked_mul(asset_share_value)
            .ok_or(AdapterError::MathOverflow)?
            .checked_div(SHARE_VALUE_SCALE)
            .ok_or(AdapterError::MathOverflow)? as u64;

        let mut return_data = value.to_le_bytes().to_vec();
        return_data.extend_from_slice(&asset_share_value.to_le_bytes());
        anchor_lang::solana_program::program::set_return_data(&return_data);

        Ok(value)
    }
}

#[derive(Accounts)]
pub struct MarginFiDeposit<'info> {
    #[account(mut, constraint = adapter_vault.mint == USDC_MINT @ AdapterError::InvalidMint)]
    pub adapter_vault: Account<'info, TokenAccount>,

    /// CHECK: MarginFi account PDA
    #[account(mut)]
    pub marginfi_account: AccountInfo<'info>,

    /// CHECK: MarginFi group
    #[account(mut)]
    pub marginfi_group: AccountInfo<'info>,

    /// CHECK: USDC Bank
    #[account(mut, address = MARGINFI_USDC_BANK @ AdapterError::InvalidBank)]
    pub usdc_bank: AccountInfo<'info>,

    /// CHECK: Bank liquidity vault
    #[account(mut)]
    pub bank_liquidity_vault: AccountInfo<'info>,

    /// CHECK: Dispatcher authority
    #[account(signer)]
    pub dispatcher_authority: AccountInfo<'info>,

    /// CHECK: MarginFi program
    #[account(address = MARGINFI_PROGRAM_ID)]
    pub marginfi_program: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct MarginFiWithdraw<'info> {
    #[account(mut, constraint = adapter_vault.mint == USDC_MINT @ AdapterError::InvalidMint)]
    pub adapter_vault: Account<'info, TokenAccount>,

    /// CHECK: MarginFi account
    #[account(mut)]
    pub marginfi_account: AccountInfo<'info>,

    /// CHECK: MarginFi group
    #[account(mut)]
    pub marginfi_group: AccountInfo<'info>,

    /// CHECK: USDC bank
    #[account(mut, address = MARGINFI_USDC_BANK @ AdapterError::InvalidBank)]
    pub usdc_bank: AccountInfo<'info>,

    /// CHECK: Bank liquidity vault
    #[account(mut)]
    pub bank_liquidity_vault: AccountInfo<'info>,

    /// CHECK: Bank liquidity vault authority
    pub bank_liquidity_vault_authority: AccountInfo<'info>,

    /// CHECK: Dispatcher authority
    #[account(signer)]
    pub dispatcher_authority: AccountInfo<'info>,

    /// CHECK: MarginFi program
    #[account(address = MARGINFI_PROGRAM_ID)]
    pub marginfi_program: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct MarginFiCurrentValue<'info> {
    /// CHECK: USDC bank for share value lookup
    #[account(address = MARGINFI_USDC_BANK)]
    pub usdc_bank: AccountInfo<'info>,

    /// CHECK: Dispatcher authority
    pub dispatcher_authority: AccountInfo<'info>,
}

fn read_marginfi_asset_share_value(bank: &AccountInfo) -> Result<u128> {
    let data = bank.try_borrow_data()?;

    if data.len() < 384 {
        return Ok(SHARE_VALUE_SCALE);
    }

    let raw_bytes: [u8; 16] = data[368..384].try_into()
        .map_err(|_| AdapterError::InvalidBankData)?;
    let raw = i128::from_le_bytes(raw_bytes);

    if raw <= 0 {
        return Ok(SHARE_VALUE_SCALE);
    }

    let value = (raw as u128)
        .checked_mul(SHARE_VALUE_SCALE)
        .unwrap_or(SHARE_VALUE_SCALE)
        .checked_div(281_474_976_710_656u128)
        .unwrap_or(SHARE_VALUE_SCALE);

    Ok(if value == 0 { SHARE_VALUE_SCALE } else { value })
}

fn invoke_marginfi_deposit(ctx: &Context<MarginFiDeposit>, amount: u64) -> Result<()> {
    let mut ix_data: Vec<u8> = vec![0x44, 0x3b, 0x22, 0x47, 0x5f, 0x2e, 0x11, 0x2c];
    ix_data.extend_from_slice(&amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(ctx.accounts.marginfi_group.key(), false),
        AccountMeta::new(ctx.accounts.marginfi_account.key(), false),
        AccountMeta::new_readonly(ctx.accounts.dispatcher_authority.key(), true),
        AccountMeta::new(ctx.accounts.usdc_bank.key(), false),
        AccountMeta::new(ctx.accounts.adapter_vault.key(), false),
        AccountMeta::new(ctx.accounts.bank_liquidity_vault.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
    ];

    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.marginfi_program.key(),
        accounts,
        data: ix_data,
    };

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.marginfi_group.to_account_info(),
            ctx.accounts.marginfi_account.to_account_info(),
            ctx.accounts.dispatcher_authority.to_account_info(),
            ctx.accounts.usdc_bank.to_account_info(),
            ctx.accounts.adapter_vault.to_account_info(),
            ctx.accounts.bank_liquidity_vault.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        &[],
    ).map_err(|_| AdapterError::MarginFiCpiFailed.into())
}

fn invoke_marginfi_withdraw(ctx: &Context<MarginFiWithdraw>, shares: u64, amount: u64) -> Result<()> {
    let mut ix_data: Vec<u8> = vec![0xd1, 0x37, 0x79, 0x73, 0x12, 0x45, 0x33, 0x2a];
    ix_data.extend_from_slice(&amount.to_le_bytes());
    ix_data.push(0);

    let accounts = vec![
        AccountMeta::new(ctx.accounts.marginfi_group.key(), false),
        AccountMeta::new(ctx.accounts.marginfi_account.key(), false),
        AccountMeta::new_readonly(ctx.accounts.dispatcher_authority.key(), true),
        AccountMeta::new(ctx.accounts.usdc_bank.key(), false),
        AccountMeta::new(ctx.accounts.adapter_vault.key(), false),
        AccountMeta::new(ctx.accounts.bank_liquidity_vault.key(), false),
        AccountMeta::new_readonly(ctx.accounts.bank_liquidity_vault_authority.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
    ];

    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.marginfi_program.key(),
        accounts,
        data: ix_data,
    };

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.marginfi_group.to_account_info(),
            ctx.accounts.marginfi_account.to_account_info(),
            ctx.accounts.dispatcher_authority.to_account_info(),
            ctx.accounts.usdc_bank.to_account_info(),
            ctx.accounts.adapter_vault.to_account_info(),
            ctx.accounts.bank_liquidity_vault.to_account_info(),
            ctx.accounts.bank_liquidity_vault_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        &[],
    ).map_err(|_| AdapterError::MarginFiCpiFailed.into())
}

#[error_code]
pub enum AdapterError {
    #[msg("Invalid token mint")]
    InvalidMint,
    #[msg("Slippage exceeded")]
    SlippageExceeded,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("CPI to MarginFi failed")]
    MarginFiCpiFailed,
    #[msg("Invalid MarginFi bank account")]
    InvalidBank,
    #[msg("Could not parse MarginFi bank data")]
    InvalidBankData,
}

#[event]
pub struct MarginFiDepositEvent {
    pub amount_in: u64,
    pub shares_out: u64,
    pub asset_share_value: u128,
    pub slot: u64,
}

#[event]
pub struct MarginFiWithdrawEvent {
    pub shares_in: u64,
    pub amount_out: u64,
    pub asset_share_value: u128,
    pub slot: u64,
}
