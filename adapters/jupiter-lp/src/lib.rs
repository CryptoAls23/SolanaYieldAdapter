use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint};

declare_id!("JAdPtJuPiTeRLPXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

pub const JUPITER_PERPS_PROGRAM: Pubkey = pubkey!("PERPHjGBqRHArX4DySjwM6UJHiR3sWAatqfdBS2qQJu");
pub const JLP_MINT: Pubkey = pubkey!("27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4");
pub const JLP_POOL: Pubkey = pubkey!("5BUwFW4nRbftYTDMbgxykoFWqWHPzahFSNAaaaJtVKsq");
pub const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const USDC_CUSTODY: Pubkey = pubkey!("7UUmD7TkBonBaHpJhFApjZE9B9Ftx7Vs2Nez3cFPFbVV");
pub const PRICE_SCALE: u64 = 1_000_000;

#[program]
pub mod jupiter_lp_adapter {
    use super::*;

    pub fn adapter_deposit(ctx: Context<JupiterDeposit>, amount: u64, min_shares_out: u64) -> Result<u64> {
        let clock = Clock::get()?;
        let jlp_price = read_jlp_price(&ctx.accounts.jlp_pool, &ctx.accounts.jlp_mint)?;

        let shares_out = (amount as u128)
            .checked_mul(PRICE_SCALE as u128)
            .ok_or(AdapterError::MathOverflow)?
            .checked_div(jlp_price as u128)
            .ok_or(AdapterError::MathOverflow)? as u64;

        require!(shares_out >= min_shares_out, AdapterError::SlippageExceeded);

        let shares_received = invoke_jupiter_add_liquidity(&ctx, amount, min_shares_out)?;

        let mut return_data = shares_received.to_le_bytes().to_vec();
        return_data.extend_from_slice(&(jlp_price as u128).to_le_bytes());
        anchor_lang::solana_program::program::set_return_data(&return_data);

        emit!(JupiterDepositEvent {
            amount_in: amount,
            jlp_minted: shares_received,
            jlp_price,
            slot: clock.slot,
        });

        msg!("Jupiter LP deposit: {} USDC -> {} JLP (price: {})", amount, shares_received, jlp_price);
        Ok(shares_received)
    }

    pub fn adapter_withdraw(ctx: Context<JupiterWithdraw>, shares: u64, min_amount_out: u64) -> Result<u64> {
        let clock = Clock::get()?;
        let jlp_price = read_jlp_price(&ctx.accounts.jlp_pool, &ctx.accounts.jlp_mint)?;

        let amount_out = (shares as u128)
            .checked_mul(jlp_price as u128)
            .ok_or(AdapterError::MathOverflow)?
            .checked_div(PRICE_SCALE as u128)
            .ok_or(AdapterError::MathOverflow)? as u64;

        require!(amount_out >= min_amount_out, AdapterError::SlippageExceeded);

        let amount_received = invoke_jupiter_remove_liquidity(&ctx, shares, min_amount_out)?;

        let mut return_data = amount_received.to_le_bytes().to_vec();
        return_data.extend_from_slice(&(jlp_price as u128).to_le_bytes());
        anchor_lang::solana_program::program::set_return_data(&return_data);

        emit!(JupiterWithdrawEvent {
            jlp_burned: shares,
            amount_out: amount_received,
            jlp_price,
            slot: clock.slot,
        });

        msg!("Jupiter LP withdraw: {} JLP -> {} USDC", shares, amount_received);
        Ok(amount_received)
    }

    pub fn adapter_current_value(ctx: Context<JupiterCurrentValue>, shares: u64) -> Result<u64> {
        let jlp_price = read_jlp_price(&ctx.accounts.jlp_pool, &ctx.accounts.jlp_mint)?;

        let value = (shares as u128)
            .checked_mul(jlp_price as u128)
            .ok_or(AdapterError::MathOverflow)?
            .checked_div(PRICE_SCALE as u128)
            .ok_or(AdapterError::MathOverflow)? as u64;

        let mut return_data = value.to_le_bytes().to_vec();
        return_data.extend_from_slice(&(jlp_price as u128).to_le_bytes());
        anchor_lang::solana_program::program::set_return_data(&return_data);

        Ok(value)
    }
}

#[derive(Accounts)]
pub struct JupiterDeposit<'info> {
    #[account(mut, constraint = adapter_vault.mint == USDC_MINT @ AdapterError::InvalidMint)]
    pub adapter_vault: Account<'info, TokenAccount>,

    #[account(mut, constraint = user_jlp_account.mint == JLP_MINT @ AdapterError::InvalidMint)]
    pub user_jlp_account: Account<'info, TokenAccount>,

    #[account(mut, address = JLP_MINT)]
    pub jlp_mint: Account<'info, Mint>,

    /// CHECK: JLP pool state
    #[account(mut, address = JLP_POOL @ AdapterError::InvalidPool)]
    pub jlp_pool: AccountInfo<'info>,

    /// CHECK: USDC custody
    #[account(mut, address = USDC_CUSTODY)]
    pub usdc_custody: AccountInfo<'info>,

    /// CHECK: Custody token account
    #[account(mut)]
    pub usdc_custody_token: AccountInfo<'info>,

    /// CHECK: Custody oracle
    pub custody_oracle: AccountInfo<'info>,

    /// CHECK: Dispatcher authority
    #[account(signer)]
    pub dispatcher_authority: AccountInfo<'info>,

    /// CHECK: Jupiter Perps program
    #[account(address = JUPITER_PERPS_PROGRAM)]
    pub jupiter_perps_program: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct JupiterWithdraw<'info> {
    #[account(mut, constraint = adapter_vault.mint == USDC_MINT @ AdapterError::InvalidMint)]
    pub adapter_vault: Account<'info, TokenAccount>,

    #[account(mut, constraint = user_jlp_account.mint == JLP_MINT @ AdapterError::InvalidMint)]
    pub user_jlp_account: Account<'info, TokenAccount>,

    #[account(mut, address = JLP_MINT)]
    pub jlp_mint: Account<'info, Mint>,

    /// CHECK: JLP pool
    #[account(mut, address = JLP_POOL @ AdapterError::InvalidPool)]
    pub jlp_pool: AccountInfo<'info>,

    /// CHECK: USDC custody
    #[account(mut, address = USDC_CUSTODY)]
    pub usdc_custody: AccountInfo<'info>,

    /// CHECK: Custody token account
    #[account(mut)]
    pub usdc_custody_token: AccountInfo<'info>,

    /// CHECK: Custody oracle
    pub custody_oracle: AccountInfo<'info>,

    /// CHECK: Dispatcher authority
    #[account(signer)]
    pub dispatcher_authority: AccountInfo<'info>,

    /// CHECK: Jupiter program
    #[account(address = JUPITER_PERPS_PROGRAM)]
    pub jupiter_perps_program: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct JupiterCurrentValue<'info> {
    /// CHECK: JLP pool for price
    #[account(address = JLP_POOL)]
    pub jlp_pool: AccountInfo<'info>,

    #[account(address = JLP_MINT)]
    pub jlp_mint: Account<'info, Mint>,

    pub dispatcher_authority: AccountInfo<'info>,
}

