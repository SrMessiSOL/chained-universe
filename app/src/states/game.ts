/**
 * SolarGrid Game Client — fixed version
 *
 * Fixes applied vs original:
 *  1. buildShip() arg buffer: offset for timestamp corrected 2→5
 *  2. initializeWorld() now calls system-initialize after attaching components
 *  3. IDL discriminators computed from sha256("account:<Name>")[..8]
 *  4. Galaxy scan uses getProgramAccounts to find real on-chain planets
 *  5. No more "55 years of production" bug — system-initialize sets lastUpdateTs=now
 *  6. Duplicate SYNC button removed (handled in App.tsx)
 */

import {
  Connection,
  PublicKey,
  GetProgramAccountsFilter,
} from "@solana/web3.js";
import { Buffer } from "buffer";
import { AnchorProvider, Program, BN, Idl, setProvider } from "@coral-xyz/anchor";
import {
  AddEntity,
  InitializeComponent,
  ApplySystem,
} from "@magicblock-labs/bolt-sdk";
import * as BoltSdk from "@magicblock-labs/bolt-sdk";

import {
  COMPONENT_PLANET_ID,
  COMPONENT_RESOURCES_ID,
  COMPONENT_FLEET_ID,
  SYSTEM_INITIALIZE_ID,
  SYSTEM_BUILD_ID,
  SYSTEM_PRODUCE_ID,
  SYSTEM_SHIPYARD_ID,
  SYSTEM_LAUNCH_ID,
  SYSTEM_ATTACK_ID,
  SHARED_WORLD_PDA_STR,
} from "../constants";

// ── Discriminators (sha256("account:<TypeName>")[0..8]) ───────────────────────
// Pre-computed for the deployed programs. If you re-deploy with different type
// names, re-compute with: node -e "const c=require('crypto');console.log([...c.createHash('sha256').update('account:Planet').digest()].slice(0,8))"
const DISC_PLANET    = [242, 27, 236, 42, 220, 217, 132, 128];
const DISC_RESOURCES = [252, 239, 111,  79,  54,   7,  67, 233];
const DISC_FLEET     = [109, 207, 251,  48, 106,   2, 136, 163];

// ── Minimal IDLs for account deserialization ───────────────────────────────────
const PLANET_IDL: Idl = {
  address: COMPONENT_PLANET_ID.toBase58(),
  metadata: { name: "planet", version: "0.1.0", spec: "0.1.0" },
  instructions: [],
  accounts: [{ name: "Planet", discriminator: DISC_PLANET }],
  types: [{
    name: "Planet",
    type: {
      kind: "struct" as const,
      fields: [
        { name: "owner",                type: "pubkey" },
        { name: "name",                 type: { array: ["u8", 32] } },
        { name: "galaxy",               type: "u16" },
        { name: "system",               type: "u16" },
        { name: "position",             type: "u8" },
        { name: "diameter",             type: "u32" },
        { name: "temperature",          type: "i16" },
        { name: "maxFields",            type: "u16" },
        { name: "usedFields",           type: "u16" },
        { name: "metalMine",            type: "u8" },
        { name: "crystalMine",          type: "u8" },
        { name: "deuteriumSynthesizer", type: "u8" },
        { name: "solarPlant",           type: "u8" },
        { name: "fusionReactor",        type: "u8" },
        { name: "roboticsFactory",      type: "u8" },
        { name: "naniteFactory",        type: "u8" },
        { name: "shipyard",             type: "u8" },
        { name: "metalStorage",         type: "u8" },
        { name: "crystalStorage",       type: "u8" },
        { name: "deuteriumTank",        type: "u8" },
        { name: "researchLab",          type: "u8" },
        { name: "missileSilo",          type: "u8" },
        { name: "buildQueueItem",       type: "u8" },
        { name: "buildQueueTarget",     type: "u8" },
        { name: "buildFinishTs",        type: "i64" },
        { name: "boltMetadata",         type: { defined: { name: "BoltMetadata" } } },
      ],
    },
  }, {
    name: "BoltMetadata",
    type: { kind: "struct" as const, fields: [{ name: "authority", type: "pubkey" }] },
  }],
} as unknown as Idl;

