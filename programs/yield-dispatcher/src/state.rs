use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct DispatcherConfig {
    pub authority: Pubkey,
    pub registry: Pubkey,
    pub fee_bps: u16,
    pub paused: bool,
    pub total_deposits: u128,
    pub total_withdrawals: u128,
    pub adapter_count: u32,
    pub bump: u8,
    pub _reserved: [u8; 64],
}

impl DispatcherConfig {
    pub const LEN: usize = 8
        + 32
        + 32
        + 2
        + 1
        + 16
        + 16
        + 4
        + 1
        + 64;

    pub const SEED: &'static [u8] = b"dispatcher_config";
}

#[account]
#[derive(Default)]
pub struct AdapterState {
    pub adapter_program: Pubkey,
    pub input_mint: Pubkey,
    pub total_deposited: u128,
    pub total_shares: u128,
    pub cumulative_deposits: u128,
    pub cumulative_withdrawals: u128,
    pub last_updated_slot: u64,
    pub bump: u8,
    pub _reserved: [u8; 32],
}

impl AdapterState {
    pub const LEN: usize = 8
        + 32
        + 32
        + 16
        + 16
        + 16
        + 16
        + 8
        + 1
        + 32;

    pub const SEED: &'static [u8] = b"adapter_state";
}

#[account]
#[derive(Default)]
pub struct Position {
    pub owner: Pubkey,
    pub adapter_program: Pubkey,
    pub shares: u128,
    pub cost_basis: u128,
    pub total_withdrawn: u128,
    pub created_slot: u64,
    pub last_action_slot: u64,
    pub bump: u8,
    pub _reserved: [u8; 32],
}

impl Position {
    pub const LEN: usize = 8
        + 32
        + 32
        + 16
        + 16
        + 16
        + 8
        + 8
        + 1
        + 32;

    pub const SEED: &'static [u8] = b"position";
}
