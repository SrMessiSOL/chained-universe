import { PublicKey } from "@solana/web3.js";

// ── Program IDs ───────────────────────────────────────────────────────────────
export const WORLD_PROGRAM_ID       = new PublicKey("WorLD15A7CrDwLcLy4fRqtaTb9fbd8o8iqiEMUDse2n");
export const COMPONENT_PLANET_ID    = new PublicKey("4AAQeP54KQy4HSjMsMS9VwVY8mWy4BisdsTwSxen4Df6");
export const COMPONENT_RESOURCES_ID = new PublicKey("CP6KoShdHvgZbGubYLct1EcQLmngZ1nsWmaKQhbJRtss");
export const COMPONENT_FLEET_ID     = new PublicKey("5UuCSuNqVXwCd7qPFQXj8Kp7DAqbB5ZuHFLZZ32paPLD");
export const SYSTEM_INITIALIZE_ID   = new PublicKey("HNiJvBSjSXm5WPmPd2TG8LD1q8sXqSF46qe3YL1vXapC");
export const SYSTEM_PRODUCE_ID      = new PublicKey("EkNaTMh1N29W6PCXDGnvh7mVzcrA1pMS3uz2xKWRUZRH");
export const SYSTEM_BUILD_ID        = new PublicKey("kk7e2mNXHaU3VVtmtzLCZGYP88MDL7EbkFbb9sySfiV");
export const SYSTEM_LAUNCH_ID       = new PublicKey("9aHGFS8VAfbEYYCkEGQBBuTKApkD5aiHotH77kMgB5bT");
export const SYSTEM_SHIPYARD_ID     = new PublicKey("FTav8UK4RKawqyGWRakZhe1zhYV7PUJgPwHK7UnEqnN9");
export const SYSTEM_ATTACK_ID       = new PublicKey("8qbBLEdrN6qC1fFJQLM7a6Jqf2xfoDNfSmTQopMELSGm");

// Set via .env — the one shared world PDA that all players join
// Create it once with the seed script: scripts/create-world.ts
export const SHARED_WORLD_PDA_STR: string =
  (import.meta as any).env?.VITE_SHARED_WORLD_PDA || "";

// ── Building definitions ──────────────────────────────────────────────────────
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

// ── Ship definitions ──────────────────────────────────────────────────────────
export const SHIPS = [
  { key: "smallCargo",     name: "Small Cargo",      icon: "📦", atk: 5,       cargo: 5_000,   cost: { m: 2000,    c: 2000,    d: 0       } },
  { key: "largeCargo",     name: "Large Cargo",      icon: "🚛", atk: 5,       cargo: 25_000,  cost: { m: 6000,    c: 6000,    d: 0       } },
  { key: "lightFighter",   name: "Light Fighter",    icon: "✈",  atk: 50,      cargo: 50,      cost: { m: 3000,    c: 1000,    d: 0       } },
  { key: "heavyFighter",   name: "Heavy Fighter",    icon: "⚡",  atk: 150,     cargo: 100,     cost: { m: 6000,    c: 4000,    d: 0       } },
  { key: "cruiser",        name: "Cruiser",          icon: "🛸", atk: 400,     cargo: 800,     cost: { m: 20000,   c: 7000,    d: 2000    } },
  { key: "battleship",     name: "Battleship",       icon: "⚔",  atk: 1000,    cargo: 1500,    cost: { m: 45000,   c: 15000,   d: 0       } },
  { key: "battlecruiser",  name: "Battlecruiser",    icon: "🔱", atk: 700,     cargo: 750,     cost: { m: 30000,   c: 40000,   d: 15000   } },
  { key: "bomber",         name: "Bomber",           icon: "💣", atk: 1000,    cargo: 500,     cost: { m: 50000,   c: 25000,   d: 15000   } },
  { key: "destroyer",      name: "Destroyer",        icon: "💥", atk: 2000,    cargo: 2000,    cost: { m: 60000,   c: 50000,   d: 15000   } },
  { key: "deathstar",      name: "Deathstar",        icon: "🌑", atk: 200000,  cargo: 1000000, cost: { m: 5000000, c: 4000000, d: 1000000 } },
  { key: "recycler",       name: "Recycler",         icon: "♻",  atk: 1,       cargo: 20_000,  cost: { m: 10000,   c: 6000,    d: 2000    } },
  { key: "espionageProbe", name: "Espionage Probe",  icon: "👁",  atk: 0,       cargo: 0,       cost: { m: 0,       c: 1000,    d: 0       } },
  { key: "colonyShip",     name: "Colony Ship",      icon: "🌍", atk: 50,      cargo: 7500,    cost: { m: 10000,   c: 20000,   d: 10000   } },
  { key: "solarSatellite", name: "Solar Satellite",  icon: "🛰",  atk: 1,       cargo: 0,       cost: { m: 0,       c: 2000,    d: 500     } },
] as const;

export const SHIP_TYPES: Record<string, number> = {
  smallCargo: 0, largeCargo: 1, lightFighter: 2, heavyFighter: 3,
  cruiser: 4, battleship: 5, battlecruiser: 6, bomber: 7,
  destroyer: 8, deathstar: 9, recycler: 10, espionageProbe: 11,
  colonyShip: 12, solarSatellite: 13,
};
