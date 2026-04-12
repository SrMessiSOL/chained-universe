# Migration History

This document records the transition from the original BOLT ECS layout to the
current `game-state` + `market` architecture used by Chained Universe /
SolarGrid.

## Original Architecture

The project started with multiple BOLT-owned component and system programs:

- `component-planet`
- `component-resources`
- `component-fleet`
- `component-investigation`
- several `system-*` programs for produce, build, research, launch, transport,
  colonization, registry, and session commit support

That design was modular, but gameplay state that logically belonged together
was physically split across multiple owners.

## What Broke Down

As delegated execution on MagicBlock / ER became more important, several issues
showed up:

1. Explicit owner-side commits became difficult when state was spread across
   multiple owner programs.
2. The old helper-style commit flow conflicted with the owner requirements of
   delegated accounts.
3. Delegation support for the new unified account model had to be reintroduced
   explicitly.
4. Gameplay instructions were still too wallet-heavy for a strategy game with
   lots of repeated actions.
5. Economy features were hard to reason about cleanly while gameplay state was
   still being consolidated.

## Step 1: Unified State

The first major change was introducing `programs/game-state`, which folds the
important gameplay fields into one owner-program account model:

- player profile
- planet state
- resources
- local research
- stationed fleet
- missions

This solved the ownership problem for explicit commits and established one
program as the source of truth for gameplay progression.

## Step 2: Owner-Program Delegation

The new program initially lacked the backend-side delegate entrypoint expected
by the ER delegation flow.

That was fixed by:

- adding a real `delegate` instruction to `game-state`
- verifying the `PlanetState` PDA seeds from on-chain account data
- delegating from the owner program instead of relying on the old BOLT setup

This enabled `PlanetState` to participate correctly in delegated execution.

## Step 3: Runtime Fixes During Delegation Rollout

During delegation rollout, backend and runtime issues were fixed, including:

- a Rust lifetime issue while deserializing the delegated PDA
- an account borrow conflict caused by holding borrowed data during a CPI
- account-shape and seed validation cleanup needed for the unified PDA model

These were resolved by narrowing borrow scope, deserializing directly from raw
data where needed, and tightening seed/account validation around the owner-side
delegation path.

## Step 4: Lower-Friction Authorization

Delegation alone did not remove wallet friction, because gameplay instructions
still needed repeated user signatures.

The authorization model then evolved toward lower-friction gameplay through:

- wallet-authorized setup and recovery paths
- vault-authorized gameplay paths tied to `AuthorizedVault`
- encrypted vault backup support
- dedicated game config for the global ANTIMATTER mint

This gave the product a more practical path for real game usage without
requiring constant wallet popups.

## Step 5: ANTIMATTER Utility

With the core state model in place, ANTIMATTER was introduced as a first-class
game economy asset:

- queue acceleration for buildings
- queue acceleration for research
- queue acceleration for ship production

This added a premium acceleration surface directly into `game-state` instead of
layering it on top of fragmented component logic.

## Step 6: Dedicated Market Program

Once the unified gameplay state was stable enough, a separate `programs/market`
was introduced for player-to-player trading.

The current marketplace model is:

- offers are created against a seller-selected planet
- the listed resources are locked on offer creation through `game-state`
- cancelling the offer refunds those locked resources
- accepting the offer moves ANTIMATTER from buyer to seller, burns the market
  fee, and credits the resources to the buyer planet

This split keeps gameplay state authoritative in `game-state` while allowing
the market to focus on listing, pricing, payment, and settlement orchestration.

## Current Result

The active architecture is now:

- `game-state` as the gameplay source of truth
- `market` as the dedicated player economy surface
- unified IDLs for both active programs
- explicit support for delegated execution and commit flow
- lower-friction vault-based gameplay authorization
- ANTIMATTER integrated into both acceleration and marketplace payment flows

The old BOLT/component architecture is kept only as historical context or local
archive material, not as the active production shape.

## Guidance For New Developers

- treat `game-state` as the source of truth for progression and per-planet state
- treat `market` as a settlement layer that must stay aligned with the deployed
  `game-state` program ID
- do not build new features on top of the old component/system layout
- read `README.md` first, then `docs/game-state-program.md`
- use `docs/unified-state-architecture.md` for the design rationale behind the
  refactor
