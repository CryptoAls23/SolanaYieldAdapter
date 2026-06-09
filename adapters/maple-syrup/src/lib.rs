use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint};

declare_id!("MAdPtMaPlEsYrUpXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

pub const MAPLE_PROGRAM_ID: Pubkey = pubkey!("MaPLEd3SertEXhgnhmkMWMNUgZ3p3WGEVnJyoaJpump");
pub const SYRUP_POOL: Pubkey = pubkey!("SyRuPCkZGH3p5M4GCjLGMKZCELiHSXqtFm4nwH8Bd9R");
pub const SYUSDC_MINT: Pubkey = pubkey!("syUSDCEXTgrkdHJHGKiDe7PVpEWXkJuWF9Wgf7tBp4k");
pub const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const VAULT_SCALE: u64 = 1_000_000_000;

#[program]
pub mod maple_syrup_adapter {
    use super::*;

    pub fn adapter_deposit(ctx: Context<MapleDeposit>, amount: u64, min_shares_out: u64) -> Result<u64> {
        let clock = Clock::get()?;
        let exchange_rate = read_maple_exchange_rate(
            &ctx.accounts.syrup_pool,
            &ctx.accounts.syusdc_mint,
        )?;

        let shares_out = (amount as u128)
            .checked_mul(VAULT_SCALE as u128)
            .ok_or(AdapterError::MathOverflow)?
            .checked_div(exchange_rate as u128)
            .ok_or(AdapterError::MathOverflow)? as u64;

        require!(shares_out >= min_shares_out, AdapterError::SlippageExceeded);

        let actual_shares = invoke_maple_deposit(&ctx, amount, shares_out)?;

        let mut return_data = actual_shares.to_le_bytes().to_vec();
        return_data.extend_from_slice(&(exchange_rate as u128).to_le_bytes());
        anchor_lang::solana_program::program::set_return_data(&return_data);

        emit!(MapleDepositEvent {
            amount_in: amount,
            shares_out: actual_shares,
            exchange_rate,
            slot: clock.slot,
        });

        msg!("Maple Syrup deposit: {} USDC -> {} syUSDC", amount, actual_shares);
        Ok(actual_shares)
    }

    pub fn adapter_withdraw(ctx: Context<MapleWithdraw>, shares: u64, min_amount_out: u64) -> Result<u64> {
        let clock = Clock::get()?;
        let exchange_rate = read_maple_exchange_rate(
            &ctx.accounts.syrup_pool,
            &ctx.accounts.syusdc_mint,
        )?;

        let amount_out = (shares as u128)
            .checked_mul(exchange_rate as u128)
            .ok_or(AdapterError::MathOverflow)?
            .checked_div(VAULT_SCALE as u128)
            .ok_or(AdapterError::MathOverflow)? as u64;

        require!(amount_out >= min_amount_out, AdapterError::SlippageExceeded);

        let actual_out = invoke_maple_redeem(&ctx, shares, min_amount_out)?;

        let mut return_data = actual_out.to_le_bytes().to_vec();
        return_data.extend_from_slice(&(exchange_rate as u128).to_le_bytes());
        anchor_lang::solana_program::program::set_return_data(&return_data);

        emit!(MapleWithdrawEvent {
            shares_in: shares,
            amount_out: actual_out,
            exchange_rate,
            slot: clock.slot,
        });

        msg!("Maple Syrup withdraw: {} syUSDC -> {} USDC", shares, actual_out);
        Ok(actual_out)
    }

    pub fn adapter_current_value(ctx: Context<MapleCurrentValue>, shares: u64) -> Result<u64> {
        let exchange_rate = read_maple_exchange_rate(
            &ctx.accounts.syrup_pool,
            &ctx.accounts.syusdc_mint,
        )?;

        let value = (shares as u128)
            .checked_mul(exchange_rate as u128)
            .ok_or(AdapterError::MathOverflow)?
            .checked_div(VAULT_SCALE as u128)
            .ok_or(AdapterError::MathOverflow)? as u64;

        let mut return_data = value.to_le_bytes().to_vec();
        return_data.extend_from_slice(&(exchange_rate as u128).to_le_bytes());
        anchor_lang::solana_program::program::set_return_data(&return_data);

        Ok(value)
    }
}

#[derive(Accounts)]
pub struct MapleDeposit<'info> {
    #[account(mut, constraint = adapter_vault.mint == USDC_MINT @ AdapterError::InvalidMint)]
    pub adapter_vault: Account<'info, TokenAccount>,

    #[account(mut, constraint = user_syusdc_account.mint == SYUSDC_MINT @ AdapterError::InvalidMint)]
    pub user_syusdc_account: Account<'info, TokenAccount>,

    #[account(mut, address = SYUSDC_MINT)]
    pub syusdc_mint: Account<'info, Mint>,

    /// CHECK: Maple Syrup pool state
    #[account(mut, address = SYRUP_POOL @ AdapterError::InvalidPool)]
    pub syrup_pool: AccountInfo<'info>,

    /// CHECK: Pool USDC vault
    #[account(mut)]
    pub pool_usdc_vault: AccountInfo<'info>,

    /// CHECK: Dispatcher authority
    #[account(signer)]
    pub dispatcher_authority: AccountInfo<'info>,

    /// CHECK: Maple program
    #[account(address = MAPLE_PROGRAM_ID)]
    pub maple_program: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct MapleWithdraw<'info> {
    #[account(mut, constraint = adapter_vault.mint == USDC_MINT @ AdapterError::InvalidMint)]
    pub adapter_vault: Account<'info, TokenAccount>,

    #[account(mut, constraint = user_syusdc_account.mint == SYUSDC_MINT @ AdapterError::InvalidMint)]
    pub user_syusdc_account: Account<'info, TokenAccount>,

    #[account(mut, address = SYUSDC_MINT)]
    pub syusdc_mint: Account<'info, Mint>,

    /// CHECK: Syrup pool
    #[account(mut, address = SYRUP_POOL @ AdapterError::InvalidPool)]
    pub syrup_pool: AccountInfo<'info>,

    /// CHECK: Pool USDC vault
    #[account(mut)]
    pub pool_usdc_vault: AccountInfo<'info>,

    /// CHECK: Dispatcher authority
    #[account(signer)]
    pub dispatcher_authority: AccountInfo<'info>,

    /// CHECK: Maple program
    #[account(address = MAPLE_PROGRAM_ID)]
    pub maple_program: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct MapleCurrentValue<'info> {
    /// CHECK: Syrup pool for exchange rate
    #[account(address = SYRUP_POOL)]
    pub syrup_pool: AccountInfo<'info>,

    #[account(address = SYUSDC_MINT)]
    pub syusdc_mint: Account<'info, Mint>,

    pub dispatcher_authority: AccountInfo<'info>,
}