fn read_jlp_price(pool: &AccountInfo, jlp_mint: &Account<Mint>) -> Result<u64> {
    let data = pool.try_borrow_data()?;

    if data.len() < 24 {
        return Ok(PRICE_SCALE);
    }

    let aum_bytes: [u8; 8] = data[8..16].try_into()
        .map_err(|_| AdapterError::InvalidPoolData)?;
    let aum_usd = u64::from_le_bytes(aum_bytes);
    let total_supply = jlp_mint.supply;

    if total_supply == 0 || aum_usd == 0 {
        return Ok(PRICE_SCALE);
    }

    let price = (aum_usd as u128)
        .checked_mul(PRICE_SCALE as u128)
        .unwrap_or(PRICE_SCALE as u128)
        .checked_div(total_supply as u128)
        .unwrap_or(PRICE_SCALE as u128) as u64;

    Ok(if price == 0 { PRICE_SCALE } else { price })
}

fn invoke_jupiter_add_liquidity(
    ctx: &Context<JupiterDeposit>,
    amount_in: u64,
    min_lp_amount: u64,
) -> Result<u64> {
    let mut ix_data: Vec<u8> = vec![0xe4, 0x4a, 0x57, 0x1e, 0xc8, 0x44, 0x3c, 0x2b];
    ix_data.extend_from_slice(&amount_in.to_le_bytes());
    ix_data.extend_from_slice(&min_lp_amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new_readonly(ctx.accounts.dispatcher_authority.key(), true),
        AccountMeta::new(ctx.accounts.jlp_pool.key(), false),
        AccountMeta::new(ctx.accounts.usdc_custody.key(), false),
        AccountMeta::new_readonly(ctx.accounts.custody_oracle.key(), false),
        AccountMeta::new(ctx.accounts.usdc_custody_token.key(), false),
        AccountMeta::new(ctx.accounts.adapter_vault.key(), false),
        AccountMeta::new(ctx.accounts.user_jlp_account.key(), false),
        AccountMeta::new(ctx.accounts.jlp_mint.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
    ];

    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.jupiter_perps_program.key(),
        accounts,
        data: ix_data,
    };

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.dispatcher_authority.to_account_info(),
            ctx.accounts.jlp_pool.to_account_info(),
            ctx.accounts.usdc_custody.to_account_info(),
            ctx.accounts.custody_oracle.to_account_info(),
            ctx.accounts.usdc_custody_token.to_account_info(),
            ctx.accounts.adapter_vault.to_account_info(),
            ctx.accounts.user_jlp_account.to_account_info(),
            ctx.accounts.jlp_mint.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        &[],
    ).map_err(|_| AdapterError::JupiterCpiFailed)?;

    let (_prog, data) = anchor_lang::solana_program::program::get_return_data()
        .unwrap_or_default();
    if data.len() >= 8 {
        Ok(u64::from_le_bytes(data[..8].try_into().unwrap()))
    } else {
        Ok(min_lp_amount)
    }
}

