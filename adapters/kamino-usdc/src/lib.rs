use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Mint, MintTo, Burn};

declare_id!("KAdPtKAmInOUSDCXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

pub const KAMINO_PROGRAM_ID: Pubkey = pubkey!("KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD");
pub const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const KUSDC_MINT: Pubkey = pubkey!("FBSyPnxtHKLBZ4UeeUyAnbtFuAmTHLtso9YtsqRDRWkB");
pub const KAMINO_USDC_RESERVE: Pubkey = pubkey!("D6q6wuQSriferjmtEkLeX43LchH4ix3raHmkX9W3LQba");
pub const KAMINO_LENDING_MARKET: Pubkey = pubkey!("7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF");
pub const RATE_SCALE: u64 = 1_000_000_000;

#[program]
pub mod kamino_usdc_adapter {
    use super::*;

    pub fn adapter_deposit(ctx: Context<KaminoDeposit>, amount: u64, min_shares_out: u64) -> Result<u64> {
        let clock = Clock::get()?;
        let exchange_rate = read_kamino_exchange_rate(&ctx.accounts.kamino_reserve)?;

        let shares_expected = (amount as u128)
            .checked_mul(RATE_SCALE as u128)
            .ok_or(AdapterError::MathOverflow)?
            .checked_div(exchange_rate as u128)
            .ok_or(AdapterError::MathOverflow)? as u64;

        require!(shares_expected >= min_shares_out, AdapterError::SlippageExceeded);

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.adapter_vault.to_account_info(),
                    to: ctx.accounts.kamino_reserve_liquidity.to_account_info(),
                    authority: ctx.accounts.dispatcher_authority.to_account_info(),
                },
            ),
            amount,
        )?;

        invoke_kamino_refresh_reserve(&ctx)?;
        let shares_minted = invoke_kamino_deposit(&ctx, amount, shares_expected)?;

        let mut return_data = shares_minted.to_le_bytes().to_vec();
        return_data.extend_from_slice(&exchange_rate.to_le_bytes());
        anchor_lang::solana_program::program::set_return_data(&return_data);

        emit!(KaminoDepositEvent {
            amount_in: amount,
            shares_minted,
            exchange_rate,
            slot: clock.slot,
        });

        msg!("Kamino deposit: {} USDC -> {} kUSDC (rate: {})", amount, shares_minted, exchange_rate);
        Ok(shares_minted)
    }

    pub fn adapter_withdraw(ctx: Context<KaminoWithdraw>, shares: u64, min_amount_out: u64) -> Result<u64> {
        let clock = Clock::get()?;
        let exchange_rate = read_kamino_exchange_rate(&ctx.accounts.kamino_reserve)?;

        let amount_out = (shares as u128)
            .checked_mul(exchange_rate as u128)
            .ok_or(AdapterError::MathOverflow)?
            .checked_div(RATE_SCALE as u128)
            .ok_or(AdapterError::MathOverflow)? as u64;

        require!(amount_out >= min_amount_out, AdapterError::SlippageExceeded);

        let amount_received = invoke_kamino_redeem(&ctx, shares, amount_out)?;

        let mut return_data = amount_received.to_le_bytes().to_vec();
        return_data.extend_from_slice(&exchange_rate.to_le_bytes());
        anchor_lang::solana_program::program::set_return_data(&return_data);

        emit!(KaminoWithdrawEvent {
            shares_redeemed: shares,
            amount_out: amount_received,
            exchange_rate,
            slot: clock.slot,
        });

        msg!("Kamino withdraw: {} kUSDC -> {} USDC", shares, amount_received);
        Ok(amount_received)
    }

    pub fn adapter_current_value(ctx: Context<KaminoCurrentValue>, shares: u64) -> Result<u64> {
        let exchange_rate = read_kamino_exchange_rate(&ctx.accounts.kamino_reserve)?;

        let value = (shares as u128)
            .checked_mul(exchange_rate as u128)
            .ok_or(AdapterError::MathOverflow)?
            .checked_div(RATE_SCALE as u128)
            .ok_or(AdapterError::MathOverflow)? as u64;

        let mut return_data = value.to_le_bytes().to_vec();
        return_data.extend_from_slice(&exchange_rate.to_le_bytes());
        anchor_lang::solana_program::program::set_return_data(&return_data);

        Ok(value)
    }
}

