// Dependencies: npm install @solana/web3.js @magicblock-labs/bolt-sdk @coral-xyz/anchor
import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
  ComputeBudgetProgram,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
  AccountMeta,
} from "@solana/web3.js";
import { AnchorProvider, setProvider } from "@coral-xyz/anchor";
import { anchor as BoltAnchor, AddEntity, InitializeComponent, ApplySystem, createDelegateInstruction, createUndelegateInstruction } from "@magicblock-labs/bolt-sdk";
export const ER_DIRECT_RPC = "https://devnet.magicblock.app";

// ─── Program IDs ──────────────────────────────────────────────────────────────
export const WORLD_PROGRAM_ID      = new PublicKey("WorLD15A7CrDwLcLy4fRqtaTb9fbd8o8iqiEMUDse2n");
export const DELEGATION_PROGRAM_ID = new PublicKey("DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh");

export const PROGRAM_IDS = {
  componentPlanet:    new PublicKey("4AAQeP54KQy4HSjMsMS9VwVY8mWy4BisdsTwSxen4Df6"),
  componentFleet:     new PublicKey("5UuCSuNqVXwCd7qPFQXj8Kp7DAqbB5ZuHFLZZ32paPLD"),
  componentResources: new PublicKey("CP6KoShdHvgZbGubYLct1EcQLmngZ1nsWmaKQhbJRtss"),
  componentResearch:  new PublicKey("9YgNLY8u8quhB6nQAj6j4fZh2fWJkM1f5M2wLg2V6P6Y"),
  systemInitialize:   new PublicKey("BvTJfpb1KMtBiKQhcNVvHJnKZAvoRALrm4GYQ2Uz36TX"),
  systemProduce:      new PublicKey("EkNaTMh1N29W6PCXDGnvh7mVzcrA1pMS3uz2xKWRUZRH"),
  systemBuild:        new PublicKey("kk7e2mNXHaU3VVtmtzLCZGYP88MDL7EbkFbb9sySfiV"),
  systemResearch:     new PublicKey("4zQaUmY8q4wM9G6vAkQTySbb7NVa8RzQQEtvavB8SshS"),
  systemLaunch:       new PublicKey("9aHGFS8VAfbEYYCkEGQBBuTKApkD5aiHotH77kMgB5bT"),
  systemShipyard:     new PublicKey("FTav8UK4RKawqyGWRakZhe1zhYV7PUJgPwHK7UnEqnN9"),
  systemSession:      new PublicKey("EASuSJPK7oY4wjgD5b4XUkkFw7Wp3gCwSzY3u7qwuaHj"),
} as const;

export const SHARED_WORLD_PDA = new PublicKey("2kGgN2BfqMdwsmdDE5TZdyqe5rCsUg27RFfH2x2i35WR");
export const RPC_ENDPOINT     = "https://api.devnet.solana.com";
export const ER_RPC           = "https://devnet-router.magicblock.app";

export const REGISTRY_PROGRAM_ID = new PublicKey("N1K6B3oiseLvLrvXELjWPdPAuhPw8MjFo3oepnHd5d3");

// ─── Data types ───────────────────────────────────────────────────────────────
export interface Mission {
  missionType:     number;
  destination:     string;
  departTs:        number;
  arriveTs:        number;
  returnTs:        number;
  sSmallCargo:     number;
  sLargeCargo:     number;
  sLightFighter:   number;
  sHeavyFighter:   number;
  sCruiser:        number;
  sBattleship:     number;
  sBattlecruiser:  number;
  sBomber:         number;
  sDestroyer:      number;
  sDeathstar:      number;
  sRecycler:       number;
  sEspionageProbe: number;
  sColonyShip:     number;
  cargoMetal:      bigint;
  cargoCrystal:    bigint;
  cargoDeuterium:  bigint;
  applied:         boolean;
}

export interface Planet {
  creator:              string;
  entity:               string;
  owner:                string;
  name:                 string;
  galaxy:               number;
  system:               number;
  position:             number;
  planetIndex:          number;
  diameter:             number;
  temperature:          number;
  maxFields:            number;
  usedFields:           number;
  metalMine:            number;
  crystalMine:          number;
  deuteriumSynthesizer: number;
  solarPlant:           number;
  fusionReactor:        number;
  roboticsFactory:      number;
  naniteFactory:        number;
  shipyard:             number;
  metalStorage:         number;
  crystalStorage:       number;
  deuteriumTank:        number;
  researchLab:          number;
  missileSilo:          number;
  buildQueueItem:       number;
  buildQueueTarget:     number;
  buildFinishTs:        number;
}

export interface Research {
  creator:          string;
  energyTech:       number;
  combustionDrive:  number;
  impulseDrive:     number;
  hyperspaceDrive:  number;
  computerTech:     number;
  astrophysics:     number;
  igrNetwork:       number;
  queueItem:        number;
  queueTarget:      number;
  researchFinishTs: number;
}

export interface Resources {
  metal:             bigint;
  crystal:           bigint;
  deuterium:         bigint;
  metalHour:         bigint;
  crystalHour:       bigint;
  deuteriumHour:     bigint;
  energyProduction:  bigint;
  energyConsumption: bigint;
  metalCap:          bigint;
  crystalCap:        bigint;
  deuteriumCap:      bigint;
  lastUpdateTs:      number;
}

export interface Fleet {
  creator:         string;
  smallCargo:      number;
  largeCargo:      number;
  lightFighter:    number;
  heavyFighter:    number;
  cruiser:         number;
  battleship:      number;
  battlecruiser:   number;
  bomber:          number;
  destroyer:       number;
  deathstar:       number;
  recycler:        number;
  espionageProbe:  number;
  colonyShip:      number;
  solarSatellite:  number;
  activeMissions:  number;
  missions:        Mission[];
}

export interface PlayerState {
  planet:       Planet;
  resources:    Resources;
  fleet:        Fleet;
  research:     Research;
  entityPda:    string;
  planetPda:    string;
  fleetPda:     string;
  resourcesPda: string;
  researchPda:  string;
  isDelegated:  boolean;
}

// ─── BOLT account layout ──────────────────────────────────────────────────────
const DISC        = 8;
const COMP_OFFSET = DISC;

// ─── PDA derivation ───────────────────────────────────────────────────────────
export function deriveComponentPda(entityPda: PublicKey, componentProgramId: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [entityPda.toBuffer()],
    componentProgramId
  )[0];
}

export function deriveRegistryPda(walletPubkey: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("registry"), walletPubkey.toBuffer(), Buffer.from([0])],
    REGISTRY_PROGRAM_ID
  )[0];
}

export function deriveRegistryPdaByIndex(walletPubkey: PublicKey, index: number): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("registry"), walletPubkey.toBuffer(), Buffer.from([index])],
    REGISTRY_PROGRAM_ID
  )[0];
}

export function deriveWalletMetaPda(walletPubkey: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("wallet_meta"), walletPubkey.toBuffer()],
    REGISTRY_PROGRAM_ID
  )[0];
}

