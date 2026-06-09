import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
} from "@solana/web3.js";
import { AnchorProvider } from "@coral-xyz/anchor";
import {
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
} from "@solana/spl-token";
import {
  makeConnection,
  makeProvider,
  airdrop,
  logSection,
} from "../utils/helpers";
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
  findDispatcherAuthoritySync,
} from "../../sdk/src";

export interface ForkContext {
  connection: Connection;
  authority: Keypair;
  user: Keypair;
  dispatcherClient: DispatcherClient;
  userDispatcherClient: DispatcherClient;
  registryClient: RegistryClient;
  dispatcherAuthority: PublicKey;
  dispatcherVault: PublicKey;
  userUsdcAta: PublicKey;
}

let _ctx: ForkContext | null = null;

export async function getOrCreateForkContext(): Promise<ForkContext> {
  if (_ctx) return _ctx;

  logSection("Mainnet-Fork Setup");

  const connection = makeConnection();
  const authority = Keypair.generate();
  const user = Keypair.generate();

  await airdrop(connection, authority.publicKey);
  await airdrop(connection, user.publicKey);

  const authProvider = makeProvider(connection, authority);
  const userProvider = makeProvider(connection, user);

  const registryClient = new RegistryClient(authProvider);
  const dispatcherClient = new DispatcherClient(authProvider);
  const userDispatcherClient = new DispatcherClient(userProvider);

  const [dispatcherAuthority] = findDispatcherAuthoritySync();
  const dispatcherVault = getAssociatedTokenAddressSync(USDC_MINT, dispatcherAuthority, true);

  await registryClient.initializeRegistry(authority.publicKey);
  await dispatcherClient.initializeDispatcher(ADAPTER_REGISTRY_PROGRAM_ID, 30);

  const adapters = [
    { id: KAMINO_ADAPTER_PROGRAM_ID, name: "Kamino Finance", desc: "USDC lending via Kamino" },
    { id: MARGINFI_ADAPTER_PROGRAM_ID, name: "MarginFi", desc: "USDC lending via MarginFi" },
    { id: JUPITER_LP_ADAPTER_PROGRAM_ID, name: "Jupiter Perps", desc: "JLP pool yield" },
    { id: MAPLE_ADAPTER_PROGRAM_ID, name: "Maple Finance", desc: "Institutional USDC lending" },
    { id: DRIFT_ADAPTER_PROGRAM_ID, name: "Drift Protocol", desc: "Insurance Fund stake" },
  ];

  for (const a of adapters) {
    await registryClient.proposeAdapter(a.id, USDC_MINT, a.name, a.desc);
    await registryClient.approveAdapter(a.id);
    console.log(`  ok  ${a.name.padEnd(20)} registered`);
  }

  const vaultInfo = await connection.getAccountInfo(dispatcherVault);
  if (!vaultInfo) {
    const tx = new Transaction().add(
      createAssociatedTokenAccountInstruction(
        authority.publicKey, dispatcherVault, dispatcherAuthority, USDC_MINT
      )
    );
    await authProvider.sendAndConfirm(tx);
  }

  const userUsdcAta = getAssociatedTokenAddressSync(USDC_MINT, user.publicKey);
  const userAtaInfo = await connection.getAccountInfo(userUsdcAta);
  if (!userAtaInfo) {
    const tx = new Transaction().add(
      createAssociatedTokenAccountInstruction(
        user.publicKey, userUsdcAta, user.publicKey, USDC_MINT
      )
    );
    await userProvider.sendAndConfirm(tx);
  }

  console.log(`\n  User USDC ATA: ${userUsdcAta.toBase58().slice(0, 20)}...`);

  _ctx = {
    connection,
    authority,
    user,
    dispatcherClient,
    userDispatcherClient,
    registryClient,
    dispatcherAuthority,
    dispatcherVault,
    userUsdcAta,
  };

  return _ctx;
}
