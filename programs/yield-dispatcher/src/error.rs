use anchor_lang::prelude::*;

#[error_code]
pub enum DispatcherError {
    #[msg("Dispatcher is currently paused")]
    Paused,

    #[msg("Adapter is not registered in the approved registry")]
    AdapterNotRegistered,

    #[msg("Adapter is not in an active state")]
    AdapterNotActive,

    #[msg("Deposit amount is zero")]
    ZeroAmount,

    #[msg("Slippage tolerance exceeded: received fewer shares than min_shares_out")]
    SlippageExceeded,

    #[msg("Withdrawal slippage exceeded: received fewer tokens than min_amount_out")]
    WithdrawSlippageExceeded,

    #[msg("Position has insufficient shares for this withdrawal")]
    InsufficientShares,

    #[msg("Arithmetic overflow in share calculation")]
    MathOverflow,

    #[msg("Adapter program ID does not match registered adapter")]
    AdapterMismatch,

    #[msg("Fee basis points must be <= 10000")]
    InvalidFeeBps,

    #[msg("Unauthorized: caller is not the dispatcher authority")]
    Unauthorized,

    #[msg("CPI call to adapter failed")]
    AdapterCpiFailed,

    #[msg("Invalid mint: token mint does not match adapter's expected mint")]
    InvalidMint,

    #[msg("Position already initialized")]
    PositionAlreadyExists,
}
