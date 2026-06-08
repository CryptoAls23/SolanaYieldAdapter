use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::Instruction,
    program::invoke_signed,
};
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{DispatcherConfig, AdapterState, Position};
use crate::error::DispatcherError;
use crate::events::DepositRouted;
use crate::interface::{build_deposit_ix_data, AdapterResult};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        mut,
        seeds = [DispatcherConfig::SEED],
        bump = config.bump
    )]
    pub config: Account<'info, DispatcherConfig>,

    #[account(
        init_if_needed,
        payer = user,
        space = AdapterState::LEN,
        seeds = [AdapterState::SEED, adapter_program.key().as_ref()],
        bump
    )]
    pub adapter_state: Account<'info, AdapterState>,

    #[account(
        mut,
        seeds = [
            Position::SEED,
            user.key().as_ref(),
            adapter_program.key().as_ref()
        ],
        bump = position.bump
    )]
    pub position: Account<'info, Position>,

    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ DispatcherError::Unauthorized,
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = dispatcher_vault.owner == dispatcher_authority.key(),
    )]
    pub dispatcher_vault: Account<'info, TokenAccount>,

    /// CHECK: Adapter program validated against registry in handler
    pub adapter_program: UncheckedAccount<'info>,

    #[account(
        seeds = [b"dispatcher_authority"],
        bump
    )]
    pub dispatcher_authority: SystemAccount<'info>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Deposit>, amount: u64, min_shares_out: u64) -> Result<()> {
    let config = &ctx.accounts.config;

    require!(!config.paused, DispatcherError::Paused);
    require!(amount > 0, DispatcherError::ZeroAmount);

    let adapter_key = ctx.accounts.adapter_program.key();

    validate_adapter_registered(
        ctx.accounts.config.registry,
        adapter_key,
        &ctx.remaining_accounts,
    )?;

    let fee = if config.fee_bps > 0 {
        (amount as u128)
            .checked_mul(config.fee_bps as u128)
            .ok_or(DispatcherError::MathOverflow)?
            .checked_div(10_000)
            .ok_or(DispatcherError::MathOverflow)? as u64
    } else {
        0
    };
    let amount_after_fee = amount.checked_sub(fee).ok_or(DispatcherError::MathOverflow)?;

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.dispatcher_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount,
    )?;

    let authority_bump = ctx.bumps.dispatcher_authority;
    let signer_seeds: &[&[&[u8]]] = &[&[b"dispatcher_authority", &[authority_bump]]];

    let ix_data = build_deposit_ix_data(amount_after_fee, min_shares_out);

    let mut account_metas = vec![
        AccountMeta::new(ctx.accounts.dispatcher_vault.key(), false),
        AccountMeta::new_readonly(ctx.accounts.dispatcher_authority.key(), true),
    ];
    for acc in ctx.remaining_accounts.iter().skip(1) {
        account_metas.push(if acc.is_writable {
            AccountMeta::new(acc.key(), acc.is_signer)
        } else {
            AccountMeta::new_readonly(acc.key(), acc.is_signer)
        });
    }

    let ix = Instruction {
        program_id: adapter_key,
        accounts: account_metas,
        data: ix_data,
    };

    let mut cpi_accounts = vec![
        ctx.accounts.dispatcher_vault.to_account_info(),
        ctx.accounts.dispatcher_authority.to_account_info(),
    ];
    for acc in ctx.remaining_accounts.iter().skip(1) {
        cpi_accounts.push(acc.to_account_info());
    }

    invoke_signed(&ix, &cpi_accounts, signer_seeds)
        .map_err(|_| DispatcherError::AdapterCpiFailed)?;

    let shares_minted = parse_shares_from_return_data()?;
    require!(shares_minted >= min_shares_out, DispatcherError::SlippageExceeded);

    let slot = Clock::get()?.slot;
    let position = &mut ctx.accounts.position;
    position.shares = position
        .shares
        .checked_add(shares_minted as u128)
        .ok_or(DispatcherError::MathOverflow)?;
    position.cost_basis = position
        .cost_basis
        .checked_add(amount_after_fee as u128)
        .ok_or(DispatcherError::MathOverflow)?;
    position.last_action_slot = slot;

    let adapter_state = &mut ctx.accounts.adapter_state;
    adapter_state.total_deposited = adapter_state
        .total_deposited
        .checked_add(amount_after_fee as u128)
        .ok_or(DispatcherError::MathOverflow)?;
    adapter_state.total_shares = adapter_state
        .total_shares
        .checked_add(shares_minted as u128)
        .ok_or(DispatcherError::MathOverflow)?;
    adapter_state.cumulative_deposits = adapter_state
        .cumulative_deposits
        .checked_add(amount_after_fee as u128)
        .ok_or(DispatcherError::MathOverflow)?;
    adapter_state.last_updated_slot = slot;

    let config = &mut ctx.accounts.config;
    config.total_deposits = config
        .total_deposits
        .checked_add(amount_after_fee as u128)
        .ok_or(DispatcherError::MathOverflow)?;

    emit!(DepositRouted {
        user: ctx.accounts.user.key(),
        adapter_program: adapter_key,
        input_mint: ctx.accounts.user_token_account.mint,
        amount_in: amount_after_fee,
        shares_minted,
        fee_charged: fee,
        slot,
    });

    msg!(
        "Deposit routed: {} tokens -> adapter {}, {} shares minted",
        amount_after_fee,
        adapter_key,
        shares_minted
    );

    Ok(())
}

fn validate_adapter_registered(
    registry: Pubkey,
    adapter: Pubkey,
    remaining_accounts: &[AccountInfo],
) -> Result<()> {
    require!(!remaining_accounts.is_empty(), DispatcherError::AdapterNotRegistered);

    let entry_info = &remaining_accounts[0];

    let (expected_pda, _bump) = Pubkey::find_program_address(
        &[b"adapter_entry", adapter.as_ref()],
        &registry,
    );
    require_keys_eq!(
        entry_info.key(),
        expected_pda,
        DispatcherError::AdapterNotRegistered
    );

    let data = entry_info.try_borrow_data()?;
    require!(data.len() > 73, DispatcherError::AdapterNotRegistered);

    let status = data[72];
    require!(status == 1, DispatcherError::AdapterNotActive);

    Ok(())
}

fn parse_shares_from_return_data() -> Result<u64> {
    let (program_id, data) = anchor_lang::solana_program::program::get_return_data()
        .ok_or(DispatcherError::AdapterCpiFailed)?;

    require!(data.len() >= 8, DispatcherError::AdapterCpiFailed);

    let shares = u64::from_le_bytes(data[..8].try_into().unwrap());
    Ok(shares)
}
