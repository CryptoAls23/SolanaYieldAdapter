use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

declare_id!("DAdPtDrIfTiNsFuNdXXXXXXXXXXXXXXXXXXXXXXXXXXX");

pub const DRIFT_PROGRAM_ID: Pubkey = pubkey!("dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH");
pub const DRIFT_STATE: Pubkey = pubkey!("4aGc3mVgZTSX6qWMi4CBZpqiEzqH5DPbAuCkQKbhkCib");
pub const DRIFT_IF_VAULT: Pubkey = pubkey!("USDRm8LfUNNYmJAtFgvE3aqr3cxnxEPpEVEUf3G8Hjf");
pub const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const USDC_SPOT_MARKET_INDEX: u16 = 0;
pub const IF_SHARE_SCALE: u128 = 1_000_000_000;

#[program]
pub mod drift_insurance_adapter {
    use super::*;

    pub fn adapter_deposit(ctx: Context<DriftDeposit>, amount: u64, min_shares_out: u64) -> Result<u64> {
        let clock = Clock::get()?;
        let exchange_rate = read_drift_if_exchange_rate(
            &ctx.accounts.spot_market,
            &ctx.accounts.insurance_fund_vault,
        )?;

        let shares_out = (amount as u128)
            .checked_mul(IF_SHARE_SCALE)
            .ok_or(AdapterError::MathOverflow)?
            .checked_div(exchange_rate)
            .ok_or(AdapterError::MathOverflow)? as u64;

        require!(shares_out >= min_shares_out, AdapterError::SlippageExceeded);

        let actual_shares = invoke_drift_stake(&ctx, amount)?;

        let mut return_data = actual_shares.to_le_bytes().to_vec();
        return_data.extend_from_slice(&exchange_rate.to_le_bytes());
        anchor_lang::solana_program::program::set_return_data(&return_data);

        emit!(DriftDepositEvent {
            amount_in: amount,
            shares_minted: actual_shares,
            exchange_rate,
            slot: clock.slot,
        });

        msg!("Drift IF stake: {} USDC -> {} IF shares", amount, actual_shares);
        Ok(actual_shares)
    }

    pub fn adapter_withdraw(ctx: Context<DriftWithdraw>, shares: u64, min_amount_out: u64) -> Result<u64> {
        let clock = Clock::get()?;
        let exchange_rate = read_drift_if_exchange_rate(
            &ctx.accounts.spot_market,
            &ctx.accounts.insurance_fund_vault,
        )?;

        let amount_out = (shares as u128)
            .checked_mul(exchange_rate)
            .ok_or(AdapterError::MathOverflow)?
            .checked_div(IF_SHARE_SCALE)
            .ok_or(AdapterError::MathOverflow)? as u64;

        require!(amount_out >= min_amount_out, AdapterError::SlippageExceeded);

        let actual_out = invoke_drift_unstake_request(&ctx, shares)?;

        let mut return_data = actual_out.to_le_bytes().to_vec();
        return_data.extend_from_slice(&exchange_rate.to_le_bytes());
        anchor_lang::solana_program::program::set_return_data(&return_data);

        emit!(DriftWithdrawEvent {
            shares_requested: shares,
            amount_expected: amount_out,
            exchange_rate,
            slot: clock.slot,
        });

        msg!(
            "Drift IF unstake requested: {} shares (~{} USDC, 13-day cooldown)",
            shares,
            amount_out
        );
        Ok(actual_out)
    }

    pub fn adapter_current_value(ctx: Context<DriftCurrentValue>, shares: u64) -> Result<u64> {
        let exchange_rate = read_drift_if_exchange_rate(
            &ctx.accounts.spot_market,
            &ctx.accounts.insurance_fund_vault,
        )?;

        let value = (shares as u128)
            .checked_mul(exchange_rate)
            .ok_or(AdapterError::MathOverflow)?
            .checked_div(IF_SHARE_SCALE)
            .ok_or(AdapterError::MathOverflow)? as u64;

        let mut return_data = value.to_le_bytes().to_vec();
        return_data.extend_from_slice(&exchange_rate.to_le_bytes());
        anchor_lang::solana_program::program::set_return_data(&return_data);

        Ok(value)
    }

    pub fn complete_unstake(ctx: Context<DriftCompleteUnstake>) -> Result<u64> {
        let amount = invoke_drift_complete_unstake(&ctx)?;
        msg!("Drift IF unstake completed: {} USDC withdrawn", amount);
        Ok(amount)
    }
}

