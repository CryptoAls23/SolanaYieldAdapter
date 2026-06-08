use anchor_lang::prelude::*;

pub trait AdapterInterface {
    fn deposit(amount: u64, min_shares_out: u64) -> Result<u64>;
    fn withdraw(shares: u64, min_amount_out: u64) -> Result<u64>;
    fn current_value(shares: u64) -> Result<u64>;
}

pub mod discriminators {
    pub const DEPOSIT: [u8; 8] = [0xf2, 0x23, 0xc6, 0x89, 0x52, 0xe1, 0xf2, 0xb6];
    pub const WITHDRAW: [u8; 8] = [0xb7, 0x12, 0x46, 0x9c, 0x94, 0x67, 0x33, 0xf4];
    pub const CURRENT_VALUE: [u8; 8] = [0x45, 0xa0, 0x37, 0x31, 0x61, 0xc3, 0x28, 0x21];
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct AdapterCpiContext {
    pub dispatcher_authority: Pubkey,
    pub nonce: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default)]
pub struct AdapterResult {
    pub amount_out: u64,
    pub exchange_rate: u128,
}

impl AdapterResult {
    pub const LEN: usize = 8 + 16;
}

pub fn build_deposit_ix_data(amount: u64, min_shares_out: u64) -> Vec<u8> {
    let mut data = discriminators::DEPOSIT.to_vec();
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&min_shares_out.to_le_bytes());
    data
}

pub fn build_withdraw_ix_data(shares: u64, min_amount_out: u64) -> Vec<u8> {
    let mut data = discriminators::WITHDRAW.to_vec();
    data.extend_from_slice(&shares.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data
}

pub fn build_current_value_ix_data(shares: u64) -> Vec<u8> {
    let mut data = discriminators::CURRENT_VALUE.to_vec();
    data.extend_from_slice(&shares.to_le_bytes());
    data
}