// ─── Patch ApplySystem args ───────────────────────────────────────────────────
function patchApplyArgs(ix: TransactionInstruction, rawArgs: Buffer): TransactionInstruction {
  const disc   = ix.data.slice(0, 8);
  const lenBuf = Buffer.alloc(4);
  lenBuf.writeUInt32LE(rawArgs.length, 0);
  return new TransactionInstruction({
    keys:      ix.keys,
    programId: ix.programId,
    data:      Buffer.concat([disc, lenBuf, rawArgs]),
  });
}

const IX_DISCRIMINATORS: Record<string, number[]> = {
  init_wallet_meta: [85, 51, 246, 106, 67, 236, 28, 180],
  register_planet: [213, 91, 78, 118, 207, 133, 98, 238],
};

function ixDiscriminator(name: string): Buffer {
  const bytes = IX_DISCRIMINATORS[name];
  if (!bytes) throw new Error(`Missing instruction discriminator for ${name}`);
  return Buffer.from(bytes);
}

// ─── Game client ──────────────────────────────────────────────────────────────
export class GameClient {
  private connection:   Connection;
  private erConnection: Connection;
  private provider:     AnchorProvider;
  private sessionActive = false;
  private erSigner:          Keypair | null = null;
  private erDirectConnection: Connection;

  constructor(connection: Connection, provider: AnchorProvider) {
    this.connection         = connection;
    this.erConnection       = new Connection(ER_RPC,        "confirmed");
    this.erDirectConnection = new Connection(ER_DIRECT_RPC, "confirmed");
    this.provider           = provider;

    setProvider(provider);
    const boltProvider = new (BoltAnchor as any).AnchorProvider(
      connection, provider.wallet, { commitment: "confirmed" }
    );
    (BoltAnchor as any).setProvider(boltProvider);
  }

  async findPlanet(walletPubkey: PublicKey): Promise<PlayerState | null> {
    console.log("[LOOKUP] ── findPlanet ──────────────────────────────");
    console.log("[LOOKUP] wallet:", walletPubkey.toBase58());
    console.log("[LOOKUP] sessionActive:", this.sessionActive);
    console.log("[LOOKUP] erSigner:", this.erSigner?.publicKey.toBase58() ?? "none");

    console.log("[LOOKUP] Strategy A: getProgramAccounts on devnet (offset:", DISC, ")");
    const planetAccounts = await this.connection.getProgramAccounts(
      PROGRAM_IDS.componentPlanet,
      {
        commitment: "confirmed",
        filters: [{ memcmp: { offset: DISC, bytes: walletPubkey.toBase58() } }],
      }
    );
    console.log("[LOOKUP] Strategy A result:", planetAccounts.length, "account(s)");

    let planetPda:  PublicKey | null = null;
    let planetData: Buffer   | null = null;

    if (planetAccounts.length > 0) {
      planetPda  = planetAccounts[0].pubkey;
      planetData = Buffer.from(planetAccounts[0].account.data);
      console.log("[LOOKUP] Strategy A: planet PDA =", planetPda.toBase58(), "size:", planetData.length);
    }

    if (!planetPda) {
      console.log("[LOOKUP] Strategy B: on-chain registry lookup...");
      const registry = await this.fetchRegistry(walletPubkey);
      if (registry) {
        console.log("[LOOKUP] Strategy B: registry found — fetching planet from", this.sessionActive ? "ER" : "devnet");
        const conn = this.sessionActive ? this.erConnection : this.connection;
        try {
          const account = await conn.getAccountInfo(registry.planetPda, "confirmed");
          if (account) {
            const data = Buffer.from(account.data);
            let creator = "";
            try { creator = new PublicKey(data.slice(DISC, DISC + 32)).toBase58(); } catch {}
            console.log("[LOOKUP] Strategy B: planet size:", data.length, "creator:", creator);
            if (creator === walletPubkey.toBase58()) {
              planetPda  = registry.planetPda;
              planetData = data;
              console.log("[LOOKUP] Strategy B: ✓ planet found via registry");
            } else {
              console.warn("[LOOKUP] Strategy B: creator mismatch in planet account");
            }
          } else {
            console.warn("[LOOKUP] Strategy B: planet account not found on", this.sessionActive ? "ER" : "devnet");
          }
        } catch (e) {
          console.error("[LOOKUP] Strategy B: planet fetch failed:", e);
        }
      } else {
        console.warn("[LOOKUP] Strategy B: no registry entry — player may not have registered yet");
      }
    }

    if (!planetPda || !planetData) {
      console.log("[LOOKUP] No planet found for this wallet");
      return null;
    }

    const planet    = deserializePlanet(planetData);
    const entityPda = new PublicKey(planet.entity);
    console.log("[LOOKUP] entityPda:", entityPda.toBase58());

    const fleetPda     = deriveComponentPda(entityPda, PROGRAM_IDS.componentFleet);
    const resourcesPda = deriveComponentPda(entityPda, PROGRAM_IDS.componentResources);
    const researchPda  = deriveComponentPda(entityPda, PROGRAM_IDS.componentResearch);

    const [planetOwnerInfo] = await this.connection.getMultipleAccountsInfo([planetPda]);
    const isDelegated = planetOwnerInfo?.owner.equals(DELEGATION_PROGRAM_ID) ?? false;
    console.log("[LOOKUP] isDelegated:", isDelegated);

    const dataConn = isDelegated ? this.erConnection : this.connection;

    const [fleetAccount, resourcesAccount, researchAccount] = await dataConn.getMultipleAccountsInfo([
      fleetPda, resourcesPda, researchPda,
    ]);

    if (!fleetAccount || !resourcesAccount || !researchAccount) {
      console.error("[LOOKUP] Missing fleet or resources — trying devnet fallback");
      const [fa2, ra2, rs2] = await this.connection.getMultipleAccountsInfo([fleetPda, resourcesPda, researchPda]);
      if (!fa2 || !ra2 || !rs2) {
        console.error("[LOOKUP] Both ER and devnet failed for fleet/resources");
        return null;
      }
      const fleet     = deserializeFleet(Buffer.from(fa2.data));
      const resources = deserializeResources(Buffer.from(ra2.data));
      const research  = deserializeResearch(Buffer.from(rs2.data));
      return {
        planet, resources, fleet, research, isDelegated,
        entityPda:    entityPda.toBase58(),
        planetPda:    planetPda.toBase58(),
        fleetPda:     fleetPda.toBase58(),
        resourcesPda: resourcesPda.toBase58(),
        researchPda:  researchPda.toBase58(),
      };
    }

    const fleet     = deserializeFleet(Buffer.from(fleetAccount.data));
    const resources = deserializeResources(Buffer.from(resourcesAccount.data));
    const research  = deserializeResearch(Buffer.from(researchAccount.data));
    console.log("[LOOKUP] ✓ Planet fully loaded — isDelegated:", isDelegated);
    return {
      planet, resources, fleet, research, isDelegated,
      entityPda:    entityPda.toBase58(),
      planetPda:    planetPda.toBase58(),
      fleetPda:     fleetPda.toBase58(),
      resourcesPda: resourcesPda.toBase58(),
      researchPda:  researchPda.toBase58(),
    };
  }

