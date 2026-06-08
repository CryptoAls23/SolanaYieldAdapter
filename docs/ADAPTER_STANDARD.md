Файл 6 з 57
Назва файлу яку вписуєш у GitHub: docs/ADAPTER_STANDARD.md
# Solana Yield Adapter Standard — Specification v0.1

> **Status:** Draft
> **Authors:** Yield Adapter Working Group
> **Last updated:** 2026-06

## Table of Contents

1. [Overview](#1-overview)
2. [Motivation](#2-motivation)
3. [Terminology](#3-terminology)
4. [Architecture](#4-architecture)
5. [Core Interface](#5-core-interface)
6. [Instruction Specification](#6-instruction-specification)
7. [Return Data Protocol](#7-return-data-protocol)
8. [Account Layout Requirements](#8-account-layout-requirements)
9. [Share Accounting](#9-share-accounting)
10. [Registry & Governance](#10-registry--governance)
11. [Error Handling](#11-error-handling)
12. [Security Requirements](#12-security-requirements)
13. [Versioning](#13-versioning)
14. [Reference Implementations](#14-reference-implementations)

## 1. Overview

The **Solana Yield Adapter Standard** defines a minimal, extensible interface that allows any yield-bearing protocol on Solana to be routed through a single **Dispatcher** program. Integrators write one integration against the Dispatcher and instantly gain access to every approved adapter.

The standard has three parts:

| Component | Description |
|---|---|
| **Dispatcher** | On-chain router that validates, routes, and accounts for all yield operations |
| **Adapter** | Thin wrapper around a specific protocol exposing the standard interface |
| **Registry** | Governance-gated approval list of adapters the Dispatcher will route to |

## 2. Motivation

Every Solana yield protocol has a different instruction set, account layout, and share accounting model. Aggregators and portfolio managers must write and maintain bespoke integrations for each one.

This standard solves that by requiring adapters to implement **three instructions with fixed discriminators**. The Dispatcher can then route to any adapter without knowing the underlying protocol.

**Design goals:**

- **Minimal** — three mandatory instructions, nothing more required
- **Extensible** — adapters may add extra instructions beyond the standard three
- **Protocol-agnostic** — the Dispatcher never imports adapter crates
- **Safe** — all adapters must be registered and approved before routing
- **Composable** — adapters are standalone programs; they work with or without the Dispatcher

## 3. Terminology

| Term | Definition |
|---|---|
| **Adapter** | An Anchor program that wraps a yield protocol and exposes the standard interface |
| **Dispatcher** | The canonical router program (yield-dispatcher) |
| **Registry** | The on-chain approval list program (adapter-registry) |
| **Shares** | The unit of account representing a user's proportional claim on deposited assets |
| **Exchange Rate** | The current ratio of underlying tokens per share, scaled to 10^9 |
| **Dispatcher Authority** | A PDA (seeds = ["dispatcher_authority"]) that acts as the signer in all adapter CPIs |
| **Position** | A PDA (seeds = ["position", user, adapter_program]) tracking a user's shares in one adapter |

## 4. Architecture

### 4.1 System diagram
User Wallet
|
| deposit / withdraw / current_value
v
Yield Dispatcher Program
|
|---- CPI ----> Kamino Adapter ----> Kamino Protocol
|---- CPI ----> MarginFi Adapter --> MarginFi Protocol
|---- CPI ----> Drift Adapter -----> Drift Protocol

### 4.2 Registry flow
Adapter Team  -->  propose_adapter  -->  Registry (Pending)
Governance    -->  approve_adapter  -->  Registry (Active)
Dispatcher    -->  validate         -->  reads status byte

## 5. Core Interface

Every adapter MUST implement exactly these three instructions:

```rust
pub fn adapter_deposit(ctx: Context<Deposit>, amount: u64, min_shares_out: u64) -> Result<u64>;
pub fn adapter_withdraw(ctx: Context<Withdraw>, shares: u64, min_amount_out: u64) -> Result<u64>;
pub fn adapter_current_value(ctx: Context<CurrentValue>, shares: u64) -> Result<u64>;
```

### 5.1 Mandatory invariants

1. All three instructions must call set_return_data before returning
2. adapter_deposit must fail if shares_minted < min_shares_out
3. The adapter must verify that the input token account holds the expected mint
4. The adapter must accept dispatcher_authority as the vault signer

## 6. Instruction Specification

### 6.1 Fixed discriminators

| Instruction | Discriminator (hex) |
|---|---|
| adapter_deposit | f2 23 c6 89 52 e1 f2 b6 |
| adapter_withdraw | b7 12 46 9c 94 67 33 f4 |
| adapter_current_value | 45 a0 37 31 61 c3 28 21 |

### 6.2 Instruction data layout

adapter_deposit:
[0..8]   discriminator
[8..16]  amount (u64 LE)
[16..24] min_shares_out (u64 LE)

adapter_withdraw:
[0..8]   discriminator
[8..16]  shares (u64 LE)
[16..24] min_amount_out (u64 LE)

adapter_current_value:
[0..8]  discriminator
[8..16] shares (u64 LE)

### 6.3 Mandatory account order

| Index | Account | Constraint |
|---|---|---|
| 0 | adapter_vault | Writable token account |
| 1 | dispatcher_authority | Read-only signer |

## 7. Return Data Protocol

### 7.1 Return data layout

[0..8]   primary_amount (u64 LE)
[8..24]  exchange_rate (u128 LE)

### 7.2 Exchange rate precision

actual_rate = exchange_rate_u128 / 10^9

Example: if 1 kUSDC = 1.02 USDC, the stored value is 1_020_000_000.

## 8. Account Layout Requirements

### 8.1 Dispatcher Authority

seeds = [b"dispatcher_authority"]
program_id = YIELD_DISPATCHER_PROGRAM_ID

### 8.2 Registry Entry status byte at offset 72

| Value | Meaning |
|---|---|
| 0 | Pending |
| 1 | Active |
| 2 | Deprecated |
| 3 | Rejected |

## 9. Share Accounting

| Protocol | Formula |
|---|---|
| Kamino | kUSDC = USDC * 10^9 / collateral_exchange_rate |
| MarginFi | shares = USDC * 10^12 / asset_share_value |
| Jupiter LP | JLP_price = pool_aum * 10^6 / jlp_supply |
| Maple Syrup | syUSDC = USDC * 10^9 / (total_assets / total_supply) |
| Drift IF | rate = if_vault_balance * 10^9 / total_if_shares |

## 10. Registry & Governance

### 10.1 Lifecycle

propose_adapter --> [Pending] --> approve_adapter --> [Active]
                            `--> reject_adapter  --> [Rejected]
[Active] --> deprecate_adapter --> [Deprecated]

### 10.2 On-chain validation

The Dispatcher reads the registry entry on every deposit and withdraw.
Deprecating an adapter takes effect immediately on the next transaction.

## 11. Error Handling

| Code | Name | When |
|---|---|---|
| 6000 | Paused | Dispatcher is globally paused |
| 6001 | AdapterNotRegistered | Registry entry PDA missing |
| 6002 | AdapterNotActive | Registry status not 1 |
| 6003 | ZeroAmount | amount == 0 |
| 6004 | SlippageExceeded | shares_minted < min_shares_out |
| 6005 | WithdrawSlippageExceeded | amount_out < min_amount_out |
| 6006 | InsufficientShares | Position shares < requested |
| 6007 | MathOverflow | Arithmetic overflow |
| 6008 | AdapterMismatch | Program ID mismatch |
| 6009 | InvalidFeeBps | Fee > 10000 bps |
| 6010 | Unauthorized | Caller is not the authority |
| 6011 | AdapterCpiFailed | CPI to adapter failed |
| 6012 | InvalidMint | Token mint mismatch |

## 12. Security Requirements

```rust
require!(token_account.mint == EXPECTED_MINT, AdapterError::InvalidMint);
require!(output >= min_output, AdapterError::SlippageExceeded);
let result = a.checked_mul(b).ok_or(AdapterError::MathOverflow)?;
```

Adapters MUST NOT:
- Transfer tokens directly to/from the user
- Generate signer PDAs that overlap with Dispatcher seeds
- Call set_return_data with fewer than 8 bytes
- Mutate state in adapter_current_value

## 13. Versioning

| Change Type | Version Bump |
|---|---|
| New optional instruction | Minor (0.x) |
| New mandatory instruction | Major (x.0) |
| Breaking account layout change | Major (x.0) |
| Discriminator change | Major (x.0) |

## 14. Reference Implementations

| Adapter | Protocol | Source |
|---|---|---|
| kamino-usdc | Kamino Finance | adapters/kamino-usdc/ |
| marginfi-usdc | MarginFi | adapters/marginfi-usdc/ |
| jupiter-lp | Jupiter Perps | adapters/jupiter-lp/ |
| maple-syrup | Maple Finance | adapters/maple-syrup/ |
| drift-insurance | Drift Protocol | adapters/drift-insurance/ |