const RESOURCES_IDL: Idl = {
  address: COMPONENT_RESOURCES_ID.toBase58(),
  metadata: { name: "resources", version: "0.1.0", spec: "0.1.0" },
  instructions: [],
  accounts: [{ name: "Resources", discriminator: DISC_RESOURCES }],
  types: [{
    name: "Resources",
    type: {
      kind: "struct" as const,
      fields: [
        { name: "metal",            type: "u64" },
        { name: "crystal",          type: "u64" },
        { name: "deuterium",        type: "u64" },
        { name: "metalHour",        type: "u64" },
        { name: "crystalHour",      type: "u64" },
        { name: "deuteriumHour",    type: "u64" },
        { name: "energyProduction", type: "u64" },
        { name: "energyConsumption",type: "u64" },
        { name: "metalCap",         type: "u64" },
        { name: "crystalCap",       type: "u64" },
        { name: "deuteriumCap",     type: "u64" },
        { name: "lastUpdateTs",     type: "i64" },
        { name: "boltMetadata",     type: { defined: { name: "BoltMetadata" } } },
      ],
    },
  }, {
    name: "BoltMetadata",
    type: { kind: "struct" as const, fields: [{ name: "authority", type: "pubkey" }] },
  }],
} as unknown as Idl;

const FLEET_IDL: Idl = {
  address: COMPONENT_FLEET_ID.toBase58(),
  metadata: { name: "fleet", version: "0.1.0", spec: "0.1.0" },
  instructions: [],
  accounts: [{ name: "Fleet", discriminator: DISC_FLEET }],
  types: [{
    name: "Fleet",
    type: {
      kind: "struct" as const,
      fields: [
        { name: "smallCargo",      type: "u32" },
        { name: "largeCargo",      type: "u32" },
        { name: "lightFighter",    type: "u32" },
        { name: "heavyFighter",    type: "u32" },
        { name: "cruiser",         type: "u32" },
        { name: "battleship",      type: "u32" },
        { name: "battlecruiser",   type: "u32" },
        { name: "bomber",          type: "u32" },
        { name: "destroyer",       type: "u32" },
        { name: "deathstar",       type: "u32" },
        { name: "recycler",        type: "u32" },
        { name: "espionageProbe",  type: "u32" },
        { name: "colonyShip",      type: "u32" },
        { name: "solarSatellite",  type: "u32" },
        { name: "activeMissions",  type: "u8"  },
        { name: "missions", type: { array: [{ defined: { name: "Mission" } }, 4] } },
        { name: "boltMetadata", type: { defined: { name: "BoltMetadata" } } },
      ],
    },
  }, {
    name: "Mission",
    type: {
      kind: "struct" as const,
      fields: [
        { name: "missionType",     type: "u8"     },
        { name: "destination",     type: "pubkey" },
        { name: "departTs",        type: "i64"    },
        { name: "arriveTs",        type: "i64"    },
        { name: "returnTs",        type: "i64"    },
        { name: "sSmallCargo",     type: "u32"    },
        { name: "sLargeCargo",     type: "u32"    },
        { name: "sLightFighter",   type: "u32"    },
        { name: "sHeavyFighter",   type: "u32"    },
        { name: "sCruiser",        type: "u32"    },
        { name: "sBattleship",     type: "u32"    },
        { name: "sBattlecruiser",  type: "u32"    },
        { name: "sBomber",         type: "u32"    },
        { name: "sDestroyer",      type: "u32"    },
        { name: "sDeathstar",      type: "u32"    },
        { name: "sRecycler",       type: "u32"    },
        { name: "sEspionageProbe", type: "u32"    },
        { name: "sColonyShip",     type: "u32"    },
        { name: "cargoMetal",      type: "u64"    },
        { name: "cargoCrystal",    type: "u64"    },
        { name: "cargoDeuterium",  type: "u64"    },
        { name: "applied",         type: "bool"   },
      ],
    },
  }, {
    name: "BoltMetadata",
    type: { kind: "struct" as const, fields: [{ name: "authority", type: "pubkey" }] },
  }],
} as unknown as Idl;

