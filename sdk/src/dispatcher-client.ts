import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
  SystemProgram,
} from "@solana/web3.js";
import {
  AnchorProvider,
  BN,
} from "@coral-xyz/anchor";
import {
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
} from "@solana/spl-token";
import {
  YIELD_DISPATCHER_PROGRAM_ID,
  USDC_MINT,
} from "./constants";
import {
  findDispatcherConfigSync,
  findDispatcherAuthoritySync,
  findAdapterStateSync,
  findPositionSync,
  findAdapterEntrySync,
} from "./pda";
import {
  DepositParams,
  WithdrawParams,
  CurrentValueParams,
  DepositResult,
  WithdrawResult,
  CurrentValueResult,
  DispatcherConfig,
  AdapterState,
  Position,
} from "./types";

export class DispatcherClient {
  readonly connection: Connection;
  readonly provider: AnchorProvider;
  readonly programId: PublicKey;
  readonly registryId: PublicKey;

  constructor(provider: AnchorProvider) {
    this.provider = provider;
    this.connection = provider.connection;
    this.programId = YIELD_DISPATCHER_PROGRAM_ID;
    this.registryId = new PublicKey(
      "AdPtReGiStRyXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
    );
  }

  async initializeDispatcher(
    registry: PublicKey,
    feeBps: number
  ): Promise<string> {
    const [config] = findDispatcherConfigSync();
    const authority = this.provider.wallet.publicKey;
    const ix = this._buildInitializeIx(config, authority, registry, feeBps);
    return this._sendAndConfirm([ix]);
  }

  async initializePosition(adapterProgram: PublicKey): Promise<string> {
    const user = this.provider.wallet.publicKey;
    const [config] = findDispatcherConfigSync();
    const [position] = findPositionSync(user, adapterProgram);
    const ix = this._buildInitializePositionIx(config, position, adapterProgram, user);
    return this._sendAndConfirm([ix]);
  }

  async deposit(params: DepositParams): Promise<DepositResult> {
    const user = this.provider.wallet.publicKey;
    const { adapterProgram, amount, minSharesOut, adapterAccounts } = params;

    const [config] = findDispatcherConfigSync();
    const [adapterState] = findAdapterStateSync(adapterProgram);
    const [position] = findPositionSync(user, adapterProgram);
    const [dispatcherAuthority] = findDispatcherAuthoritySync();
    const [adapterEntry] = findAdapterEntrySync(adapterProgram);

    const userTokenAccount = getAssociatedTokenAddressSync(USDC_MINT, user);
    const dispatcherVault = getAssociatedTokenAddressSync(
      USDC_MINT,
      dispatcherAuthority,
      true
    );

    const ixs: TransactionInstruction[] = [];

    const vaultInfo = await this.connection.getAccountInfo(dispatcherVault);
    if (!vaultInfo) {
      ixs.push(
        createAssociatedTokenAccountInstruction(
          user,
          dispatcherVault,
          dispatcherAuthority,
          USDC_MINT
        )
      );
    }

    const depositIx = this._buildDepositIx({
      config,
      adapterState,
      position,
      userTokenAccount,
      dispatcherVault,
      adapterProgram,
      dispatcherAuthority,
      user,
      adapterEntry,
      adapterAccounts,
      amount,
      minSharesOut,
    });
    ixs.push(depositIx);

    const sig = await this._sendAndConfirm(ixs);
    const { sharesMinted, feeCharged } = await this._parseDepositEvents(sig);
    return { txSignature: sig, sharesMinted, feeCharged };
  }