  // ── Find any player's state by wallet pubkey (for attack targeting) ─────────
  async findPlayerByWallet(walletPubkey: PublicKey): Promise<{
    entityPda: string; fleetPda: string; resourcesPda: string; researchPda: string;
  } | null> {
    try {
      const registry = await this.fetchRegistry(walletPubkey);
      if (!registry) return null;
      const planetAcc = await this.connection.getAccountInfo(registry.planetPda, "confirmed");
      if (!planetAcc) return null;
      const planet    = deserializePlanet(Buffer.from(planetAcc.data));
      const entityPda = new PublicKey(planet.entity);
      const fleetPda     = deriveComponentPda(entityPda, PROGRAM_IDS.componentFleet);
      const resourcesPda = deriveComponentPda(entityPda, PROGRAM_IDS.componentResources);
      const researchPda  = deriveComponentPda(entityPda, PROGRAM_IDS.componentResearch);
      return {
        entityPda:    entityPda.toBase58(),
        fleetPda:     fleetPda.toBase58(),
        resourcesPda: resourcesPda.toBase58(),
        researchPda:  researchPda.toBase58(),
      };
    } catch (e) {
      console.error("[findPlayerByWallet] failed:", e);
      return null;
    }
  }

  // ── Initialize a new planet ───────────────────────────────────────────────
  async initializePlanet(planetName = "Homeworld"): Promise<PlayerState> {
    const payer = this.provider.wallet.publicKey;
    console.log(`[INIT] Creating planet "${planetName}" for:`, payer.toBase58());

    const addEntityResult = await AddEntity({
      payer,
      world:      SHARED_WORLD_PDA,
      connection: this.connection,
    });
    const entityTx = addEntityResult.transaction;
    const entitySig = await this.provider.sendAndConfirm(entityTx, []);
    console.log("[1] Entity created:", entitySig);

    const entityPda = addEntityResult.entityPda;
    console.log("[1] Entity PDA:", entityPda.toBase58());

    const [planetInit, resourcesInit, fleetInit, researchInit] = await Promise.all([
      InitializeComponent({ payer, entity: entityPda, componentId: PROGRAM_IDS.componentPlanet    }),
      InitializeComponent({ payer, entity: entityPda, componentId: PROGRAM_IDS.componentResources }),
      InitializeComponent({ payer, entity: entityPda, componentId: PROGRAM_IDS.componentFleet     }),
      InitializeComponent({ payer, entity: entityPda, componentId: PROGRAM_IDS.componentResearch  }),
    ]);

    const planetPda    = planetInit.componentPda;
    const resourcesPda = resourcesInit.componentPda;
    const fleetPda     = fleetInit.componentPda;
    const researchPda  = researchInit.componentPda;

    const initTx = new Transaction().add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 }),
      ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 100_000 }),
      planetInit.transaction.instructions[0],
      resourcesInit.transaction.instructions[0],
      fleetInit.transaction.instructions[0],
      researchInit.transaction.instructions[0],
    );
    const initSig = await this.provider.sendAndConfirm(initTx, []);
    console.log("[2] Components confirmed:", initSig);

    const args = Buffer.alloc(65, 0);
    args.writeBigInt64LE(BigInt(Math.floor(Date.now() / 1000)), 0);
    const nameBytes = Buffer.from(planetName.slice(0, 19), "utf8");
    nameBytes.copy(args, 13);
    entityPda.toBuffer().copy(args, 32);
    args.writeUInt8(0, 64);

    const { transaction: applyTx } = await ApplySystem({
      authority: payer,
      systemId:  PROGRAM_IDS.systemInitialize,
      world:     SHARED_WORLD_PDA,
      entities: [{
        entity: entityPda,
        components: [
          { componentId: PROGRAM_IDS.componentPlanet    },
          { componentId: PROGRAM_IDS.componentResources },
          { componentId: PROGRAM_IDS.componentFleet     },
          { componentId: PROGRAM_IDS.componentResearch  },
        ],
      }],
      args: [],
    });

    const sysTx = new Transaction().add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 }),
      ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 100_000 }),
      patchApplyArgs(applyTx.instructions[0], args),
    );
    const sysSig = await this.provider.sendAndConfirm(sysTx, []);
    console.log("[3] system_initialize confirmed:", sysSig);

    await new Promise(r => setTimeout(r, 3000));

    let state: PlayerState | null = null;
    for (let attempt = 1; attempt <= 5; attempt++) {
      console.log(`[INIT] Fetching planet attempt ${attempt}/5...`);
      state = await this.findPlanet(payer);
      if (state) break;
      if (attempt < 5) await new Promise(r => setTimeout(r, 2000));
    }
    if (!state) throw new Error("Planet created but RPC propagation timed out — refresh the page");

    console.log("[4] Writing player registry...");
    try {
      await this.registerPlanet(
        new PublicKey(state.entityPda),
        new PublicKey(state.planetPda),
        state.planet.galaxy,
        state.planet.system,
        state.planet.position,
      );
      console.log("[4] Registry written");
    } catch (e) {
      console.warn("[4] Registry write failed (non-fatal):", e);
    }

    return state;
  }

  // ── System actions ─────────────────────────────────────────────────────────

  async startBuild(entityPda: PublicKey, buildingIdx: number): Promise<string> {
    const args = Buffer.alloc(10);
    args.writeUInt8(0, 0);
    args.writeUInt8(buildingIdx, 1);
    args.writeBigInt64LE(BigInt(Math.floor(Date.now() / 1000)), 2);
    return this.applySystem("start_build", entityPda, PROGRAM_IDS.systemBuild, [
      { componentId: PROGRAM_IDS.componentPlanet },
      { componentId: PROGRAM_IDS.componentResources },
    ], args);
  }

  async finishBuild(entityPda: PublicKey): Promise<string> {
    const args = Buffer.alloc(10);
    args.writeUInt8(1, 0);
    args.writeBigInt64LE(BigInt(Math.floor(Date.now() / 1000)), 2);
    return this.applySystem("finish_build", entityPda, PROGRAM_IDS.systemBuild, [
      { componentId: PROGRAM_IDS.componentPlanet },
      { componentId: PROGRAM_IDS.componentResources },
    ], args);
  }

  async startResearch(entityPda: PublicKey, techIdx: number): Promise<string> {
    const args = Buffer.alloc(10);
    args.writeUInt8(0, 0);
    args.writeUInt8(techIdx, 1);
    args.writeBigInt64LE(BigInt(Math.floor(Date.now() / 1000)), 2);
    return this.applySystem("start_research", entityPda, PROGRAM_IDS.systemResearch, [
      { componentId: PROGRAM_IDS.componentPlanet },
      { componentId: PROGRAM_IDS.componentResources },
      { componentId: PROGRAM_IDS.componentResearch },
    ], args);
  }

  async finishResearch(entityPda: PublicKey): Promise<string> {
    const args = Buffer.alloc(10);
    args.writeUInt8(1, 0);
    args.writeUInt8(0, 1);
    args.writeBigInt64LE(BigInt(Math.floor(Date.now() / 1000)), 2);
    return this.applySystem("finish_research", entityPda, PROGRAM_IDS.systemResearch, [
      { componentId: PROGRAM_IDS.componentPlanet },
      { componentId: PROGRAM_IDS.componentResources },
      { componentId: PROGRAM_IDS.componentResearch },
    ], args);
  }

  async buildShip(entityPda: PublicKey, shipType: number, quantity: number): Promise<string> {
    const args = Buffer.alloc(13);
    args.writeUInt8(shipType, 0);
    args.writeUInt32LE(quantity, 1);
    args.writeBigInt64LE(BigInt(Math.floor(Date.now() / 1000)), 5);
    return this.applySystem("build_ship", entityPda, PROGRAM_IDS.systemShipyard, [
      { componentId: PROGRAM_IDS.componentFleet },
      { componentId: PROGRAM_IDS.componentResources },
      { componentId: PROGRAM_IDS.componentResearch },
    ], args);
  }

  async launchFleet(
    entityPda: PublicKey,
    ships: { lf?:number; hf?:number; cr?:number; bs?:number; bc?:number;
             bm?:number; ds?:number; de?:number; sc?:number; lc?:number;
             rec?:number; ep?:number; col?:number },
    cargo: { metal?:bigint; crystal?:bigint; deuterium?:bigint },
    missionType: number,
    flightSeconds: number,
    speedFactor = 100,
  ): Promise<string> {
    if (missionType !== 2 && missionType !== 5) {
      throw new Error("Only Transport (2) and Colonize (5) missions are supported.");
    }
    const args = Buffer.alloc(94, 0);
    args.writeUInt8(missionType, 0);
    args.writeUInt32LE(ships.lf  ?? 0,  1);  args.writeUInt32LE(ships.hf  ?? 0,  5);
    args.writeUInt32LE(ships.cr  ?? 0,  9);  args.writeUInt32LE(ships.bs  ?? 0, 13);
    args.writeUInt32LE(ships.bc  ?? 0, 17);  args.writeUInt32LE(ships.bm  ?? 0, 21);
    args.writeUInt32LE(ships.ds  ?? 0, 25);  args.writeUInt32LE(ships.de  ?? 0, 29);
    args.writeUInt32LE(ships.sc  ?? 0, 33);  args.writeUInt32LE(ships.lc  ?? 0, 37);
    args.writeUInt32LE(ships.rec ?? 0, 41);  args.writeUInt32LE(ships.ep  ?? 0, 45);
    args.writeUInt32LE(ships.col ?? 0, 49);
    args.writeBigUInt64LE(cargo.metal     ?? 0n, 53);
    args.writeBigUInt64LE(cargo.crystal   ?? 0n, 61);
    args.writeBigUInt64LE(cargo.deuterium ?? 0n, 69);
    args.writeUInt8(speedFactor, 77);
    args.writeBigInt64LE(BigInt(Math.floor(Date.now() / 1000)), 78);
    args.writeBigInt64LE(BigInt(flightSeconds), 86);
    return this.applySystem("launch_fleet", entityPda, PROGRAM_IDS.systemLaunch, [
      { componentId: PROGRAM_IDS.componentFleet },
      { componentId: PROGRAM_IDS.componentResources },
    ], args);
  }

  async applyAttack(
    _attackerEntityPda: PublicKey,
    _defenderEntityPda: PublicKey,
    _missionSlot: number,
  ): Promise<string> {
    throw new Error("Attack flow has been removed; only transport and colonize are supported.");
  }

  isSessionActive(): boolean { return this.sessionActive; }

  restoreSession(): void {
    console.log("[CLIENT] Restoring session from on-chain delegation...");
    this.sessionActive = true;
    try {
      const stored = sessionStorage.getItem("_er_burner");
      if (stored) {
        const secretKey = Uint8Array.from(JSON.parse(stored));
        this.erSigner   = Keypair.fromSecretKey(secretKey);
        console.log("[CLIENT] ✓ Burner recovered from sessionStorage:", this.erSigner.publicKey.toBase58());
      } else {
        console.log("[CLIENT] No burner in sessionStorage — wallet will sign for endSession");
      }
    } catch (e) {
      console.warn("[CLIENT] Could not restore burner:", e);
    }
  }

  async startSession(entityPda: PublicKey): Promise<void> {
    const payer = this.provider.wallet.publicKey;
    const planetPda    = deriveComponentPda(entityPda, PROGRAM_IDS.componentPlanet);
    const resourcesPda = deriveComponentPda(entityPda, PROGRAM_IDS.componentResources);
    const fleetPda     = deriveComponentPda(entityPda, PROGRAM_IDS.componentFleet);
    const researchPda  = deriveComponentPda(entityPda, PROGRAM_IDS.componentResearch);

    console.log("[SESSION] ── startSession ─────────────────────────────");

    const [pAcc, rAcc, fAcc, rsAcc] = await this.connection.getMultipleAccountsInfo([
      planetPda, resourcesPda, fleetPda, researchPda,
    ]);
    if (!pAcc || !rAcc || !fAcc || !rsAcc) throw new Error("Cannot start session: one or more component accounts missing");

    const alreadyDelegated = pAcc.owner.equals(DELEGATION_PROGRAM_ID);
    if (alreadyDelegated) {
      throw new Error("Accounts are already delegated. End the current session first before starting a new one.");
    }

    const burner = Keypair.generate();
    console.log("[SESSION] Burner keypair:", burner.publicKey.toBase58());

    const fundTx = new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: payer,
        toPubkey:   burner.publicKey,
        lamports:   10_000_000,
      })
    );
    const fundSig = await this.provider.sendAndConfirm(fundTx, []);
    console.log("[SESSION] Burner funded:", fundSig);

    const buildDelegateIx = (componentProgramId: PublicKey, componentPda: PublicKey) =>
      createDelegateInstruction({
        entity:       entityPda,
        account:      componentPda,
        ownerProgram: componentProgramId,
        payer,
      });

    const delegateTx = new Transaction().add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 800_000 }),
      ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 50_000 }),
      buildDelegateIx(PROGRAM_IDS.componentPlanet,    planetPda),
      buildDelegateIx(PROGRAM_IDS.componentResources, resourcesPda),
      buildDelegateIx(PROGRAM_IDS.componentFleet,     fleetPda),
      buildDelegateIx(PROGRAM_IDS.componentResearch,  researchPda),
    );
    const delegateSig = await this.provider.sendAndConfirm(delegateTx, []);
    console.log("[SESSION] Delegate confirmed:", delegateSig);

    await new Promise(r => setTimeout(r, 2000));

    this.erSigner      = burner;
    this.sessionActive = true;
    try {
      sessionStorage.setItem("_er_burner", JSON.stringify(Array.from(burner.secretKey)));
    } catch (e) {
      console.warn("[SESSION] Could not persist burner (non-fatal):", e);
    }
    console.log("[SESSION] ✓ Session active");
  }

  async endSession(entityPda: PublicKey): Promise<void> {
    const planetPdaCheck = deriveComponentPda(entityPda, PROGRAM_IDS.componentPlanet);
    const planetAccCheck = await this.connection.getAccountInfo(planetPdaCheck, "confirmed");
    const isDelegatedOnChain = planetAccCheck?.owner.equals(DELEGATION_PROGRAM_ID) ?? false;
    console.log("[END_SESSION] isDelegated on-chain:", isDelegatedOnChain);

    if (!isDelegatedOnChain && !this.sessionActive) {
      throw new Error("No active session to end — accounts are not delegated");
    }

    const payer        = this.provider.wallet.publicKey;
    const planetPda    = deriveComponentPda(entityPda, PROGRAM_IDS.componentPlanet);
    const resourcesPda = deriveComponentPda(entityPda, PROGRAM_IDS.componentResources);
    const fleetPda     = deriveComponentPda(entityPda, PROGRAM_IDS.componentFleet);
    const researchPda  = deriveComponentPda(entityPda, PROGRAM_IDS.componentResearch);

    const erConn = this.erDirectConnection;

    const undelegatePayer = this.erSigner?.publicKey || payer;
    const buildUndelegateIx = (componentProgramId: PublicKey, delegatedAccount: PublicKey) =>
      createUndelegateInstruction({
        payer:            undelegatePayer,
        delegatedAccount: delegatedAccount,
        componentPda:     componentProgramId,
      });

    const ixPlanet    = buildUndelegateIx(PROGRAM_IDS.componentPlanet,    planetPda);
    const ixResources = buildUndelegateIx(PROGRAM_IDS.componentResources, resourcesPda);
    const ixFleet     = buildUndelegateIx(PROGRAM_IDS.componentFleet,     fleetPda);
    const ixResearch  = buildUndelegateIx(PROGRAM_IDS.componentResearch,  researchPda);

    const erSigner = this.erSigner;

    const sendUndelegateTx = async (): Promise<string> => {
      const { blockhash, lastValidBlockHeight } = await erConn.getLatestBlockhash("confirmed");

      const freshTx = new Transaction();
      freshTx.add(ixPlanet);
      freshTx.add(ixResources);
      freshTx.add(ixFleet);
      freshTx.add(ixResearch);
      freshTx.feePayer        = undelegatePayer;
      freshTx.recentBlockhash = blockhash;

      if (erSigner) {
        freshTx.sign(erSigner);
      } else {
        await this.provider.wallet.signTransaction(freshTx);
      }

      console.log(`[END_SESSION] blockhash: ${blockhash.slice(0,8)}... signing and sending...`);
      const txSig = await erConn.sendRawTransaction(freshTx.serialize(), { skipPreflight: true });
      await erConn.confirmTransaction({ signature: txSig, blockhash, lastValidBlockHeight }, "confirmed");
      return txSig;
    };

    let sig: string | null = null;
    for (let attempt = 1; attempt <= 5; attempt++) {
      try {
        console.log(`[END_SESSION] Sending undelegate to ER (attempt ${attempt}/5)...`);
        sig = await sendUndelegateTx();
        break;
      } catch (e: any) {
        const isBlockhash = e?.message?.includes("Blockhash not found") || e?.message?.includes("-32003");
        console.warn(`[END_SESSION] Attempt ${attempt} failed:`, e?.message?.slice(0, 80));
        if (!isBlockhash || attempt === 5) throw e;
        await new Promise(r => setTimeout(r, 500));
      }
    }
    if (!sig) throw new Error("Undelegate failed after all retries");
    console.log("[END_SESSION] Undelegate tx sent:", sig);

    await new Promise(r => setTimeout(r, 3000));

    if (this.erSigner) try {
      const burner = this.erSigner;
      const burnerBalance = await this.connection.getBalance(burner.publicKey);
      const refund = burnerBalance - 5000;
      if (refund > 0) {
        const recoverTx = new Transaction().add(
          SystemProgram.transfer({
            fromPubkey: burner.publicKey,
            toPubkey:   payer,
            lamports:   refund,
          })
        );
        const { blockhash, lastValidBlockHeight } = await this.connection.getLatestBlockhash("confirmed");
        recoverTx.recentBlockhash = blockhash;
        recoverTx.feePayer        = burner.publicKey;
        recoverTx.sign(burner);
        const recSig = await this.connection.sendRawTransaction(recoverTx.serialize());
        await this.connection.confirmTransaction({ signature: recSig, blockhash, lastValidBlockHeight }, "confirmed");
        console.log("[SESSION] Recovered", (refund / LAMPORTS_PER_SOL).toFixed(4), "SOL from burner");
      }
    } catch (e) {
      console.warn("[SESSION] Burner SOL recovery failed (non-critical):", e);
    }

    this.erSigner      = null;
    this.sessionActive = false;
    try { sessionStorage.removeItem("_er_burner"); } catch {}
    console.log("[END_SESSION] ✓ Session ended — all state saved on Solana devnet");
  }

  private async ensureWalletMeta(wallet: PublicKey): Promise<PublicKey> {
    const walletMetaPda = deriveWalletMetaPda(wallet);
    const existing = await this.connection.getAccountInfo(walletMetaPda, "confirmed");
    if (existing) return walletMetaPda;

    const ix = new TransactionInstruction({
      programId: REGISTRY_PROGRAM_ID,
      keys: [
        { pubkey: wallet, isSigner: true, isWritable: true },
        { pubkey: walletMetaPda, isSigner: false, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: ixDiscriminator("init_wallet_meta"),
    });
    await this.provider.sendAndConfirm(new Transaction().add(ix), []);
    return walletMetaPda;
  }

  async registerPlanet(
    entityPda: PublicKey,
    planetPda: PublicKey,
    galaxy: number,
    system: number,
    position: number,
  ): Promise<string> {
    const wallet = this.provider.wallet.publicKey;
    const walletMetaPda = await this.ensureWalletMeta(wallet);
    const planetCount = await this.fetchPlanetCount(wallet);
    const registryPda = deriveRegistryPdaByIndex(wallet, planetCount);
    const coordPda = PublicKey.findProgramAddressSync(
      [Buffer.from("coord"), Buffer.from(Uint8Array.of(galaxy & 0xff, (galaxy >> 8) & 0xff)), Buffer.from(Uint8Array.of(system & 0xff, (system >> 8) & 0xff)), Buffer.from([position & 0xff])],
      REGISTRY_PROGRAM_ID
    )[0];

    const args = Buffer.alloc(8 + 32 + 32 + 2 + 2 + 1);
    ixDiscriminator("register_planet").copy(args, 0);
    entityPda.toBuffer().copy(args, 8);
    planetPda.toBuffer().copy(args, 40);
    args.writeUInt16LE(galaxy, 72);
    args.writeUInt16LE(system, 74);
    args.writeUInt8(position, 76);

    const ix = new TransactionInstruction({
      programId: REGISTRY_PROGRAM_ID,
      keys: [
        { pubkey: wallet, isSigner: true, isWritable: true },
        { pubkey: walletMetaPda, isSigner: false, isWritable: true },
        { pubkey: registryPda, isSigner: false, isWritable: true },
        { pubkey: coordPda, isSigner: false, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: args,
    });

    const sig = await this.provider.sendAndConfirm(new Transaction().add(ix), []);
    console.log("[REGISTRY] ✓ Registered planet index", planetCount, ":", sig);
    return sig;
  }

  async fetchPlanetCount(walletPubkey: PublicKey): Promise<number> {
    const walletMetaPda = deriveWalletMetaPda(walletPubkey);
    try {
      const account = await this.connection.getAccountInfo(walletMetaPda, "confirmed");
      if (!account) return 0;
      const data = Buffer.from(account.data);
      if (data.length < 8 + 32 + 1) return 0;
      return data.readUInt8(8 + 32);
    } catch (e) {
      console.error("[REGISTRY] fetchPlanetCount failed:", e);
      return 0;
    }
  }

  async fetchRegistry(walletPubkey: PublicKey, index = 0): Promise<{ entityPda: PublicKey; planetPda: PublicKey } | null> {
    const registryPda = deriveRegistryPdaByIndex(walletPubkey, index);
    const account = await this.connection.getAccountInfo(registryPda, "confirmed");
    if (!account) return null;
    const data = Buffer.from(account.data);
    if (data.length < 8 + 32 + 1 + 32 + 32) return null;
    const entityPda = new PublicKey(data.slice(8 + 33, 8 + 33 + 32));
    const planetPda = new PublicKey(data.slice(8 + 33 + 32, 8 + 33 + 64));
    return { entityPda, planetPda };
  }

  // ── Internal: build + send an ApplySystem transaction ────────────────────
  private async applySystem(
    label:      string,
    entityPda:  PublicKey,
    systemId:   PublicKey,
    components: { componentId: PublicKey }[],
    rawArgs:    Buffer,
  ): Promise<string> {
    console.log(`[SYS:${label}] Sending... sessionActive:`, this.sessionActive, "erSigner:", !!this.erSigner);
    const authority = (this.sessionActive && this.erSigner)
      ? this.erSigner.publicKey
      : this.provider.wallet.publicKey;

    try {
      let sig: string;
      if (this.sessionActive && this.erSigner) {
        const erConn    = this.erDirectConnection;
        const erSigner  = this.erSigner;

        const { transaction: applyTx } = await ApplySystem({
          authority,
          systemId,
          world:     SHARED_WORLD_PDA,
          entities:  [{ entity: entityPda, components }],
          args:      [],
        });
        const patchedIx = patchApplyArgs(applyTx.instructions[0], rawArgs);

        const sendWithFreshBlockhash = async (): Promise<string> => {
          const { blockhash, lastValidBlockHeight } = await erConn.getLatestBlockhash("confirmed");
          const tx = new Transaction().add(
            ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 }),
            ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 100_000 }),
            patchedIx,
          );
          tx.recentBlockhash = blockhash;
          tx.feePayer        = erSigner.publicKey;
          tx.sign(erSigner);
          const txSig = await erConn.sendRawTransaction(tx.serialize(), { skipPreflight: true });
          await erConn.confirmTransaction({ signature: txSig, blockhash, lastValidBlockHeight }, "confirmed");
          return txSig;
        };

        let erSig: string | undefined;
        const maxRetries = 5;
        for (let attempt = 1; attempt <= maxRetries; attempt++) {
          try {
            erSig = await sendWithFreshBlockhash();
            break;
          } catch (retryErr: any) {
            const isBlockhash = retryErr?.message?.includes("Blockhash not found") || retryErr?.message?.includes("-32003");
            console.warn(`[SYS:${label}] Attempt ${attempt} failed:`, retryErr?.message?.slice(0, 60));
            if (!isBlockhash || attempt === maxRetries) throw retryErr;
            await new Promise(r => setTimeout(r, 300));
          }
        }
        if (!erSig) throw new Error(`[SYS:${label}] Failed after ${maxRetries} attempts`);
        sig = erSig;
      } else {
        const { transaction: applyTx } = await ApplySystem({
          authority,
          systemId,
          world:     SHARED_WORLD_PDA,
          entities:  [{ entity: entityPda, components }],
          args:      [],
        });
        const tx = new Transaction().add(
          ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 }),
          ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 100_000 }),
          patchApplyArgs(applyTx.instructions[0], rawArgs),
        );
        sig = await this.provider.sendAndConfirm(tx, [], { commitment: "confirmed" });
      }
      console.log(`[SYS:${label}] Confirmed:`, sig);
      return sig;
    } catch (err: any) {
      console.error(`[SYS:${label}] Failed:`, err.message);
      if (err.logs) console.log(`[SYS:${label}] Logs:`, err.logs);
      throw err;
    }
  }
}

