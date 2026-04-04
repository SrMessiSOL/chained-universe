// scripts/approve-systems.ts
import * as anchor from "@coral-xyz/anchor";
import { Connection, Keypair, PublicKey, clusterApiUrl } from "@solana/web3.js";
import { ApproveSystem } from "@magicblock-labs/bolt-sdk";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";

async function main() {
  // ── ALWAYS pass the world PDA explicitly — never derive or create ──
  const WORLD_PDA = "5KY9KS6iKAwSDq3LbErP7LEPLdTPhqBLJ5VLG1555X8N";
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
    { name: "initialize", id: "GHBGdcof2e5tsPe2vP3zJYNxJscojY7J7gdRXCsgdpY9" },
    { name: "produce",    id: "DNNJg4A1yirXgUN5cdJ4ozuG8zJVkmxB2AsWvTqVsbk4" },
    { name: "build",      id: "E94HChSfw57Px2BJPKLnoaj17v6NKN7vXnoQGLSpxUve" },
    { name: "shipyard",   id: "74wxuTRib19TzJyXNaeyPVcsFFFqBq8phtRSSPDsK2q2" },
    { name: "launch",     id: "BVn9NZ51LqhbDowqhaJvxmXK6VGsP1k3dLtJEL8Fjmxv" },
    { name: "research",   id: "CXwXVUeovhbpXGWpHk56SgrnH2DwoqoTSErgtrJghK5Z" },
    { name: "system_initialize_new_colony",   id: "DapYcTdYUwB7qWhmqMGZU6V1vqS3NEagzt15fnWwfQMC"  },
    { name: "system_resolve_colonize",   id: "AuYuVgjpX64Fea3zGtUaEHjoewwyWBeT8Srsh8EXFhGL"  },
    { name: "system_resolve_transport",   id: "DkzcueEX3ca9haAmFoHKsW7JQVFxBfeZJX1VdHSdPnYP"  },
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