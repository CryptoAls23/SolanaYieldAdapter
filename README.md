# Solana Yield Adapter Standard

A minimal, governance-gated routing layer that lets any protocol on Solana plug into a single yield interface.

User → Dispatcher → Registry check → Adapter → Underlying Protocol
---

## What's in here

| Directory | Description |
|---|---|
| `programs/yield-dispatcher/` | Core router program — routes deposits, withdrawals, value queries |
| `programs/adapter-registry/` | On-chain governance-gated approval list |
| `adapters/kamino-usdc/` | Kamino Finance USDC lending adapter |
| `adapters/marginfi-usdc/` | MarginFi USDC lending adapter |
| `adapters/jupiter-lp/` | Jupiter Perpetuals LP adapter |
| `adapters/maple-syrup/` | Maple Finance Syrup USDC adapter |
| `adapters/drift-insurance/` | Drift Protocol Insurance Fund adapter |
| `sdk/` | TypeScript SDK — `DispatcherClient`, `RegistryClient`, account builders |
| `tests/unit/` | Localnet tests for dispatcher and registry logic |
| `tests/integration/` | Mainnet-fork tests for all five adapters |
| `scripts/` | Devnet deploy script |
| `docs/` | Specification and developer guide |

---

## Quick start

```bash
# 1. Install
yarn install
avm use 0.31.1

# 2. Build
anchor build

# 3. Run unit tests (localnet)
yarn test:unit

# 4. Run mainnet-fork integration tests
ANCHOR_PROVIDER_URL=https://api.mainnet-beta.solana.com yarn test:fork

# 5. Deploy to devnet
export ANCHOR_WALLET=~/.config/solana/id.json
yarn ts-node scripts/deploy-devnet.ts
```

---

## The interface

Every adapter exposes exactly three instructions. Names are fixed — the Dispatcher calls them by discriminator without knowing the concrete program.

```rust
// Deposit tokens, receive shares
fn adapter_deposit(amount: u64, min_shares_out: u64) -> Result<u64>

// Burn shares, receive tokens
fn adapter_withdraw(shares: u64, min_amount_out: u64) -> Result<u64>

// Read current value — no state changes
fn adapter_current_value(shares: u64) -> Result<u64>
```

Return data (24 bytes, little-endian):
[0..8]   primary_amount  u64  — shares minted or tokens out
[8..24]  exchange_rate  u128  — underlying tokens per share × 10^9
---

## SDK usage

```typescript
import { DispatcherClient, RegistryClient, KAMINO_ADAPTER_PROGRAM_ID,
         buildKaminoAccounts, findDispatcherAuthoritySync } from "@yield-adapter/sdk";
import { AnchorProvider } from "@coral-xyz/anchor";
import BN from "bn.js";

const provider = AnchorProvider.env();
const client = new DispatcherClient(provider);

// Initialize position (once per user per adapter)
await client.initializePosition(KAMINO_ADAPTER_PROGRAM_ID);

// Deposit 10 USDC
const [dispatcherAuthority] = findDispatcherAuthoritySync();
const result = await client.deposit({
  adapterProgram: KAMINO_ADAPTER_PROGRAM_ID,
  amount: new BN(10_000_000),        // 10 USDC
  minSharesOut: new BN(9_800_000),   // 2% slippage
  adapterAccounts: buildKaminoAccounts(
    provider.wallet.publicKey,
    dispatcherAuthority,
    dispatcherVault,
    kaminoReserveLiquidity,
    kaminoLendingMarketAuthority,
  ),
});

console.log("Shares minted:", result.sharesMinted.toString());
```

---

## Registry

```typescript
const registry = new RegistryClient(provider);

await registry.proposeAdapter(
  MY_ADAPTER_PROGRAM_ID,
  USDC_MINT,
  "My Protocol",
  "Yield from my protocol"
);

await registry.approveAdapter(MY_ADAPTER_PROGRAM_ID);
```

---

## Devnet deployment

Registry program: `AdPtReGiStRyXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX`
Dispatcher program: `YieLdDiSPaTcHeRvAuLtXXXXXXXXXXXXXXXXXXXXXXX`

Explorer:
https://explorer.solana.com/address/AdPtReGiStRyXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX?cluster=devnet

---

## Reference adapters

| Adapter | Protocol | APY source | Cooldown |
|---|---|---|---|
| Kamino USDC | Kamino Finance | Borrow interest + liquidation fees | None |
| MarginFi USDC | MarginFi | Borrow interest | None |
| Jupiter LP | Jupiter Perps | 70% of trading + borrow fees | None |
| Maple Syrup | Maple Finance | Institutional lending yield | None |
| Drift IF | Drift Protocol | Liquidation revenue share | 13 days |

---

## Build your own adapter

See [`docs/BUILD_YOUR_OWN_ADAPTER.md`](docs/BUILD_YOUR_OWN_ADAPTER.md).
The full interface specification is in [`docs/ADAPTER_STANDARD.md`](docs/ADAPTER_STANDARD.md).

---

## Tech stack

- **Anchor** 0.31.1
- **Solana** 2.2.20
- **Rust** (programs)
- **TypeScript** (SDK + tests)
- **Mocha + Chai** (test runner)

---

## License

MIT