  async withdraw(params: WithdrawParams): Promise<WithdrawResult> {
    const user = this.provider.wallet.publicKey;
    const { adapterProgram, shares, minAmountOut, adapterAccounts } = params;

    const [config] = findDispatcherConfigSync();
    const [adapterState] = findAdapterStateSync(adapterProgram);
    const [position] = findPositionSync(user, adapterProgram);
    const [dispatcherAuthority] = findDispatcherAuthoritySync();
    const [adapterEntry] = findAdapterEntrySync(adapterProgram);

    const userTokenAccount = getAssociatedTokenAddressSync(USDC_MINT, user);
    const dispatcherVault = getAssociatedTokenAddressSync(
      USDC_MINT,
      dispatcherAuthority,
      true
    );

    const withdrawIx = this._buildWithdrawIx({
      config,
      adapterState,
      position,
      userTokenAccount,
      dispatcherVault,
      adapterProgram,
      dispatcherAuthority,
      user,
      adapterEntry,
      adapterAccounts,
      shares,
      minAmountOut,
    });

    const sig = await this._sendAndConfirm([withdrawIx]);
    const { amountOut } = await this._parseWithdrawEvents(sig);
    return { txSignature: sig, amountOut };
  }

  async currentValue(params: CurrentValueParams): Promise<CurrentValueResult> {
    const user = this.provider.wallet.publicKey;
    const { adapterProgram, shares, adapterAccounts } = params;

    const [config] = findDispatcherConfigSync();
    const [position] = findPositionSync(user, adapterProgram);
    const [dispatcherAuthority] = findDispatcherAuthoritySync();

    const ix = this._buildCurrentValueIx({
      config,
      position,
      adapterProgram,
      dispatcherAuthority,
      user,
      adapterAccounts,
      shares,
    });

    const sig = await this._sendAndConfirm([ix]);
    return this._parseCurrentValueResult(sig);
  }

  async fetchDispatcherConfig(): Promise<DispatcherConfig | null> {
    const [config] = findDispatcherConfigSync();
    const info = await this.connection.getAccountInfo(config);
    if (!info) return null;
    return this._decodeDispatcherConfig(info.data);
  }

  async fetchAdapterState(adapterProgram: PublicKey): Promise<AdapterState | null> {
    const [pda] = findAdapterStateSync(adapterProgram);
    const info = await this.connection.getAccountInfo(pda);
    if (!info) return null;
    return this._decodeAdapterState(info.data);
  }

  async fetchPosition(
    user: PublicKey,
    adapterProgram: PublicKey
  ): Promise<Position | null> {
    const [pda] = findPositionSync(user, adapterProgram);
    const info = await this.connection.getAccountInfo(pda);
    if (!info) return null;
    return this._decodePosition(info.data);
  }

