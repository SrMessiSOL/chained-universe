import { Connection, Keypair, clusterApiUrl, PublicKey, Transaction, TransactionInstruction } from "@solana/web3.js";
import { AnchorProvider, Wallet } from "@coral-xyz/anchor";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";

async function main() {
  const walletPath = path.join(os.homedir(), ".config", "solana", "devnet.json");
  const raw = JSON.parse(fs.readFileSync(walletPath, "utf8"));
  const kp = Keypair.fromSecretKey(Uint8Array.from(raw));
  const conn = new Connection(clusterApiUrl("devnet"), "confirmed");
  const provider = new AnchorProvider(conn, new Wallet(kp), { commitment: "confirmed" });

  console.log("Current authority wallet:", kp.publicKey.toBase58());

  // Paste the PDA from the previous creation
  const worldPda = new PublicKey("JEJfSmLKMVLkm3zaR6xEf81ST3Svi5T93ovCGp5tNQgq"); // <-- CHANGE THIS TO YOUR LATEST PDA

  try {
    const tx = new Transaction();

    // Update authority instruction (discriminator 1 for update_authority - try 1 first)
    const updateIx = new TransactionInstruction({
      keys: [
        { pubkey: kp.publicKey, isSigner: true, isWritable: false }, // current authority / signer
        { pubkey: worldPda, isSigner: false, isWritable: true },     // world account
        { pubkey: kp.publicKey, isSigner: false, isWritable: false }, // new authority (your wallet)
      ],
      programId: new PublicKey("WorLD15A7CrDwLcLy4fRqtaTb9fbd8o8iqiEMUDse2n"),
      data: Buffer.from([1]), // try 1, if fails change to 2 or 3
    });
    tx.add(updateIx);

    const sig = await provider.sendAndConfirm(tx, [kp], { commitment: "confirmed" });
    console.log("Authority updated! Tx:", sig);

    // Verify
    const acc = await conn.getAccountInfo(worldPda);
    if (acc) {
      const data = Buffer.from(acc.data);
      const authority = new PublicKey(data.slice(8, 40));
      console.log("New stored authority:", authority.toBase58());
      console.log("Matches your wallet?", authority.equals(kp.publicKey) ? "YES ✓" : "NO");
    }
  } catch (e: unknown) {
    console.error("Update failed:", e);
    if (e instanceof Error) console.log("Message:", e.message);
    if (e && typeof e === "object" && "logs" in e) {
      console.log("Logs:");
      (e as any).logs.forEach((log: string) => console.log("  ", log));
    }
  }
}

main().catch(console.error);