fn read_maple_exchange_rate(pool: &AccountInfo, syusdc_mint: &Account<Mint>) -> Result<u64> {
    let data = pool.try_borrow_data()?;

    if data.len() < 24 {
        return Ok(VAULT_SCALE);
    }

    let total_assets_bytes: [u8; 8] = data[8..16].try_into()
        .map_err(|_| AdapterError::InvalidPoolData)?;
    let total_assets = u64::from_le_bytes(total_assets_bytes);
    let total_supply = syusdc_mint.supply;

    if total_supply == 0 || total_assets == 0 {
        return Ok(VAULT_SCALE);
    }

    let rate = (total_assets as u128)
        .checked_mul(VAULT_SCALE as u128)
        .unwrap_or(VAULT_SCALE as u128)
        .checked_div(total_supply as u128)
        .unwrap_or(VAULT_SCALE as u128) as u64;

    Ok(if rate == 0 { VAULT_SCALE } else { rate })
}

fn invoke_maple_deposit(
    ctx: &Context<MapleDeposit>,
    amount: u64,
    expected_shares: u64,
) -> Result<u64> {
    let mut ix_data: Vec<u8> = vec![0xf2, 0x23, 0xc6, 0x89, 0x52, 0xe1, 0xf2, 0xb6];
    ix_data.extend_from_slice(&amount.to_le_bytes());
    ix_data.extend_from_slice(&expected_shares.to_le_bytes());

    let accounts = vec![
        AccountMeta::new_readonly(ctx.accounts.dispatcher_authority.key(), true),
        AccountMeta::new(ctx.accounts.syrup_pool.key(), false),
        AccountMeta::new(ctx.accounts.adapter_vault.key(), false),
        AccountMeta::new(ctx.accounts.pool_usdc_vault.key(), false),
        AccountMeta::new(ctx.accounts.user_syusdc_account.key(), false),
        AccountMeta::new(ctx.accounts.syusdc_mint.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
    ];

    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.maple_program.key(),
        accounts,
        data: ix_data,
    };

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.dispatcher_authority.to_account_info(),
            ctx.accounts.syrup_pool.to_account_info(),
            ctx.accounts.adapter_vault.to_account_info(),
            ctx.accounts.pool_usdc_vault.to_account_info(),
            ctx.accounts.user_syusdc_account.to_account_info(),
            ctx.accounts.syusdc_mint.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        &[],
    ).map_err(|_| AdapterError::MapleCpiFailed)?;

    let (_prog, data) = anchor_lang::solana_program::program::get_return_data()
        .unwrap_or_default();
    if data.len() >= 8 {
        Ok(u64::from_le_bytes(data[..8].try_into().unwrap()))
    } else {
        Ok(expected_shares)
    }
}

