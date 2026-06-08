Solana Yield Adapter Standard — Specification v0.1

Status: Draft
Authors: Yield Adapter Working Group
Last updated: 2026-06

Table of Contents

Overview
Motivation
Terminology
Architecture
Core Interface
Instruction Specification
Return Data Protocol
Account Layout Requirements
Share Accounting
Registry & Governance
Error Handling
Security Requirements
Versioning
Reference Implementations

1. Overview
The Solana Yield Adapter Standard defines a minimal, extensible interface that allows any yield-bearing protocol on Solana to be routed through a single Dispatcher program. Integrators write one integration against the Dispatcher and instantly gain access to every approved adapter.
User → Dispatcher → Adapter → Underlying Protocol
         (router)   (wrapper)  (Kamino / MarginFi / etc.)
The standard has three parts:
ComponentDescriptionDispatcherOn-chain router that validates, routes, and accounts for all yield operationsAdapterThin wrapper around a specific protocol exposing the standard interfaceRegistryGovernance-gated approval list of adapters the Dispatcher will route to
2. Motivation
Every Solana yield protocol has a different instruction set, account layout, and share accounting model. Aggregators and portfolio managers must write and maintain bespoke integrations for each one.
This standard solves that by requiring adapters to implement three instructions with fixed discriminators. The Dispatcher can then route to any adapter without knowing the underlying protocol.
Design goals:

Minimal — three mandatory instructions, nothing more required
Extensible — adapters may add extra instructions beyond the standard three
Protocol-agnostic — the Dispatcher never imports adapter crates
Safe — all adapters must be registered and approved before routing
Composable — adapters are standalone programs; they work with or without the Dispatcher

3. Terminology
TermDefinitionAdapterAn Anchor program that wraps a yield protocol and exposes the standard interfaceDispatcherThe canonical router program (yield-dispatcher)RegistryThe on-chain approval list program (adapter-registry)SharesThe unit of account representing a user's proportional claim on deposited assetsExchange RateThe current ratio of underlying tokens per share, scaled to 10^9Dispatcher AuthorityA PDA (seeds = ["dispatcher_authority"]) that acts as the signer in all adapter CPIsPositionA PDA (seeds = ["position", user, adapter_program]) tracking a user's shares in one adapter
4. Architecture
4.1 System diagram
User Wallet
     |
     | deposit / withdraw / current_value
     v
Yield Dispatcher Program
     |
     | 1. Check registry: is adapter Active?
     | 2. Transfer tokens user -> dispatcher vault
     | 3. CPI -> adapter
     | 4. Parse return data
     | 5. Update Position + AdapterState
     | 6. Emit event
     |
     |---- CPI ----> Kamino Adapter ----> Kamino Protocol
     |---- CPI ----> MarginFi Adapter --> MarginFi Protocol
     |---- CPI ----> Drift Adapter -----> Drift Protocol
4.2 Registry flow
Adapter Team  -->  propose_adapter  -->  Registry (Pending)
Governance    -->  approve_adapter  -->  Registry (Active)
Dispatcher    -->  validate         -->  reads status byte
5. Core Interface
Every adapter MUST implement exactly these three instructions:
rust/// Deposit amount of the input token into the underlying protocol.
/// MUST set return data: [shares_minted: u64 LE][exchange_rate: u128 LE]
pub fn adapter_deposit(ctx: Context<Deposit>, amount: u64, min_shares_out: u64) -> Result<u64>;

/// Withdraw shares from the underlying protocol.
/// MUST set return data: [amount_out: u64 LE][exchange_rate: u128 LE]
pub fn adapter_withdraw(ctx: Context<Withdraw>, shares: u64, min_amount_out: u64) -> Result<u64>;

/// Return the current value of shares in base token units.
/// MUST NOT mutate any state.
/// MUST set return data: [value: u64 LE][exchange_rate: u128 LE]
pub fn adapter_current_value(ctx: Context<CurrentValue>, shares: u64) -> Result<u64>;
5.1 Mandatory invariants
Every adapter MUST uphold these invariants:

Non-zero return data — all three instructions must call set_return_data before returning
Slippage enforcement — adapter_deposit must fail if shares_minted < min_shares_out
Mint check — the adapter must verify that the input token account holds the expected mint
Authority check — the adapter must accept dispatcher_authority as the vault signer
No self-signed CPIs — adapters must not generate their own PDA signers that conflict with the dispatcher's

