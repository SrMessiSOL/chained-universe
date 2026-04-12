# Chained Universe / SolarGrid

Chained Universe is a Solana + Anchor backend for a space strategy MMO. The project has moved away from the older split BOLT ECS model and now centers on a unified gameplay owner program plus a dedicated player market program.

## Current Architecture

The active on-chain programs are:

- `game-state`: unified gameplay owner for planets, resources, fleets,
  research, missions, authorization, commit flow, and ANTIMATTER-powered queue
  acceleration
- `market`: player-to-player resource marketplace that settles against
  `game-state` and uses ANTIMATTER as the payment asset

Core concepts in the current model:

- `PlayerProfile` tracks wallet ownership and colony count
- `PlanetState` is the main per-planet gameplay PDA and source of truth for
  progression
- `AuthorizedVault` lets gameplay run through a wallet-approved vault signer
  without requiring constant wallet popups
- `GameConfig` stores the global ANTIMATTER mint used for queue acceleration
- `MarketOffer` stores resource listings that lock resources at offer creation,
  refund them on cancellation, and deliver them on purchase

## Product Surface

The current gameplay and economy flow covers:

- player initialization
- homeworld and colony creation
- resource settlement
- building queues
- research queues
- ship construction
- fleet launch
- transport resolution
- colonization resolution
- player-authorized vault flow for lower-friction gameplay
- delegated execution and explicit commit support
- ANTIMATTER acceleration for building, research, and ship production
- peer-to-peer resource trading through the marketplace

## Repository Layout

- [`programs/game-state`](programs/game-state):
  unified gameplay program
- [`programs/market`](programs/market):
  marketplace program
- [`docs/game-state-program.md`](docs/game-state-program.md):
  game-state program reference
- [`docs/unified-state-architecture.md`](docs/unified-state-architecture.md):
  technical design rationale for the unified owner model
- [`docs/migration-history.md`](docs/migration-history.md):
  migration story from BOLT components to the current architecture
- [`Anchor.toml`](Anchor.toml):
  Anchor workspace config
- [`Cargo.toml`](Cargo.toml):
  Rust workspace config

## Requirements

- Rust toolchain
- Solana CLI
- Anchor CLI `0.30.1`

## Build

Build the workspace:

```bash
anchor build
```

Build only the gameplay program:

```bash
anchor build --program-name game_state
```

Build only the market program:

```bash
anchor build --program-name market
```

## Deploy

Example deploy flow:

```bash
anchor deploy --program-name game_state
anchor deploy --program-name market
```

After deploying:

- regenerate and distribute [`target/idl/game_state.json`](target/idl/game_state.json)
- regenerate and distribute [`target/idl/market.json`](target/idl/market.json)
- keep client constants and deployed program IDs aligned across both programs

## Gameplay Notes

- `game-state` is the source of truth for player progression and per-planet
  state
- the vault-signed path is the main low-friction gameplay path in the current
  architecture
- explicit commit helpers remain built into the owner program for delegated
  execution support
- ANTIMATTER is used as both a queue-acceleration asset and the market payment
  asset

## Market Notes

- sellers create offers against a selected planet
- resource amounts are locked on the seller planet when an offer is created
- cancelling an offer refunds the locked resources to the seller planet
- accepting an offer transfers ANTIMATTER from buyer to seller, burns the
  configured fee, and credits the purchased resources to the buyer planet
- the market program is intentionally coupled to the deployed `game-state`
  program ID, so redeploys must keep those constants synchronized

## Migration Context

This repository originally used many BOLT ECS components and systems. That
model worked early on, but it made explicit owner-side commits difficult once
gameplay moved toward delegated execution and more complex multi-surface state.
The unified `game-state` program solved that by bringing the gameplay state
that must commit together under a single owner, and the `market` program was
added later as a dedicated economy surface that settles directly against that
state.

For the technical reasoning and migration path:

- read [`docs/unified-state-architecture.md`](docs/unified-state-architecture.md)
- read [`docs/migration-history.md`](docs/migration-history.md)

## New Developer Onboarding

Start here:

1. Read this `README`
2. Read [`docs/game-state-program.md`](docs/game-state-program.md)
3. Read [`docs/migration-history.md`](docs/migration-history.md)
4. Build the workspace and inspect the generated IDLs
5. Update clients against `game_state.json` and `market.json`, not the old
   component/system IDLs