#[derive(Accounts)]
pub struct DriftDeposit<'info> {
    #[account(mut, constraint = adapter_vault.mint == USDC_MINT @ AdapterError::InvalidMint)]
    pub adapter_vault: Account<'info, TokenAccount>,

    /// CHECK: Insurance fund stake PDA
    #[account(mut)]
    pub insurance_fund_stake: AccountInfo<'info>,

    /// CHECK: USDC spot market
    #[account(mut)]
    pub spot_market: AccountInfo<'info>,

    /// CHECK: Insurance fund vault
    #[account(mut, address = DRIFT_IF_VAULT @ AdapterError::InvalidVault)]
    pub insurance_fund_vault: Account<'info, TokenAccount>,

    /// CHECK: Drift global state
    #[account(address = DRIFT_STATE)]
    pub drift_state: AccountInfo<'info>,

    /// CHECK: Dispatcher authority
    #[account(signer)]
    pub dispatcher_authority: AccountInfo<'info>,

    /// CHECK: Drift program
    #[account(address = DRIFT_PROGRAM_ID)]
    pub drift_program: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct DriftWithdraw<'info> {
    #[account(mut, constraint = adapter_vault.mint == USDC_MINT @ AdapterError::InvalidMint)]
    pub adapter_vault: Account<'info, TokenAccount>,

    /// CHECK: Insurance fund stake
    #[account(mut)]
    pub insurance_fund_stake: AccountInfo<'info>,

    /// CHECK: USDC spot market
    #[account(mut)]
    pub spot_market: AccountInfo<'info>,

    /// CHECK: IF vault
    #[account(mut, address = DRIFT_IF_VAULT @ AdapterError::InvalidVault)]
    pub insurance_fund_vault: Account<'info, TokenAccount>,

    /// CHECK: Drift state
    #[account(address = DRIFT_STATE)]
    pub drift_state: AccountInfo<'info>,

    /// CHECK: Dispatcher authority
    #[account(signer)]
    pub dispatcher_authority: AccountInfo<'info>,

    /// CHECK: Drift program
    #[account(address = DRIFT_PROGRAM_ID)]
    pub drift_program: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct DriftCurrentValue<'info> {
    /// CHECK: USDC spot market
    pub spot_market: AccountInfo<'info>,

    /// CHECK: IF vault
    #[account(address = DRIFT_IF_VAULT)]
    pub insurance_fund_vault: Account<'info, TokenAccount>,

    pub dispatcher_authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct DriftCompleteUnstake<'info> {
    #[account(mut, constraint = adapter_vault.mint == USDC_MINT @ AdapterError::InvalidMint)]
    pub adapter_vault: Account<'info, TokenAccount>,

    /// CHECK: Insurance fund stake
    #[account(mut)]
    pub insurance_fund_stake: AccountInfo<'info>,

    /// CHECK: USDC spot market
    #[account(mut)]
    pub spot_market: AccountInfo<'info>,

    /// CHECK: IF vault
    #[account(mut, address = DRIFT_IF_VAULT @ AdapterError::InvalidVault)]
    pub insurance_fund_vault: Account<'info, TokenAccount>,

    /// CHECK: Drift state
    #[account(address = DRIFT_STATE)]
    pub drift_state: AccountInfo<'info>,

    /// CHECK: Dispatcher authority
    #[account(signer)]
    pub dispatcher_authority: AccountInfo<'info>,

    /// CHECK: Drift program
    #[account(address = DRIFT_PROGRAM_ID)]
    pub drift_program: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

fn read_drift_if_exchange_rate(
    spot_market: &AccountInfo,
    if_vault: &Account<TokenAccount>,
) -> Result<u128> {
    let data = spot_market.try_borrow_data()?;

    if data.len() < 220 {
        return Ok(IF_SHARE_SCALE);
    }

    let shares_bytes: [u8; 16] = data[200..216].try_into()
        .map_err(|_| AdapterError::InvalidSpotMarket)?;
    let total_shares = u128::from_le_bytes(shares_bytes);
    let vault_balance = if_vault.amount as u128;

    if total_shares == 0 || vault_balance == 0 {
        return Ok(IF_SHARE_SCALE);
    }

    let rate = vault_balance
        .checked_mul(IF_SHARE_SCALE)
        .unwrap_or(IF_SHARE_SCALE)
        .checked_div(total_shares)
        .unwrap_or(IF_SHARE_SCALE);

    Ok(if rate == 0 { IF_SHARE_SCALE } else { rate })
}

