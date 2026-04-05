# Migration History

This document records the transition from the original BOLT ECS layout to the
unified `game-state` program.

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
4. Even after delegation worked, gameplay instructions still required the
   wallet signer because authorization had not yet moved to a session model.

## Step 1: Unified State

The first major change was introducing `programs/game-state`, which folds the
important gameplay fields into one owner-program account model:

- player profile
- planet state
- resources
- local research
- stationed fleet
- missions

This solved the ownership problem for explicit commits.

## Step 2: Owner-Program Delegation

The new program initially lacked the backend-side delegate entrypoint expected
by the ER delegation flow.

That was fixed by:

- adding a real `delegate` instruction to `game-state`
- verifying the `PlanetState` PDA seeds from on-chain account data
- delegating from the owner program instead of relying on the old BOLT setup

This enabled `PlanetState` to participate correctly in delegated execution.

## Step 3: Runtime Fixes

During delegation rollout, two backend issues were fixed:

- a Rust lifetime issue while deserializing the delegated PDA
- an account borrow conflict caused by holding a borrowed data reference during
  the delegation CPI

These were resolved by:

- deserializing `PlanetState` directly from raw account data
- narrowing the borrow scope before invoking the CPI

## Step 4: Session Authorization

Delegation alone did not remove wallet popups, because gameplay instructions
still required `authority: Signer`.

The next change was adding session-authorized instruction variants:

- `*_session` gameplay instructions
- external GPL session token validation
- checks that the token authority, target program, signer, and expiry all match

This is what makes burner/session-driven ER gameplay possible in the unified
model.

## Result

The repo is now being cleaned up around one gameplay owner program:

- `game-state`
- unified IDL
- delegation built into the owner program
- explicit commit support
- session-authorized gameplay path

The old BOLT/component architecture is kept only as historical context or local
archive material, not as the active production shape.

## Guidance For New Developers

- treat `game-state` as the source of truth
- do not build new features on top of the old component/system layout
- read `README.md` first, then `docs/game-state-program.md`
- use `docs/unified-state-architecture.md` for the design rationale behind the
  refactor
