use anchor_lang::prelude::*;
use crate::state::DispatcherConfig;
use crate::error::DispatcherError;
use crate::events::DispatcherConfigUpdated;

#[derive(Accounts)]
pub struct UpdateDispatcherConfig<'info> {
    #[account(
        mut,
        seeds = [DispatcherConfig::SEED],
        bump = config.bump,
        has_one = authority @ DispatcherError::Unauthorized
    )]
    pub config: Account<'info, DispatcherConfig>,

    pub authority: Signer<'info>,
}

pub fn handler(
    ctx: Context<UpdateDispatcherConfig>,
    new_fee_bps: Option<u16>,
    paused: Option<bool>,
) -> Result<()> {
    let config = &mut ctx.accounts.config;

    if let Some(fee) = new_fee_bps {
        require!(fee <= 10_000, DispatcherError::InvalidFeeBps);
        config.fee_bps = fee;
    }

    if let Some(p) = paused {
        config.paused = p;
    }

    emit!(DispatcherConfigUpdated {
        authority: config.authority,
        new_fee_bps: config.fee_bps,
        paused: config.paused,
        slot: Clock::get()?.slot,
    });

    Ok(())
}
