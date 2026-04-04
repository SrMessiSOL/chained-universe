# Chained Universe

Chained Universe is a Solana + Anchor + BOLT game backend for a space strategy MMO. It models planets, resources, fleets, research, transport, colonization, sessions, and world registration as composable on-chain components and systems.

This repository contains the on-chain programs, world-management scripts, and test scaffolding used to run the game on devnet with MagicBlock/BOLT.

## Architecture

The project is organized around BOLT ECS-style components and systems.

### Components

- `component-planet`: persistent planet state, coordinates, owner, buildings, and build queue state
- `component-resources`: resource balances, production rates, storage caps, and last settlement timestamp
- `component-fleet`: ship counts plus up to 4 flat mission slots
- `component-investigation`: research levels and research queue state

### Systems

- `system-initialize`: initializes a new homeworld
- `system-produce`: settles and updates resource production
- `system-build`: starts and finishes building upgrades
- `system-research`: starts and finishes research
- `system-shipyard`: builds ships by consuming resources
- `system-launch`: launches transport and colonize missions
- `system-resolve-transport`: resolves transport arrivals/returns
- `system-resolve-colonize`: resolves the source fleet after colonization
- `system-initialize-new-colony`: initializes the newly created colony planet/resources
- `system-session`: session-oriented game flow for delegated accounts
- `system-registry`: wallet/planet/coordinate registry helpers

## Repository Layout

- [`programs`](/c:/solargrid-v9/solargrid2/solargrid/programs): all Anchor/BOLT on-chain programs
- [`scripts`](/c:/solargrid-v9/solargrid2/solargrid/scripts): world creation and system approval scripts
- [`tests`](/c:/solargrid-v9/solargrid2/solargrid/tests): validator fixtures and test support
- [`Anchor.toml`](/c:/solargrid-v9/solargrid2/solargrid/Anchor.toml): Anchor config
- [`Cargo.toml`](/c:/solargrid-v9/solargrid2/solargrid/Cargo.toml): Rust workspace config
- [`package.json`](/c:/solargrid-v9/solargrid2/solargrid/package.json): TS tooling and SDK dependencies

## Requirements

- Rust toolchain
- Solana CLI
- Anchor CLI `0.30.1`
- Node.js / npm
- a funded devnet wallet at `~/.config/solana/devnet.json`

Anchor is pinned in [`Anchor.toml`](/c:/solargrid-v9/solargrid2/solargrid/Anchor.toml) to:

- `anchor_version = "0.30.1"`

## Install

```bash
npm install
```

## Build

Build the full workspace:

```bash
anchor build
```

Build a single program:

```bash
anchor build --program-name system_launch --ignore-keys
```

## Local Development Notes

The provider cluster is currently set to `devnet` in [`Anchor.toml`](/c:/solargrid-v9/solargrid2/solargrid/Anchor.toml).

This project also includes validator/test configuration for the BOLT world program fixture.

## Create and Approve a World

Create a new world and approve the systems configured in the script:

```bash
npx ts-node scripts/create-world.ts
```

Approve systems for an existing world:

```bash
npx ts-node scripts/approve-systems.ts
```

Important:

- [`scripts/approve-systems.ts`](/c:/solargrid-v9/solargrid2/solargrid/scripts/approve-systems.ts) currently contains a hardcoded `WORLD_PDA`
- the frontend or client using this repo must point to the same world PDA

## Current Program IDs

- `componentPlanet`: `GSQbXfwxMWkW2bGASsKe4i8WupDPMRCLybZHRPJoXC6P`
- `componentResources`: `66QnMWuqE9B8vE9iSP1qWMk8R2yybci4NNJtdL3xiGjW`
- `componentFleet`: `CsHSUWnCL4rTi9WYcVRXyy2Sq9TgcH4Lr7WcZNViG5NY`
- `componentInvestigation`: `EC83xSy52aXakXJFqgXni5Ked9TSo8QmQff1pjtumTbG`
- `systemInitialize`: `GHBGdcof2e5tsPe2vP3zJYNxJscojY7J7gdRXCsgdpY9`
- `systemProduce`: `DNNJg4A1yirXgUN5cdJ4ozuG8zJVkmxB2AsWvTqVsbk4`
- `systemBuild`: `E94HChSfw57Px2BJPKLnoaj17v6NKN7vXnoQGLSpxUve`
- `systemResearch`: `CXwXVUeovhbpXGWpHk56SgrnH2DwoqoTSErgtrJghK5Z`
- `systemLaunch`: `BVn9NZ51LqhbDowqhaJvxmXK6VGsP1k3dLtJEL8Fjmxv`
- `systemResolveTransport`: `DkzcueEX3ca9haAmFoHKsW7JQVFxBfeZJX1VdHSdPnYP`
- `systemResolveColonize`: `AuYuVgjpX64Fea3zGtUaEHjoewwyWBeT8Srsh8EXFhGL`
- `systemInitializeNewColony`: `DapYcTdYUwB7qWhmqMGZU6V1vqS3NEagzt15fnWwfQMC`
- `systemShipyard`: `74wxuTRib19TzJyXNaeyPVcsFFFqBq8phtRSSPDsK2q2`
- `systemSession`: `BHRu4DADM4NsJvnvqY5znDUsrdvTrnkKyee9eYZ7Yd9G`
- `systemRegistry`: `BV6JwMdA9gLfG5ut2VBzbmQoJTXUu5umXErBqv4V3PJq`

## Gameplay Flow

1. Create a world.
2. Approve systems for that world.
3. Create a homeworld entity and initialize its components.
4. Produce resources and upgrade buildings.
5. Research technologies.
6. Build ships in the shipyard.
7. Launch transport or colonize missions.
8. Resolve missions and register new planets.

## Important Implementation Notes

- `component-fleet` stores missions as flat fields instead of nested structs
- colonize/transport missions use `target_galaxy`, `target_system`, `target_position`, and `colony_name`
- `system-initialize-new-colony` initializes only `planet` and `resources` so it stays under Solana return-data limits
- `InitializeComponent` is relied on to create default `fleet` and `investigation` accounts for fresh colonies

## Formatting

Run prettier checks:

```bash
npm run lint
```

Auto-fix formatting:

```bash
npm run lint:fix
```

## Troubleshooting

- If Borsh deserialization fails after changing a component layout, recreate world/entity state or migrate accounts.
- If a program upgrade fails with insufficient `ProgramData` space, extend the program first.
- If a system fails with `Return data too large`, reduce the number or size of components returned by that system.

## License

Add a license file if you plan to publish or share this project broadly.
