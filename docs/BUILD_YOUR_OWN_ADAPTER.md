# Build Your Own Adapter

> **Goal:** Ship a working adapter in under a day.
> **Prerequisites:** Rust, Anchor 0.31.1, Solana CLI 2.2.20, Node.js 18+

## Table of Contents

1. [What you're building](#1-what-youre-building)
2. [Setup (15 min)](#2-setup-15-min)
3. [Create the adapter program (30 min)](#3-create-the-adapter-program-30-min)
4. [Implement the three instructions (2-3 h)](#4-implement-the-three-instructions-23-h)
5. [Write tests (1 h)](#5-write-tests-1-h)
6. [Register on devnet (15 min)](#6-register-on-devnet-15-min)
7. [Checklist before submitting](#7-checklist-before-submitting)
8. [FAQ & common mistakes](#8-faq--common-mistakes)

## 1. What you're building

An adapter is a small Anchor program that wraps one yield protocol and exposes three instructions:
adapter_deposit(amount, min_shares_out)  -> shares_minted
adapter_withdraw(shares, min_amount_out) -> amount_out
adapter_current_value(shares)            -> value_in_usdc

The Dispatcher calls your adapter via CPI. Your adapter calls the underlying protocol via CPI.
Dispatcher --> Your Adapter --> Underlying Protocol
<-- return_data --

## 2. Setup (15 min)

### 2.1 Fork and clone the repo
git clone https://github.com/your-org/solana-yield-adapter
cd solana-yield-adapter

### 2.2 Install dependencies
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup component add rustfmt clippy
sh -c "$(curl -sSfL https://release.solana.com/v2.2.20/install)"
cargo install --git https://github.com/coral-xyz/anchor avm --locked
avm install 0.31.1
avm use 0.31.1
yarn install

### 2.3 Verify
anchor --version   # anchor-cli 0.31.1
solana --version   # solana-cli 2.2.20

### 2.4 Create your adapter directory
mkdir -p adapters/my-protocol/src
cp adapters/kamino-usdc/Cargo.toml adapters/my-protocol/Cargo.toml

Edit adapters/my-protocol/Cargo.toml:
[package]
name = "my-protocol-adapter"
version = "0.1.0"
edition = "2021"
[lib]
crate-type = ["cdylib", "lib"]
name = "my_protocol_adapter"
[features]
default = []
cpi = ["no-entrypoint"]
no-entrypoint = []
no-idl = []
idl-build = ["anchor-lang/idl-build", "anchor-spl/idl-build"]
[dependencies]
anchor-lang = { workspace = true }
anchor-spl = { workspace = true }
borsh = { workspace = true }

Add to workspace Cargo.toml:
[workspace]
members = [
"adapters/my-protocol",
]

## 3. Create the adapter program (30 min)

Create adapters/my-protocol/src/lib.rs:
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
declare_id!("MyProtocolAdapterXXXXXXXXXXXXXXXXXXXXXXXXXXX");
pub const MY_PROTOCOL_PROGRAM_ID: Pubkey =
pubkey!("<MY_PROTOCOL_PROGRAM_ID>");
pub const USDC_MINT: Pubkey =
pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const RATE_SCALE: u64 = 1_000_000_000;
#[program]
pub mod my_protocol_adapter {
use super::*;
pub fn adapter_deposit(
    ctx: Context<MyDeposit>,
    amount: u64,
    min_shares_out: u64,
) -> Result<u64> {
    todo!("implement deposit")
}

pub fn adapter_withdraw(
    ctx: Context<MyWithdraw>,
    shares: u64,
    min_amount_out: u64,
) -> Result<u64> {
    todo!("implement withdraw")
}

pub fn adapter_current_value(
    ctx: Context<MyCurrentValue>,
    shares: u64,
) -> Result<u64> {
    todo!("implement current_value")
}
}
#[derive(Accounts)]
pub struct MyDeposit<'info> {
#[account(mut, constraint = adapter_vault.mint == USDC_MINT @ AdapterError::InvalidMint)]
pub adapter_vault: Account<'info, TokenAccount>,
/// CHECK: Provided and signed by the Dispatcher CPI
#[account(signer)]
pub dispatcher_authority: AccountInfo<'info>,

pub token_program: Program<'info, Token>,
}
#[derive(Accounts)]
pub struct MyWithdraw<'info> {
#[account(mut, constraint = adapter_vault.mint == USDC_MINT @ AdapterError::InvalidMint)]
pub adapter_vault: Account<'info, TokenAccount>,
/// CHECK: Dispatcher authority
#[account(signer)]
pub dispatcher_authority: AccountInfo<'info>,

pub token_program: Program<'info, Token>,
}
#[derive(Accounts)]
pub struct MyCurrentValue<'info> {
/// CHECK: Dispatcher authority
pub dispatcher_authority: AccountInfo<'info>,
}
#[error_code]
pub enum AdapterError {
#[msg("Invalid token mint")]
InvalidMint,
#[msg("Slippage tolerance exceeded")]
SlippageExceeded,
#[msg("Arithmetic overflow")]
MathOverflow,
#[msg("CPI to underlying protocol failed")]
ProtocolCpiFailed,
}

## 4. Implement the three instructions (2-3 h)

### 4.1 adapter_deposit
pub fn adapter_deposit(
ctx: Context<MyDeposit>,
amount: u64,
min_shares_out: u64,
) -> Result<u64> {
let exchange_rate = read_exchange_rate(&ctx.accounts.protocol_state)?;
let shares_out = (amount as u128)
    .checked_mul(RATE_SCALE as u128)
    .ok_or(AdapterError::MathOverflow)?
    .checked_div(exchange_rate as u128)
    .ok_or(AdapterError::MathOverflow)? as u64;

require!(shares_out >= min_shares_out, AdapterError::SlippageExceeded);

invoke_protocol_deposit(&ctx, amount)?;

let mut return_data = shares_out.to_le_bytes().to_vec();
return_data.extend_from_slice(&(exchange_rate as u128).to_le_bytes());
anchor_lang::solana_program::program::set_return_data(&return_data);

Ok(shares_out)
}

### 4.2 adapter_withdraw
pub fn adapter_withdraw(
ctx: Context<MyWithdraw>,
shares: u64,
min_amount_out: u64,
) -> Result<u64> {
let exchange_rate = read_exchange_rate(&ctx.accounts.protocol_state)?;
let amount_out = (shares as u128)
    .checked_mul(exchange_rate as u128)
    .ok_or(AdapterError::MathOverflow)?
    .checked_div(RATE_SCALE as u128)
    .ok_or(AdapterError::MathOverflow)? as u64;

require!(amount_out >= min_amount_out, AdapterError::SlippageExceeded);

invoke_protocol_withdraw(&ctx, shares)?;

let mut return_data = amount_out.to_le_bytes().to_vec();
return_data.extend_from_slice(&(exchange_rate as u128).to_le_bytes());
anchor_lang::solana_program::program::set_return_data(&return_data);

Ok(amount_out)
}

### 4.3 adapter_current_value
pub fn adapter_current_value(
ctx: Context<MyCurrentValue>,
shares: u64,
) -> Result<u64> {
let exchange_rate = read_exchange_rate(&ctx.accounts.protocol_state)?;
let value = (shares as u128)
    .checked_mul(exchange_rate as u128)
    .ok_or(AdapterError::MathOverflow)?
    .checked_div(RATE_SCALE as u128)
    .ok_or(AdapterError::MathOverflow)? as u64;

let mut return_data = value.to_le_bytes().to_vec();
return_data.extend_from_slice(&(exchange_rate as u128).to_le_bytes());
anchor_lang::solana_program::program::set_return_data(&return_data);

Ok(value)
}

### 4.4 Reading the exchange rate
fn read_exchange_rate(state_account: &AccountInfo) -> Result<u128> {
let data = state_account.try_borrow_data()?;
if data.len() < YOUR_OFFSET + 8 {
    return Ok(RATE_SCALE as u128);
}

let raw_bytes: [u8; 8] = data[YOUR_OFFSET..YOUR_OFFSET + 8]
    .try_into()
    .map_err(|_| AdapterError::InvalidStateData)?;
let raw = u64::from_le_bytes(raw_bytes);

let rate = (raw as u128)
    .checked_mul(1_000)
    .unwrap_or(RATE_SCALE as u128);

Ok(if rate == 0 { RATE_SCALE as u128 } else { rate })
}

### 4.5 Writing the CPI
fn invoke_protocol_deposit(ctx: &Context<MyDeposit>, amount: u64) -> Result<()> {
let mut ix_data: Vec<u8> = vec![0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11, 0x22];
ix_data.extend_from_slice(&amount.to_le_bytes());
let accounts = vec![
    AccountMeta::new(ctx.accounts.adapter_vault.key(), false),
    AccountMeta::new_readonly(ctx.accounts.dispatcher_authority.key(), true),
];

let ix = anchor_lang::solana_program::instruction::Instruction {
    program_id: MY_PROTOCOL_PROGRAM_ID,
    accounts,
    data: ix_data,
};

anchor_lang::solana_program::program::invoke_signed(
    &ix,
    &[
        ctx.accounts.adapter_vault.to_account_info(),
        ctx.accounts.dispatcher_authority.to_account_info(),
    ],
    &[],
).map_err(|_| AdapterError::ProtocolCpiFailed.into())
}

## 5. Write tests (1 h)

### 5.1 Unit test
import { expect } from "chai";
import { BN } from "@coral-xyz/anchor";
describe("My Protocol Adapter — unit", () => {
it("calculates shares correctly at 1:1 rate", () => {
const amount = new BN(1_000_000);
const rate = new BN(1_000_000_000);
const shares = amount.mul(new BN(1_000_000_000)).div(rate);
expect(shares.toString()).to.equal("1000000");
});
it("rejects when shares_out < min_shares_out", () => {
const sharesOut = new BN(990_000);
const minSharesOut = new BN(995_000);
expect(sharesOut.lt(minSharesOut)).to.be.true;
});
});

### 5.2 Integration test
import { expect } from "chai";
import { BN } from "@coral-xyz/anchor";
import { TEN_USDC, SLIPPAGE_BPS, applySlippage, assertGt } from "../utils/helpers";
import { MY_PROTOCOL_ADAPTER_PROGRAM_ID, findDispatcherAuthoritySync } from "../../sdk/src";
import { getOrCreateForkContext } from "./fork-setup";
describe("Adapter: My Protocol [mainnet-fork]", () => {
let ctx: any;
before(async () => {
ctx = await getOrCreateForkContext();
await ctx.userDispatcherClient.initializePosition(MY_PROTOCOL_ADAPTER_PROGRAM_ID);
});
it("deposits 10 USDC and receives shares", async () => {
const [dispatcherAuthority] = findDispatcherAuthoritySync();
const result = await ctx.userDispatcherClient.deposit({
adapterProgram: MY_PROTOCOL_ADAPTER_PROGRAM_ID,
amount: TEN_USDC,
minSharesOut: applySlippage(TEN_USDC, SLIPPAGE_BPS),
adapterAccounts: buildMyProtocolAccounts(
ctx.user.publicKey,
ctx.dispatcherVault,
dispatcherAuthority
),
});
assertGt(result.sharesMinted, new BN(0), "sharesMinted");
});
it("withdraws and receives USDC back", async () => {
const [dispatcherAuthority] = findDispatcherAuthoritySync();
const position = await ctx.dispatcherClient.fetchPosition(
ctx.user.publicKey,
MY_PROTOCOL_ADAPTER_PROGRAM_ID
);
const result = await ctx.userDispatcherClient.withdraw({
adapterProgram: MY_PROTOCOL_ADAPTER_PROGRAM_ID,
shares: new BN(position.shares.toString()),
minAmountOut: applySlippage(TEN_USDC, SLIPPAGE_BPS),
adapterAccounts: buildMyProtocolAccounts(
ctx.user.publicKey,
ctx.dispatcherVault,
dispatcherAuthority
),
});
assertGt(result.amountOut, new BN(0), "amountOut");
});
});

Run it:
ANCHOR_PROVIDER_URL=https://api.mainnet-beta.solana.com yarn test:fork

## 6. Register on devnet (15 min)
anchor build
anchor deploy --program-name my_protocol_adapter --provider.cluster devnet

Then register in the SDK — add to sdk/src/constants.ts:
export const MY_PROTOCOL_ADAPTER_PROGRAM_ID = new PublicKey(
"<YOUR_DEPLOYED_PROGRAM_ID>"
);

Propose and approve on devnet:
const registry = new RegistryClient(provider);
await registry.proposeAdapter(MY_PROTOCOL_ADAPTER_PROGRAM_ID, USDC_MINT, "My Protocol", "Description");
await registry.approveAdapter(MY_PROTOCOL_ADAPTER_PROGRAM_ID);

## 7. Checklist before submitting

Interface compliance:
- [ ] Instructions named exactly adapter_deposit, adapter_withdraw, adapter_current_value
- [ ] dispatcher_authority at account index 1 in all three Accounts structs
- [ ] adapter_vault at account index 0 in deposit and withdraw
- [ ] All three instructions call set_return_data with exactly 24 bytes
- [ ] Exchange rate scaled to 10^9

Correctness:
- [ ] deposit: shares_out >= min_shares_out check present
- [ ] withdraw: amount_out >= min_amount_out check present
- [ ] current_value: no state mutations
- [ ] All arithmetic uses checked_* methods

Tests:
- [ ] Unit tests cover share calculations
- [ ] Integration test: deposit passes on mainnet-fork
- [ ] Integration test: current_value approximates deposited amount
- [ ] Integration test: withdraw returns funds

## 8. FAQ & common mistakes

**"CPI to adapter returned error 6011 AdapterCpiFailed"**

You forgot to call set_return_data. Fix:
let mut return_data = shares_out.to_le_bytes().to_vec();
return_data.extend_from_slice(&exchange_rate.to_le_bytes());
anchor_lang::solana_program::program::set_return_data(&return_data);

**"Error: AdapterNotRegistered"**

Your adapter is not approved in the registry. Run proposeAdapter and approveAdapter.

**"Error: InvalidMint"**

USDC mint addresses differ per cluster:
- Mainnet: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
- Devnet:  4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU

**"My CPI fails with missing required signature"**

Pass empty seeds in invoke_signed — the Dispatcher's signer context propagates:
anchor_lang::solana_program::program::invoke_signed(&ix, &accounts, &[])?;

**"How do I handle protocols with cooldowns like Drift?"**

Return 0 from adapter_withdraw when withdrawal is queued. Add a separate complete_withdrawal instruction for after the cooldown.
