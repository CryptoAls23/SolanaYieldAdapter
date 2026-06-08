use anchor_lang::prelude::*;
use crate::state::{RegistryConfig, AdapterEntry, status};
use crate::error::RegistryError;

#[derive(Accounts)]
pub struct RejectAdapter<'info> {
    #[account(
        seeds = [RegistryConfig::SEED],
        bump = config.bump,
        has_one = governance @ RegistryError::Unauthorized
    )]
    pub config: Account<'info, RegistryConfig>,

    #[account(mut)]
    pub adapter_entry: Account<'info, AdapterEntry>,

    pub governance: Signer<'info>,
}

pub fn handler(ctx: Context<RejectAdapter>, reason: String) -> Result<()> {
    require!(reason.len() <= 128, RegistryError::ReasonTooLong);

    let entry = &mut ctx.accounts.adapter_entry;
    require!(entry.status == status::PENDING, RegistryError::NotPending);

    entry.status = status::REJECTED;
    entry.actioned_slot = Clock::get()?.slot;

    let reason_bytes = reason.as_bytes();
    entry.action_reason[..reason_bytes.len()].copy_from_slice(reason_bytes);
    entry.action_reason_len = reason_bytes.len() as u8;

    msg!("Adapter rejected: {}, reason: {}", entry.adapter_program, reason);
    Ok(())
}
