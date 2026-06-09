import { expect } from "chai";
import { Keypair, Connection, PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import {
  makeConnection,
  makeProvider,
  airdrop,
  logSection,
  logResult,
  ONE_USDC,
} from "../utils/helpers";
import {
  DispatcherClient,
  RegistryClient,
  KAMINO_ADAPTER_PROGRAM_ID,
  findPositionSync,
} from "../../sdk/src";

describe("Yield Dispatcher", () => {
  let connection: Connection;
  let authority: Keypair;
  let user: Keypair;
  let dispatcherClient: DispatcherClient;
  let registryClient: RegistryClient;
  let registryProgramId: PublicKey;

  before(async () => {
    logSection("Dispatcher - Setup");
    connection = makeConnection();
    authority = Keypair.generate();
    user = Keypair.generate();
    await airdrop(connection, authority.publicKey);
    await airdrop(connection, user.publicKey);
    const authProvider = makeProvider(connection, authority);
    dispatcherClient = new DispatcherClient(authProvider);
    registryClient = new RegistryClient(authProvider);
    registryProgramId = new PublicKey("AdPtReGiStRyXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");
    await registryClient.initializeRegistry(authority.publicKey);
    const sig = await dispatcherClient.initializeDispatcher(registryProgramId, 30);
    logResult("Dispatcher initialized", sig.slice(0, 12) + "...");
  });

  it("initializes with correct parameters", async () => {
    const config = await dispatcherClient.fetchDispatcherConfig();
    expect(config).to.not.be.null;
    expect(config!.feeBps).to.equal(30);
    expect(config!.paused).to.be.false;
    expect(config!.totalDeposits.toString()).to.equal("0");
    logResult("feeBps", config!.feeBps);
    logResult("paused", config!.paused.toString());
  });

  it("rejects fee_bps > 10000", async () => {
    try {
      await dispatcherClient.initializeDispatcher(registryProgramId, 10001);
      expect.fail("Should have thrown");
    } catch (err: any) {
      expect(err.message).to.include("InvalidFeeBps");
      logResult("fee > 10000 rejected", "ok");
    }
  });

  it("initializes a position for user+adapter", async () => {
    const userDispatcher = new DispatcherClient(makeProvider(connection, user));
    const sig = await userDispatcher.initializePosition(KAMINO_ADAPTER_PROGRAM_ID);
    expect(sig).to.be.a("string");
    const position = await dispatcherClient.fetchPosition(
      user.publicKey,
      KAMINO_ADAPTER_PROGRAM_ID
    );
    expect(position).to.not.be.null;
    expect(position!.shares.toString()).to.equal("0");
    expect(position!.owner.toBase58()).to.equal(user.publicKey.toBase58());
    logResult("initial shares", position!.shares.toString());
  });

  it("rejects deposit with zero amount", async () => {
    try {
      await dispatcherClient.deposit({
        adapterProgram: KAMINO_ADAPTER_PROGRAM_ID,
        amount: new BN(0),
        minSharesOut: new BN(0),
        adapterAccounts: [],
      });
      expect.fail("Should have thrown");
    } catch (err: any) {
      expect(err.message).to.include("ZeroAmount");
      logResult("zero deposit rejected", "ok");
    }
  });

  it("rejects deposit to unregistered adapter", async () => {
    const fakeAdapter = Keypair.generate().publicKey;
    try {
      await dispatcherClient.deposit({
        adapterProgram: fakeAdapter,
        amount: ONE_USDC,
        minSharesOut: new BN(0),
        adapterAccounts: [],
      });
      expect.fail("Should have thrown");
    } catch (err: any) {
      expect(err.message).to.include("AdapterNotRegistered");
      logResult("unregistered adapter rejected", "ok");
    }
  });

  it("rejects withdrawal with insufficient shares", async () => {
    const userDispatcher = new DispatcherClient(makeProvider(connection, user));
    try {
      await userDispatcher.withdraw({
        adapterProgram: KAMINO_ADAPTER_PROGRAM_ID,
        shares: new BN(1_000_000_000),
        minAmountOut: new BN(0),
        adapterAccounts: [],
      });
      expect.fail("Should have thrown");
    } catch (err: any) {
      expect(err.message).to.include("InsufficientShares");
      logResult("insufficient shares rejected", "ok");
    }
  });
});