// ─── Binary read helpers ──────────────────────────────────────────────────────
function readU8(d: Buffer, o: number): number  { return d.readUInt8(o); }
function readU16(d: Buffer, o: number): number  { return d.readUInt16LE(o); }
function readU32(d: Buffer, o: number): number  { return d.readUInt32LE(o); }
function readU64(d: Buffer, o: number): bigint  { return d.readBigUInt64LE(o); }
function readI16(d: Buffer, o: number): number  { return d.readInt16LE(o); }
function readI64(d: Buffer, o: number): number  { return Number(d.readBigInt64LE(o)); }
function readPubkey(d: Buffer, o: number): string {
  return new PublicKey(d.slice(o, o + 32)).toBase58();
}

// ─── Deserializers ────────────────────────────────────────────────────────────
export function deserializePlanet(data: Buffer): Planet {
  let o = DISC;
  const creator              = readPubkey(data, o); o += 32;
  const entity               = readPubkey(data, o); o += 32;
  const owner                = readPubkey(data, o); o += 32;
  const nameRaw              = data.slice(o, o + 32); o += 32;
  const name                 = Buffer.from(nameRaw).toString("utf8").replace(/\0/g, "").trim();
  const galaxy               = readU16(data, o); o += 2;
  const system               = readU16(data, o); o += 2;
  const position             = readU8(data, o);  o += 1;
  const planetIndex          = readU8(data, o);  o += 1;
  const diameter             = readU32(data, o); o += 4;
  const temperature          = readI16(data, o); o += 2;
  const maxFields            = readU16(data, o); o += 2;
  const usedFields           = readU16(data, o); o += 2;
  const metalMine            = readU8(data, o); o += 1;
  const crystalMine          = readU8(data, o); o += 1;
  const deuteriumSynthesizer = readU8(data, o); o += 1;
  const solarPlant           = readU8(data, o); o += 1;
  const fusionReactor        = readU8(data, o); o += 1;
  const roboticsFactory      = readU8(data, o); o += 1;
  const naniteFactory        = readU8(data, o); o += 1;
  const shipyard             = readU8(data, o); o += 1;
  const metalStorage         = readU8(data, o); o += 1;
  const crystalStorage       = readU8(data, o); o += 1;
  const deuteriumTank        = readU8(data, o); o += 1;
  const researchLab          = readU8(data, o); o += 1;
  const missileSilo          = readU8(data, o); o += 1;
  const buildQueueItem       = readU8(data, o); o += 1;
  const buildQueueTarget     = readU8(data, o); o += 1;
  const buildFinishTs        = readI64(data, o);
  return {
    creator, entity, owner, name, galaxy, system, position, planetIndex,
    diameter, temperature, maxFields, usedFields,
    metalMine, crystalMine, deuteriumSynthesizer, solarPlant,
    fusionReactor, roboticsFactory, naniteFactory, shipyard,
    metalStorage, crystalStorage, deuteriumTank, researchLab,
    missileSilo, buildQueueItem, buildQueueTarget, buildFinishTs,
  };
}

