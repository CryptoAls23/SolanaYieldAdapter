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
  MARGINFI_ADAPTER_PROGRAM_ID,
  JUPITER_LP_ADAPTER_PROGRAM_ID,
  MAPLE_ADAPTER_PROGRAM_ID,
  DRIFT_ADAPTER_PROGRAM_ID,
  MARGINFI_USDC_BANK,
  JLP_POOL,
  JLP_MINT,
  MAPLE_SYRUP_POOL,
  SYUSDC_MINT,
  DRIFT_STATE,
  DRIFT_IF_VAULT,
  DRIFT_USDC_SPOT_MARKET,
  findDispatcherAuthoritySync,
  buildMarginFiAccounts,
  buildJupiterLpAccounts,
  buildMapleAccounts,
  buildDriftAccounts,
  findDriftInsuranceFundStake,
  findMarginFiAccount,
  MARGINFI_PROGRAM_ID,
  DRIFT_PROGRAM_ID,
} from "../../sdk/src";
import { getOrCreateForkContext, ForkContext } from "./fork-setup";

describe("Adapter: MarginFi USDC [mainnet-fork]", () => {
  let ctx: ForkContext;
  let marginfiAccount: PublicKey;
  let bankLiquidityVault: PublicKey;
  let bankLiquidityVaultAuthority: PublicKey;

  before(async () => {
    logSection("MarginFi USDC Adapter - Mainnet Fork");
    ctx = await getOrCreateForkContext();
    [marginfiAccount] = findMarginFiAccount(ctx.user.publicKey);
    [bankLiquidityVault] = PublicKey.findProgramAddressSync(
      [Buffer.from("liquidity_vault"), MARGINFI_USDC_BANK.toBuffer()],
      MARGINFI_PROGRAM_ID
    );
    [bankLiquidityVaultAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from("liquidity_vault_auth"), MARGINFI_USDC_BANK.toBuffer()],
      MARGINFI_PROGRAM_ID
    );
    await ctx.userDispatcherClient.initializePosition(MARGINFI_ADAPTER_PROGRAM_ID);
    logResult("position initialized", "ok");
  });

  it("reads MarginFi bank data from mainnet", async () => {
    const bankInfo = await ctx.connection.getAccountInfo(MARGINFI_USDC_BANK);
    expect(bankInfo).to.not.be.null;
    expect(bankInfo!.data.length).to.be.greaterThan(380);
    logResult("bank account size", bankInfo!.data.length);
  });

  it("deposits 10 USDC into MarginFi bank", async () => {
    const [dispatcherAuthority] = findDispatcherAuthoritySync();
    const accs = buildMarginFiAccounts(
      marginfiAccount, bankLiquidityVault,
      bankLiquidityVaultAuthority, dispatcherAuthority, ctx.dispatcherVault
    );
    const result = await ctx.userDispatcherClient.deposit({
      adapterProgram: MARGINFI_ADAPTER_PROGRAM_ID,
      amount: TEN_USDC,
      minSharesOut: applySlippage(TEN_USDC, SLIPPAGE_BPS),
      adapterAccounts: accs,
    });
    assertGt(result.sharesMinted, new BN(0), "sharesMinted");
    logResult("shares minted", result.sharesMinted.toString());
  });

  it("current_value approx deposited amount", async () => {
    const [dispatcherAuthority] = findDispatcherAuthoritySync();
    const position = await ctx.dispatcherClient.fetchPosition(
      ctx.user.publicKey, MARGINFI_ADAPTER_PROGRAM_ID
    );
    const result = await ctx.userDispatcherClient.currentValue({
      adapterProgram: MARGINFI_ADAPTER_PROGRAM_ID,
      shares: new BN(position!.shares.toString()),
      adapterAccounts: [
        { pubkey: MARGINFI_USDC_BANK, isSigner: false, isWritable: false },
        { pubkey: dispatcherAuthority, isSigner: false, isWritable: false },
      ],
    });
    assertApproxEqual(result.value, TEN_USDC, 100, "current_value");
    logResult("current value", result.value.toString());
  });

  it("withdraws and receives USDC back", async () => {
    const [dispatcherAuthority] = findDispatcherAuthoritySync();
    const position = await ctx.dispatcherClient.fetchPosition(
      ctx.user.publicKey, MARGINFI_ADAPTER_PROGRAM_ID
    );
    const accs = buildMarginFiAccounts(
      marginfiAccount, bankLiquidityVault,
      bankLiquidityVaultAuthority, dispatcherAuthority, ctx.dispatcherVault
    );
    const result = await ctx.userDispatcherClient.withdraw({
      adapterProgram: MARGINFI_ADAPTER_PROGRAM_ID,
      shares: new BN(position!.shares.toString()),
      minAmountOut: applySlippage(TEN_USDC, SLIPPAGE_BPS),
      adapterAccounts: accs,
    });
    assertGt(result.amountOut, new BN(0), "amountOut");
    logResult("amount out", result.amountOut.toString());
  });
});

