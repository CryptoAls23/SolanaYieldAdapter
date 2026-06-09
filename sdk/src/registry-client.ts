import {
  PublicKey,
  Transaction,
  TransactionInstruction,
  SystemProgram,
} from "@solana/web3.js";
import { AnchorProvider } from "@coral-xyz/anchor";
import { ADAPTER_REGISTRY_PROGRAM_ID } from "./constants";
import {
  findRegistryConfig,
  findAdapterEntrySync,
  findAdapterEntry,
} from "./pda";
import { AdapterEntry, RegistryConfig, ADAPTER_STATUS } from "./types";
import BN from "bn.js";

export class RegistryClient {
  readonly provider: AnchorProvider;
  readonly programId: PublicKey;

  constructor(provider: AnchorProvider) {
    this.provider = provider;
    this.programId = ADAPTER_REGISTRY_PROGRAM_ID;
  }

  async initializeRegistry(governance: PublicKey): Promise<string> {
    const [config] = await findRegistryConfig();
    const payer = this.provider.wallet.publicKey;

    const discriminator = Buffer.from([0x1a, 0xb3, 0x7e, 0x22, 0x45, 0xf1, 0x9c, 0x88]);
    const buf = Buffer.alloc(8 + 32);
    discriminator.copy(buf, 0);
    governance.toBuffer().copy(buf, 8);

    const ix = new TransactionInstruction({
      programId: this.programId,
      keys: [
        { pubkey: config, isSigner: false, isWritable: true },
        { pubkey: payer, isSigner: true, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: buf,
    });

    const tx = new Transaction().add(ix);
    return this.provider.sendAndConfirm(tx);
  }

  async proposeAdapter(
    adapterProgram: PublicKey,
    inputMint: PublicKey,
    protocolName: string,
    description: string
  ): Promise<string> {
    const [config] = await findRegistryConfig();
    const [entry] = findAdapterEntrySync(adapterProgram);
    const proposer = this.provider.wallet.publicKey;

    const discriminator = Buffer.from([0x3c, 0x7a, 0x12, 0xf4, 0x88, 0x21, 0xb5, 0x9e]);
    const nameBytes = Buffer.from(protocolName, "utf8");
    const descBytes = Buffer.from(description, "utf8");
    const nameLenBuf = Buffer.alloc(4);
    nameLenBuf.writeUInt32LE(nameBytes.length);
    const descLenBuf = Buffer.alloc(4);
    descLenBuf.writeUInt32LE(descBytes.length);

    const data = Buffer.concat([
      discriminator,
      inputMint.toBuffer(),
      nameLenBuf,
      nameBytes,
      descLenBuf,
      descBytes,
    ]);

    const ix = new TransactionInstruction({
      programId: this.programId,
      keys: [
        { pubkey: config, isSigner: false, isWritable: true },
        { pubkey: entry, isSigner: false, isWritable: true },
        { pubkey: adapterProgram, isSigner: false, isWritable: false },
        { pubkey: proposer, isSigner: true, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data,
    });

    const tx = new Transaction().add(ix);
    return this.provider.sendAndConfirm(tx);
  }

  async approveAdapter(adapterProgram: PublicKey): Promise<string> {
    const [config] = await findRegistryConfig();
    const [entry] = findAdapterEntrySync(adapterProgram);
    const governance = this.provider.wallet.publicKey;

    const discriminator = Buffer.from([0x7f, 0x2a, 0xb1, 0x45, 0x93, 0xe8, 0x1c, 0x33]);

    const ix = new TransactionInstruction({
      programId: this.programId,
      keys: [
        { pubkey: config, isSigner: false, isWritable: true },
        { pubkey: entry, isSigner: false, isWritable: true },
        { pubkey: governance, isSigner: true, isWritable: false },
      ],
      data: Buffer.from(discriminator),
    });

    const tx = new Transaction().add(ix);
    return this.provider.sendAndConfirm(tx);
  }

  async rejectAdapter(adapterProgram: PublicKey, reason: string): Promise<string> {
    const [config] = await findRegistryConfig();
    const [entry] = findAdapterEntrySync(adapterProgram);
    const governance = this.provider.wallet.publicKey;

    const discriminator = Buffer.from([0x91, 0x4c, 0x23, 0xa7, 0x55, 0xe2, 0x8b, 0x1f]);
    const reasonBytes = Buffer.from(reason, "utf8");
    const lenBuf = Buffer.alloc(4);
    lenBuf.writeUInt32LE(reasonBytes.length);
    const data = Buffer.concat([Buffer.from(discriminator), lenBuf, reasonBytes]);

    const ix = new TransactionInstruction({
      programId: this.programId,
      keys: [
        { pubkey: config, isSigner: false, isWritable: false },
        { pubkey: entry, isSigner: false, isWritable: true },
        { pubkey: governance, isSigner: true, isWritable: false },
      ],
      data,
    });

    const tx = new Transaction().add(ix);
    return this.provider.sendAndConfirm(tx);
  }

  async deprecateAdapter(adapterProgram: PublicKey, reason: string): Promise<string> {
    const [config] = await findRegistryConfig();
    const [entry] = findAdapterEntrySync(adapterProgram);
    const governance = this.provider.wallet.publicKey;

    const discriminator = Buffer.from([0x44, 0xb8, 0x9e, 0x13, 0x72, 0xc5, 0x3a, 0xf7]);
    const reasonBytes = Buffer.from(reason, "utf8");
    const lenBuf = Buffer.alloc(4);
    lenBuf.writeUInt32LE(reasonBytes.length);
    const data = Buffer.concat([Buffer.from(discriminator), lenBuf, reasonBytes]);

    const ix = new TransactionInstruction({
      programId: this.programId,
      keys: [
        { pubkey: config, isSigner: false, isWritable: true },
        { pubkey: entry, isSigner: false, isWritable: true },
        { pubkey: governance, isSigner: true, isWritable: false },
      ],
      data,
    });

    const tx = new Transaction().add(ix);
    return this.provider.sendAndConfirm(tx);
  }

  async fetchRegistryConfig(): Promise<RegistryConfig | null> {
    const [pda] = await findRegistryConfig();
    const info = await this.provider.connection.getAccountInfo(pda);
    if (!info) return null;
    return this._decodeRegistryConfig(info.data);
  }

  async fetchAdapterEntry(adapterProgram: PublicKey): Promise<AdapterEntry | null> {
    const [pda] = await findAdapterEntry(adapterProgram);
    const info = await this.provider.connection.getAccountInfo(pda);
    if (!info) return null;
    return this._decodeAdapterEntry(info.data);
  }

  async isAdapterActive(adapterProgram: PublicKey): Promise<boolean> {
    const entry = await this.fetchAdapterEntry(adapterProgram);
    return entry?.status === ADAPTER_STATUS.ACTIVE;
  }

  async listActiveAdapters(knownAdapters: PublicKey[]): Promise<AdapterEntry[]> {
    const entries = await Promise.all(
      knownAdapters.map((p) => this.fetchAdapterEntry(p))
    );
    return entries.filter(
      (e): e is AdapterEntry => e !== null && e.status === ADAPTER_STATUS.ACTIVE
    );
  }

  private _decodeRegistryConfig(data: Buffer): RegistryConfig {
    let offset = 8;
    const governance = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
    const totalProposed = data.readUInt32LE(offset); offset += 4;
    const totalActive = data.readUInt32LE(offset); offset += 4;
    const bump = data[offset];
    return { governance, totalProposed, totalActive, bump };
  }

  private _decodeAdapterEntry(data: Buffer): AdapterEntry {
    let offset = 8;
    const adapterProgram = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
    const inputMint = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
    const status = data[offset]; offset += 1;
    const protocolNameRaw = data.slice(offset, offset + 64); offset += 64;
    const descriptionRaw = data.slice(offset, offset + 256); offset += 256;
    const protocolNameLen = data[offset]; offset += 1;
    const descriptionLen = data.readUInt16LE(offset); offset += 2;
    const proposer = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
    const proposedSlot = new BN(data.slice(offset, offset + 8), "le"); offset += 8;
    const actionedSlot = new BN(data.slice(offset, offset + 8), "le"); offset += 8;
    const actionReasonRaw = data.slice(offset, offset + 128); offset += 128;
    const actionReasonLen = data[offset]; offset += 1;
    const bump = data[offset];

    return {
      adapterProgram,
      inputMint,
      status,
      protocolName: protocolNameRaw.slice(0, protocolNameLen).toString("utf8"),
      description: descriptionRaw.slice(0, descriptionLen).toString("utf8"),
      proposer,
      proposedSlot,
      actionedSlot,
      actionReason: actionReasonRaw.slice(0, actionReasonLen).toString("utf8"),
      bump,
    };
  }
}
