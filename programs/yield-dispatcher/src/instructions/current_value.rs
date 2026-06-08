use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use crate::state::{DispatcherConfig, Position};
use crate::error::DispatcherError;
use crate::interface::build_current_value_ix_data;

#[derive(Accounts)]
pub struct CurrentValue<'info> {
    #[account(
        seeds = [DispatcherConfig::SEED],
        bump = config.bump
    )]
    pub config: Account<'info, DispatcherConfig>,

    #[account(
        seeds = [
            Position::SEED,
            user.key().as_ref(),
            adapter_program.key().as_ref()
        ],
        bump = position.bump
    )]
    pub position: Account<'info, Position>,

    /// CHECK: Adapter program
    pub adapter_program: UncheckedAccount<'info>,

    #[account(seeds = [b"dispatcher_authority"], bump)]
    pub dispatcher_authority: SystemAccount<'info>,

    pub user: Signer<'info>,
}

pub fn handler(ctx: Context<CurrentValue>, shares: u64) -> Result<u64> {
    let adapter_key = ctx.accounts.adapter_program.key();
    let authority_bump = ctx.bumps.dispatcher_authority;
    let signer_seeds: &[&[&[u8]]] = &[&[b"dispatcher_authority", &[authority_bump]]];

    let ix_data = build_current_value_ix_data(shares);

    let mut account_metas = vec![
        AccountMeta::new_readonly(ctx.accounts.dispatcher_authority.key(), true),
    ];
    for acc in ctx.remaining_accounts.iter() {
        account_metas.push(if acc.is_writable {
            AccountMeta::new(acc.key(), false)
        } else {
            AccountMeta::new_readonly(acc.key(), false)
        });
    }

    let ix = Instruction {
        program_id: adapter_key,
        accounts: account_metas,
        data: ix_data,
    };

    let mut cpi_accounts = vec![ctx.accounts.dispatcher_authority.to_account_info()];
    for acc in ctx.remaining_accounts.iter() {
        cpi_accounts.push(acc.to_account_info());
    }

    invoke_signed(&ix, &cpi_accounts, signer_seeds)
        .map_err(|_| DispatcherError::AdapterCpiFailed)?;

    let (_program_id, data) = anchor_lang::solana_program::program::get_return_data()
        .ok_or(DispatcherError::AdapterCpiFailed)?;
    require!(data.len() >= 8, DispatcherError::AdapterCpiFailed);

    let value = u64::from_le_bytes(data[..8].try_into().unwrap());
    Ok(value)
}