describe("Adapter: Jupiter LP [mainnet-fork]", () => {
  let ctx: ForkContext;
  let custodyTokenAccount: PublicKey;
  let custodyOracle: PublicKey;

  before(async () => {
    logSection("Jupiter LP Adapter - Mainnet Fork");
    ctx = await getOrCreateForkContext();
    custodyTokenAccount = new PublicKey("AQCGyheWPLeo6Qp9WpYS9m3Qj479t7R636N9ey1rEjEn");
    custodyOracle = new PublicKey("Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD");
    await ctx.userDispatcherClient.initializePosition(JUPITER_LP_ADAPTER_PROGRAM_ID);
    logResult("position initialized", "ok");
  });

  it("reads JLP pool and mint from mainnet", async () => {
    const [poolInfo, mintInfo] = await Promise.all([
      ctx.connection.getAccountInfo(JLP_POOL),
      ctx.connection.getAccountInfo(JLP_MINT),
    ]);
    expect(poolInfo).to.not.be.null;
    expect(mintInfo).to.not.be.null;
    logResult("JLP pool size", poolInfo!.data.length);
  });

  it("deposits 10 USDC and receives JLP shares", async () => {
    const [dispatcherAuthority] = findDispatcherAuthoritySync();
    const accs = buildJupiterLpAccounts(
      ctx.user.publicKey, ctx.dispatcherVault,
      dispatcherAuthority, custodyTokenAccount, custodyOracle
    );
    const result = await ctx.userDispatcherClient.deposit({
      adapterProgram: JUPITER_LP_ADAPTER_PROGRAM_ID,
      amount: TEN_USDC,
      minSharesOut: applySlippage(TEN_USDC, SLIPPAGE_BPS),
      adapterAccounts: accs,
    });
    assertGt(result.sharesMinted, new BN(0), "JLP minted");
    logResult("JLP minted", result.sharesMinted.toString());
  });

  it("current_value reflects JLP price", async () => {
    const [dispatcherAuthority] = findDispatcherAuthoritySync();
    const position = await ctx.dispatcherClient.fetchPosition(
      ctx.user.publicKey, JUPITER_LP_ADAPTER_PROGRAM_ID
    );
    const result = await ctx.userDispatcherClient.currentValue({
      adapterProgram: JUPITER_LP_ADAPTER_PROGRAM_ID,
      shares: new BN(position!.shares.toString()),
      adapterAccounts: [
        { pubkey: JLP_POOL, isSigner: false, isWritable: false },
        { pubkey: JLP_MINT, isSigner: false, isWritable: false },
        { pubkey: dispatcherAuthority, isSigner: false, isWritable: false },
      ],
    });
    assertGt(result.value, new BN(0), "JLP value");
    logResult("JLP position value", result.value.toString());
  });

  it("withdraws JLP and receives USDC back", async () => {
    const [dispatcherAuthority] = findDispatcherAuthoritySync();
    const position = await ctx.dispatcherClient.fetchPosition(
      ctx.user.publicKey, JUPITER_LP_ADAPTER_PROGRAM_ID
    );
    const accs = buildJupiterLpAccounts(
      ctx.user.publicKey, ctx.dispatcherVault,
      dispatcherAuthority, custodyTokenAccount, custodyOracle
    );
    const result = await ctx.userDispatcherClient.withdraw({
      adapterProgram: JUPITER_LP_ADAPTER_PROGRAM_ID,
      shares: new BN(position!.shares.toString()),
      minAmountOut: applySlippage(TEN_USDC, SLIPPAGE_BPS * 3),
      adapterAccounts: accs,
    });
    assertGt(result.amountOut, new BN(0), "USDC out");
    logResult("USDC received", result.amountOut.toString());
  });
});

