import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

export interface DispatcherConfig {
  authority: PublicKey;
  registry: PublicKey;
  feeBps: number;
  paused: boolean;
  totalDeposits: BN;
  totalWithdrawals: BN;
  adapterCount: number;
  bump: number;
}

export interface AdapterState {
  adapterProgram: PublicKey;
  inputMint: PublicKey;
  totalDeposited: BN;
  totalShares: BN;
  cumulativeDeposits: BN;
  cumulativeWithdrawals: BN;
  lastUpdatedSlot: BN;
  bump: number;
}

export interface Position {
  owner: PublicKey;
  adapterProgram: PublicKey;
  shares: BN;
  costBasis: BN;
  totalWithdrawn: BN;
  createdSlot: BN;
  lastActionSlot: BN;
  bump: number;
}

export interface AdapterEntry {
  adapterProgram: PublicKey;
  inputMint: PublicKey;
  status: number;
  protocolName: string;
  description: string;
  proposer: PublicKey;
  proposedSlot: BN;
  actionedSlot: BN;
  actionReason: string;
  bump: number;
}

export interface RegistryConfig {
  governance: PublicKey;
  totalProposed: number;
  totalActive: number;
  bump: number;
}

export interface DepositParams {
  adapterProgram: PublicKey;
  amount: BN;
  minSharesOut: BN;
  adapterAccounts: AdapterAccountMeta[];
}

export interface WithdrawParams {
  adapterProgram: PublicKey;
  shares: BN;
  minAmountOut: BN;
  adapterAccounts: AdapterAccountMeta[];
}

export interface CurrentValueParams {
  adapterProgram: PublicKey;
  shares: BN;
  adapterAccounts: AdapterAccountMeta[];
}

export interface AdapterAccountMeta {
  pubkey: PublicKey;
  isSigner: boolean;
  isWritable: boolean;
}

export interface DepositResult {
  txSignature: string;
  sharesMinted: BN;
  feeCharged: BN;
}

export interface WithdrawResult {
  txSignature: string;
  amountOut: BN;
}

export interface CurrentValueResult {
  value: BN;
  exchangeRate: BN;
}

export interface AdapterMeta {
  programId: PublicKey;
  name: string;
  protocol: string;
  inputMint: PublicKey;
  shareToken?: PublicKey;
  hasUnstakeCooldown?: boolean;
  cooldownDays?: number;
}

export type AdapterStatus = "Pending" | "Active" | "Deprecated" | "Rejected";

export function adapterStatusFromCode(code: number): AdapterStatus {
  switch (code) {
    case 0: return "Pending";
    case 1: return "Active";
    case 2: return "Deprecated";
    case 3: return "Rejected";
    default: return "Pending";
  }
}
