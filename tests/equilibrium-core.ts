import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { EquilibriumCore } from "../target/types/equilibrium_core";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  createAssociatedTokenAccount,
  mintTo,
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import { expect } from "chai";

describe("equilibrium-core", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.EquilibriumCore as Program<EquilibriumCore>;
  const wallet = provider.wallet as anchor.Wallet;

  // Test tokens
  let usdcMint: PublicKey;
  let usdtMint: PublicKey;
  let pyusdMint: PublicKey;
  let partnerTokenMint: PublicKey;

  // User token accounts
  let userUsdcAccount: PublicKey;
  let userUsdtAccount: PublicKey;
  let userPyusdAccount: PublicKey;
  let userPartnerTokenAccount: PublicKey;

  // AMM Config
  let ammConfig: PublicKey;
  let ammConfigBump: number;

  // Seed Pool
  let seedPool: PublicKey;
  let seedPoolBump: number;
  let seedPoolLpMint: PublicKey;
  let userSeedPoolLpAccount: PublicKey;

  // Pool token accounts
  let poolUsdcAccount: PublicKey;
  let poolUsdtAccount: PublicKey;
  let poolPyusdAccount: PublicKey;

  // Growth Pool
  let growthPool: PublicKey;
  let growthPoolBump: number;
  let growthPoolLpMint: PublicKey;
  let userGrowthPoolLpAccount: PublicKey;

  // Growth Pool token accounts
  let poolUsdcStarAccount: PublicKey;
  let poolPartnerTokenAccount: PublicKey;

  // User position
  let userSeedPosition: PublicKey;
  let userGrowthPosition: PublicKey;

  // Constants
  const DECIMALS = 6;
  const INITIAL_MINT_AMOUNT = 10_000_000_000; // 10,000 tokens with 6 decimals
  const DEFAULT_AMPLIFICATION = 200;
  const DEFAULT_WEIGHTS = [4500, 3500, 2000]; // 45% USDC, 35% USDT, 20% PYUSD

  // Initialize test tokens and accounts
  before(async () => {
    console.log("Setting up test environment...");

    // Create test token mints
    const usdcKeypair = Keypair.generate();
    usdcMint = await createMint(
      provider.connection,
      wallet.payer,
      wallet.publicKey,
      null,
      DECIMALS,
      usdcKeypair
    );
    console.log("USDC Mint created:", usdcMint.toString());

    const usdtKeypair = Keypair.generate();
    usdtMint = await createMint(
      provider.connection,
      wallet.payer,
      wallet.publicKey,
      null,
      DECIMALS,
      usdtKeypair
    );
    console.log("USDT Mint created:", usdtMint.toString());

    const pyusdKeypair = Keypair.generate();
    pyusdMint = await createMint(
      provider.connection,
      wallet.payer,
      wallet.publicKey,
      null,
      DECIMALS,
      pyusdKeypair
    );
    console.log("PYUSD Mint created:", pyusdMint.toString());

    const partnerKeypair = Keypair.generate();
    partnerTokenMint = await createMint(
      provider.connection,
      wallet.payer,
      wallet.publicKey,
      null,
      DECIMALS,
      partnerKeypair
    );
    console.log("Partner Token Mint created:", partnerTokenMint.toString());

    // Create user token accounts
    userUsdcAccount = await createAssociatedTokenAccount(
      provider.connection,
      wallet.payer,
      usdcMint,
      wallet.publicKey
    );
    console.log("User USDC Account created:", userUsdcAccount.toString());

    userUsdtAccount = await createAssociatedTokenAccount(
      provider.connection,
      wallet.payer,
      usdtMint,
      wallet.publicKey
    );
    console.log("User USDT Account created:", userUsdtAccount.toString());

    userPyusdAccount = await createAssociatedTokenAccount(
      provider.connection,
      wallet.payer,
      pyusdMint,
      wallet.publicKey
    );
    console.log("User PYUSD Account created:", userPyusdAccount.toString());

    userPartnerTokenAccount = await createAssociatedTokenAccount(
      provider.connection,
      wallet.payer,
      partnerTokenMint,
      wallet.publicKey
    );
    console.log(
      "User Partner Token Account created:",
      userPartnerTokenAccount.toString()
    );

    // Mint some tokens to the user
    await mintTo(
      provider.connection,
      wallet.payer,
      usdcMint,
      userUsdcAccount,
      wallet.publicKey,
      INITIAL_MINT_AMOUNT
    );

    await mintTo(
      provider.connection,
      wallet.payer,
      usdtMint,
      userUsdtAccount,
      wallet.publicKey,
      INITIAL_MINT_AMOUNT
    );

    await mintTo(
      provider.connection,
      wallet.payer,
      pyusdMint,
      userPyusdAccount,
      wallet.publicKey,
      INITIAL_MINT_AMOUNT
    );

    await mintTo(
      provider.connection,
      wallet.payer,
      partnerTokenMint,
      userPartnerTokenAccount,
      wallet.publicKey,
      INITIAL_MINT_AMOUNT
    );

    console.log("Tokens minted to user accounts");

    // Find AMM config PDA
    const [ammConfigPda, ammConfigPdaBump] =
      await PublicKey.findProgramAddressSync(
        [Buffer.from("amm-config")],
        program.programId
      );
    ammConfig = ammConfigPda;
    ammConfigBump = ammConfigPdaBump;
    console.log("AMM Config PDA:", ammConfig.toString());

    // Find Seed Pool PDA
    const [seedPoolPda, seedPoolPdaBump] =
      await PublicKey.findProgramAddressSync(
        [Buffer.from("pool"), Buffer.from("seed")],
        program.programId
      );
    seedPool = seedPoolPda;
    seedPoolBump = seedPoolPdaBump;
    console.log("Seed Pool PDA:", seedPool.toString());

    // Find LP mint PDA for Seed Pool
    const [seedPoolLpMintPda] = await PublicKey.findProgramAddressSync(
      [Buffer.from("lp-mint"), seedPool.toBuffer()],
      program.programId
    );
    seedPoolLpMint = seedPoolLpMintPda;
    console.log("Seed Pool LP Mint PDA:", seedPoolLpMint.toString());

    // Find pool token accounts for Seed Pool
    const [poolUsdcAccountPda] = await PublicKey.findProgramAddressSync(
      [Buffer.from("pool-token"), seedPool.toBuffer(), usdcMint.toBuffer()],
      program.programId
    );
    poolUsdcAccount = poolUsdcAccountPda;

    const [poolUsdtAccountPda] = await PublicKey.findProgramAddressSync(
      [Buffer.from("pool-token"), seedPool.toBuffer(), usdtMint.toBuffer()],
      program.programId
    );
    poolUsdtAccount = poolUsdtAccountPda;

    const [poolPyusdAccountPda] = await PublicKey.findProgramAddressSync(
      [Buffer.from("pool-token"), seedPool.toBuffer(), pyusdMint.toBuffer()],
      program.programId
    );
    poolPyusdAccount = poolPyusdAccountPda;

    // Get the user's associated token account for Seed Pool LP tokens
    userSeedPoolLpAccount = await getAssociatedTokenAddress(
      seedPoolLpMint,
      wallet.publicKey
    );

    // Find user position PDA for Seed Pool
    const [userSeedPositionPda] = await PublicKey.findProgramAddressSync(
      [
        Buffer.from("user-position"),
        wallet.publicKey.toBuffer(),
        seedPool.toBuffer(),
      ],
      program.programId
    );
    userSeedPosition = userSeedPositionPda;

    // Find Growth Pool PDA
    const [growthPoolPda, growthPoolPdaBump] =
      await PublicKey.findProgramAddressSync(
        [
          Buffer.from("pool"),
          Buffer.from("growth"),
          partnerTokenMint.toBuffer(),
        ],
        program.programId
      );
    growthPool = growthPoolPda;
    growthPoolBump = growthPoolPdaBump;
    console.log("Growth Pool PDA:", growthPool.toString());

    // Find LP mint PDA for Growth Pool
    const [growthPoolLpMintPda] = await PublicKey.findProgramAddressSync(
      [Buffer.from("lp-mint"), growthPool.toBuffer()],
      program.programId
    );
    growthPoolLpMint = growthPoolLpMintPda;

    // Find pool token accounts for Growth Pool
    const [poolUsdcStarAccountPda] = await PublicKey.findProgramAddressSync(
      [
        Buffer.from("pool-token"),
        growthPool.toBuffer(),
        seedPoolLpMint.toBuffer(),
      ],
      program.programId
    );
    poolUsdcStarAccount = poolUsdcStarAccountPda;

    const [poolPartnerTokenAccountPda] = await PublicKey.findProgramAddressSync(
      [
        Buffer.from("pool-token"),
        growthPool.toBuffer(),
        partnerTokenMint.toBuffer(),
      ],
      program.programId
    );
    poolPartnerTokenAccount = poolPartnerTokenAccountPda;

    // Get the user's associated token account for Growth Pool LP tokens
    userGrowthPoolLpAccount = await getAssociatedTokenAddress(
      growthPoolLpMint,
      wallet.publicKey
    );

    // Find user position PDA for Growth Pool
    const [userGrowthPositionPda] = await PublicKey.findProgramAddressSync(
      [
        Buffer.from("user-position"),
        wallet.publicKey.toBuffer(),
        growthPool.toBuffer(),
      ],
      program.programId
    );
    userGrowthPosition = userGrowthPositionPda;
  });

  it("Initializes the AMM configuration", async () => {
    console.log("Initializing AMM configuration...");

    await program.methods
      .initialize(
        new anchor.BN(DEFAULT_AMPLIFICATION),
        DEFAULT_WEIGHTS.map((w) => new anchor.BN(w))
      )
      .accounts({
        authority: wallet.publicKey,
        ammConfig: ammConfig,
      })
      .remainingAccounts([
        {
          pubkey: SystemProgram.programId,
          isWritable: false,
          isSigner: false,
        },
      ])
      .rpc();

    // Fetch and verify the AMM config
    const ammConfigAccount = await program.account.ammConfig.fetch(ammConfig);
    expect(ammConfigAccount.authority.toString()).to.equal(
      wallet.publicKey.toString()
    );
    expect(ammConfigAccount.defaultAmplification.toNumber()).to.equal(
      DEFAULT_AMPLIFICATION
    );
    expect(
      ammConfigAccount.defaultTargetWeights.map((w) => w.toNumber())
    ).to.deep.equal(DEFAULT_WEIGHTS);

    console.log("AMM configuration initialized successfully");
  });

  it("Creates a Seed Pool", async () => {
    console.log("Creating Seed Pool...");

    const initialAmounts = [1_000_000, 1_000_000, 1_000_000]; // Initial liquidity of 1 token each

    await program.methods
      .createSeedPool(
        new anchor.BN(DEFAULT_AMPLIFICATION),
        DEFAULT_WEIGHTS.map((w) => new anchor.BN(w)),
        initialAmounts.map((a) => new anchor.BN(a))
      )
      .accounts({
        payer: wallet.publicKey,
        ammConfig: ammConfig,
        pool: seedPool,
        tokenMintA: usdcMint,
        tokenMintB: usdtMint,
        tokenMintC: pyusdMint,
        userTokenA: userUsdcAccount,
        userTokenB: userUsdtAccount,
        userTokenC: userPyusdAccount,
        poolTokenA: poolUsdcAccount,
        poolTokenB: poolUsdtAccount,
        poolTokenC: poolPyusdAccount,
        lpMint: seedPoolLpMint,
      })
      .remainingAccounts([
        {
          pubkey: userSeedPoolLpAccount,
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: TOKEN_PROGRAM_ID,
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: ASSOCIATED_TOKEN_PROGRAM_ID,
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: wallet.publicKey, // authority
          isWritable: true,
          isSigner: true,
        },
        {
          pubkey: SystemProgram.programId,
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: SYSVAR_RENT_PUBKEY,
          isWritable: false,
          isSigner: false,
        },
      ])
      .rpc();

    // Fetch and verify the Seed Pool
    const seedPoolAccount = await program.account.pool.fetch(seedPool);
    expect(seedPoolAccount.poolType).to.deep.equal({ seed: {} });
    expect(seedPoolAccount.ammConfig.toString()).to.equal(ammConfig.toString());
    expect(seedPoolAccount.lpMint.toString()).to.equal(
      seedPoolLpMint.toString()
    );
    expect(
      seedPoolAccount.targetWeights.map((w) => w.toNumber())
    ).to.deep.equal(DEFAULT_WEIGHTS);
    expect(seedPoolAccount.amplification.toNumber()).to.equal(
      DEFAULT_AMPLIFICATION
    );
    expect(seedPoolAccount.reserves.map((r) => r.toNumber())).to.deep.equal(
      initialAmounts
    );

    console.log("Seed Pool created successfully");
  });

  it("Adds more liquidity to the Seed Pool", async () => {
    console.log("Adding more liquidity to Seed Pool...");

    const additionalAmounts = [500_000, 500_000, 500_000]; // Add 0.5 tokens more of each
    const minLpAmount = 1_000_000; // Expect at least 1 LP token (considering 3 tokens total)
    const concentration = 1000; // Concentration factor (1000 = 1.0)

    await program.methods
      .deposit(
        additionalAmounts.map((a) => new anchor.BN(a)),
        new anchor.BN(minLpAmount),
        new anchor.BN(concentration)
      )
      .accounts({
        user: wallet.publicKey,
        pool: seedPool,
        lpMint: seedPoolLpMint,
        userLpToken: userSeedPoolLpAccount,
        userTokenA: userUsdcAccount,
        userTokenB: userUsdtAccount,
        userTokenC: userPyusdAccount,
        tokenMintA: usdcMint,
        tokenMintB: usdtMint,
        tokenMintC: pyusdMint,
        poolTokenA: poolUsdcAccount,
        poolTokenB: poolUsdtAccount,
        poolTokenC: poolPyusdAccount,
        userPosition: userSeedPosition,
      })
      .remainingAccounts([
        {
          pubkey: TOKEN_PROGRAM_ID,
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: SystemProgram.programId,
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: SYSVAR_RENT_PUBKEY,
          isWritable: false,
          isSigner: false,
        },
      ])
      .rpc();

    // Verify the increased liquidity in the pool
    const seedPoolAccount = await program.account.pool.fetch(seedPool);
    const expectedReserves = [1_500_000, 1_500_000, 1_500_000]; // Original + additional
    expect(seedPoolAccount.reserves.map((r) => r.toNumber())).to.deep.equal(
      expectedReserves
    );

    // Verify LP tokens were minted to the user
    const userPosition = await program.account.userPosition.fetch(
      userSeedPosition
    );
    expect(userPosition.isActive).to.be.true;
    expect(userPosition.lpAmount.toNumber()).to.be.greaterThan(minLpAmount);

    console.log("Additional liquidity added to Seed Pool successfully");
  });

  it("Creates a Growth Pool", async () => {
    console.log("Creating Growth Pool...");

    // First, we need to get USD* (Seed Pool LP tokens) to use in the Growth Pool
    // We already have some from creating the Seed Pool and adding liquidity

    const initialUsdcStarAmount = 500_000; // Using 0.5 USD* tokens
    const initialPartnerAmount = 500_000; // Using 0.5 partner tokens

    await program.methods
      .createGrowthPool(
        new anchor.BN(DEFAULT_AMPLIFICATION),
        new anchor.BN(initialUsdcStarAmount),
        new anchor.BN(initialPartnerAmount)
      )
      .accounts({
        payer: wallet.publicKey,
        ammConfig: ammConfig,
        seedPool: seedPool,
        pool: growthPool,
        usdcStarMint: seedPoolLpMint,
        partnerTokenMint: partnerTokenMint,
        userUsdcStar: userSeedPoolLpAccount,
        userPartnerToken: userPartnerTokenAccount,
        poolUsdcStar: poolUsdcStarAccount,
        poolPartnerToken: poolPartnerTokenAccount,
        lpMint: growthPoolLpMint,
      })
      .remainingAccounts([
        {
          pubkey: userGrowthPoolLpAccount, // userLpToken
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: TOKEN_PROGRAM_ID, // tokenProgram
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: SystemProgram.programId, // system_program -> systemProgram
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: SYSVAR_RENT_PUBKEY, // rent
          isWritable: false,
          isSigner: false,
        },
      ])
      .rpc();

    // Fetch and verify the Growth Pool
    const growthPoolAccount = await program.account.pool.fetch(growthPool);
    expect(growthPoolAccount.poolType).to.deep.equal({ growth: {} });
    expect(growthPoolAccount.ammConfig.toString()).to.equal(
      ammConfig.toString()
    );
    expect(growthPoolAccount.lpMint.toString()).to.equal(
      growthPoolLpMint.toString()
    );
    expect(growthPoolAccount.seedPool.toString()).to.equal(seedPool.toString());
    expect(growthPoolAccount.reserves.map((r) => r.toNumber())).to.deep.equal([
      initialUsdcStarAmount,
      initialPartnerAmount,
    ]);

    console.log("Growth Pool created successfully");
  });

  it("Performs a swap from USDC to USDT in the Seed Pool", async () => {
    console.log("Swapping USDC to USDT in Seed Pool...");

    const amountIn = 200_000; // 0.2 USDC
    const minAmountOut = 190_000; // Expect at least 0.19 USDT (accounting for fees)

    await program.methods
      .swap(new anchor.BN(amountIn), new anchor.BN(minAmountOut))
      .accounts({
        user: wallet.publicKey,
        pool: seedPool,
        tokenMintIn: usdcMint,
        tokenMintOut: usdtMint,
        userTokenIn: userUsdcAccount,
        userTokenOut: userUsdtAccount,
        poolTokenIn: poolUsdcAccount,
        poolTokenOut: poolUsdtAccount,
      })
      .remainingAccounts([
        {
          pubkey: TOKEN_PROGRAM_ID,
          isWritable: false,
          isSigner: false,
        },
      ])
      .rpc();

    // Verify the swap changed the reserves
    const seedPoolAccount = await program.account.pool.fetch(seedPool);
    expect(seedPoolAccount.reserves[0].toNumber()).to.be.greaterThan(
      1_500_000 + 200_000 - 10
    ); // USDC increased
    expect(seedPoolAccount.reserves[1].toNumber()).to.be.lessThan(1_500_000); // USDT decreased

    console.log("Swap from USDC to USDT completed successfully");
  });

  it("Performs a swap from Partner Token to USDC via Growth Pool", async () => {
    console.log(
      "Swapping Partner Token to USDC via Growth Pool and Seed Pool..."
    );

    // First swap Partner Token for USD* in the Growth Pool
    const partnerAmountIn = 100_000; // 0.1 Partner Token
    const minUsdcStarAmountOut = 90_000; // Expect at least 0.09 USD* (accounting for fees)

    // This is a multi-hop swap that would need to be implemented in a frontend
    // For simplicity, we'll do it in two steps:

    // Step 1: Swap Partner Token for USD* in Growth Pool
    await program.methods
      .swap(new anchor.BN(partnerAmountIn), new anchor.BN(minUsdcStarAmountOut))
      .accounts({
        user: wallet.publicKey,
        pool: growthPool,
        tokenMintIn: partnerTokenMint,
        tokenMintOut: seedPoolLpMint,
        userTokenIn: userPartnerTokenAccount,
        userTokenOut: userSeedPoolLpAccount,
        poolTokenIn: poolPartnerTokenAccount,
        poolTokenOut: poolUsdcStarAccount,
      })
      .remainingAccounts([
        {
          pubkey: TOKEN_PROGRAM_ID,
          isWritable: false,
          isSigner: false,
        },
      ])
      .rpc();

    // Verify the swap in Growth Pool changed the reserves
    const growthPoolAccount = await program.account.pool.fetch(growthPool);
    expect(growthPoolAccount.reserves[1].toNumber()).to.be.greaterThan(500_000); // Partner Token increased
    expect(growthPoolAccount.reserves[0].toNumber()).to.be.lessThan(500_000); // USD* decreased

    console.log("First hop: Partner Token to USD* completed successfully");

    // Step 2: Now we can use the USD* to redeem USDC from the Seed Pool
    // This would be equivalent to "withdrawing single asset" in the Seed Pool
    // However, for simplicity, we'll just withdraw proportionally and check USDC

    // We'll withdraw 50,000 USD* tokens (0.05 USD*)
    const usdcStarToWithdraw = 50_000;
    const minAmountsOut = [15_000, 15_000, 15_000]; // Min expected outputs based on pool weights

    await program.methods
      .withdraw(
        new anchor.BN(usdcStarToWithdraw),
        minAmountsOut.map((a) => new anchor.BN(a))
      )
      .accounts({
        user: wallet.publicKey,
        pool: seedPool,
        lpMint: seedPoolLpMint,
        userLpToken: userSeedPoolLpAccount,
        userTokenA: userUsdcAccount,
        userTokenB: userUsdtAccount,
        userTokenC: userPyusdAccount,
        tokenMintA: usdcMint,
        tokenMintB: usdtMint,
        tokenMintC: pyusdMint,
        poolTokenA: poolUsdcAccount,
        poolTokenB: poolUsdtAccount,
        poolTokenC: poolPyusdAccount,
        userPosition: userSeedPosition,
      })
      .remainingAccounts([
        {
          pubkey: TOKEN_PROGRAM_ID,
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: SystemProgram.programId,
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: SYSVAR_RENT_PUBKEY,
          isWritable: false,
          isSigner: false,
        },
      ])
      .rpc();

    console.log("Second hop: USD* to USDC, USDT, PYUSD completed successfully");
    console.log("Multi-hop swap completed: Partner Token → USD* → Stablecoins");
  });

  it("Withdraws liquidity from the Seed Pool", async () => {
    console.log("Withdrawing liquidity from Seed Pool...");

    // Get user position before withdraw
    const userPositionBefore = await program.account.userPosition.fetch(
      userSeedPosition
    );
    const lpAmountToWithdraw = userPositionBefore.lpAmount.toNumber() / 2; // Withdraw half the position

    // Minimum amounts to receive based on current reserves and LP amount ratio
    const seedPoolAccount = await program.account.pool.fetch(seedPool);
    const reservesBeforeWithdraw = seedPoolAccount.reserves.map((r) =>
      r.toNumber()
    );

    // Get token supply first
    const tokenSupply = await program.provider.connection.getTokenSupply(
      seedPoolLpMint
    );
    const tokenSupplyAmount = Number(tokenSupply.value.amount);

    const minAmountsOut = reservesBeforeWithdraw.map((reserve) =>
      Math.floor((reserve * lpAmountToWithdraw) / tokenSupplyAmount / 2)
    );

    await program.methods
      .withdraw(
        new anchor.BN(lpAmountToWithdraw),
        minAmountsOut.map((a) => new anchor.BN(a))
      )
      .accounts({
        user: wallet.publicKey,
        pool: seedPool,
        lpMint: seedPoolLpMint,
        userLpToken: userSeedPoolLpAccount,
        userTokenA: userUsdcAccount,
        userTokenB: userUsdtAccount,
        userTokenC: userPyusdAccount,
        tokenMintA: usdcMint,
        tokenMintB: usdtMint,
        tokenMintC: pyusdMint,
        poolTokenA: poolUsdcAccount,
        poolTokenB: poolUsdtAccount,
        poolTokenC: poolPyusdAccount,
        userPosition: userSeedPosition,
      })
      .remainingAccounts([
        {
          pubkey: TOKEN_PROGRAM_ID,
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: SystemProgram.programId,
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: SYSVAR_RENT_PUBKEY,
          isWritable: false,
          isSigner: false,
        },
      ])
      .rpc();

    // Verify the position was updated
    const userPositionAfter = await program.account.userPosition.fetch(
      userSeedPosition
    );
    expect(userPositionAfter.lpAmount.toNumber()).to.be.lessThan(
      userPositionBefore.lpAmount.toNumber()
    );

    // Verify the pool reserves decreased
    const seedPoolAccountAfter = await program.account.pool.fetch(seedPool);
    for (let i = 0; i < 3; i++) {
      expect(seedPoolAccountAfter.reserves[i].toNumber()).to.be.lessThan(
        reservesBeforeWithdraw[i]
      );
    }

    console.log("Liquidity withdrawn from Seed Pool successfully");
  });

  it("Withdraws liquidity from the Growth Pool", async () => {
    console.log("Withdrawing liquidity from Growth Pool...");

    // First, we need to create a position by depositing some liquidity
    const depositAmounts = [50_000, 50_000]; // 0.05 USD* and 0.05 Partner tokens
    const minLpAmount = 50_000; // Expect at least 0.05 LP tokens
    const concentration = 1000; // Concentration factor (1000 = 1.0)

    await program.methods
      .deposit(
        depositAmounts.map((a) => new anchor.BN(a)),
        new anchor.BN(minLpAmount),
        new anchor.BN(concentration)
      )
      .accounts({
        user: wallet.publicKey,
        pool: growthPool,
        lpMint: growthPoolLpMint,
        userTokenA: userSeedPoolLpAccount, // USD*
        userTokenB: userPartnerTokenAccount, // Partner Token
        tokenMintA: seedPoolLpMint, // USD*
        tokenMintB: partnerTokenMint, // Partner Token
        poolTokenA: poolUsdcStarAccount,
        poolTokenB: poolPartnerTokenAccount,
        userPosition: userGrowthPosition,
      })
      .remainingAccounts([
        {
          pubkey: userGrowthPoolLpAccount, // userLpToken
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: TOKEN_PROGRAM_ID, // tokenProgram
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: SystemProgram.programId, // system_program -> systemProgram
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: SYSVAR_RENT_PUBKEY, // rent
          isWritable: false,
          isSigner: false,
        },
      ])
      .rpc();

    // Now withdraw the liquidity
    const userPositionBefore = await program.account.userPosition.fetch(
      userGrowthPosition
    );
    const lpAmountToWithdraw = userPositionBefore.lpAmount.toNumber() / 2; // Withdraw half

    // Minimum amounts to receive
    const minAmountsOut = [20_000, 20_000]; // Minimum 0.02 of each token

    await program.methods
      .withdraw(
        new anchor.BN(lpAmountToWithdraw),
        minAmountsOut.map((a) => new anchor.BN(a))
      )
      .accounts({
        user: wallet.publicKey,
        pool: growthPool,
        lpMint: growthPoolLpMint,
        userLpToken: userGrowthPoolLpAccount,
        userTokenA: userSeedPoolLpAccount,
        userTokenB: userPartnerTokenAccount,
        tokenMintA: seedPoolLpMint,
        tokenMintB: partnerTokenMint,
        poolTokenA: poolUsdcStarAccount,
        poolTokenB: poolPartnerTokenAccount,
        userPosition: userGrowthPosition,
      })
      .remainingAccounts([
        {
          pubkey: userGrowthPoolLpAccount, // userLpToken
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: TOKEN_PROGRAM_ID, // tokenProgram
          isWritable: false,
          isSigner: false,
        },
      ])
      .rpc();

    // Verify the position was updated
    const userPositionAfter = await program.account.userPosition.fetch(
      userGrowthPosition
    );
    expect(userPositionAfter.lpAmount.toNumber()).to.be.lessThan(
      userPositionBefore.lpAmount.toNumber()
    );

    console.log("Liquidity withdrawn from Growth Pool successfully");
  });
});
