use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{DispatcherConfig, AdapterState, Position};
use crate::error::DispatcherError;
use crate::events::WithdrawRouted;
use crate::interface::build_withdraw_ix_data;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
        mut,
        seeds = [DispatcherConfig::SEED],
        bump = config.bump
    )]
    pub config: Account<'info, DispatcherConfig>,

    #[account(
        mut,
        seeds = [AdapterState::SEED, adapter_program.key().as_ref()],
        bump = adapter_state.bump
    )]
    pub adapter_state: Account<'info, AdapterState>,

    #[account(
        mut,
        seeds = [
            Position::SEED,
            user.key().as_ref(),
            adapter_program.key().as_ref()
        ],
        bump = position.bump,
        constraint = position.owner == user.key() @ DispatcherError::Unauthorized
    )]
    pub position: Account<'info, Position>,

    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ DispatcherError::Unauthorized
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = dispatcher_vault.owner == dispatcher_authority.key()
    )]
    pub dispatcher_vault: Account<'info, TokenAccount>,

    /// CHECK: Validated against registry in handler
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

pub fn handler(ctx: Context<Withdraw>, shares: u64, min_amount_out: u64) -> Result<()> {
    let config = &ctx.accounts.config;
    require!(!config.paused, DispatcherError::Paused);
    require!(shares > 0, DispatcherError::ZeroAmount);

    let position = &ctx.accounts.position;
    require!(
        position.shares >= shares as u128,
        DispatcherError::InsufficientShares
    );

    let adapter_key = ctx.accounts.adapter_program.key();

    validate_adapter_registered(
        config.registry,
        adapter_key,
        ctx.remaining_accounts,
    )?;

    let authority_bump = ctx.bumps.dispatcher_authority;
    let signer_seeds: &[&[&[u8]]] = &[&[b"dispatcher_authority", &[authority_bump]]];

    let ix_data = build_withdraw_ix_data(shares, min_amount_out);

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

    let amount_out = parse_amount_from_return_data()?;
    require!(amount_out >= min_amount_out, DispatcherError::WithdrawSlippageExceeded);

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.dispatcher_vault.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.dispatcher_authority.to_account_info(),
            },
            signer_seeds,
        ),
        amount_out,
    )?;

    let slot = Clock::get()?.slot;

    let position = &mut ctx.accounts.position;
    position.shares = position
        .shares
        .checked_sub(shares as u128)
        .ok_or(DispatcherError::MathOverflow)?;
    position.total_withdrawn = position
        .total_withdrawn
        .checked_add(amount_out as u128)
        .ok_or(DispatcherError::MathOverflow)?;
    position.last_action_slot = slot;

    let adapter_state = &mut ctx.accounts.adapter_state;
    adapter_state.total_shares = adapter_state
        .total_shares
        .checked_sub(shares as u128)
        .ok_or(DispatcherError::MathOverflow)?;
    adapter_state.cumulative_withdrawals = adapter_state
        .cumulative_withdrawals
        .checked_add(amount_out as u128)
        .ok_or(DispatcherError::MathOverflow)?;
    adapter_state.last_updated_slot = slot;

    let config = &mut ctx.accounts.config;
    config.total_withdrawals = config
        .total_withdrawals
        .checked_add(amount_out as u128)
        .ok_or(DispatcherError::MathOverflow)?;

    emit!(WithdrawRouted {
        user: ctx.accounts.user.key(),
        adapter_program: adapter_key,
        input_mint: ctx.accounts.user_token_account.mint,
        shares_burned: shares,
        amount_out,
        slot,
    });

    msg!(
        "Withdraw routed: {} shares -> {} tokens from adapter {}",
        shares,
        amount_out,
        adapter_key
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
    let (expected_pda, _) = Pubkey::find_program_address(
        &[b"adapter_entry", adapter.as_ref()],
        &registry,
    );
    require_keys_eq!(entry_info.key(), expected_pda, DispatcherError::AdapterNotRegistered);
    let data = entry_info.try_borrow_data()?;
    require!(data.len() > 73, DispatcherError::AdapterNotRegistered);
    require!(data[72] == 1, DispatcherError::AdapterNotActive);
    Ok(())
}

fn parse_amount_from_return_data() -> Result<u64> {
    let (_program_id, data) = anchor_lang::solana_program::program::get_return_data()
        .ok_or(DispatcherError::AdapterCpiFailed)?;
    require!(data.len() >= 8, DispatcherError::AdapterCpiFailed);
    Ok(u64::from_le_bytes(data[..8].try_into().unwrap()))
}
