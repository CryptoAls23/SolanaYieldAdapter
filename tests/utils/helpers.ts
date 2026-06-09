import {
  Connection,
  Keypair,
  PublicKey,
  LAMPORTS_PER_SOL,
  Transaction,
} from "@solana/web3.js";
import {
  AnchorProvider,
  Wallet,
  BN,
} from "@coral-xyz/anchor";
import {
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
} from "@solana/spl-token";
import { USDC_MINT } from "../../sdk/src/constants";

export const IS_MAINNET_FORK = process.env.ANCHOR_PROVIDER_URL?.includes("mainnet");
export const RPC_URL = process.env.ANCHOR_PROVIDER_URL ?? "http://127.0.0.1:8899";

export const ONE_USDC = new BN(1_000_000);
export const TEN_USDC = ONE_USDC.muln(10);
export const HUNDRED_USDC = ONE_USDC.muln(100);
export const SLIPPAGE_BPS = 200;

export function applySlippage(amount: BN, slippageBps: number): BN {
  return amount.muln(10_000 - slippageBps).divn(10_000);
}

export function makeProvider(connection: Connection, payer: Keypair): AnchorProvider {
  const wallet = new Wallet(payer);
  return new AnchorProvider(connection, wallet, { commitment: "confirmed" });
}

export function makeConnection(): Connection {
  return new Connection(RPC_URL, "confirmed");
}

export async function airdrop(
  connection: Connection,
  pubkey: PublicKey,
  lamports: number = 10 * LAMPORTS_PER_SOL
): Promise<void> {
  const sig = await connection.requestAirdrop(pubkey, lamports);
  await connection.confirmTransaction(sig, "confirmed");
}

export async function getOrCreateAta(
  connection: Connection,
  payer: Keypair,
  mint: PublicKey,
  owner: PublicKey
): Promise<PublicKey> {
  const ata = getAssociatedTokenAddressSync(mint, owner, true);
  const info = await connection.getAccountInfo(ata);
  if (!info) {
    const tx = new Transaction().add(
      createAssociatedTokenAccountInstruction(payer.publicKey, ata, owner, mint)
    );
    const sig = await connection.sendTransaction(tx, [payer]);
    await connection.confirmTransaction(sig, "confirmed");
  }
  return ata;
}

export async function getTokenBalance(
  connection: Connection,
  tokenAccount: PublicKey
): Promise<BN> {
  try {
    const info = await connection.getTokenAccountBalance(tokenAccount);
    return new BN(info.value.amount);
  } catch {
    return new BN(0);
  }
}

export function assertApproxEqual(
  actual: BN,
  expected: BN,
  toleranceBps: number = 100,
  label: string = "value"
): void {
  const diff = actual.sub(expected).abs();
  const tolerance = expected.muln(toleranceBps).divn(10_000);
  if (diff.gt(tolerance)) {
    throw new Error(
      `${label}: expected ~${expected.toString()}, got ${actual.toString()} ` +
        `(diff ${diff.toString()}, tolerance ${tolerance.toString()})`
    );
  }
}

export function assertGt(actual: BN, floor: BN, label: string = "value"): void {
  if (actual.lte(floor)) {
    throw new Error(
      `${label}: expected > ${floor.toString()}, got ${actual.toString()}`
    );
  }
}

export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export function logSection(title: string): void {
  console.log("\n" + "-".repeat(60));
  console.log(`  ${title}`);
  console.log("-".repeat(60));
}

export function logResult(label: string, value: string | BN | number): void {
  const v = value instanceof BN ? value.toString() : String(value);
  console.log(`  ok  ${label.padEnd(30)} ${v}`);
}
