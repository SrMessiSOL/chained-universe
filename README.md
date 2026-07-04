# Chained Universe Backend

Solana + Anchor backend for GAMESOL, an on-chain space strategy game.

The active backend is centered on two programs:

- `game_state`: owns player profiles, planets, resources, buildings, research,
  fleets, missions, quests, store purchases, alliances, vault authorization,
  and ANTIMATTER-powered acceleration.
- `market`: handles player-to-player resource and planet listings, settles
  purchases against `game_state`, and routes ANTIMATTER market fees to the
  protocol treasury.

## Current Architecture

Core accounts:

- `PlayerProfile`: per-wallet player profile and planet count.
- `PlanetState`: per-planet gameplay source of truth.
- `PlanetCoordinates`: public coordinate registry for galaxy/system/position.
- `AuthorizedVault`: wallet-approved signer used for routine gameplay actions.
- `VaultBackup`: encrypted vault recovery data.
- `QuestState` and `QuestProgressState`: tutorial and recurring quest state.
- `StoreConfig` and `StorePurchaseState`: USDC store config and period limits.
- `AllianceState`, `AllianceMembership`, `AllianceTreasuryState`: alliance
  membership, deposits, buildings, and shared progression.
- `MarketOffer` and `PlanetListing`: resource and planet market listings.

## Repository Layout

- `programs/game-state`: gameplay program
- `programs/market`: market program
- `docs/game-state-program.md`: gameplay program reference
- `docs/mainnet-readiness.md`: production readiness checklist
- `scripts/init-devnet-configs.cjs`: devnet config initialization helper
- `Anchor.toml`: Anchor workspace config
- `Cargo.toml`: Rust workspace config

## Requirements

- Rust toolchain
- Solana CLI
- Anchor CLI `0.30.1`

## Build

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

Example devnet deploy flow:

```bash
anchor deploy --program-name game_state
anchor deploy --program-name market
```

If Anchor builds the `.so` but IDL generation fails, the compiled program can
still be upgraded with `solana program deploy` using the configured upgrade
authority.

After deploying:

- verify on-chain program IDs match `Anchor.toml`
- verify frontend constants match the deployed program IDs and token mints
- initialize or update game, store, and market config accounts as needed
- run a manual smoke test through the production frontend

## Economy Notes

- ANTIMATTER is used for acceleration, market payments, alliance costs, and
  selected economy actions.
- USDC store purchases route to the configured treasury account.
- Market fees and alliance ANTIMATTER deposits route to the protocol
  ANTIMATTER treasury.
- Player resources are game-state balances, not SPL tokens.

## Development Notes

- `cargo check --workspace` is the quick local verification pass.
- The frontend lives in a separate repository:

```text
https://github.com/SrMessiSOL/chained-universe-frontend
```

- Keep backend, frontend, deployed program binaries, and GitHub branches aligned
  before production testing.