fn invoke_jupiter_remove_liquidity(
    ctx: &Context<JupiterWithdraw>,
    lp_amount: u64,
    min_out: u64,
) -> Result<u64> {
    let mut ix_data: Vec<u8> = vec![0x80, 0x35, 0x20, 0xe8, 0x4d, 0x07, 0x55, 0x1f];
    ix_data.extend_from_slice(&lp_amount.to_le_bytes());
    ix_data.extend_from_slice(&min_out.to_le_bytes());

    let accounts = vec![
        AccountMeta::new_readonly(ctx.accounts.dispatcher_authority.key(), true),
        AccountMeta::new(ctx.accounts.jlp_pool.key(), false),
        AccountMeta::new(ctx.accounts.usdc_custody.key(), false),
        AccountMeta::new_readonly(ctx.accounts.custody_oracle.key(), false),
        AccountMeta::new(ctx.accounts.usdc_custody_token.key(), false),
        AccountMeta::new(ctx.accounts.user_jlp_account.key(), false),
        AccountMeta::new(ctx.accounts.adapter_vault.key(), false),
        AccountMeta::new(ctx.accounts.jlp_mint.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
    ];

    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.jupiter_perps_program.key(),
        accounts,
        data: ix_data,
    };

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.dispatcher_authority.to_account_info(),
            ctx.accounts.jlp_pool.to_account_info(),
            ctx.accounts.usdc_custody.to_account_info(),
            ctx.accounts.custody_oracle.to_account_info(),
            ctx.accounts.usdc_custody_token.to_account_info(),
            ctx.accounts.user_jlp_account.to_account_info(),
            ctx.accounts.adapter_vault.to_account_info(),
            ctx.accounts.jlp_mint.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        &[],
    ).map_err(|_| AdapterError::JupiterCpiFailed)?;

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
    #[msg("CPI to Jupiter Perps failed")]
    JupiterCpiFailed,
    #[msg("Invalid JLP pool account")]
    InvalidPool,
    #[msg("Could not parse JLP pool data")]
    InvalidPoolData,
}

#[event]
pub struct JupiterDepositEvent {
    pub amount_in: u64,
    pub jlp_minted: u64,
    pub jlp_price: u64,
    pub slot: u64,
}

#[event]
pub struct JupiterWithdrawEvent {
    pub jlp_burned: u64,
    pub amount_out: u64,
    pub jlp_price: u64,
    pub slot: u64,
}
