import { PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import {
  KAMINO_LENDING_MARKET,
  KAMINO_USDC_RESERVE,
  KUSDC_MINT,
  KAMINO_PROGRAM_ID,
  MARGINFI_USDC_BANK,
  MARGINFI_GROUP,
  MARGINFI_PROGRAM_ID,
  JLP_POOL,
  JLP_MINT,
  JLP_USDC_CUSTODY,
  JUPITER_PERPS_PROGRAM_ID,
  MAPLE_SYRUP_POOL,
  SYUSDC_MINT,
  MAPLE_PROGRAM_ID,
  DRIFT_STATE,
  DRIFT_IF_VAULT,
  DRIFT_USDC_SPOT_MARKET,
  DRIFT_PROGRAM_ID,
} from "../constants";
import { AdapterAccountMeta } from "../types";

export function buildKaminoAccounts(
  user: PublicKey,
  dispatcherAuthority: PublicKey,
  dispatcherVault: PublicKey,
  kaminoReserveLiquidity: PublicKey,
  kaminoLendingMarketAuthority: PublicKey
): AdapterAccountMeta[] {
  const userKusdcAccount = getAssociatedTokenAddressSync(KUSDC_MINT, user);

  return [
    { pubkey: dispatcherVault, isSigner: false, isWritable: true },
    { pubkey: kaminoReserveLiquidity, isSigner: false, isWritable: true },
    { pubkey: userKusdcAccount, isSigner: false, isWritable: true },
    { pubkey: KUSDC_MINT, isSigner: false, isWritable: true },
    { pubkey: KAMINO_USDC_RESERVE, isSigner: false, isWritable: true },
    { pubkey: KAMINO_LENDING_MARKET, isSigner: false, isWritable: false },
    { pubkey: kaminoLendingMarketAuthority, isSigner: false, isWritable: false },
    { pubkey: dispatcherAuthority, isSigner: false, isWritable: false },
    { pubkey: KAMINO_PROGRAM_ID, isSigner: false, isWritable: false },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
  ];
}

export function buildMarginFiAccounts(
  marginfiAccount: PublicKey,
  bankLiquidityVault: PublicKey,
  bankLiquidityVaultAuthority: PublicKey,
  dispatcherAuthority: PublicKey,
  dispatcherVault: PublicKey
): AdapterAccountMeta[] {
  return [
    { pubkey: dispatcherVault, isSigner: false, isWritable: true },
    { pubkey: marginfiAccount, isSigner: false, isWritable: true },
    { pubkey: MARGINFI_GROUP, isSigner: false, isWritable: true },
    { pubkey: MARGINFI_USDC_BANK, isSigner: false, isWritable: true },
    { pubkey: bankLiquidityVault, isSigner: false, isWritable: true },
    { pubkey: bankLiquidityVaultAuthority, isSigner: false, isWritable: false },
    { pubkey: dispatcherAuthority, isSigner: false, isWritable: false },
    { pubkey: MARGINFI_PROGRAM_ID, isSigner: false, isWritable: false },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
  ];
}

export function buildJupiterLpAccounts(
  user: PublicKey,
  dispatcherVault: PublicKey,
  dispatcherAuthority: PublicKey,
  custodyTokenAccount: PublicKey,
  custodyOracle: PublicKey
): AdapterAccountMeta[] {
  const userJlpAccount = getAssociatedTokenAddressSync(JLP_MINT, user);

  return [
    { pubkey: dispatcherVault, isSigner: false, isWritable: true },
    { pubkey: userJlpAccount, isSigner: false, isWritable: true },
    { pubkey: JLP_MINT, isSigner: false, isWritable: true },
    { pubkey: JLP_POOL, isSigner: false, isWritable: true },
    { pubkey: JLP_USDC_CUSTODY, isSigner: false, isWritable: true },
    { pubkey: custodyTokenAccount, isSigner: false, isWritable: true },
    { pubkey: custodyOracle, isSigner: false, isWritable: false },
    { pubkey: dispatcherAuthority, isSigner: false, isWritable: false },
    { pubkey: JUPITER_PERPS_PROGRAM_ID, isSigner: false, isWritable: false },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
  ];
}

export function buildMapleAccounts(
  user: PublicKey,
  dispatcherVault: PublicKey,
  dispatcherAuthority: PublicKey,
  poolUsdcVault: PublicKey
): AdapterAccountMeta[] {
  const userSyusdcAccount = getAssociatedTokenAddressSync(SYUSDC_MINT, user);

  return [
    { pubkey: dispatcherVault, isSigner: false, isWritable: true },
    { pubkey: userSyusdcAccount, isSigner: false, isWritable: true },
    { pubkey: SYUSDC_MINT, isSigner: false, isWritable: true },
    { pubkey: MAPLE_SYRUP_POOL, isSigner: false, isWritable: true },
    { pubkey: poolUsdcVault, isSigner: false, isWritable: true },
    { pubkey: dispatcherAuthority, isSigner: false, isWritable: false },
    { pubkey: MAPLE_PROGRAM_ID, isSigner: false, isWritable: false },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
  ];
}

export function buildDriftAccounts(
  insuranceFundStake: PublicKey,
  dispatcherVault: PublicKey,
  dispatcherAuthority: PublicKey
): AdapterAccountMeta[] {
  return [
    { pubkey: dispatcherVault, isSigner: false, isWritable: true },
    { pubkey: insuranceFundStake, isSigner: false, isWritable: true },
    { pubkey: DRIFT_USDC_SPOT_MARKET, isSigner: false, isWritable: true },
    { pubkey: DRIFT_IF_VAULT, isSigner: false, isWritable: true },
    { pubkey: DRIFT_STATE, isSigner: false, isWritable: false },
    { pubkey: dispatcherAuthority, isSigner: false, isWritable: false },
    { pubkey: DRIFT_PROGRAM_ID, isSigner: false, isWritable: false },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
  ];
}

export function findDriftInsuranceFundStake(
  user: PublicKey,
  marketIndex: number = 0
): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("insurance_fund_stake"),
      user.toBuffer(),
      Buffer.from(new Uint16Array([marketIndex]).buffer),
    ],
    DRIFT_PROGRAM_ID
  );
  return pda;
}

export function findMarginFiAccount(
  user: PublicKey,
  group: PublicKey = MARGINFI_GROUP
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("marginfi_account"), group.toBuffer(), user.toBuffer()],
    MARGINFI_PROGRAM_ID
  );
}