#[derive(Accounts)]
pub struct KaminoDeposit<'info> {
    #[account(mut, constraint = adapter_vault.mint == USDC_MINT @ AdapterError::InvalidMint)]
    pub adapter_vault: Account<'info, TokenAccount>,

    /// CHECK: Kamino reserve account
    #[account(mut, address = KAMINO_USDC_RESERVE @ AdapterError::InvalidReserve)]
    pub kamino_reserve: AccountInfo<'info>,

    /// CHECK: Kamino reserve liquidity token account
    #[account(mut)]
    pub kamino_reserve_liquidity: AccountInfo<'info>,

    #[account(mut, constraint = user_kusdc_account.mint == KUSDC_MINT @ AdapterError::InvalidMint)]
    pub user_kusdc_account: Account<'info, TokenAccount>,

    #[account(mut, address = KUSDC_MINT)]
    pub kusdc_mint: Account<'info, Mint>,

    /// CHECK: Kamino lending market
    #[account(address = KAMINO_LENDING_MARKET @ AdapterError::InvalidMarket)]
    pub lending_market: AccountInfo<'info>,

    /// CHECK: Kamino lending market authority PDA
    pub lending_market_authority: AccountInfo<'info>,

    /// CHECK: Dispatcher authority
    #[account(signer)]
    pub dispatcher_authority: AccountInfo<'info>,

    /// CHECK: Kamino program
    #[account(address = KAMINO_PROGRAM_ID)]
    pub kamino_program: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct KaminoWithdraw<'info> {
    #[account(mut, constraint = adapter_vault.mint == USDC_MINT @ AdapterError::InvalidMint)]
    pub adapter_vault: Account<'info, TokenAccount>,

    /// CHECK: Kamino reserve
    #[account(mut, address = KAMINO_USDC_RESERVE @ AdapterError::InvalidReserve)]
    pub kamino_reserve: AccountInfo<'info>,

    /// CHECK: Kamino reserve liquidity
    #[account(mut)]
    pub kamino_reserve_liquidity: AccountInfo<'info>,

    #[account(mut, constraint = user_kusdc_account.mint == KUSDC_MINT @ AdapterError::InvalidMint)]
    pub user_kusdc_account: Account<'info, TokenAccount>,

    #[account(mut, address = KUSDC_MINT)]
    pub kusdc_mint: Account<'info, Mint>,

    /// CHECK: Kamino lending market
    #[account(address = KAMINO_LENDING_MARKET)]
    pub lending_market: AccountInfo<'info>,

    /// CHECK: Kamino lending market authority
    pub lending_market_authority: AccountInfo<'info>,

    /// CHECK: Dispatcher authority
    #[account(signer)]
    pub dispatcher_authority: AccountInfo<'info>,

    /// CHECK: Kamino program
    #[account(address = KAMINO_PROGRAM_ID)]
    pub kamino_program: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct KaminoCurrentValue<'info> {
    /// CHECK: Kamino reserve for exchange rate lookup
    #[account(address = KAMINO_USDC_RESERVE)]
    pub kamino_reserve: AccountInfo<'info>,

    /// CHECK: Dispatcher authority
    pub dispatcher_authority: AccountInfo<'info>,
}

fn read_kamino_exchange_rate(reserve: &AccountInfo) -> Result<u128> {
    let data = reserve.try_borrow_data()?;

    if data.len() < 345 {
        return Ok(RATE_SCALE as u128);
    }

    let rate_bytes: [u8; 16] = data[329..345].try_into()
        .map_err(|_| AdapterError::InvalidReserveData)?;
    let kamino_rate = u128::from_le_bytes(rate_bytes);

    if kamino_rate == 0 {
        return Ok(RATE_SCALE as u128);
    }

    let rate = kamino_rate
        .checked_div(1_000_000_000)
        .unwrap_or(RATE_SCALE as u128);

    Ok(rate)
}

fn invoke_kamino_refresh_reserve(ctx: &Context<KaminoDeposit>) -> Result<()> {
    let ix_data: Vec<u8> = vec![0x02, 0x84, 0xca, 0xf1, 0x5e, 0x12, 0x22, 0xf6];

    let accounts = vec![
        AccountMeta::new(ctx.accounts.kamino_reserve.key(), false),
        AccountMeta::new_readonly(ctx.accounts.lending_market.key(), false),
    ];

    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.kamino_program.key(),
        accounts,
        data: ix_data,
    };

    anchor_lang::solana_program::program::invoke(
        &ix,
        &[
            ctx.accounts.kamino_reserve.to_account_info(),
            ctx.accounts.lending_market.to_account_info(),
            ctx.accounts.kamino_program.to_account_info(),
        ],
    ).map_err(|_| AdapterError::KaminoCpiFailed.into())
}