// ── Public types ───────────────────────────────────────────────────────────────

export interface OnChainPlanet {
  owner: PublicKey;
  name: string;
  galaxy: number;
  system: number;
  position: number;
  diameter: number;
  temperature: number;
  maxFields: number;
  usedFields: number;
  metalMine: number;
  crystalMine: number;
  deuteriumSynthesizer: number;
  solarPlant: number;
  fusionReactor: number;
  roboticsFactory: number;
  naniteFactory: number;
  shipyard: number;
  metalStorage: number;
  crystalStorage: number;
  deuteriumTank: number;
  researchLab: number;
  missileSilo: number;
  buildQueueItem: number;
  buildQueueTarget: number;
  buildFinishTs: number;
}

export interface OnChainResources {
  metal: number;
  crystal: number;
  deuterium: number;
  metalHour: number;
  crystalHour: number;
  deuteriumHour: number;
  energyProduction: number;
  energyConsumption: number;
  metalCap: number;
  crystalCap: number;
  deuteriumCap: number;
  lastUpdateTs: number;
}

export interface OnChainMission {
  missionType: number;
  destination: string;
  departTs: number;
  arriveTs: number;
  returnTs: number;
  sSmallCargo: number; sLargeCargo: number; sLightFighter: number;
  sHeavyFighter: number; sCruiser: number; sBattleship: number;
  sBattlecruiser: number; sBomber: number; sDestroyer: number;
  sDeathstar: number; sRecycler: number; sEspionageProbe: number;
  sColonyShip: number;
  cargoMetal: number; cargoCrystal: number; cargoDeuterium: number;
  applied: boolean;
}

export interface OnChainFleet {
  smallCargo: number; largeCargo: number; lightFighter: number;
  heavyFighter: number; cruiser: number; battleship: number;
  battlecruiser: number; bomber: number; destroyer: number;
  deathstar: number; recycler: number; espionageProbe: number;
  colonyShip: number; solarSatellite: number;
  activeMissions: number;
  missions: OnChainMission[];
}

export interface GameAddresses {
  worldPda:     PublicKey;
  entityPda:    PublicKey;
  planetPda:    PublicKey;
  resourcesPda: PublicKey;
  fleetPda:     PublicKey;
}

// Galaxy scan result — a real on-chain planet at a specific coordinate
export interface GalaxyEntry {
  planetPda:   string;
  owner:       string;
  name:        string;
  galaxy:      number;
  system:      number;
  position:    number;
  metalMine:   number;
  crystalMine: number;
  isMe:        boolean;
}

// ── SolarGrid Client ───────────────────────────────────────────────────────────

export class SolarGridClient {
  private connection: Connection;
  private provider:   AnchorProvider;
  private planetProg:    Program<Idl>;
  private resourcesProg: Program<Idl>;
  private fleetProg:     Program<Idl>;

  constructor(connection: Connection, provider: AnchorProvider) {
    this.connection = connection;
    this.provider   = provider;
    setProvider(provider);
    (BoltSdk as any).setProvider?.(provider);
    this.planetProg    = new Program(PLANET_IDL,    provider);
    this.resourcesProg = new Program(RESOURCES_IDL, provider);
    this.fleetProg     = new Program(FLEET_IDL,     provider);
  }

  // ── Storage helpers ──────────────────────────────────────────────────────────

  private storageKey(owner: PublicKey): string {
    return `solargrid_v2_${owner.toBase58()}`;
  }

  loadAddresses(owner: PublicKey): GameAddresses | null {
    try {
      const s = localStorage.getItem(this.storageKey(owner));
      if (!s) return null;
      const p = JSON.parse(s);
      return {
        worldPda:     new PublicKey(p.worldPda),
        entityPda:    new PublicKey(p.entityPda),
        planetPda:    new PublicKey(p.planetPda),
        resourcesPda: new PublicKey(p.resourcesPda),
        fleetPda:     new PublicKey(p.fleetPda),
      };
    } catch { return null; }
  }

