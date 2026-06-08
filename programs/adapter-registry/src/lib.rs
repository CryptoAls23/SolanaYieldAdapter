use anchor_lang::prelude::*;

declare_id!("AdPtReGiStRyXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

pub mod error;
pub mod state;
pub mod instructions;

use instructions::*;

#[program]
pub mod adapter_registry {
    use super::*;

    pub fn initialize_registry(
        ctx: Context<InitializeRegistry>,
        governance: Pubkey,
    ) -> Result<()> {
        instructions::initialize::handler(ctx, governance)
    }

    pub fn propose_adapter(
        ctx: Context<ProposeAdapter>,
        params: ProposeAdapterParams,
    ) -> Result<()> {
        instructions::propose::handler(ctx, params)
    }

    pub fn approve_adapter(ctx: Context<ApproveAdapter>) -> Result<()> {
        instructions::approve::handler(ctx)
    }

    pub fn reject_adapter(ctx: Context<RejectAdapter>, reason: String) -> Result<()> {
        instructions::reject::handler(ctx, reason)
    }

    pub fn deprecate_adapter(ctx: Context<DeprecateAdapter>, reason: String) -> Result<()> {
        instructions::deprecate::handler(ctx, reason)
    }

    pub fn transfer_governance(
        ctx: Context<TransferGovernance>,
        new_governance: Pubkey,
    ) -> Result<()> {
        instructions::governance::handler(ctx, new_governance)
    }
}
