import { PublicKey } from "@solana/web3.js";
import {
  YIELD_DISPATCHER_PROGRAM_ID,
  ADAPTER_REGISTRY_PROGRAM_ID,
  SEEDS,
} from "./constants";

export async function findDispatcherConfig(): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddress(
    [SEEDS.DISPATCHER_CONFIG],
    YIELD_DISPATCHER_PROGRAM_ID
  );
}

export async function findDispatcherAuthority(): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddress(
    [SEEDS.DISPATCHER_AUTHORITY],
    YIELD_DISPATCHER_PROGRAM_ID
  );
}

export async function findAdapterState(
  adapterProgram: PublicKey
): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddress(
    [SEEDS.ADAPTER_STATE, adapterProgram.toBuffer()],
    YIELD_DISPATCHER_PROGRAM_ID
  );
}

export async function findPosition(
  user: PublicKey,
  adapterProgram: PublicKey
): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddress(
    [SEEDS.POSITION, user.toBuffer(), adapterProgram.toBuffer()],
    YIELD_DISPATCHER_PROGRAM_ID
  );
}

export async function findRegistryConfig(): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddress(
    [SEEDS.REGISTRY_CONFIG],
    ADAPTER_REGISTRY_PROGRAM_ID
  );
}

export async function findAdapterEntry(
  adapterProgram: PublicKey
): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddress(
    [SEEDS.ADAPTER_ENTRY, adapterProgram.toBuffer()],
    ADAPTER_REGISTRY_PROGRAM_ID
  );
}

export function findDispatcherConfigSync(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEEDS.DISPATCHER_CONFIG],
    YIELD_DISPATCHER_PROGRAM_ID
  );
}

export function findDispatcherAuthoritySync(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEEDS.DISPATCHER_AUTHORITY],
    YIELD_DISPATCHER_PROGRAM_ID
  );
}

export function findAdapterStateSync(
  adapterProgram: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEEDS.ADAPTER_STATE, adapterProgram.toBuffer()],
    YIELD_DISPATCHER_PROGRAM_ID
  );
}

export function findPositionSync(
  user: PublicKey,
  adapterProgram: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEEDS.POSITION, user.toBuffer(), adapterProgram.toBuffer()],
    YIELD_DISPATCHER_PROGRAM_ID
  );
}

export function findAdapterEntrySync(
  adapterProgram: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEEDS.ADAPTER_ENTRY, adapterProgram.toBuffer()],
    ADAPTER_REGISTRY_PROGRAM_ID
  );
}