fn invoke_kamino_deposit(
    ctx: &Context<KaminoDeposit>,
    liquidity_amount: u64,
    _expected_shares: u64,
) -> Result<u64> {
    let mut ix_data: Vec<u8> = vec![0xf2, 0x23, 0xc6, 0x89, 0x52, 0xe1, 0xf2, 0xb6];
    ix_data.extend_from_slice(&liquidity_amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new_readonly(ctx.accounts.lending_market.key(), false),
        AccountMeta::new_readonly(ctx.accounts.lending_market_authority.key(), false),
        AccountMeta::new(ctx.accounts.adapter_vault.key(), false),
        AccountMeta::new(ctx.accounts.kamino_reserve.key(), false),
        AccountMeta::new(ctx.accounts.kamino_reserve_liquidity.key(), false),
        AccountMeta::new(ctx.accounts.kusdc_mint.key(), false),
        AccountMeta::new(ctx.accounts.user_kusdc_account.key(), false),
        AccountMeta::new_readonly(ctx.accounts.dispatcher_authority.key(), true),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
    ];

    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.kamino_program.key(),
        accounts,
        data: ix_data,
    };

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.lending_market.to_account_info(),
            ctx.accounts.lending_market_authority.to_account_info(),
            ctx.accounts.adapter_vault.to_account_info(),
            ctx.accounts.kamino_reserve.to_account_info(),
            ctx.accounts.kamino_reserve_liquidity.to_account_info(),
            ctx.accounts.kusdc_mint.to_account_info(),
            ctx.accounts.user_kusdc_account.to_account_info(),
            ctx.accounts.dispatcher_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.kamino_program.to_account_info(),
        ],
        &[],
    ).map_err(|_| AdapterError::KaminoCpiFailed)?;

    let (_prog, data) = anchor_lang::solana_program::program::get_return_data()
        .unwrap_or_default();
    if data.len() >= 8 {
        Ok(u64::from_le_bytes(data[..8].try_into().unwrap()))
    } else {
        Ok(_expected_shares)
    }
}

fn invoke_kamino_redeem(
    ctx: &Context<KaminoWithdraw>,
    collateral_amount: u64,
    _expected_out: u64,
) -> Result<u64> {
    let mut ix_data: Vec<u8> = vec![0xb7, 0x12, 0x46, 0x9c, 0x94, 0x67, 0x33, 0xf4];
    ix_data.extend_from_slice(&collateral_amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new_readonly(ctx.accounts.lending_market.key(), false),
        AccountMeta::new_readonly(ctx.accounts.lending_market_authority.key(), false),
        AccountMeta::new(ctx.accounts.user_kusdc_account.key(), false),
        AccountMeta::new(ctx.accounts.kamino_reserve.key(), false),
        AccountMeta::new(ctx.accounts.kamino_reserve_liquidity.key(), false),
        AccountMeta::new(ctx.accounts.kusdc_mint.key(), false),
        AccountMeta::new(ctx.accounts.adapter_vault.key(), false),
        AccountMeta::new_readonly(ctx.accounts.dispatcher_authority.key(), true),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
    ];

    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.kamino_program.key(),
        accounts,
        data: ix_data,
    };

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.lending_market.to_account_info(),
            ctx.accounts.lending_market_authority.to_account_info(),
            ctx.accounts.user_kusdc_account.to_account_info(),
            ctx.accounts.kamino_reserve.to_account_info(),
            ctx.accounts.kamino_reserve_liquidity.to_account_info(),
            ctx.accounts.kusdc_mint.to_account_info(),
            ctx.accounts.adapter_vault.to_account_info(),
            ctx.accounts.dispatcher_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.kamino_program.to_account_info(),
        ],
        &[],
    ).map_err(|_| AdapterError::KaminoCpiFailed)?;

    let (_prog, data) = anchor_lang::solana_program::program::get_return_data()
        .unwrap_or_default();
    if data.len() >= 8 {
        Ok(u64::from_le_bytes(data[..8].try_into().unwrap()))
    } else {
        Ok(_expected_out)
    }
}

#[error_code]
pub enum AdapterError {
    #[msg("Invalid token mint for this adapter")]
    InvalidMint,
    #[msg("Slippage exceeded")]
    SlippageExceeded,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("CPI to Kamino program failed")]
    KaminoCpiFailed,
    #[msg("Invalid Kamino reserve account")]
    InvalidReserve,
    #[msg("Invalid Kamino lending market")]
    InvalidMarket,
    #[msg("Could not parse Kamino reserve data")]
    InvalidReserveData,
}

#[event]
pub struct KaminoDepositEvent {
    pub amount_in: u64,
    pub shares_minted: u64,
    pub exchange_rate: u128,
    pub slot: u64,
}

#[event]
pub struct KaminoWithdrawEvent {
    pub shares_redeemed: u64,
    pub amount_out: u64,
    pub exchange_rate: u128,
    pub slot: u64,
}