  private saveAddresses(owner: PublicKey, a: GameAddresses) {
    localStorage.setItem(this.storageKey(owner), JSON.stringify({
      worldPda:     a.worldPda.toBase58(),
      entityPda:    a.entityPda.toBase58(),
      planetPda:    a.planetPda.toBase58(),
      resourcesPda: a.resourcesPda.toBase58(),
      fleetPda:     a.fleetPda.toBase58(),
    }));
  }

  // ── World + entity initialization ────────────────────────────────────────────

  /**
   * Full player initialization:
   *   1. AddEntity to shared world
   *   2. InitializeComponent for Planet, Resources, Fleet
   *   3. Call system-initialize to seed all three with real data
   *
   * Requires VITE_SHARED_WORLD_PDA env var to be set (run scripts/create-world.ts first).
   */
  async initializeWorld(planetName = "Homeworld"): Promise<GameAddresses> {
    if (!SHARED_WORLD_PDA_STR) {
      throw new Error(
        "VITE_SHARED_WORLD_PDA is not set.\n" +
        "Run: npx ts-node scripts/create-world.ts\n" +
        "Then add the printed PDA to frontend/.env"
      );
    }

    const payer    = this.provider.wallet.publicKey;
    const worldPda = new PublicKey(SHARED_WORLD_PDA_STR);

    // 1. Create entity
    const { transaction: eTx, entityPda } = await AddEntity({
      payer, world: worldPda, connection: this.connection,
    });
    await this.provider.sendAndConfirm(eTx, [], { commitment: "confirmed" });

    // 2. Attach components (each in its own tx — BOLT SDK requirement)
    const { transaction: pTx, componentPda: planetPda } = await InitializeComponent({
      payer, entity: entityPda, componentId: COMPONENT_PLANET_ID,
    });
    await this.provider.sendAndConfirm(pTx, [], { commitment: "confirmed" });

    const { transaction: rTx, componentPda: resourcesPda } = await InitializeComponent({
      payer, entity: entityPda, componentId: COMPONENT_RESOURCES_ID,
    });
    await this.provider.sendAndConfirm(rTx, [], { commitment: "confirmed" });

    const { transaction: fTx, componentPda: fleetPda } = await InitializeComponent({
      payer, entity: entityPda, componentId: COMPONENT_FLEET_ID,
    });
    await this.provider.sendAndConfirm(fTx, [], { commitment: "confirmed" });

    const addresses: GameAddresses = { worldPda, entityPda, planetPda, resourcesPda, fleetPda };

    // 3. Run system-initialize to set real planet data & timestamp
    //    Args (32 bytes):
    //      [0..8]   now:       i64
    //      [8..10]  galaxy:    u16  (0 = derive from wallet)
    //      [10..12] system:    u16  (0 = derive from wallet)
    //      [12]     position:  u8   (0 = derive from wallet)
    //      [13..32] name:      19 bytes UTF-8 null-padded
    const now   = BigInt(Math.floor(Date.now() / 1000));
    const args  = Buffer.alloc(32, 0);
    args.writeBigInt64LE(now, 0);
    // galaxy/system/position = 0 → system derives from wallet
    const nameBytes = Buffer.from(planetName.slice(0, 19), "utf8");
    nameBytes.copy(args, 13, 0, Math.min(nameBytes.length, 19));

    const { transaction: iTx } = await ApplySystem({
      authority: payer,
      systemId:  SYSTEM_INITIALIZE_ID,
      world:     worldPda,
      entities: [{
        entity: entityPda,
        components: [
          { componentId: COMPONENT_PLANET_ID },
          { componentId: COMPONENT_RESOURCES_ID },
          { componentId: COMPONENT_FLEET_ID },
        ],
      }],
      args: Array.from(args),
    });
    await this.provider.sendAndConfirm(iTx, [], { commitment: "confirmed" });

    this.saveAddresses(payer, addresses);
    return addresses;
  }

