use anchor_lang::prelude::*;
use crate::state::RegistryConfig;
use crate::error::RegistryError;

#[derive(Accounts)]
pub struct TransferGovernance<'info> {
    #[account(
        mut,
        seeds = [RegistryConfig::SEED],
        bump = config.bump,
        has_one = governance @ RegistryError::Unauthorized
    )]
    pub config: Account<'info, RegistryConfig>,

    pub governance: Signer<'info>,
}

pub fn handler(ctx: Context<TransferGovernance>, new_governance: Pubkey) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let old_governance = config.governance;
    config.governance = new_governance;

    msg!(
        "Governance transferred from {} to {}",
        old_governance,
        new_governance
    );
    Ok(())
}
