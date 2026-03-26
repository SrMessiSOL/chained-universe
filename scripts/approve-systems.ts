// scripts/approve-systems.ts
import * as anchor from "@coral-xyz/anchor";
import { Connection, Keypair, PublicKey, clusterApiUrl } from "@solana/web3.js";
import { ApproveSystem } from "@magicblock-labs/bolt-sdk";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";

async function main() {
  // ── ALWAYS pass the world PDA explicitly — never derive or create ──
  const WORLD_PDA = "2kGgN2BfqMdwsmdDE5TZdyqe5rCsUg27RFfH2x2i35WR";
  if (!WORLD_PDA) {
    console.error("Usage: WORLD_PDA=<address> npx ts-node scripts/approve-systems.ts");
    process.exit(1);
  }

  const walletPath = path.join(os.homedir(), ".config", "solana", "devnet.json");
  const kp = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync(walletPath, "utf8")))
  );
  const conn = new Connection(clusterApiUrl("devnet"), "confirmed");
  const provider = new anchor.AnchorProvider(
    conn, new anchor.Wallet(kp), { commitment: "confirmed" }
  );

  const worldPda = new PublicKey(WORLD_PDA);
  console.log("Wallet :", kp.publicKey.toBase58());
  console.log("World  :", worldPda.toBase58());

  // Verify the world account exists before trying to approve
  const worldInfo = await conn.getAccountInfo(worldPda);
  if (!worldInfo) {
    console.error("ERROR: World account not found on-chain. Check your WORLD_PDA.");
    process.exit(1);
  }
  console.log("World account confirmed on-chain ✓\n");

  const systems = [
    { name: "system_initialize", id: "BvTJfpb1KMtBiKQhcNVvHJnKZAvoRALrm4GYQ2Uz36TX" },
    { name: "system_produce",    id: "EkNaTMh1N29W6PCXDGnvh7mVzcrA1pMS3uz2xKWRUZRH" },
    { name: "system_build",      id: "kk7e2mNXHaU3VVtmtzLCZGYP88MDL7EbkFbb9sySfiV"  },
    { name: "system_shipyard",   id: "FTav8UK4RKawqyGWRakZhe1zhYV7PUJgPwHK7UnEqnN9"  },
    { name: "system_launch",     id: "9aHGFS8VAfbEYYCkEGQBBuTKApkD5aiHotH77kMgB5bT"  },
    { name: "system_attack",     id: "8qbBLEdrN6qC1fFJQLM7a6Jqf2xfoDNfSmTQopMELSGm"  },
    { name: "system_session",    id: "9Zt7h1n2sHjLh3mLZy5c8XoVqj6iKZt7eGzqjYpXoV4a"  },
    { name: "system_registry",   id: "N1K6B3oiseLvLrvXELjWPdPAuhPw8MjFo3oepnHd5d3"  },
  ];

  for (const sys of systems) {
    process.stdout.write(`Approving ${sys.name}... `);
    try {
      const { transaction } = await ApproveSystem({
        authority: kp.publicKey,
        systemToApprove: new PublicKey(sys.id),
        world: worldPda,
      });
      const sig = await provider.sendAndConfirm(transaction, [kp]);
      console.log(`✓ ${sig.slice(0, 20)}...`);
    } catch (err: any) {
      const msg = err?.message || "";
      // "already approved" is benign
      if (msg.includes("already") || msg.includes("0x0")) {
        console.log("already approved, skipping");
      } else {
        console.log(`FAILED: ${msg.slice(0, 100)}`);
        if (err?.logs) err.logs.forEach((l: string) => console.log("  ", l));
      }
    }
  }

  console.log("\nDone. Set in your frontend:");
  console.log(`VITE_SHARED_WORLD_PDA=${worldPda.toBase58()}`);
}

main().catch(console.error);