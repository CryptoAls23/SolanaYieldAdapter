import { expect } from "chai";
import { Keypair, Connection } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import {
  makeConnection,
  makeProvider,
  airdrop,
  logSection,
  logResult,
} from "../utils/helpers";
import {
  RegistryClient,
  USDC_MINT,
  KAMINO_ADAPTER_PROGRAM_ID,
  MARGINFI_ADAPTER_PROGRAM_ID,
  ADAPTER_STATUS,
} from "../../sdk/src";

describe("Adapter Registry", () => {
  let connection: Connection;
  let governance: Keypair;
  let proposer: Keypair;
  let registryClient: RegistryClient;
  let govRegistryClient: RegistryClient;

  before(async () => {
    logSection("Adapter Registry - Setup");
    connection = makeConnection();
    governance = Keypair.generate();
    proposer = Keypair.generate();
    await airdrop(connection, governance.publicKey);
    await airdrop(connection, proposer.publicKey);
    govRegistryClient = new RegistryClient(makeProvider(connection, governance));
    registryClient = new RegistryClient(makeProvider(connection, proposer));
    const sig = await govRegistryClient.initializeRegistry(governance.publicKey);
    logResult("Registry initialized", sig.slice(0, 12) + "...");
  });

  it("initializes with correct governance", async () => {
    const config = await govRegistryClient.fetchRegistryConfig();
    expect(config).to.not.be.null;
    expect(config!.governance.toBase58()).to.equal(governance.publicKey.toBase58());
    expect(config!.totalProposed).to.equal(0);
    expect(config!.totalActive).to.equal(0);
    logResult("governance", config!.governance.toBase58().slice(0, 16) + "...");
  });

  it("accepts a proposal from any caller", async () => {
    const sig = await registryClient.proposeAdapter(
      KAMINO_ADAPTER_PROGRAM_ID,
      USDC_MINT,
      "Kamino Finance",
      "USDC lending on Kamino Finance"
    );
    expect(sig).to.be.a("string");
    const entry = await registryClient.fetchAdapterEntry(KAMINO_ADAPTER_PROGRAM_ID);
    expect(entry).to.not.be.null;
    expect(entry!.status).to.equal(ADAPTER_STATUS.PENDING);
    expect(entry!.protocolName).to.equal("Kamino Finance");
    logResult("status", "Pending ok");
  });

  it("rejects approval by non-governance", async () => {
    try {
      await registryClient.approveAdapter(KAMINO_ADAPTER_PROGRAM_ID);
      expect.fail("Should have thrown");
    } catch (err: any) {
      expect(err.message).to.include("Unauthorized");
      logResult("non-governance approval rejected", "ok");
    }
  });

  it("governance can approve a pending adapter", async () => {
    const sig = await govRegistryClient.approveAdapter(KAMINO_ADAPTER_PROGRAM_ID);
    expect(sig).to.be.a("string");
    const entry = await govRegistryClient.fetchAdapterEntry(KAMINO_ADAPTER_PROGRAM_ID);
    expect(entry!.status).to.equal(ADAPTER_STATUS.ACTIVE);
    const config = await govRegistryClient.fetchRegistryConfig();
    expect(config!.totalActive).to.equal(1);
    logResult("status after approval", "Active ok");
  });

  it("governance can reject a pending adapter", async () => {
    await registryClient.proposeAdapter(
      MARGINFI_ADAPTER_PROGRAM_ID,
      USDC_MINT,
      "MarginFi",
      "USDC lending on MarginFi"
    );
    const sig = await govRegistryClient.rejectAdapter(
      MARGINFI_ADAPTER_PROGRAM_ID,
      "Audit pending"
    );
    expect(sig).to.be.a("string");
    const entry = await govRegistryClient.fetchAdapterEntry(MARGINFI_ADAPTER_PROGRAM_ID);
    expect(entry!.status).to.equal(ADAPTER_STATUS.REJECTED);
    logResult("rejection stored", "ok");
  });

  it("governance can deprecate an active adapter", async () => {
    const sig = await govRegistryClient.deprecateAdapter(
      KAMINO_ADAPTER_PROGRAM_ID,
      "Security vulnerability discovered"
    );
    expect(sig).to.be.a("string");
    const entry = await govRegistryClient.fetchAdapterEntry(KAMINO_ADAPTER_PROGRAM_ID);
    expect(entry!.status).to.equal(ADAPTER_STATUS.DEPRECATED);
    const config = await govRegistryClient.fetchRegistryConfig();
    expect(config!.totalActive).to.equal(0);
    logResult("deprecated status", "ok");
  });

  it("isAdapterActive returns false for deprecated", async () => {
    const active = await govRegistryClient.isAdapterActive(KAMINO_ADAPTER_PROGRAM_ID);
    expect(active).to.be.false;
    logResult("isAdapterActive(deprecated)", "false ok");
  });
});
