use anchor_lang::prelude::*;
use crate::state::RegistryConfig;

#[derive(Accounts)]
pub struct InitializeRegistry<'info> {
    #[account(
        init,
        payer = payer,
        space = RegistryConfig::LEN,
        seeds = [RegistryConfig::SEED],
        bump
    )]
    pub config: Account<'info, RegistryConfig>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitializeRegistry>, governance: Pubkey) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.governance = governance;
    config.total_proposed = 0;
    config.total_active = 0;
    config.bump = ctx.bumps.config;
    msg!("Registry initialized with governance: {}", governance);
    Ok(())
}
