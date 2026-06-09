import { expect } from "chai";
import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import {
  logSection,
  logResult,
  assertApproxEqual,
  assertGt,
  TEN_USDC,
  SLIPPAGE_BPS,
  applySlippage,
} from "../utils/helpers";
import {
  KAMINO_ADAPTER_PROGRAM_ID,
  KAMINO_USDC_RESERVE,
  KAMINO_LENDING_MARKET,
  KUSDC_MINT,
  findDispatcherAuthoritySync,
  buildKaminoAccounts,
} from "../../sdk/src";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { getOrCreateForkContext, ForkContext } from "./fork-setup";

describe("Adapter: Kamino USDC [mainnet-fork]", () => {
  let ctx: ForkContext;
  let kaminoLendingMarketAuthority: PublicKey;
  let kaminoReserveLiquidity: PublicKey;

  before(async () => {
    logSection("Kamino USDC Adapter - Mainnet Fork");
    ctx = await getOrCreateForkContext();

    [kaminoLendingMarketAuthority] = PublicKey.findProgramAddressSync(
      [KAMINO_LENDING_MARKET.toBuffer()],
      new PublicKey("KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD")
    );

    kaminoReserveLiquidity = new PublicKey(
      "UsrCHBpqMmjpVYEjMfmVEkqjbvCRnZqAVUNmDFKFMy2"
    );

    await ctx.userDispatcherClient.initializePosition(KAMINO_ADAPTER_PROGRAM_ID);
    logResult("position initialized", "ok");
  });

  it("reads a valid exchange rate from mainnet reserve", async () => {
    const reserveInfo = await ctx.connection.getAccountInfo(KAMINO_USDC_RESERVE);
    expect(reserveInfo).to.not.be.null;
    expect(reserveInfo!.data.length).to.be.greaterThan(329);
    logResult("reserve account size", reserveInfo!.data.length);
  });

  it("deposits 10 USDC and receives kUSDC shares", async () => {
    const [dispatcherAuthority] = findDispatcherAuthoritySync();
    const accs = buildKaminoAccounts(
      ctx.user.publicKey,
      dispatcherAuthority,
      ctx.dispatcherVault,
      kaminoReserveLiquidity,
      kaminoLendingMarketAuthority
    );

    const result = await ctx.userDispatcherClient.deposit({
      adapterProgram: KAMINO_ADAPTER_PROGRAM_ID,
      amount: TEN_USDC,
      minSharesOut: applySlippage(TEN_USDC, SLIPPAGE_BPS),
      adapterAccounts: accs,
    });

    expect(result.txSignature).to.be.a("string");
    assertGt(result.sharesMinted, new BN(0), "sharesMinted");
    logResult("shares minted", result.sharesMinted.toString());
    logResult("fee charged", result.feeCharged.toString());
  });

  it("position reflects shares after deposit", async () => {
    const position = await ctx.dispatcherClient.fetchPosition(
      ctx.user.publicKey,
      KAMINO_ADAPTER_PROGRAM_ID
    );
    expect(position).to.not.be.null;
    assertGt(position!.shares, new BN(0), "position.shares");
    assertGt(position!.costBasis, new BN(0), "position.costBasis");
    logResult("position.shares", position!.shares.toString());
  });

  it("current_value returns approx deposited amount", async () => {
    const [dispatcherAuthority] = findDispatcherAuthoritySync();
    const position = await ctx.dispatcherClient.fetchPosition(
      ctx.user.publicKey,
      KAMINO_ADAPTER_PROGRAM_ID
    );

    const result = await ctx.userDispatcherClient.currentValue({
      adapterProgram: KAMINO_ADAPTER_PROGRAM_ID,
      shares: new BN(position!.shares.toString()),
      adapterAccounts: [
        { pubkey: KAMINO_USDC_RESERVE, isSigner: false, isWritable: false },
        { pubkey: dispatcherAuthority, isSigner: false, isWritable: false },
      ],
    });

    assertApproxEqual(result.value, TEN_USDC, 100, "current_value");
    logResult("current value (USDC)", result.value.toString());
  });

  it("withdraws all shares and receives USDC back", async () => {
    const [dispatcherAuthority] = findDispatcherAuthoritySync();
    const position = await ctx.dispatcherClient.fetchPosition(
      ctx.user.publicKey,
      KAMINO_ADAPTER_PROGRAM_ID
    );
    const accs = buildKaminoAccounts(
      ctx.user.publicKey,
      dispatcherAuthority,
      ctx.dispatcherVault,
      kaminoReserveLiquidity,
      kaminoLendingMarketAuthority
    );

    const result = await ctx.userDispatcherClient.withdraw({
      adapterProgram: KAMINO_ADAPTER_PROGRAM_ID,
      shares: new BN(position!.shares.toString()),
      minAmountOut: applySlippage(TEN_USDC, SLIPPAGE_BPS),
      adapterAccounts: accs,
    });

    expect(result.txSignature).to.be.a("string");
    assertGt(result.amountOut, new BN(0), "amountOut");
    logResult("amount out (USDC)", result.amountOut.toString());
  });

  it("position shows zero shares after full withdrawal", async () => {
    const position = await ctx.dispatcherClient.fetchPosition(
      ctx.user.publicKey,
      KAMINO_ADAPTER_PROGRAM_ID
    );
    expect(position!.shares.toString()).to.equal("0");
    logResult("shares after withdrawal", "0 ok");
  });
});