describe("Adapter: Maple Syrup [mainnet-fork]", () => {
  let ctx: ForkContext;
  let poolUsdcVault: PublicKey;

  before(async () => {
    logSection("Maple Finance Syrup - Mainnet Fork");
    ctx = await getOrCreateForkContext();
    [poolUsdcVault] = PublicKey.findProgramAddressSync(
      [Buffer.from("pool_usdc_vault"), MAPLE_SYRUP_POOL.toBuffer()],
      new PublicKey("MaPLEd3SertEXhgnhmkMWMNUgZ3p3WGEVnJyoaJpump")
    );
    await ctx.userDispatcherClient.initializePosition(MAPLE_ADAPTER_PROGRAM_ID);
    logResult("position initialized", "ok");
  });

  it("reads Maple pool state from mainnet", async () => {
    const poolInfo = await ctx.connection.getAccountInfo(MAPLE_SYRUP_POOL);
    expect(poolInfo).to.not.be.null;
    logResult("pool account size", poolInfo!.data.length);
  });

  it("deposits 10 USDC and receives syUSDC", async () => {
    const [dispatcherAuthority] = findDispatcherAuthoritySync();
    const accs = buildMapleAccounts(
      ctx.user.publicKey, ctx.dispatcherVault, dispatcherAuthority, poolUsdcVault
    );
    const result = await ctx.userDispatcherClient.deposit({
      adapterProgram: MAPLE_ADAPTER_PROGRAM_ID,
      amount: TEN_USDC,
      minSharesOut: applySlippage(TEN_USDC, SLIPPAGE_BPS),
      adapterAccounts: accs,
    });
    assertGt(result.sharesMinted, new BN(0), "syUSDC minted");
    logResult("syUSDC minted", result.sharesMinted.toString());
  });

  it("current_value matches deposit amount", async () => {
    const [dispatcherAuthority] = findDispatcherAuthoritySync();
    const position = await ctx.dispatcherClient.fetchPosition(
      ctx.user.publicKey, MAPLE_ADAPTER_PROGRAM_ID
    );
    const result = await ctx.userDispatcherClient.currentValue({
      adapterProgram: MAPLE_ADAPTER_PROGRAM_ID,
      shares: new BN(position!.shares.toString()),
      adapterAccounts: [
        { pubkey: MAPLE_SYRUP_POOL, isSigner: false, isWritable: false },
        { pubkey: SYUSDC_MINT, isSigner: false, isWritable: false },
        { pubkey: dispatcherAuthority, isSigner: false, isWritable: false },
      ],
    });
    assertApproxEqual(result.value, TEN_USDC, 100, "syrup value");
    logResult("syrup value", result.value.toString());
  });

  it("redeems syUSDC back to USDC", async () => {
    const [dispatcherAuthority] = findDispatcherAuthoritySync();
    const position = await ctx.dispatcherClient.fetchPosition(
      ctx.user.publicKey, MAPLE_ADAPTER_PROGRAM_ID
    );
    const accs = buildMapleAccounts(
      ctx.user.publicKey, ctx.dispatcherVault, dispatcherAuthority, poolUsdcVault
    );
    const result = await ctx.userDispatcherClient.withdraw({
      adapterProgram: MAPLE_ADAPTER_PROGRAM_ID,
      shares: new BN(position!.shares.toString()),
      minAmountOut: applySlippage(TEN_USDC, SLIPPAGE_BPS),
      adapterAccounts: accs,
    });
    assertGt(result.amountOut, new BN(0), "USDC out");
    logResult("USDC redeemed", result.amountOut.toString());
  });
});