export function deserializeResearch(data: Buffer): Research {
  let o = DISC;
  const creator          = readPubkey(data, o); o += 32;
  const energyTech       = readU8(data, o); o += 1;
  const combustionDrive  = readU8(data, o); o += 1;
  const impulseDrive     = readU8(data, o); o += 1;
  const hyperspaceDrive  = readU8(data, o); o += 1;
  const computerTech     = readU8(data, o); o += 1;
  const astrophysics     = readU8(data, o); o += 1;
  const igrNetwork       = readU8(data, o); o += 1;
  const queueItem        = readU8(data, o); o += 1;
  const queueTarget      = readU8(data, o); o += 1;
  const researchFinishTs = readI64(data, o);
  return {
    creator,
    energyTech,
    combustionDrive,
    impulseDrive,
    hyperspaceDrive,
    computerTech,
    astrophysics,
    igrNetwork,
    queueItem,
    queueTarget,
    researchFinishTs,
  };
}

export function deserializeResources(data: Buffer): Resources {
  let o = DISC;
  const metal             = readU64(data, o); o += 8;
  const crystal           = readU64(data, o); o += 8;
  const deuterium         = readU64(data, o); o += 8;
  const metalHour         = readU64(data, o); o += 8;
  const crystalHour       = readU64(data, o); o += 8;
  const deuteriumHour     = readU64(data, o); o += 8;
  const energyProduction  = readU64(data, o); o += 8;
  const energyConsumption = readU64(data, o); o += 8;
  const metalCap          = readU64(data, o); o += 8;
  const crystalCap        = readU64(data, o); o += 8;
  const deuteriumCap      = readU64(data, o); o += 8;
  const lastUpdateTs      = readI64(data, o);
  return {
    metal, crystal, deuterium,
    metalHour, crystalHour, deuteriumHour,
    energyProduction, energyConsumption,
    metalCap, crystalCap, deuteriumCap,
    lastUpdateTs,
  };
}

