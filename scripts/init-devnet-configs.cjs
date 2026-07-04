const fs = require("fs");
const {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} = require("@solana/web3.js");
const {
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
} = require("@solana/spl-token");

const RPC_URL = process.env.GAMESOL_RPC || "https://api.devnet.solana.com";
const KEYPAIR_PATH = process.env.GAMESOL_KEYPAIR || "tmp-devnet-keypair.json";

const GAME_STATE_PROGRAM_ID = new PublicKey("FJGxh6SKgNoTVzHj98oBsC2oaEy8ovadVJf8rDUNaEHb");
const MARKET_PROGRAM_ID = new PublicKey("Dow7f1UqLGKyvs1D2uNR5c6bmAdnKRy2ZDtnsa4UhApp");
const ANTIMATTER_MINT = new PublicKey("FAeZLeqohcxNBpwGrbYBLj2TavFqt4353mT6qY6Z7YFh");
const DEVNET_USDC_MINT = new PublicKey("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU");

const IX = {
  initializeGameConfig: Buffer.from([45, 61, 80, 55, 152, 63, 158, 47]),
  initializeStoreConfig: Buffer.from([231, 234, 72, 35, 116, 218, 119, 251]),
  initializeStoreConfigCamel: Buffer.from([142, 22, 154, 179, 202, 173, 198, 218]),
  initializeMarket: Buffer.from([35, 35, 189, 193, 155, 48, 170, 203]),
  initializeEscrow: Buffer.from([243, 160, 77, 153, 11, 92, 48, 209]),
};

function keypairFromFile(path) {
  const secret = JSON.parse(fs.readFileSync(path, "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(secret));
}

function derivePda(seed, programId) {
  return PublicKey.findProgramAddressSync([Buffer.from(seed)], programId)[0];
}

async function exists(connection, pubkey) {
  return (await connection.getAccountInfo(pubkey, "confirmed")) !== null;
}

async function sendIxs(connection, payer, label, instructions) {
  if (instructions.length === 0) {
    console.log(`${label}: no-op`);
    return null;
  }
  const tx = new Transaction().add(...instructions);
  const sig = await sendAndConfirmTransaction(connection, tx, [payer], {
    commitment: "confirmed",
    preflightCommitment: "confirmed",
  });
  console.log(`${label}: ${sig}`);
  return sig;
}

function printSendError(label, err) {
  console.warn(`${label}: ${err.message || err}`);
  if (err.logs) {
    for (const log of err.logs) {
      console.warn(`  ${log}`);
    }
  }
}

async function main() {
  const connection = new Connection(RPC_URL, "confirmed");
  const payer = keypairFromFile(KEYPAIR_PATH);
  const admin = payer.publicKey;

  const gameConfig = derivePda("game_config", GAME_STATE_PROGRAM_ID);
  const storeConfig = derivePda("store_config", GAME_STATE_PROGRAM_ID);
  const marketConfig = derivePda("market_config", MARKET_PROGRAM_ID);
  const marketEscrow = derivePda("market_escrow", MARKET_PROGRAM_ID);
  const marketAuthority = derivePda("market_authority", MARKET_PROGRAM_ID);
  const treasuryUsdc = getAssociatedTokenAddressSync(DEVNET_USDC_MINT, admin);
  const treasuryAntimatter = getAssociatedTokenAddressSync(ANTIMATTER_MINT, admin);

  console.log("admin:", admin.toBase58());
  console.log("game_config:", gameConfig.toBase58());
  console.log("store_config:", storeConfig.toBase58());
  console.log("market_config:", marketConfig.toBase58());
  console.log("market_escrow:", marketEscrow.toBase58());
  console.log("treasury_usdc:", treasuryUsdc.toBase58());
  console.log("treasury_antimatter:", treasuryAntimatter.toBase58());

  const ataIxs = [];
  if (!(await exists(connection, treasuryUsdc))) {
    ataIxs.push(createAssociatedTokenAccountInstruction(admin, treasuryUsdc, admin, DEVNET_USDC_MINT));
  }
  if (!(await exists(connection, treasuryAntimatter))) {
    ataIxs.push(createAssociatedTokenAccountInstruction(admin, treasuryAntimatter, admin, ANTIMATTER_MINT));
  }
  await sendIxs(connection, payer, "create treasury token accounts", ataIxs);

  const gameIxs = [];
  if (!(await exists(connection, gameConfig))) {
    gameIxs.push(new TransactionInstruction({
      programId: GAME_STATE_PROGRAM_ID,
      keys: [
        { pubkey: admin, isSigner: true, isWritable: true },
        { pubkey: gameConfig, isSigner: false, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: Buffer.concat([IX.initializeGameConfig, ANTIMATTER_MINT.toBuffer()]),
    }));
  }
  await sendIxs(connection, payer, "initialize game config", gameIxs);

  if (!(await exists(connection, storeConfig))) {
    const storeKeys = [
      { pubkey: admin, isSigner: true, isWritable: true },
      { pubkey: DEVNET_USDC_MINT, isSigner: false, isWritable: false },
      { pubkey: treasuryUsdc, isSigner: false, isWritable: false },
      { pubkey: storeConfig, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ];
    const storeIx = (disc) => new TransactionInstruction({
      programId: GAME_STATE_PROGRAM_ID,
      keys: storeKeys,
      data: Buffer.concat([disc, Buffer.from([1])]),
    });
    try {
      await sendIxs(connection, payer, "initialize store config", [storeIx(IX.initializeStoreConfig)]);
    } catch (err) {
      printSendError("initialize store config with snake_case discriminator failed", err);
      console.warn("trying camelCase discriminator");
      try {
        await sendIxs(connection, payer, "initialize store config fallback", [storeIx(IX.initializeStoreConfigCamel)]);
      } catch (fallbackErr) {
        printSendError("initialize store config fallback failed", fallbackErr);
      }
    }
  } else {
    await sendIxs(connection, payer, "initialize store config", []);
  }

  const marketIxs = [];
  if (!(await exists(connection, marketConfig))) {
    marketIxs.push(new TransactionInstruction({
      programId: MARKET_PROGRAM_ID,
      keys: [
        { pubkey: admin, isSigner: true, isWritable: true },
        { pubkey: marketConfig, isSigner: false, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: Buffer.concat([IX.initializeMarket, ANTIMATTER_MINT.toBuffer()]),
    }));
  }
  await sendIxs(connection, payer, "initialize market config", marketIxs);

  const escrowIxs = [];
  if (!(await exists(connection, marketEscrow))) {
    escrowIxs.push(new TransactionInstruction({
      programId: MARKET_PROGRAM_ID,
      keys: [
        { pubkey: admin, isSigner: true, isWritable: true },
        { pubkey: marketConfig, isSigner: false, isWritable: false },
        { pubkey: ANTIMATTER_MINT, isSigner: false, isWritable: false },
        { pubkey: marketEscrow, isSigner: false, isWritable: true },
        { pubkey: marketAuthority, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: IX.initializeEscrow,
    }));
  }
  await sendIxs(connection, payer, "initialize market escrow", escrowIxs);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