describe("Adapter: Drift Insurance Fund [mainnet-fork]", () => {
  let ctx: ForkContext;
  let ifStake: PublicKey;

  before(async () => {
    logSection("Drift Insurance Fund - Mainnet Fork");
    ctx = await getOrCreateForkContext();
    ifStake = findDriftInsuranceFundStake(ctx.user.publicKey, 0);
    await ctx.userDispatcherClient.initializePosition(DRIFT_ADAPTER_PROGRAM_ID);
    logResult("position initialized", "ok");
    logResult("IF stake PDA", ifStake.toBase58().slice(0, 20) + "...");
  });

  it("reads Drift state and IF vault from mainnet", async () => {
    const [stateInfo, vaultInfo, marketInfo] = await Promise.all([
      ctx.connection.getAccountInfo(DRIFT_STATE),
      ctx.connection.getAccountInfo(DRIFT_IF_VAULT),
      ctx.connection.getAccountInfo(DRIFT_USDC_SPOT_MARKET),
    ]);
    expect(stateInfo).to.not.be.null;
    expect(vaultInfo).to.not.be.null;
    expect(marketInfo).to.not.be.null;
    logResult("Drift state size", stateInfo!.data.length);
  });

  it("stakes 10 USDC into Drift Insurance Fund", async () => {
    const [dispatcherAuthority] = findDispatcherAuthoritySync();
    const accs = buildDriftAccounts(ifStake, ctx.dispatcherVault, dispatcherAuthority);
    const result = await ctx.userDispatcherClient.deposit({
      adapterProgram: DRIFT_ADAPTER_PROGRAM_ID,
      amount: TEN_USDC,
      minSharesOut: applySlippage(TEN_USDC, SLIPPAGE_BPS),
      adapterAccounts: accs,
    });
    assertGt(result.sharesMinted, new BN(0), "IF shares");
    logResult("IF shares minted", result.sharesMinted.toString());
  });

  it("current_value reflects exchange rate", async () => {
    const [dispatcherAuthority] = findDispatcherAuthoritySync();
    const position = await ctx.dispatcherClient.fetchPosition(
      ctx.user.publicKey, DRIFT_ADAPTER_PROGRAM_ID
    );
    const result = await ctx.userDispatcherClient.currentValue({
      adapterProgram: DRIFT_ADAPTER_PROGRAM_ID,
      shares: new BN(position!.shares.toString()),
      adapterAccounts: [
        { pubkey: DRIFT_USDC_SPOT_MARKET, isSigner: false, isWritable: false },
        { pubkey: DRIFT_IF_VAULT, isSigner: false, isWritable: false },
        { pubkey: dispatcherAuthority, isSigner: false, isWritable: false },
      ],
    });
    assertApproxEqual(result.value, TEN_USDC, 100, "IF value");
    logResult("IF position value", result.value.toString());
  });

  it("requests unstake (13-day cooldown begins)", async () => {
    const [dispatcherAuthority] = findDispatcherAuthoritySync();
    const position = await ctx.dispatcherClient.fetchPosition(
      ctx.user.publicKey, DRIFT_ADAPTER_PROGRAM_ID
    );
    const accs = buildDriftAccounts(ifStake, ctx.dispatcherVault, dispatcherAuthority);
    const result = await ctx.userDispatcherClient.withdraw({
      adapterProgram: DRIFT_ADAPTER_PROGRAM_ID,
      shares: new BN(position!.shares.toString()),
      minAmountOut: new BN(0),
      adapterAccounts: accs,
    });
    expect(result.amountOut.toString()).to.equal("0");
    logResult("unstake requested", "ok");
    logResult("note", "tokens released after 13-day cooldown");
  });
});
