const fs = require("fs");
const {
  Connection,
  Keypair,
  PublicKey,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  TransactionInstruction,
  TransactionMessage,
} = require("@solana/web3.js");
const squads = require("@sqds/multisig");

const RPC_URL = process.env.SOLANA_RPC_URL || "https://api.devnet.solana.com";
const KEYPAIR_PATH = process.env.SOLANA_KEYPAIR;
const UPGRADE_BUFFER = process.env.UPGRADE_BUFFER;

const MULTISIG = new PublicKey("HZiFUDSVcXVJGw8vvHkw9xwLBkTJubZf3HEL1wCxP5EM");
const VAULT_INDEX = 0;
const PRIVATE_STATE_PROGRAM_ID = new PublicKey("HHF3gZKAGLL5GB633tz9U8aGT8HxAaPnSi2YZpgF7d4K");
const BPF_LOADER_UPGRADEABLE_PROGRAM_ID = new PublicKey("BPFLoaderUpgradeab1e11111111111111111111111");

function readKeypair(path) {
  if (!path) {
    throw new Error("Set SOLANA_KEYPAIR to the Squads member keypair path.");
  }
  const bytes = JSON.parse(fs.readFileSync(path, "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(bytes));
}

async function confirm(connection, signature) {
  const latest = await connection.getLatestBlockhash();
  await connection.confirmTransaction({ signature, ...latest }, "confirmed");
}

async function main() {
  if (!UPGRADE_BUFFER) {
    throw new Error("Set UPGRADE_BUFFER to the buffer account produced by `solana program write-buffer`.");
  }

  const connection = new Connection(RPC_URL, "confirmed");
  const member = readKeypair(KEYPAIR_PATH);
  const upgradeBuffer = new PublicKey(UPGRADE_BUFFER);
  const [vaultPda] = squads.getVaultPda({ multisigPda: MULTISIG, index: VAULT_INDEX });
  const [programData] = PublicKey.findProgramAddressSync(
    [PRIVATE_STATE_PROGRAM_ID.toBuffer()],
    BPF_LOADER_UPGRADEABLE_PROGRAM_ID,
  );

  const upgradeData = Buffer.alloc(4);
  upgradeData.writeUInt32LE(3, 0);
  const upgradeIx = new TransactionInstruction({
    programId: BPF_LOADER_UPGRADEABLE_PROGRAM_ID,
    keys: [
      { pubkey: programData, isSigner: false, isWritable: true },
      { pubkey: PRIVATE_STATE_PROGRAM_ID, isSigner: false, isWritable: true },
      { pubkey: upgradeBuffer, isSigner: false, isWritable: true },
      { pubkey: vaultPda, isSigner: false, isWritable: true },
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: vaultPda, isSigner: true, isWritable: false },
    ],
    data: upgradeData,
  });

  const multisig = await squads.accounts.Multisig.fromAccountAddress(connection, MULTISIG);
  const transactionIndex = BigInt(multisig.transactionIndex.toString()) + 1n;
  const { blockhash } = await connection.getLatestBlockhash();
  const transactionMessage = new TransactionMessage({
    payerKey: vaultPda,
    recentBlockhash: blockhash,
    instructions: [upgradeIx],
  });

  const createTx = await squads.rpc.vaultTransactionCreate({
    connection,
    feePayer: member,
    multisigPda: MULTISIG,
    transactionIndex,
    creator: member.publicKey,
    rentPayer: member.publicKey,
    vaultIndex: VAULT_INDEX,
    ephemeralSigners: 0,
    transactionMessage,
    memo: "Upgrade GAMESOL private-state public-planet binding",
    sendOptions: { skipPreflight: false, maxRetries: 5 },
  });
  await confirm(connection, createTx);

  const proposalTx = await squads.rpc.proposalCreate({
    connection,
    feePayer: member,
    creator: member,
    rentPayer: member,
    multisigPda: MULTISIG,
    transactionIndex,
    isDraft: false,
    sendOptions: { skipPreflight: false, maxRetries: 5 },
  });
  await confirm(connection, proposalTx);

  const approveTx = await squads.rpc.proposalApprove({
    connection,
    feePayer: member,
    member,
    multisigPda: MULTISIG,
    transactionIndex,
    memo: "Approve private-state public-planet binding upgrade",
    sendOptions: { skipPreflight: false, maxRetries: 5 },
  });
  await confirm(connection, approveTx);

  const executeTx = await squads.rpc.vaultTransactionExecute({
    connection,
    feePayer: member,
    multisigPda: MULTISIG,
    transactionIndex,
    member: member.publicKey,
    sendOptions: { skipPreflight: false, maxRetries: 5 },
  });
  await confirm(connection, executeTx);

  console.log(JSON.stringify({
    rpcUrl: RPC_URL,
    member: member.publicKey.toBase58(),
    multisig: MULTISIG.toBase58(),
    vault: vaultPda.toBase58(),
    program: PRIVATE_STATE_PROGRAM_ID.toBase58(),
    programData: programData.toBase58(),
    upgradeBuffer: upgradeBuffer.toBase58(),
    transactionIndex: transactionIndex.toString(),
    signatures: {
      createTx,
      proposalTx,
      approveTx,
      executeTx,
    },
  }, null, 2));
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});
