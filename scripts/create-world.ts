import * as anchor from "@coral-xyz/anchor";
import { Connection, Keypair, clusterApiUrl, PublicKey } from "@solana/web3.js";
import { 
  InitializeNewWorld, 
  AddAuthority, 
  AddEntity,
  InitializeComponent,
  ApproveSystem,
  ApplySystem 
} from "@magicblock-labs/bolt-sdk";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";

async function main() {
  const walletPath = path.join(os.homedir(), ".config", "solana", "devnet.json");
  if (!fs.existsSync(walletPath)) {
    console.error("Wallet not found. Create one: solana-keygen new --outfile ~/.config/solana/devnet.json");
    process.exit(1);
  }

  const raw = JSON.parse(fs.readFileSync(walletPath, "utf8"));
  const kp = Keypair.fromSecretKey(Uint8Array.from(raw));
  const conn = new Connection(clusterApiUrl("devnet"), "confirmed");
  const wallet = new anchor.Wallet(kp);
  const provider = new anchor.AnchorProvider(conn, wallet, { commitment: "confirmed" });
  anchor.setProvider(provider);

  console.log("Signer / authority:", kp.publicKey.toBase58());

  const balance = await conn.getBalance(kp.publicKey);
  console.log(`Balance: ${(balance / 1e9).toFixed(4)} SOL`);

  console.log("\nStep 1: Creating new world...");
  const { transaction: initTx, worldPda } = await InitializeNewWorld({
    payer: kp.publicKey,
    connection: conn,
  });

  console.log("World PDA:", worldPda.toBase58());

  const initSig = await provider.sendAndConfirm(initTx, [kp]);
  console.log("World created → Sig:", initSig);

  const worldAcc = await conn.getAccountInfo(worldPda);
  if (!worldAcc) {
    console.error("World account not found");
    process.exit(1);
  }
  console.log(`Data length: ${worldAcc.data.length} bytes`);

  console.log("\nStep 2: Adding self as authority...");
  const { transaction: addAuthTx } = await AddAuthority({
    authority: kp.publicKey,
    newAuthority: kp.publicKey,
    world: worldPda,
    connection: conn,
  });

  const addAuthSig = await provider.sendAndConfirm(addAuthTx, [kp]);
  console.log("Authority added → Sig:", addAuthSig);

  console.log("\nStep 3: Registering systems with ApproveSystem (no execution)...");
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
    console.log(`→ Registering ${sys.name} (${sys.id})`);
    try {
      const { transaction: approveTx } = await ApproveSystem({
        authority: kp.publicKey,
        systemToApprove: new PublicKey(sys.id),
        world: worldPda,
      });

      const sig = await provider.sendAndConfirm(approveTx, [kp]);
      console.log(`  Registered! Sig: ${sig}`);
    } catch (err: any) {
      console.error(`  Failed: ${err.message}`);
      if (err.logs) {
        console.log("Logs:");
        err.logs.forEach((l: string) => console.log(`    ${l}`));
      }
    }
  }

  console.log("\nDone! World ready for frontend.");
  console.log(`VITE_SHARED_WORLD_PDA=${worldPda.toBase58()}`);
  console.log("\nNext: In frontend, use this PDA with MagicBlock SDK to connect and play (session keys / rollup for no-signing).");
}

main().catch(err => {
  console.error("Fatal error:", err);
  process.exit(1);
});