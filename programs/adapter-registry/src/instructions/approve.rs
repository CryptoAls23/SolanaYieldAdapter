use anchor_lang::prelude::*;
use crate::state::{RegistryConfig, AdapterEntry, status};
use crate::error::RegistryError;

#[derive(Accounts)]
pub struct ApproveAdapter<'info> {
    #[account(
        mut,
        seeds = [RegistryConfig::SEED],
        bump = config.bump,
        has_one = governance @ RegistryError::Unauthorized
    )]
    pub config: Account<'info, RegistryConfig>,

    #[account(mut)]
    pub adapter_entry: Account<'info, AdapterEntry>,

    pub governance: Signer<'info>,
}

pub fn handler(ctx: Context<ApproveAdapter>) -> Result<()> {
    let entry = &mut ctx.accounts.adapter_entry;
    require!(entry.status == status::PENDING, RegistryError::NotPending);

    entry.status = status::ACTIVE;
    entry.actioned_slot = Clock::get()?.slot;

    let config = &mut ctx.accounts.config;
    config.total_active = config.total_active.saturating_add(1);

    msg!("Adapter approved: {}", entry.adapter_program);
    Ok(())
}