  private _buildInitializePositionIx(
    config: PublicKey,
    position: PublicKey,
    adapterProgram: PublicKey,
    user: PublicKey
  ): TransactionInstruction {
    const discriminator = Buffer.from([
      0x3d, 0x4a, 0x7e, 0x12, 0xab, 0x31, 0xc5, 0x98,
    ]);

    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        { pubkey: config, isSigner: false, isWritable: false },
        { pubkey: position, isSigner: false, isWritable: true },
        { pubkey: adapterProgram, isSigner: false, isWritable: false },
        { pubkey: user, isSigner: true, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: discriminator,
    });
  }

  private _buildInitializeIx(
    config: PublicKey,
    authority: PublicKey,
    registry: PublicKey,
    feeBps: number
  ): TransactionInstruction {
    const discriminator = Buffer.from([
      0x17, 0xf7, 0x3e, 0x84, 0x12, 0xc3, 0x5a, 0x91,
    ]);
    const buf = Buffer.alloc(8 + 32 + 2);
    discriminator.copy(buf, 0);
    registry.toBuffer().copy(buf, 8);
    buf.writeUInt16LE(feeBps, 40);

    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        { pubkey: config, isSigner: false, isWritable: true },
        { pubkey: authority, isSigner: true, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: buf,
    });
  }

  private _buildDepositIx(args: {
    config: PublicKey;
    adapterState: PublicKey;
    position: PublicKey;
    userTokenAccount: PublicKey;
    dispatcherVault: PublicKey;
    adapterProgram: PublicKey;
    dispatcherAuthority: PublicKey;
    user: PublicKey;
    adapterEntry: PublicKey;
    adapterAccounts: Array<{ pubkey: PublicKey; isSigner: boolean; isWritable: boolean }>;
    amount: BN;
    minSharesOut: BN;
  }): TransactionInstruction {
    const discriminator = Buffer.from([
      0xf2, 0x23, 0xc6, 0x89, 0x52, 0xe1, 0xf2, 0xb6,
    ]);
    const buf = Buffer.alloc(8 + 8 + 8);
    discriminator.copy(buf, 0);
    args.amount.toArrayLike(Buffer, "le", 8).copy(buf, 8);
    args.minSharesOut.toArrayLike(Buffer, "le", 8).copy(buf, 16);

    const keys = [
      { pubkey: args.config, isSigner: false, isWritable: true },
      { pubkey: args.adapterState, isSigner: false, isWritable: true },
      { pubkey: args.position, isSigner: false, isWritable: true },
      { pubkey: args.userTokenAccount, isSigner: false, isWritable: true },
      { pubkey: args.dispatcherVault, isSigner: false, isWritable: true },
      { pubkey: args.adapterProgram, isSigner: false, isWritable: false },
      { pubkey: args.dispatcherAuthority, isSigner: false, isWritable: false },
      { pubkey: args.user, isSigner: true, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: args.adapterEntry, isSigner: false, isWritable: false },
      ...args.adapterAccounts,
    ];

    return new TransactionInstruction({
      programId: this.programId,
      keys,
      data: buf,
    });
  }

  private _buildWithdrawIx(args: {
    config: PublicKey;
    adapterState: PublicKey;
    position: PublicKey;
    userTokenAccount: PublicKey;
    dispatcherVault: PublicKey;
    adapterProgram: PublicKey;
    dispatcherAuthority: PublicKey;
    user: PublicKey;
    adapterEntry: PublicKey;
    adapterAccounts: Array<{ pubkey: PublicKey; isSigner: boolean; isWritable: boolean }>;
    shares: BN;
    minAmountOut: BN;
  }): TransactionInstruction {
    const discriminator = Buffer.from([
      0xb7, 0x12, 0x46, 0x9c, 0x94, 0x67, 0x33, 0xf4,
    ]);
    const buf = Buffer.alloc(8 + 8 + 8);
    discriminator.copy(buf, 0);
    args.shares.toArrayLike(Buffer, "le", 8).copy(buf, 8);
    args.minAmountOut.toArrayLike(Buffer, "le", 8).copy(buf, 16);

    const keys = [
      { pubkey: args.config, isSigner: false, isWritable: true },
      { pubkey: args.adapterState, isSigner: false, isWritable: true },
      { pubkey: args.position, isSigner: false, isWritable: true },
      { pubkey: args.userTokenAccount, isSigner: false, isWritable: true },
      { pubkey: args.dispatcherVault, isSigner: false, isWritable: true },
      { pubkey: args.adapterProgram, isSigner: false, isWritable: false },
      { pubkey: args.dispatcherAuthority, isSigner: false, isWritable: false },
      { pubkey: args.user, isSigner: true, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: args.adapterEntry, isSigner: false, isWritable: false },
      ...args.adapterAccounts,
    ];

    return new TransactionInstruction({
      programId: this.programId,
      keys,
      data: buf,
    });
  }

  private _buildCurrentValueIx(args: {
    config: PublicKey;
    position: PublicKey;
    adapterProgram: PublicKey;
    dispatcherAuthority: PublicKey;
    user: PublicKey;
    adapterAccounts: Array<{ pubkey: PublicKey; isSigner: boolean; isWritable: boolean }>;
    shares: BN;
  }): TransactionInstruction {
    const discriminator = Buffer.from([
      0x45, 0xa0, 0x37, 0x31, 0x61, 0xc3, 0x28, 0x21,
    ]);
    const buf = Buffer.alloc(8 + 8);
    discriminator.copy(buf, 0);
    args.shares.toArrayLike(Buffer, "le", 8).copy(buf, 8);

    const keys = [
      { pubkey: args.config, isSigner: false, isWritable: false },
      { pubkey: args.position, isSigner: false, isWritable: false },
      { pubkey: args.adapterProgram, isSigner: false, isWritable: false },
      { pubkey: args.dispatcherAuthority, isSigner: false, isWritable: false },
      { pubkey: args.user, isSigner: true, isWritable: false },
      ...args.adapterAccounts,
    ];

    return new TransactionInstruction({ programId: this.programId, keys, data: buf });
  }

  private async _sendAndConfirm(ixs: TransactionInstruction[]): Promise<string> {
    const tx = new Transaction().add(...ixs);
    return this.provider.sendAndConfirm(tx);
  }

  private _decodeDispatcherConfig(data: Buffer): DispatcherConfig {
    let offset = 8;
    const authority = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
    const registry = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
    const feeBps = data.readUInt16LE(offset); offset += 2;
    const paused = data[offset] === 1; offset += 1;
    const totalDeposits = new BN(data.slice(offset, offset + 16), "le"); offset += 16;
    const totalWithdrawals = new BN(data.slice(offset, offset + 16), "le"); offset += 16;
    const adapterCount = data.readUInt32LE(offset); offset += 4;
    const bump = data[offset];
    return { authority, registry, feeBps, paused, totalDeposits, totalWithdrawals, adapterCount, bump };
  }

  private _decodeAdapterState(data: Buffer): AdapterState {
    let offset = 8;
    const adapterProgram = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
    const inputMint = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
    const totalDeposited = new BN(data.slice(offset, offset + 16), "le"); offset += 16;
    const totalShares = new BN(data.slice(offset, offset + 16), "le"); offset += 16;
    const cumulativeDeposits = new BN(data.slice(offset, offset + 16), "le"); offset += 16;
    const cumulativeWithdrawals = new BN(data.slice(offset, offset + 16), "le"); offset += 16;
    const lastUpdatedSlot = new BN(data.slice(offset, offset + 8), "le"); offset += 8;
    const bump = data[offset];
    return { adapterProgram, inputMint, totalDeposited, totalShares, cumulativeDeposits, cumulativeWithdrawals, lastUpdatedSlot, bump };
  }

  private _decodePosition(data: Buffer): Position {
    let offset = 8;
    const owner = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
    const adapterProgram = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
    const shares = new BN(data.slice(offset, offset + 16), "le"); offset += 16;
    const costBasis = new BN(data.slice(offset, offset + 16), "le"); offset += 16;
    const totalWithdrawn = new BN(data.slice(offset, offset + 16), "le"); offset += 16;
    const createdSlot = new BN(data.slice(offset, offset + 8), "le"); offset += 8;
    const lastActionSlot = new BN(data.slice(offset, offset + 8), "le"); offset += 8;
    const bump = data[offset];
    return { owner, adapterProgram, shares, costBasis, totalWithdrawn, createdSlot, lastActionSlot, bump };
  }

  private async _parseDepositEvents(sig: string): Promise<{ sharesMinted: BN; feeCharged: BN }> {
    try {
      const tx = await this.connection.getTransaction(sig, {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
      });
      const logs = tx?.meta?.logMessages ?? [];
      for (const log of logs) {
        if (log.includes("shares_minted")) {
          const match = log.match(/shares_minted: (\d+), fee_charged: (\d+)/);
          if (match) {
            return { sharesMinted: new BN(match[1]), feeCharged: new BN(match[2]) };
          }
        }
      }
    } catch (_) {}
    return { sharesMinted: new BN(0), feeCharged: new BN(0) };
  }

  private async _parseWithdrawEvents(sig: string): Promise<{ amountOut: BN }> {
    try {
      const tx = await this.connection.getTransaction(sig, {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
      });
      const logs = tx?.meta?.logMessages ?? [];
      for (const log of logs) {
        if (log.includes("amount_out")) {
          const match = log.match(/amount_out: (\d+)/);
          if (match) return { amountOut: new BN(match[1]) };
        }
      }
    } catch (_) {}
    return { amountOut: new BN(0) };
  }

  private async _parseCurrentValueResult(sig: string): Promise<CurrentValueResult> {
    try {
      const tx = await this.connection.getTransaction(sig, {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
      });
      const returnData = tx?.meta?.returnData;
      if (returnData?.data) {
        const buf = Buffer.from(returnData.data[0], "base64");
        const value = new BN(buf.slice(0, 8), "le");
        const exchangeRate = new BN(buf.slice(8, 24), "le");
        return { value, exchangeRate };
      }
    } catch (_) {}
    return { value: new BN(0), exchangeRate: new BN(0) };
  }
}
