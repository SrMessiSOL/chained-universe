const fs = require("fs");
const crypto = require("crypto");
const {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} = require("@solana/web3.js");

const RPC_URL = process.env.SOLANA_RPC_URL || "https://api.devnet.solana.com";
const KEYPAIR_PATH = process.env.SOLANA_KEYPAIR;
const PRIVATE_STATE_PROGRAM_ID = new PublicKey("HHF3gZKAGLL5GB633tz9U8aGT8HxAaPnSi2YZpgF7d4K");

const SPY_REPORT_REQUEST_DISCRIMINATOR = Buffer.from([174, 246, 219, 122, 153, 169, 29, 117]);
const PUBLISH_SPY_REPORT_DISCRIMINATOR = Buffer.from([140, 217, 42, 129, 115, 74, 159, 167]);

function readKeypair(path) {
  if (!path) {
    throw new Error("Set SOLANA_KEYPAIR to the dev resolver keypair path.");
  }
  const bytes = JSON.parse(fs.readFileSync(path, "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(bytes));
}

function readPubkey(buffer, offset) {
  return new PublicKey(buffer.subarray(offset, offset + 32));
}

function readU64(buffer, offset) {
  return buffer.readBigUInt64LE(offset);
}

function readI64(buffer, offset) {
  return Number(buffer.readBigInt64LE(offset));
}

function parseSpyReportRequest(pubkey, data) {
  if (!data.subarray(0, 8).equals(SPY_REPORT_REQUEST_DISCRIMINATOR)) {
    throw new Error("Invalid spy report request discriminator.");
  }
  let offset = 8;
  const targetPlanet = readPubkey(data, offset); offset += 32;
  const targetAuthority = readPubkey(data, offset); offset += 32;
  const spyAuthority = readPubkey(data, offset); offset += 32;
  const resolver = readPubkey(data, offset); offset += 32;
  const targetEpoch = readU64(data, offset); offset += 8;
  const reportNonce = readU64(data, offset); offset += 8;
  const revealLevelCap = data.readUInt8(offset); offset += 1;
  const encryptedInputHash = data.subarray(offset, offset + 32); offset += 32;
  const requestCommitment = data.subarray(offset, offset + 32); offset += 32;
  const createdAt = readI64(data, offset); offset += 8;
  const resolved = data.readUInt8(offset) !== 0; offset += 1;
  const bump = data.readUInt8(offset);
  return {
    pubkey,
    targetPlanet,
    targetAuthority,
    spyAuthority,
    resolver,
    targetEpoch,
    reportNonce,
    revealLevelCap,
    encryptedInputHash,
    requestCommitment,
    createdAt,
    resolved,
    bump,
  };
}

function u64Le(value) {
  const out = Buffer.alloc(8);
  out.writeBigUInt64LE(value);
  return out;
}

function deriveSpyReportPda(privatePlanet, spyAuthority, reportNonce) {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("spy-report"),
      privatePlanet.toBuffer(),
      spyAuthority.toBuffer(),
      u64Le(reportNonce),
    ],
    PRIVATE_STATE_PROGRAM_ID,
  )[0];
}

function hash(...parts) {
  const hasher = crypto.createHash("sha256");
  for (const part of parts) hasher.update(part);
  return hasher.digest();
}

function buildPublishIx(request, resolver) {
  const spyReport = deriveSpyReportPda(request.targetPlanet, request.spyAuthority, request.reportNonce);
  const reportCiphertextHash = hash(
    Buffer.from("GAMESOL_DEV_SPY_REPORT_CIPHERTEXT"),
    request.pubkey.toBuffer(),
    request.encryptedInputHash,
    request.requestCommitment,
  );
  const reportCommitment = hash(
    Buffer.from("GAMESOL_DEV_SPY_REPORT_COMMITMENT"),
    reportCiphertextHash,
    request.targetPlanet.toBuffer(),
    request.spyAuthority.toBuffer(),
    u64Le(request.targetEpoch),
    u64Le(request.reportNonce),
    Buffer.from([request.revealLevelCap]),
  );
  const data = Buffer.concat([
    PUBLISH_SPY_REPORT_DISCRIMINATOR,
    reportCiphertextHash,
    reportCommitment,
  ]);
  return new TransactionInstruction({
    programId: PRIVATE_STATE_PROGRAM_ID,
    keys: [
      { pubkey: resolver.publicKey, isSigner: true, isWritable: false },
      { pubkey: request.spyAuthority, isSigner: true, isWritable: true },
      { pubkey: request.targetPlanet, isSigner: false, isWritable: true },
      { pubkey: request.pubkey, isSigner: false, isWritable: true },
      { pubkey: spyReport, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

async function main() {
  const connection = new Connection(RPC_URL, "confirmed");
  const resolver = readKeypair(KEYPAIR_PATH);
  const requestFilter = process.env.SPY_REQUEST;
  const accounts = requestFilter
    ? [{ pubkey: new PublicKey(requestFilter), account: await connection.getAccountInfo(new PublicKey(requestFilter), "confirmed") }]
    : await connection.getProgramAccounts(PRIVATE_STATE_PROGRAM_ID, {
        commitment: "confirmed",
        filters: [{ memcmp: { offset: 0, bytes: bs58Encode(SPY_REPORT_REQUEST_DISCRIMINATOR) } }],
      });

  const requests = accounts
    .filter(({ account }) => !!account)
    .map(({ pubkey, account }) => parseSpyReportRequest(pubkey, Buffer.from(account.data)))
    .filter((request) =>
      request.resolver.equals(resolver.publicKey) &&
      request.spyAuthority.equals(resolver.publicKey) &&
      !request.resolved
    );

  if (requests.length === 0) {
    console.log("No dev-resolvable private spy requests found for", resolver.publicKey.toBase58());
    return;
  }

  const request = requests[0];
  const tx = new Transaction().add(buildPublishIx(request, resolver));
  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash("confirmed");
  tx.recentBlockhash = blockhash;
  tx.feePayer = resolver.publicKey;
  tx.sign(resolver);
  const signature = await connection.sendRawTransaction(tx.serialize(), {
    skipPreflight: false,
    preflightCommitment: "confirmed",
  });
  await connection.confirmTransaction({ signature, blockhash, lastValidBlockHeight }, "confirmed");
  console.log(JSON.stringify({
    signature,
    resolver: resolver.publicKey.toBase58(),
    request: request.pubkey.toBase58(),
    targetPlanet: request.targetPlanet.toBase58(),
    spyAuthority: request.spyAuthority.toBase58(),
    reportNonce: request.reportNonce.toString(),
  }, null, 2));
}

function bs58Encode(bytes) {
  const alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
  let digits = [0];
  for (const byte of bytes) {
    let carry = byte;
    for (let i = 0; i < digits.length; i++) {
      carry += digits[i] << 8;
      digits[i] = carry % 58;
      carry = Math.floor(carry / 58);
    }
    while (carry > 0) {
      digits.push(carry % 58);
      carry = Math.floor(carry / 58);
    }
  }
  let zeros = 0;
  for (const byte of bytes) {
    if (byte === 0) zeros++;
    else break;
  }
  return "1".repeat(zeros) + digits.reverse().map((digit) => alphabet[digit]).join("");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
