use anchor_lang::prelude::*;
use crate::state::{DispatcherConfig, Position};
use crate::error::DispatcherError;
use crate::events::HarvestCompleted;

#[derive(Accounts)]
#[instruction()]
pub struct InitializePosition<'info> {
    #[account(
        seeds = [DispatcherConfig::SEED],
        bump = config.bump
    )]
    pub config: Account<'info, DispatcherConfig>,

    #[account(
        init,
        payer = user,
        space = Position::LEN,
        seeds = [
            Position::SEED,
            user.key().as_ref(),
            adapter_program.key().as_ref()
        ],
        bump
    )]
    pub position: Account<'info, Position>,

    /// CHECK: Any registered adapter program
    pub adapter_program: UncheckedAccount<'info>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_handler(ctx: Context<InitializePosition>) -> Result<()> {
    let position = &mut ctx.accounts.position;
    position.owner = ctx.accounts.user.key();
    position.adapter_program = ctx.accounts.adapter_program.key();
    position.shares = 0;
    position.cost_basis = 0;
    position.total_withdrawn = 0;
    position.created_slot = Clock::get()?.slot;
    position.last_action_slot = Clock::get()?.slot;
    position.bump = ctx.bumps.position;

    msg!(
        "Position initialized for user {} on adapter {}",
        ctx.accounts.user.key(),
        ctx.accounts.adapter_program.key()
    );

    Ok(())
}

#[derive(Accounts)]
pub struct Harvest<'info> {
    #[account(
        seeds = [DispatcherConfig::SEED],
        bump = config.bump
    )]
    pub config: Account<'info, DispatcherConfig>,

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

    /// CHECK: Adapter program
    pub adapter_program: UncheckedAccount<'info>,

    #[account(seeds = [b"dispatcher_authority"], bump)]
    pub dispatcher_authority: SystemAccount<'info>,

    #[account(mut)]
    pub user: Signer<'info>,
}

pub fn handler(ctx: Context<Harvest>) -> Result<()> {
    let slot = Clock::get()?.slot;

    emit!(HarvestCompleted {
        user: ctx.accounts.user.key(),
        adapter_program: ctx.accounts.adapter_program.key(),
        yield_harvested: 0,
        slot,
    });

    Ok(())
}