function deserializeMission(data: Buffer, offset: number): { mission: Mission; bytesRead: number } {
  let o = offset;
  const missionType     = readU8(data, o);     o += 1;
  const destination     = readPubkey(data, o); o += 32;
  const departTs        = readI64(data, o);    o += 8;
  const arriveTs        = readI64(data, o);    o += 8;
  const returnTs        = readI64(data, o);    o += 8;
  const sSmallCargo     = readU32(data, o);    o += 4;
  const sLargeCargo     = readU32(data, o);    o += 4;
  const sLightFighter   = readU32(data, o);    o += 4;
  const sHeavyFighter   = readU32(data, o);    o += 4;
  const sCruiser        = readU32(data, o);    o += 4;
  const sBattleship     = readU32(data, o);    o += 4;
  const sBattlecruiser  = readU32(data, o);    o += 4;
  const sBomber         = readU32(data, o);    o += 4;
  const sDestroyer      = readU32(data, o);    o += 4;
  const sDeathstar      = readU32(data, o);    o += 4;
  const sRecycler       = readU32(data, o);    o += 4;
  const sEspionageProbe = readU32(data, o);    o += 4;
  const sColonyShip     = readU32(data, o);    o += 4;
  const cargoMetal      = readU64(data, o);    o += 8;
  const cargoCrystal    = readU64(data, o);    o += 8;
  const cargoDeuterium  = readU64(data, o);    o += 8;
  const applied         = readU8(data, o) !== 0; o += 1;
  return {
    mission: {
      missionType, destination, departTs, arriveTs, returnTs,
      sSmallCargo, sLargeCargo, sLightFighter, sHeavyFighter,
      sCruiser, sBattleship, sBattlecruiser, sBomber, sDestroyer,
      sDeathstar, sRecycler, sEspionageProbe, sColonyShip,
      cargoMetal, cargoCrystal, cargoDeuterium, applied,
    },
    bytesRead: o - offset,
  };
}

