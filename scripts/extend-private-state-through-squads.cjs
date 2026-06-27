const fs = require("fs");
const {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  TransactionInstruction,
  TransactionMessage,
} = require("@solana/web3.js");
const squads = require("@sqds/multisig");

const RPC_URL = process.env.SOLANA_RPC_URL || "https://api.devnet.solana.com";
const KEYPAIR_PATH = process.env.SOLANA_KEYPAIR;
const ADDITIONAL_BYTES = Number(process.env.ADDITIONAL_BYTES || "65536");

const MULTISIG = new PublicKey("HZiFUDSVcXVJGw8vvHkw9xwLBkTJubZf3HEL1wCxP5EM");
const VAULT_INDEX = 0;
const PRIVATE_STATE_PROGRAM_ID = new PublicKey("HHF3gZKAGLL5GB633tz9U8aGT8HxAaPnSi2YZpgF7d4K");
const BPF_LOADER_UPGRADEABLE_PROGRAM_ID = new PublicKey("BPFLoaderUpgradeab1e11111111111111111111111");

function readKeypair(path) {
  if (!path) throw new Error("Set SOLANA_KEYPAIR to the Squads member keypair path.");
  return Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(path, "utf8"))));
}

async function confirm(connection, signature) {
  const latest = await connection.getLatestBlockhash();
  await connection.confirmTransaction({ signature, ...latest }, "confirmed");
}

async function main() {
  const connection = new Connection(RPC_URL, "confirmed");
  const member = readKeypair(KEYPAIR_PATH);
  const [vaultPda] = squads.getVaultPda({ multisigPda: MULTISIG, index: VAULT_INDEX });
  const [programData] = PublicKey.findProgramAddressSync(
    [PRIVATE_STATE_PROGRAM_ID.toBuffer()],
    BPF_LOADER_UPGRADEABLE_PROGRAM_ID,
  );

  const data = Buffer.alloc(8);
  data.writeUInt32LE(6, 0); // UpgradeableLoaderInstruction::ExtendProgram
  data.writeUInt32LE(ADDITIONAL_BYTES, 4);

  const extendIx = new TransactionInstruction({
    programId: BPF_LOADER_UPGRADEABLE_PROGRAM_ID,
    keys: [
      { pubkey: programData, isSigner: false, isWritable: true },
      { pubkey: PRIVATE_STATE_PROGRAM_ID, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: vaultPda, isSigner: true, isWritable: true },
    ],
    data,
  });

  const multisig = await squads.accounts.Multisig.fromAccountAddress(connection, MULTISIG);
  const transactionIndex = BigInt(multisig.transactionIndex.toString()) + 1n;
  const { blockhash } = await connection.getLatestBlockhash();
  const transactionMessage = new TransactionMessage({
    payerKey: vaultPda,
    recentBlockhash: blockhash,
    instructions: [extendIx],
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
    memo: `Extend GAMESOL private-state by ${ADDITIONAL_BYTES} bytes`,
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
    memo: "Approve private-state data extension",
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
    additionalBytes: ADDITIONAL_BYTES,
    transactionIndex: transactionIndex.toString(),
    signatures: { createTx, proposalTx, approveTx, executeTx },
  }, null, 2));
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