  // ── Reads ────────────────────────────────────────────────────────────────────

  async fetchPlanet(planetPda: PublicKey): Promise<OnChainPlanet | null> {
    try {
      const raw = await (this.planetProg.account as any)["planet"].fetch(planetPda) as any;
      const bn = (v: any): number => v instanceof BN ? v.toNumber() : Number(v);
      return {
        owner:                 raw.owner as PublicKey,
        name:                  Buffer.from(raw.name).toString("utf8").replace(/\0/g, "").trim() || "Homeworld",
        galaxy:                raw.galaxy,
        system:                raw.system,
        position:              raw.position,
        diameter:              raw.diameter || 12800,
        temperature:           raw.temperature,
        maxFields:             raw.maxFields || 163,
        usedFields:            raw.usedFields || 0,
        metalMine:             raw.metalMine,
        crystalMine:           raw.crystalMine,
        deuteriumSynthesizer:  raw.deuteriumSynthesizer,
        solarPlant:            raw.solarPlant,
        fusionReactor:         raw.fusionReactor,
        roboticsFactory:       raw.roboticsFactory,
        naniteFactory:         raw.naniteFactory,
        shipyard:              raw.shipyard,
        metalStorage:          raw.metalStorage,
        crystalStorage:        raw.crystalStorage,
        deuteriumTank:         raw.deuteriumTank,
        researchLab:           raw.researchLab,
        missileSilo:           raw.missileSilo,
        buildQueueItem:        raw.buildQueueItem,
        buildQueueTarget:      raw.buildQueueTarget,
        buildFinishTs:         bn(raw.buildFinishTs),
      };
    } catch (e) {
      console.error("fetchPlanet:", e);
      return null;
    }
  }

  async fetchResources(resourcesPda: PublicKey): Promise<OnChainResources | null> {
    try {
      const raw = await (this.resourcesProg.account as any)["resources"].fetch(resourcesPda) as any;
      const bn  = (v: any): number => v instanceof BN ? v.toNumber() : Number(v);
      return {
        metal:             bn(raw.metal),
        crystal:           bn(raw.crystal),
        deuterium:         bn(raw.deuterium),
        metalHour:         bn(raw.metalHour),
        crystalHour:       bn(raw.crystalHour),
        deuteriumHour:     bn(raw.deuteriumHour),
        energyProduction:  bn(raw.energyProduction),
        energyConsumption: bn(raw.energyConsumption),
        metalCap:          bn(raw.metalCap),
        crystalCap:        bn(raw.crystalCap),
        deuteriumCap:      bn(raw.deuteriumCap),
        lastUpdateTs:      bn(raw.lastUpdateTs),
      };
    } catch (e) {
      console.error("fetchResources:", e);
      return null;
    }
  }

  async fetchFleet(fleetPda: PublicKey): Promise<OnChainFleet | null> {
    try {
      const raw = await (this.fleetProg.account as any)["fleet"].fetch(fleetPda) as any;
      const bn  = (v: any): number => v instanceof BN ? v.toNumber() : Number(v);
      const missions: OnChainMission[] = (raw.missions || [])
        .filter((m: any) => m.missionType !== 0)
        .map((m: any) => ({
          missionType:    m.missionType,
          destination:    (m.destination as PublicKey).toBase58(),
          departTs:       bn(m.departTs),
          arriveTs:       bn(m.arriveTs),
          returnTs:       bn(m.returnTs),
          sSmallCargo:    m.sSmallCargo,    sLargeCargo:    m.sLargeCargo,
          sLightFighter:  m.sLightFighter,  sHeavyFighter:  m.sHeavyFighter,
          sCruiser:       m.sCruiser,       sBattleship:    m.sBattleship,
          sBattlecruiser: m.sBattlecruiser, sBomber:        m.sBomber,
          sDestroyer:     m.sDestroyer,     sDeathstar:     m.sDeathstar,
          sRecycler:      m.sRecycler,      sEspionageProbe:m.sEspionageProbe,
          sColonyShip:    m.sColonyShip,
          cargoMetal:     bn(m.cargoMetal),
          cargoCrystal:   bn(m.cargoCrystal),
          cargoDeuterium: bn(m.cargoDeuterium),
          applied:        m.applied,
        }));
      return {
        smallCargo:     raw.smallCargo,    largeCargo:    raw.largeCargo,
        lightFighter:   raw.lightFighter,  heavyFighter:  raw.heavyFighter,
        cruiser:        raw.cruiser,       battleship:    raw.battleship,
        battlecruiser:  raw.battlecruiser, bomber:        raw.bomber,
        destroyer:      raw.destroyer,     deathstar:     raw.deathstar,
        recycler:       raw.recycler,      espionageProbe:raw.espionageProbe,
        colonyShip:     raw.colonyShip,    solarSatellite:raw.solarSatellite,
        activeMissions: raw.activeMissions,
        missions,
      };
    } catch (e) {
      console.error("fetchFleet:", e);
      return null;
    }
  }