export function deserializeFleet(data: Buffer): Fleet {
  let o = DISC;
  const creator        = readPubkey(data, o); o += 32;
  const smallCargo     = readU32(data, o); o += 4;
  const largeCargo     = readU32(data, o); o += 4;
  const lightFighter   = readU32(data, o); o += 4;
  const heavyFighter   = readU32(data, o); o += 4;
  const cruiser        = readU32(data, o); o += 4;
  const battleship     = readU32(data, o); o += 4;
  const battlecruiser  = readU32(data, o); o += 4;
  const bomber         = readU32(data, o); o += 4;
  const destroyer      = readU32(data, o); o += 4;
  const deathstar      = readU32(data, o); o += 4;
  const recycler       = readU32(data, o); o += 4;
  const espionageProbe = readU32(data, o); o += 4;
  const colonyShip     = readU32(data, o); o += 4;
  const solarSatellite = readU32(data, o); o += 4;
  const activeMissions = readU8(data, o);  o += 1;
  const missions: Mission[] = [];
  for (let i = 0; i < 4; i++) {
    const { mission, bytesRead } = deserializeMission(data, o);
    missions.push(mission);
    o += bytesRead;
  }
  return {
    creator, smallCargo, largeCargo, lightFighter, heavyFighter,
    cruiser, battleship, battlecruiser, bomber, destroyer, deathstar,
    recycler, espionageProbe, colonyShip, solarSatellite,
    activeMissions, missions,
  };
}

// ─── Static metadata ──────────────────────────────────────────────────────────
export const BUILDINGS = [
  { idx: 0,  key: "metalMine",            name: "Metal Mine",            icon: "⬡",  desc: "Extracts metal from the planet crust." },
  { idx: 1,  key: "crystalMine",          name: "Crystal Mine",          icon: "◈",  desc: "Processes surface crystal formations." },
  { idx: 2,  key: "deuteriumSynthesizer", name: "Deuterium Synthesizer", icon: "◉",  desc: "Extracts deuterium from the atmosphere." },
  { idx: 3,  key: "solarPlant",           name: "Solar Plant",           icon: "☀",  desc: "Converts sunlight to energy." },
  { idx: 4,  key: "fusionReactor",        name: "Fusion Reactor",        icon: "⚛",  desc: "Burns deuterium for high-yield energy." },
  { idx: 5,  key: "roboticsFactory",      name: "Robotics Factory",      icon: "🤖", desc: "Automated workers — halves build time." },
  { idx: 6,  key: "naniteFactory",        name: "Nanite Factory",        icon: "🔬", desc: "Nano assemblers — massive build speed." },
  { idx: 7,  key: "shipyard",             name: "Shipyard",              icon: "🚀", desc: "Constructs ships and defense units." },
  { idx: 8,  key: "metalStorage",         name: "Metal Storage",         icon: "🏗", desc: "Increases metal cap." },
  { idx: 9,  key: "crystalStorage",       name: "Crystal Storage",       icon: "🏗", desc: "Increases crystal cap." },
  { idx: 10, key: "deuteriumTank",        name: "Deuterium Tank",        icon: "🏗", desc: "Increases deuterium cap." },
  { idx: 11, key: "researchLab",          name: "Research Lab",          icon: "🔭", desc: "Required for all technology research." },
  { idx: 12, key: "missileSilo",          name: "Missile Silo",          icon: "🎯", desc: "Stores interplanetary missiles." },
] as const;