fn invoke_drift_stake(ctx: &Context<DriftDeposit>, amount: u64) -> Result<u64> {
    let mut ix_data: Vec<u8> = vec![0x1f, 0x4b, 0x12, 0x8c, 0x09, 0x37, 0x2b, 0xa4];
    ix_data.extend_from_slice(&(USDC_SPOT_MARKET_INDEX as u32).to_le_bytes());
    ix_data.extend_from_slice(&amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(ctx.accounts.drift_state.key(), false),
        AccountMeta::new(ctx.accounts.spot_market.key(), false),
        AccountMeta::new(ctx.accounts.insurance_fund_stake.key(), false),
        AccountMeta::new_readonly(ctx.accounts.dispatcher_authority.key(), true),
        AccountMeta::new(ctx.accounts.adapter_vault.key(), false),
        AccountMeta::new(ctx.accounts.insurance_fund_vault.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
    ];

    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.drift_program.key(),
        accounts,
        data: ix_data,
    };

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.drift_state.to_account_info(),
            ctx.accounts.spot_market.to_account_info(),
            ctx.accounts.insurance_fund_stake.to_account_info(),
            ctx.accounts.dispatcher_authority.to_account_info(),
            ctx.accounts.adapter_vault.to_account_info(),
            ctx.accounts.insurance_fund_vault.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        &[],
    ).map_err(|_| AdapterError::DriftCpiFailed)?;

    let (_prog, data) = anchor_lang::solana_program::program::get_return_data()
        .unwrap_or_default();
    if data.len() >= 8 {
        Ok(u64::from_le_bytes(data[..8].try_into().unwrap()))
    } else {
        Ok(amount)
    }
}

fn invoke_drift_unstake_request(ctx: &Context<DriftWithdraw>, shares: u64) -> Result<u64> {
    let mut ix_data: Vec<u8> = vec![0x9a, 0x3e, 0xf1, 0x23, 0x4a, 0xb8, 0x71, 0x5c];
    ix_data.extend_from_slice(&(USDC_SPOT_MARKET_INDEX as u32).to_le_bytes());
    ix_data.extend_from_slice(&shares.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(ctx.accounts.drift_state.key(), false),
        AccountMeta::new(ctx.accounts.spot_market.key(), false),
        AccountMeta::new(ctx.accounts.insurance_fund_stake.key(), false),
        AccountMeta::new_readonly(ctx.accounts.dispatcher_authority.key(), true),
        AccountMeta::new(ctx.accounts.insurance_fund_vault.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
    ];

    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.drift_program.key(),
        accounts,
        data: ix_data,
    };

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.drift_state.to_account_info(),
            ctx.accounts.spot_market.to_account_info(),
            ctx.accounts.insurance_fund_stake.to_account_info(),
            ctx.accounts.dispatcher_authority.to_account_info(),
            ctx.accounts.insurance_fund_vault.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        &[],
    ).map_err(|_| AdapterError::DriftCpiFailed)?;

    Ok(0)
}

fn invoke_drift_complete_unstake(ctx: &Context<DriftCompleteUnstake>) -> Result<u64> {
    let mut ix_data: Vec<u8> = vec![0x3c, 0x77, 0xa1, 0x9f, 0x2b, 0xe4, 0x56, 0xd3];
    ix_data.extend_from_slice(&(USDC_SPOT_MARKET_INDEX as u32).to_le_bytes());

    let accounts = vec![
        AccountMeta::new(ctx.accounts.drift_state.key(), false),
        AccountMeta::new(ctx.accounts.spot_market.key(), false),
        AccountMeta::new(ctx.accounts.insurance_fund_stake.key(), false),
        AccountMeta::new_readonly(ctx.accounts.dispatcher_authority.key(), true),
        AccountMeta::new(ctx.accounts.adapter_vault.key(), false),
        AccountMeta::new(ctx.accounts.insurance_fund_vault.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
    ];

    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.drift_program.key(),
        accounts,
        data: ix_data,
    };

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.drift_state.to_account_info(),
            ctx.accounts.spot_market.to_account_info(),
            ctx.accounts.insurance_fund_stake.to_account_info(),
            ctx.accounts.dispatcher_authority.to_account_info(),
            ctx.accounts.adapter_vault.to_account_info(),
            ctx.accounts.insurance_fund_vault.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        &[],
    ).map_err(|_| AdapterError::DriftCpiFailed)?;

    let (_prog, data) = anchor_lang::solana_program::program::get_return_data()
        .unwrap_or_default();
    if data.len() >= 8 {
        Ok(u64::from_le_bytes(data[..8].try_into().unwrap()))
    } else {
        Ok(0)
    }
}

#[error_code]
pub enum AdapterError {
    #[msg("Invalid token mint")]
    InvalidMint,
    #[msg("Slippage exceeded")]
    SlippageExceeded,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("CPI to Drift failed")]
    DriftCpiFailed,
    #[msg("Invalid Drift IF vault")]
    InvalidVault,
    #[msg("Invalid spot market data")]
    InvalidSpotMarket,
}

#[event]
pub struct DriftDepositEvent {
    pub amount_in: u64,
    pub shares_minted: u64,
    pub exchange_rate: u128,
    pub slot: u64,
}

#[event]
pub struct DriftWithdrawEvent {
    pub shares_requested: u64,
    pub amount_expected: u64,
    pub exchange_rate: u128,
    pub slot: u64,
}