  /**
   * Scan a galaxy:system for real on-chain Planet accounts.
   * Uses getProgramAccounts with memcmp filters on galaxy (offset 40) and system (offset 42).
   * Account layout after 8-byte discriminator:
   *   +0   owner:    Pubkey (32)
   *   +32  name:     [u8;32]
   *   +64  galaxy:   u16
   *   +66  system:   u16
   *   +68  position: u8
   */
  async scanGalaxy(galaxy: number, system: number, myOwner?: PublicKey): Promise<GalaxyEntry[]> {
    try {
      const gBuf = Buffer.alloc(2);
      gBuf.writeUInt16LE(galaxy, 0);
      const sBuf = Buffer.alloc(2);
      sBuf.writeUInt16LE(system, 0);

      // Offsets: 8 (disc) + 32 (owner) + 32 (name) = 72 for galaxy, 74 for system
      const filters: GetProgramAccountsFilter[] = [
        { memcmp: { offset: 8 + 32 + 32,     bytes: gBuf.toString("base64"), encoding: "base64" } },
        { memcmp: { offset: 8 + 32 + 32 + 2, bytes: sBuf.toString("base64"), encoding: "base64" } },
      ];

      const accounts = await this.connection.getProgramAccounts(COMPONENT_PLANET_ID, {
        filters,
        commitment: "confirmed",
      });

      const results: GalaxyEntry[] = [];
      for (const { pubkey, account } of accounts) {
        try {
          // Parse manually — Anchor decode may fail if discriminator differs
          const data = account.data;
          if (data.length < 77) continue;
          // Skip 8-byte discriminator
          const owner    = new PublicKey(data.slice(8, 40));
          const nameRaw  = data.slice(40, 72);
          const name     = Buffer.from(nameRaw).toString("utf8").replace(/\0/g, "").trim() || "Unknown";
          const galaxyV  = data.readUInt16LE(72);
          const systemV  = data.readUInt16LE(74);
          const positionV = data.readUInt8(76);
          const metalMine  = data.readUInt8(81); // offset: 77 diameter(4) + 79 temp(2) + 81 maxFields...
          // Simplified: just show what we have
          results.push({
            planetPda:   pubkey.toBase58(),
            owner:       owner.toBase58(),
            name,
            galaxy:      galaxyV,
            system:      systemV,
            position:    positionV,
            metalMine:   0,
            crystalMine: 0,
            isMe:        myOwner ? owner.equals(myOwner) : false,
          });
        } catch { /* skip bad accounts */ }
      }

      // Sort by position
      results.sort((a, b) => a.position - b.position);
      return results;
    } catch (e) {
      console.error("scanGalaxy:", e);
      return [];
    }
  }

  // ── Writes ───────────────────────────────────────────────────────────────────

