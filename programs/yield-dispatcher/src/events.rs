use anchor_lang::prelude::*;

#[event]
pub struct DepositRouted {
    pub user: Pubkey,
    pub adapter_program: Pubkey,
    pub input_mint: Pubkey,
    pub amount_in: u64,
    pub shares_minted: u64,
    pub fee_charged: u64,
    pub slot: u64,
}

#[event]
pub struct WithdrawRouted {
    pub user: Pubkey,
    pub adapter_program: Pubkey,
    pub input_mint: Pubkey,
    pub shares_burned: u64,
    pub amount_out: u64,
    pub slot: u64,
}

#[event]
pub struct DispatcherConfigUpdated {
    pub authority: Pubkey,
    pub new_fee_bps: u16,
    pub paused: bool,
    pub slot: u64,
}

#[event]
pub struct HarvestCompleted {
    pub user: Pubkey,
    pub adapter_program: Pubkey,
    pub yield_harvested: u64,
    pub slot: u64,
}
