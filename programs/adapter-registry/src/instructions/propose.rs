use anchor_lang::prelude::*;
use crate::state::{RegistryConfig, AdapterEntry, status};
use crate::error::RegistryError;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ProposeAdapterParams {
    pub input_mint: Pubkey,
    pub protocol_name: String,
    pub description: String,
}

#[derive(Accounts)]
#[instruction(params: ProposeAdapterParams)]
pub struct ProposeAdapter<'info> {
    #[account(
        mut,
        seeds = [RegistryConfig::SEED],
        bump = config.bump
    )]
    pub config: Account<'info, RegistryConfig>,

    #[account(
        init,
        payer = proposer,
        space = AdapterEntry::LEN,
        seeds = [AdapterEntry::SEED, adapter_program.key().as_ref()],
        bump
    )]
    pub adapter_entry: Account<'info, AdapterEntry>,

    /// CHECK: The program being proposed as an adapter
    pub adapter_program: UncheckedAccount<'info>,

    #[account(mut)]
    pub proposer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<ProposeAdapter>, params: ProposeAdapterParams) -> Result<()> {
    require!(
        params.protocol_name.len() <= 64,
        RegistryError::NameTooLong
    );
    require!(
        params.description.len() <= 256,
        RegistryError::DescriptionTooLong
    );

    let entry = &mut ctx.accounts.adapter_entry;
    entry.adapter_program = ctx.accounts.adapter_program.key();
    entry.input_mint = params.input_mint;
    entry.status = status::PENDING;
    entry.proposer = ctx.accounts.proposer.key();
    entry.proposed_slot = Clock::get()?.slot;
    entry.bump = ctx.bumps.adapter_entry;

    let name_bytes = params.protocol_name.as_bytes();
    entry.protocol_name[..name_bytes.len()].copy_from_slice(name_bytes);
    entry.protocol_name_len = name_bytes.len() as u8;

    let desc_bytes = params.description.as_bytes();
    entry.description[..desc_bytes.len()].copy_from_slice(desc_bytes);
    entry.description_len = desc_bytes.len() as u16;

    let config = &mut ctx.accounts.config;
    config.total_proposed = config.total_proposed.saturating_add(1);

    msg!(
        "Adapter proposed: {} ({})",
        params.protocol_name,
        ctx.accounts.adapter_program.key()
    );

    Ok(())
}
