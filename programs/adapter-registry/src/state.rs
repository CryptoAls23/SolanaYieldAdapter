use anchor_lang::prelude::*;

#[error_code]
pub enum RegistryError {
    #[msg("Unauthorized: caller is not the governance authority")]
    Unauthorized,

    #[msg("Adapter is not in Pending state")]
    NotPending,

    #[msg("Adapter is not in Active state")]
    NotActive,

    #[msg("Adapter name too long (max 64 chars)")]
    NameTooLong,

    #[msg("Adapter description too long (max 256 chars)")]
    DescriptionTooLong,

    #[msg("Reason string too long (max 128 chars)")]
    ReasonTooLong,

    #[msg("Adapter already exists in the registry")]
    AlreadyExists,
}

#[account]
#[derive(Default)]
pub struct RegistryConfig {
    pub governance: Pubkey,
    pub total_proposed: u32,
    pub total_active: u32,
    pub bump: u8,
    pub _reserved: [u8; 32],
}

impl RegistryConfig {
    pub const LEN: usize = 8 + 32 + 4 + 4 + 1 + 32;
    pub const SEED: &'static [u8] = b"registry_config";
}

#[account]
pub struct AdapterEntry {
    pub adapter_program: Pubkey,
    pub input_mint: Pubkey,
    pub status: u8,
    pub protocol_name: [u8; 64],
    pub description: [u8; 256],
    pub protocol_name_len: u8,
    pub description_len: u16,
    pub proposer: Pubkey,
    pub proposed_slot: u64,
    pub actioned_slot: u64,
    pub action_reason: [u8; 128],
    pub action_reason_len: u8,
    pub bump: u8,
    pub _reserved: [u8; 16],
}

impl AdapterEntry {
    pub const LEN: usize = 8
        + 32
        + 32
        + 1
        + 64
        + 256
        + 1
        + 2
        + 32
        + 8
        + 8
        + 128
        + 1
        + 1
        + 16;

    pub const SEED: &'static [u8] = b"adapter_entry";

    pub fn status_label(&self) -> &'static str {
        match self.status {
            0 => "Pending",
            1 => "Active",
            2 => "Deprecated",
            3 => "Rejected",
            _ => "Unknown",
        }
    }
}

pub mod status {
    pub const PENDING: u8 = 0;
    pub const ACTIVE: u8 = 1;
    pub const DEPRECATED: u8 = 2;
    pub const REJECTED: u8 = 3;
}
