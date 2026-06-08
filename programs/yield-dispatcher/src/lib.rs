use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint};
use anchor_spl::associated_token::AssociatedToken;

declare_id!("YieLdDiSPaTcHeRvAuLtXXXXXXXXXXXXXXXXXXXXXXX");

pub mod error;
pub mod state;
pub mod instructions;
pub mod interface;
pub mod events;

use instructions::*;

#[program]
pub mod yield_dispatcher {
    use super::*;

    pub fn initialize_dispatcher(
        ctx: Context<InitializeDispatcher>,
        registry: Pubkey,
        fee_bps: u16,
    ) -> Result<()> {
        instructions::initialize::handler(ctx, registry, fee_bps)
    }

    pub fn update_dispatcher_config(
        ctx: Context<UpdateDispatcherConfig>,
        new_fee_bps: Option<u16>,
        paused: Option<bool>,
    ) -> Result<()> {
        instructions::update_config::handler(ctx, new_fee_bps, paused)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64, min_shares_out: u64) -> Result<()> {
        instructions::deposit::handler(ctx, amount, min_shares_out)
    }

    pub fn withdraw(ctx: Context<Withdraw>, shares: u64, min_amount_out: u64) -> Result<()> {
        instructions::withdraw::handler(ctx, shares, min_amount_out)
    }

    pub fn current_value(ctx: Context<CurrentValue>, shares: u64) -> Result<u64> {
        instructions::current_value::handler(ctx, shares)
    }

    pub fn initialize_position(ctx: Context<InitializePosition>) -> Result<()> {
        instructions::position::initialize_handler(ctx)
    }

    pub fn harvest(ctx: Context<Harvest>) -> Result<()> {
        instructions::harvest::handler(ctx)
    }
}
