use anchor_lang::prelude::*;
use crate::state::DispatcherConfig;
use crate::error::DispatcherError;
use crate::events::DispatcherConfigUpdated;

#[derive(Accounts)]
pub struct InitializeDispatcher<'info> {
    #[account(
        init,
        payer = authority,
        space = DispatcherConfig::LEN,
        seeds = [DispatcherConfig::SEED],
        bump
    )]
    pub config: Account<'info, DispatcherConfig>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<InitializeDispatcher>,
    registry: Pubkey,
    fee_bps: u16,
) -> Result<()> {
    require!(fee_bps <= 10_000, DispatcherError::InvalidFeeBps);

    let config = &mut ctx.accounts.config;
    config.authority = ctx.accounts.authority.key();
    config.registry = registry;
    config.fee_bps = fee_bps;
    config.paused = false;
    config.total_deposits = 0;
    config.total_withdrawals = 0;
    config.adapter_count = 0;
    config.bump = ctx.bumps.config;

    emit!(DispatcherConfigUpdated {
        authority: config.authority,
        new_fee_bps: fee_bps,
        paused: false,
        slot: Clock::get()?.slot,
    });

    msg!(
        "Dispatcher initialized. Registry: {}, Fee: {} bps",
        registry,
        fee_bps
    );

    Ok(())
}
