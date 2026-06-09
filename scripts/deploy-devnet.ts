import { Connection, Keypair, PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { AnchorProvider, Wallet } from "@coral-xyz/anchor";
import * as fs from "fs";

import {
  DispatcherClient,
  RegistryClient,
  USDC_MINT,
  KAMINO_ADAPTER_PROGRAM_ID,
  MARGINFI_ADAPTER_PROGRAM_ID,
  JUPITER_LP_ADAPTER_PROGRAM_ID,
  MAPLE_ADAPTER_PROGRAM_ID,
  DRIFT_ADAPTER_PROGRAM_ID,
  ADAPTER_REGISTRY_PROGRAM_ID,
  findDispatcherConfigSync,
  findRegistryConfig,
} from "../sdk/src";

const DEVNET_RPC = process.env.ANCHOR_PROVIDER_URL ?? "https://api.devnet.solana.com";

async function loadKeypair(filePath: string): Promise<Keypair> {
  const expanded = filePath.replace("~", process.env.HOME ?? "");
  const raw = JSON.parse(fs.readFileSync(expanded, "utf-8"));
  return Keypair.fromSecretKey(Uint8Array.from(raw));
}

async function main() {
  console.log("==================================================");
  console.log("  Solana Yield Adapter Standard - Devnet Deploy  ");
  console.log("==================================================\n");

  const walletPath = process.env.ANCHOR_WALLET ?? "~/.config/solana/id.json";
  const payer = await loadKeypair(walletPath);
  const connection = new Connection(DEVNET_RPC, "confirmed");
  const provider = new AnchorProvider(connection, new Wallet(payer), {
    commitment: "confirmed",
  });

  console.log(`  Network:    ${DEVNET_RPC}`);
  console.log(`  Authority:  ${payer.publicKey.toBase58()}`);

  const balance = await connection.getBalance(payer.publicKey);
  console.log(`  Balance:    ${(balance / LAMPORTS_PER_SOL).toFixed(4)} SOL\n`);

  if (balance < 0.5 * LAMPORTS_PER_SOL) {
    console.error("Insufficient SOL. Run: solana airdrop 2 --url devnet");
    process.exit(1);
  }

  const registryClient = new RegistryClient(provider);
  const dispatcherClient = new DispatcherClient(provider);

  console.log("-- Step 1: Initialize Registry");
  const [registryConfigPda] = await findRegistryConfig();
  const existingRegistry = await connection.getAccountInfo(registryConfigPda);
  if (existingRegistry) {
    console.log("  ok  Registry already initialized, skipping");
  } else {
    const sig = await registryClient.initializeRegistry(payer.publicKey);
    console.log(`  ok  Registry initialized: ${sig.slice(0, 20)}...`);
  }

  console.log("\n-- Step 2: Initialize Dispatcher");
  const [dispatcherConfigPda] = findDispatcherConfigSync();
  const existingDispatcher = await connection.getAccountInfo(dispatcherConfigPda);
  if (existingDispatcher) {
    console.log("  ok  Dispatcher already initialized, skipping");
  } else {
    const sig = await dispatcherClient.initializeDispatcher(
      ADAPTER_REGISTRY_PROGRAM_ID, 30
    );
    console.log(`  ok  Dispatcher initialized: ${sig.slice(0, 20)}...`);
  }

  console.log("\n-- Step 3: Register Adapters");
  const adapters = [
    { id: KAMINO_ADAPTER_PROGRAM_ID, name: "Kamino Finance", desc: "USDC lending via Kamino kToken vault" },
    { id: MARGINFI_ADAPTER_PROGRAM_ID, name: "MarginFi", desc: "USDC lending on MarginFi protocol" },
    { id: JUPITER_LP_ADAPTER_PROGRAM_ID, name: "Jupiter Perps", desc: "JLP liquidity pool yield" },
    { id: MAPLE_ADAPTER_PROGRAM_ID, name: "Maple Finance", desc: "Institutional USDC lending via Maple Syrup" },
    { id: DRIFT_ADAPTER_PROGRAM_ID, name: "Drift Protocol", desc: "Drift Insurance Fund USDC stake" },
  ];

  for (const adapter of adapters) {
    try {
      const existing = await registryClient.fetchAdapterEntry(adapter.id);
      if (existing) {
        console.log(`  skip ${adapter.name.padEnd(20)} already registered`);
        continue;
      }
    } catch (_) {}

    try {
      await registryClient.proposeAdapter(adapter.id, USDC_MINT, adapter.name, adapter.desc);
      await registryClient.approveAdapter(adapter.id);
      console.log(`  ok  ${adapter.name.padEnd(20)} proposed and approved`);
    } catch (err: any) {
      console.error(`  err ${adapter.name}: ${err.message}`);
    }
  }

  console.log("\n==================================================");
  console.log("  Deployment Summary");
  console.log("==================================================");
  const config = await registryClient.fetchRegistryConfig();
  const dispatcherConfig = await dispatcherClient.fetchDispatcherConfig();
  console.log(`  Registry:        ${ADAPTER_REGISTRY_PROGRAM_ID.toBase58()}`);
  console.log(`  Dispatcher:      ${dispatcherClient.programId.toBase58()}`);
  console.log(`  Governance:      ${config?.governance.toBase58()}`);
  console.log(`  Fee:             ${dispatcherConfig?.feeBps} bps`);
  console.log(`  Active adapters: ${config?.totalActive}`);
  console.log(`\n  Explorer: https://explorer.solana.com/address/${ADAPTER_REGISTRY_PROGRAM_ID.toBase58()}?cluster=devnet`);
  console.log("\n  Devnet deployment complete!");
}

main().catch((err) => {
  console.error("\nDeploy failed:", err);
  process.exit(1);
});
