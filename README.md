# Chained Universe

Chained Universe is a Solana + Anchor backend for a space strategy MMO. The
repository now centers on a single gameplay owner program,
[`programs/game-state`](programs/game-state), which replaces the older split
BOLT component/system architecture.

## Current Architecture

The active on-chain program is:

- `game-state`: unified planet, economy, fleet, research, mission, delegation,
  commit, and session-authorized gameplay logic

Core concepts in the current model:

- `PlayerProfile` tracks wallet ownership and colony count
- `PlanetState` is the main per-planet gameplay PDA
- gameplay supports both wallet-authorized instructions and `*_session`
  instructions for burner/session execution on ER
- explicit commit helpers are built into the owner program, so delegated state
  can be committed without relying on the old multi-program BOLT flow

## Repository Layout

- [`programs/game-state`](programs/game-state):
  unified gameplay program
- [`docs/game-state-program.md`](docs/game-state-program.md):
  program reference
- [`docs/unified-state-architecture.md`](docs/unified-state-architecture.md):
  technical design rationale
- [`docs/migration-history.md`](docs/migration-history.md):
  migration story from BOLT components to unified state
- [`Anchor.toml`](Anchor.toml):
  streamlined Anchor config
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

## Deploy

Example devnet flow:

```bash
anchor deploy --program-name game_state
```

After deploying, regenerate and distribute the IDL from
[`target/idl/game_state.json`](target/idl/game_state.json).

## Gameplay Surface

The current instruction set covers:

- player initialization
- homeworld and colony creation
- resource settlement
- build queue start/finish
- research queue start/finish
- ship construction
- fleet launch
- transport resolution
- colonize resolution
- delegation
- explicit commit / commit-and-undelegate flows
- session-authorized gameplay variants for ER execution

## Session and Delegation Notes

- `delegate` enables owner-program delegation for `PlanetState`
- `commit_planet_state` and `commit_two_planet_states` commit delegated state
- `*_session` instructions validate an external GPL session token so gameplay
  can be authorized by a burner/session signer instead of the wallet

## Migration Context

This repository used to be organized around many BOLT ECS components and
systems. That design was useful early on, but it made explicit owner-side
commits difficult once gameplay moved to delegated execution. The unified
`game-state` program was introduced to solve that by bringing all gameplay
state that must commit together under one owner program.

For the technical reasoning and migration path:

- read [`docs/unified-state-architecture.md`](docs/unified-state-architecture.md)
- read [`docs/migration-history.md`](docs/migration-history.md)

## New Developer Onboarding

Start here:

1. Read this `README`
2. Read [`docs/game-state-program.md`](docs/game-state-program.md)
3. Read [`docs/migration-history.md`](docs/migration-history.md)
4. Build the program and inspect the generated IDL
5. Update clients against `game_state.json`, not the old component/system IDLs