fn invoke_maple_redeem(
    ctx: &Context<MapleWithdraw>,
    shares: u64,
    min_out: u64,
) -> Result<u64> {
    let mut ix_data: Vec<u8> = vec![0xb7, 0x12, 0x46, 0x9c, 0x94, 0x67, 0x33, 0xf4];
    ix_data.extend_from_slice(&shares.to_le_bytes());
    ix_data.extend_from_slice(&min_out.to_le_bytes());

    let accounts = vec![
        AccountMeta::new_readonly(ctx.accounts.dispatcher_authority.key(), true),
        AccountMeta::new(ctx.accounts.syrup_pool.key(), false),
        AccountMeta::new(ctx.accounts.user_syusdc_account.key(), false),
        AccountMeta::new(ctx.accounts.syusdc_mint.key(), false),
        AccountMeta::new(ctx.accounts.pool_usdc_vault.key(), false),
        AccountMeta::new(ctx.accounts.adapter_vault.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
    ];

    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.maple_program.key(),
        accounts,
        data: ix_data,
    };

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.dispatcher_authority.to_account_info(),
            ctx.accounts.syrup_pool.to_account_info(),
            ctx.accounts.user_syusdc_account.to_account_info(),
            ctx.accounts.syusdc_mint.to_account_info(),
            ctx.accounts.pool_usdc_vault.to_account_info(),
            ctx.accounts.adapter_vault.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        &[],
    ).map_err(|_| AdapterError::MapleCpiFailed)?;

    let (_prog, data) = anchor_lang::solana_program::program::get_return_data()
        .unwrap_or_default();
    if data.len() >= 8 {
        Ok(u64::from_le_bytes(data[..8].try_into().unwrap()))
    } else {
        Ok(min_out)
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
    #[msg("CPI to Maple failed")]
    MapleCpiFailed,
    #[msg("Invalid Maple pool account")]
    InvalidPool,
    #[msg("Could not parse Maple pool data")]
    InvalidPoolData,
}

#[event]
pub struct MapleDepositEvent {
    pub amount_in: u64,
    pub shares_out: u64,
    pub exchange_rate: u64,
    pub slot: u64,
}

#[event]
pub struct MapleWithdrawEvent {
    pub shares_in: u64,
    pub amount_out: u64,
    pub exchange_rate: u64,
    pub slot: u64,
}
