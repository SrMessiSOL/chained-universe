import { PublicKey } from "@solana/web3.js";

export function getPlayerEntityPda(player: PublicKey, world: PublicKey): PublicKey {
  const [entityPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("entity"),
      player.toBuffer(),
      world.toBuffer(),
    ],
    new PublicKey("WorLD15A7CrDwLcLy4fRqtaTb9fbd8o8iqiEMUDse2n") // ← your WORLD_PROGRAM_ID
  );
  return entityPda;
}