  async startBuild(worldPda: PublicKey, entityPda: PublicKey, buildingIdx: number): Promise<string> {
    const now  = BigInt(Math.floor(Date.now() / 1000));
    const args = Buffer.alloc(10);
    args.writeUInt8(0, 0);            // instruction = start
    args.writeUInt8(buildingIdx, 1);  // which building
    args.writeBigInt64LE(now, 2);     // timestamp at offset 2 (matches Rust)
    const { transaction } = await ApplySystem({
      authority: this.provider.wallet.publicKey,
      systemId:  SYSTEM_BUILD_ID,
      world:     worldPda,
      entities: [{ entity: entityPda, components: [
        { componentId: COMPONENT_PLANET_ID },
        { componentId: COMPONENT_RESOURCES_ID },
      ]}],
      args: Array.from(args),
    });
    return this.provider.sendAndConfirm(transaction, [], { commitment: "confirmed" });
  }

  async finishBuild(worldPda: PublicKey, entityPda: PublicKey): Promise<string> {
    const now  = BigInt(Math.floor(Date.now() / 1000));
    const args = Buffer.alloc(10);
    args.writeUInt8(1, 0);   // instruction = finish
    args.writeUInt8(0, 1);   // ignored
    args.writeBigInt64LE(now, 2);
    const { transaction } = await ApplySystem({
      authority: this.provider.wallet.publicKey,
      systemId:  SYSTEM_BUILD_ID,
      world:     worldPda,
      entities: [{ entity: entityPda, components: [
        { componentId: COMPONENT_PLANET_ID },
        { componentId: COMPONENT_RESOURCES_ID },
      ]}],
      args: Array.from(args),
    });
    return this.provider.sendAndConfirm(transaction, [], { commitment: "confirmed" });
  }

  async settleProduction(worldPda: PublicKey, entityPda: PublicKey): Promise<string> {
    const now  = BigInt(Math.floor(Date.now() / 1000));
    const args = Buffer.alloc(8);
    args.writeBigInt64LE(now, 0);
    const { transaction } = await ApplySystem({
      authority: this.provider.wallet.publicKey,
      systemId:  SYSTEM_PRODUCE_ID,
      world:     worldPda,
      entities: [{ entity: entityPda, components: [
        { componentId: COMPONENT_RESOURCES_ID },
      ]}],
      args: Array.from(args),
    });
    return this.provider.sendAndConfirm(transaction, [], { commitment: "confirmed" });
  }

  /**
   * FIXED: arg buffer layout
   *   [0]     ship_type: u8
   *   [1..5]  quantity:  u32 LE
   *   [5..13] now:       i64 LE   ← was [2..10], overlapping quantity
   */
  async buildShip(worldPda: PublicKey, entityPda: PublicKey, shipType: number, quantity: number): Promise<string> {
    const now  = BigInt(Math.floor(Date.now() / 1000));
    const args = Buffer.alloc(13);
    args.writeUInt8(shipType, 0);
    args.writeUInt32LE(quantity, 1);
    args.writeBigInt64LE(now, 5);   // FIXED: was offset 2
    const { transaction } = await ApplySystem({
      authority: this.provider.wallet.publicKey,
      systemId:  SYSTEM_SHIPYARD_ID,
      world:     worldPda,
      entities: [{ entity: entityPda, components: [
        { componentId: COMPONENT_FLEET_ID },
        { componentId: COMPONENT_RESOURCES_ID },
      ]}],
      args: Array.from(args),
    });
    return this.provider.sendAndConfirm(transaction, [], { commitment: "confirmed" });
  }