export type BuildingKey = typeof BUILDINGS[number]["key"];

export const SHIPS = [
  { key: "smallCargo",     name: "Small Cargo",     icon: "📦", atk: 5,      cargo: 5_000,   cost: { m: 2000,    c: 2000,    d: 0       } },
  { key: "largeCargo",     name: "Large Cargo",      icon: "🚛", atk: 5,      cargo: 25_000,  cost: { m: 6000,    c: 6000,    d: 0       } },
  { key: "lightFighter",   name: "Light Fighter",    icon: "✈",  atk: 50,     cargo: 50,      cost: { m: 3000,    c: 1000,    d: 0       } },
  { key: "heavyFighter",   name: "Heavy Fighter",    icon: "⚡",  atk: 150,    cargo: 100,     cost: { m: 6000,    c: 4000,    d: 0       } },
  { key: "cruiser",        name: "Cruiser",           icon: "🛸", atk: 400,    cargo: 800,     cost: { m: 20000,   c: 7000,    d: 2000    } },
  { key: "battleship",     name: "Battleship",        icon: "⚔",  atk: 1000,   cargo: 1500,    cost: { m: 45000,   c: 15000,   d: 0       } },
  { key: "battlecruiser",  name: "Battlecruiser",     icon: "🔱", atk: 700,    cargo: 750,     cost: { m: 30000,   c: 40000,   d: 15000   } },
  { key: "bomber",         name: "Bomber",            icon: "💣", atk: 1000,   cargo: 500,     cost: { m: 50000,   c: 25000,   d: 15000   } },
  { key: "destroyer",      name: "Destroyer",         icon: "💥", atk: 2000,   cargo: 2000,    cost: { m: 60000,   c: 50000,   d: 15000   } },
  { key: "deathstar",      name: "Deathstar",         icon: "🌑", atk: 200000, cargo: 1000000, cost: { m: 5000000, c: 4000000, d: 1000000 } },
  { key: "recycler",       name: "Recycler",          icon: "♻",  atk: 1,      cargo: 20_000,  cost: { m: 10000,   c: 6000,    d: 2000    } },
  { key: "espionageProbe", name: "Espionage Probe",   icon: "👁",  atk: 0,      cargo: 0,       cost: { m: 0,       c: 1000,    d: 0       } },
  { key: "colonyShip",     name: "Colony Ship",       icon: "🌍", atk: 50,     cargo: 7500,    cost: { m: 10000,   c: 20000,   d: 10000   } },
  { key: "solarSatellite", name: "Solar Satellite",   icon: "🛰",  atk: 1,      cargo: 0,       cost: { m: 0,       c: 2000,    d: 500     } },
] as const;

// Ship type index mapping (matches on-chain order in system_shipyard)
export const SHIP_TYPE_IDX: Record<string, number> = {
  smallCargo: 0, largeCargo: 1, lightFighter: 2, heavyFighter: 3,
  cruiser: 4, battleship: 5, battlecruiser: 6, bomber: 7,
  destroyer: 8, deathstar: 9, recycler: 10, espionageProbe: 11,
  colonyShip: 12, solarSatellite: 13,
};

export const MISSION_LABELS: Record<number, string> = {
  2: "TRANSPORT",
  5: "COLONIZE",
};

// ─── Cost & time formulas ─────────────────────────────────────────────────────
const BASE_COSTS: Record<number, [number, number, number]> = {
  0:  [60,      15,     0],      1:  [48,      24,     0],
  2:  [225,     75,     0],      3:  [75,      30,     0],
  4:  [900,     360,    900],    5:  [400,     120,    200],
  6:  [1000000, 500000, 100000], 7:  [400,     200,    100],
  8:  [1000,    0,      0],      9:  [1000,    500,    0],
  10: [1000,    1000,   0],      11: [200,     400,    200],
  12: [20,      20,     0],
};

function pow15(n: number): number {
  let r = 1;
  for (let i = 0; i < n; i++) r = r * 1.5;
  return r;
}

export function upgradeCost(idx: number, currentLevel: number): [number, number, number] {
  const [bm, bc, bd] = BASE_COSTS[idx] ?? [0, 0, 0];
  const mult = pow15(currentLevel);
  return [Math.floor(bm * mult), Math.floor(bc * mult), Math.floor(bd * mult)];
}

export function buildTimeSecs(idx: number, nextLevel: number, robotics: number): number {
  const [bm, bc] = BASE_COSTS[idx] ?? [0, 0];
  const total = (bm + bc) * pow15(nextLevel - 1);
  return Math.max(1, Math.floor(total / (5 * (1 + robotics))));
}

// ─── Display helpers ──────────────────────────────────────────────────────────
export function fmt(n: bigint | number): string {
  const v = typeof n === "bigint" ? Number(n) : n;
  if (v >= 1_000_000_000) return (v / 1_000_000_000).toFixed(2) + "B";
  if (v >= 1_000_000)     return (v / 1_000_000).toFixed(2) + "M";
  if (v >= 1_000)         return (v / 1_000).toFixed(1) + "K";
  return v.toLocaleString();
}

export function fmtCountdown(totalSecs: number): string {
  if (totalSecs <= 0) return "READY";
  const h = Math.floor(totalSecs / 3600);
  const m = Math.floor((totalSecs % 3600) / 60);
  const s = totalSecs % 60;
  if (h > 0) return `${h}h ${String(m).padStart(2, "0")}m`;
  if (m > 0) return `${m}m ${String(s).padStart(2, "0")}s`;
  return `${s}s`;
}

export function missionProgress(m: Mission, nowTs: number): number {
  if (m.applied) {
    const total   = m.returnTs - m.arriveTs;
    const elapsed = nowTs - m.arriveTs;
    return total <= 0 ? 100 : Math.min(100, Math.floor((elapsed / total) * 100));
  }
  const total   = m.arriveTs - m.departTs;
  const elapsed = nowTs - m.departTs;
  return total <= 0 ? 100 : Math.min(100, Math.floor((elapsed / total) * 100));
}

export function energyEfficiency(res: Resources): number {
  if (res.energyConsumption === 0n) return 100;
  return Math.min(100, Number(res.energyProduction * 100n / res.energyConsumption));
}