6. Instruction Specification
6.1 Fixed discriminators
InstructionDiscriminator (hex)adapter_depositf2 23 c6 89 52 e1 f2 b6adapter_withdrawb7 12 46 9c 94 67 33 f4adapter_current_value45 a0 37 31 61 c3 28 21
6.2 Instruction data layout
adapter_deposit
[0..8]   discriminator  (8 bytes)
[8..16]  amount         (u64, little-endian)
[16..24] min_shares_out (u64, little-endian)
adapter_withdraw
[0..8]   discriminator  (8 bytes)
[8..16]  shares         (u64, little-endian)
[16..24] min_amount_out (u64, little-endian)
adapter_current_value
[0..8]  discriminator (8 bytes)
[8..16] shares        (u64, little-endian)
6.3 Mandatory account order
IndexAccountConstraint0adapter_vaultWritable token account1dispatcher_authorityRead-only signer
7. Return Data Protocol
7.1 Return data layout
All three instructions use the same 24-byte layout:
[0..8]   primary_amount  (u64 LE)
[8..24]  exchange_rate   (u128 LE)
7.2 Exchange rate precision
actual_rate = exchange_rate_u128 / 10^9
Example: if 1 kUSDC = 1.02 USDC, the stored value is 1_020_000_000.
8. Account Layout Requirements
8.1 Dispatcher Authority
seeds = [b"dispatcher_authority"]
program_id = YIELD_DISPATCHER_PROGRAM_ID
8.2 Registry Entry layout
Status byte at offset 72:
ValueMeaning0Pending1Active2Deprecated3Rejected
9. Share Accounting
9.1 Reference formulas
ProtocolFormulaKaminokUSDC = USDC * 10^9 / collateral_exchange_rateMarginFishares = USDC * 10^12 / asset_share_valueJupiter LPJLP_price = pool_aum * 10^6 / jlp_supplyMaple SyrupsyUSDC = USDC * 10^9 / (total_assets / total_supply)Drift IFrate = if_vault_balance * 10^9 / total_if_shares
10. Registry & Governance
10.1 Lifecycle
propose_adapter --> [Pending] --> approve_adapter --> [Active]
                            `--> reject_adapter  --> [Rejected]

[Active] --> deprecate_adapter --> [Deprecated]
10.2 On-chain validation
The Dispatcher reads the registry entry on every deposit and withdraw. Deprecating an adapter takes effect immediately on the next transaction.
11. Error Handling
CodeNameWhen6000PausedDispatcher is globally paused6001AdapterNotRegisteredRegistry entry PDA missing6002AdapterNotActiveRegistry status not 16003ZeroAmountamount == 06004SlippageExceededshares_minted < min_shares_out6005WithdrawSlippageExceededamount_out < min_amount_out6006InsufficientSharesPosition shares < requested6007MathOverflowArithmetic overflow6008AdapterMismatchProgram ID mismatch6009InvalidFeeBpsFee > 10000 bps6010UnauthorizedCaller is not the authority6011AdapterCpiFailedCPI to adapter failed6012InvalidMintToken mint mismatch
12. Security Requirements
12.1 Mandatory checks
rust// 1. Verify input mint
require!(token_account.mint == EXPECTED_MINT, AdapterError::InvalidMint);

// 2. Enforce slippage
require!(output >= min_output, AdapterError::SlippageExceeded);

// 3. Check for math overflow
let result = a.checked_mul(b).ok_or(AdapterError::MathOverflow)?;
12.2 What adapters MUST NOT do

Must not transfer tokens directly to/from the user
Must not generate their own signer PDAs that overlap with Dispatcher seeds
Must not call set_return_data with fewer than 8 bytes
Must not mutate state in adapter_current_value

13. Versioning
Change TypeVersion BumpNew optional instructionMinor (0.x)New mandatory instructionMajor (x.0)Breaking account layout changeMajor (x.0)Discriminator changeMajor (x.0)
14. Reference Implementations
AdapterProtocolSourcekamino-usdcKamino Financeadapters/kamino-usdc/marginfi-usdcMarginFiadapters/marginfi-usdc/jupiter-lpJupiter Perpsadapters/jupiter-lp/maple-syrupMaple Financeadapters/maple-syrup/drift-insuranceDrift Protocoladapters/drift-insurance/
For implementation questions see BUILD_YOUR_OWN_ADAPTER.md.