  async launchFleet(params: {
    worldPda:        PublicKey;
    entityPda:       PublicKey;
    missionType:     number;
    ships:           Record<string, number>;
    cargoMetal?:     number;
    cargoCrystal?:   number;
    cargoDeuterium?: number;
    speedFactor?:    number;
    flightSeconds:   number;
  }): Promise<string> {
    const { worldPda, entityPda, missionType, ships, flightSeconds } = params;
    const speedFactor = params.speedFactor || 100;
    const now = BigInt(Math.floor(Date.now() / 1000));
    const args = Buffer.alloc(94);
    let o = 0;
    args.writeUInt8(missionType, o++);
    const order = ["lightFighter","heavyFighter","cruiser","battleship",
      "battlecruiser","bomber","destroyer","deathstar",
      "smallCargo","largeCargo","recycler","espionageProbe","colonyShip"];
    for (const k of order) { args.writeUInt32LE(ships[k] || 0, o); o += 4; }
    args.writeBigUInt64LE(BigInt(params.cargoMetal    || 0), o); o += 8;
    args.writeBigUInt64LE(BigInt(params.cargoCrystal  || 0), o); o += 8;
    args.writeBigUInt64LE(BigInt(params.cargoDeuterium|| 0), o); o += 8;
    args.writeUInt8(speedFactor, o++);
    args.writeBigInt64LE(now, o); o += 8;
    args.writeBigInt64LE(BigInt(flightSeconds), o);
    const { transaction } = await ApplySystem({
      authority: this.provider.wallet.publicKey,
      systemId:  SYSTEM_LAUNCH_ID,
      world:     worldPda,
      entities: [{ entity: entityPda, components: [
        { componentId: COMPONENT_FLEET_ID },
        { componentId: COMPONENT_RESOURCES_ID },
      ]}],
      args: Array.from(args),
    });
    return this.provider.sendAndConfirm(transaction, [], { commitment: "confirmed" });
  }
}

// ── Pure utility functions ─────────────────────────────────────────────────────

export function formatNum(n: number): string {
  if (n >= 1e9) return (n / 1e9).toFixed(2) + "B";
  if (n >= 1e6) return (n / 1e6).toFixed(2) + "M";
  if (n >= 1e3) return (n / 1e3).toFixed(1) + "K";
  return Math.floor(n).toLocaleString();
}

export function formatDuration(secs: number): string {
  secs = Math.max(0, Math.floor(secs));
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  return [h, m, s].map(v => String(v).padStart(2, "0")).join(":");
}

/** Client-side resource interpolation between chain reads */
export function pendingProduction(res: OnChainResources, nowSec: number) {
  if (res.lastUpdateTs === 0) {
    return { metal: res.metal, crystal: res.crystal, deuterium: res.deuterium };
  }
  const dt = Math.max(0, nowSec - res.lastUpdateTs);
  const eff = res.energyConsumption === 0
    ? 1
    : Math.min(1, res.energyProduction / res.energyConsumption);
  return {
    metal:     Math.min(res.metal     + Math.floor(res.metalHour     * dt / 3600 * eff), res.metalCap),
    crystal:   Math.min(res.crystal   + Math.floor(res.crystalHour   * dt / 3600 * eff), res.crystalCap),
    deuterium: Math.min(res.deuterium + Math.floor(res.deuteriumHour * dt / 3600 * eff), res.deuteriumCap),
  };
}

export function buildCost(idx: number, currentLevel: number) {
  const bases: [number,number,number][] = [
    [60,15,0],[48,24,0],[225,75,0],[75,30,0],[900,360,900],
    [400,120,200],[1_000_000,500_000,100_000],[400,200,100],
    [1000,0,0],[1000,500,0],[1000,1000,0],[200,400,200],[20,20,0],
  ];
  const [bm,bc,bd] = bases[idx] || [0,0,0];
  const mult = Math.pow(1.5, currentLevel);
  return { m: Math.round(bm*mult), c: Math.round(bc*mult), d: Math.round(bd*mult) };
}

export function buildSeconds(idx: number, level: number, robotics: number): number {
  const { m, c } = buildCost(idx, level - 1);
  return Math.max(1, Math.floor((m + c) / (2500 * (1 + robotics))));
}

export function galaxyDistance(g1:number,s1:number,p1:number,g2:number,s2:number,p2:number) {
  if (g1 !== g2) return 20_000 + Math.abs(g1-g2)*40_000;
  if (s1 !== s2) return 2_700  + Math.abs(s1-s2)*95;
  return 1_000 + Math.abs(p1-p2)*1_000;
}

export function estimateFlightSeconds(distance: number, baseSpeed: number, speedFactor: number) {
  const speed = baseSpeed * speedFactor / 100;
  if (speed <= 0) return 86400;
  return Math.max(10, Math.floor((35_000/100) * Math.sqrt((10*distance)/speed) + 10));
}
