# Unified-State Architecture Plan

This document sketches the recommended long-term replacement for the current
multi-owner BOLT component layout when you need explicit deterministic commits
while accounts remain delegated on MagicBlock.

## Why Change

The current model splits gameplay state across independently owned accounts:

- `component-planet`
- `component-resources`
- `component-fleet`
- `component-investigation`

That is a good fit for ECS-style composition, but it conflicts with explicit
MagicBlock commit instructions. A commit helper running from a separate program
like `system_session` cannot commit those accounts because it is not their
owner. This is the failure you observed from `ScheduleCommit`.

To support deterministic explicit commit without undelegating, the state that
must be committed together needs to be owned by the same program.

## Target Model

Move persisted gameplay state under a single owner program.

Recommended shape:

- one gameplay owner program, for example `programs/game-state`
- one primary PDA per planet/entity, for example `planet_state`
- optional secondary PDAs also owned by the same program if account size becomes
  too large

The key requirement is not "one account only". The key requirement is "one
owner program for all state that must commit together".

## Recommended Account Layout

Start with one PDA per planet:

```rust
#[account]
pub struct PlanetState {
    pub creator: Pubkey,
    pub entity: Pubkey,
    pub owner: Pubkey,
    pub name: [u8; 32],

    pub galaxy: u16,
    pub system: u16,
    pub position: u8,
    pub planet_index: u32,
    pub diameter: u32,
    pub temperature: i16,
    pub max_fields: u16,
    pub used_fields: u16,

    pub metal_mine: u8,
    pub crystal_mine: u8,
    pub deuterium_synthesizer: u8,
    pub solar_plant: u8,
    pub fusion_reactor: u8,
    pub robotics_factory: u8,
    pub nanite_factory: u8,
    pub shipyard: u8,
    pub metal_storage: u8,
    pub crystal_storage: u8,
    pub deuterium_tank: u8,
    pub research_lab: u8,
    pub missile_silo: u8,

    pub build_queue_item: u8,
    pub build_queue_target: u8,
    pub build_finish_ts: i64,

    pub metal: u64,
    pub crystal: u64,
    pub deuterium: u64,
    pub metal_hour: u64,
    pub crystal_hour: u64,
    pub deuterium_hour: u64,
    pub energy_production: u64,
    pub energy_consumption: u64,
    pub metal_cap: u64,
    pub crystal_cap: u64,
    pub deuterium_cap: u64,
    pub last_update_ts: i64,

    pub energy_tech: u8,
    pub combustion_drive: u8,
    pub impulse_drive: u8,
    pub hyperspace_drive: u8,
    pub computer_tech: u8,
    pub astrophysics: u8,
    pub igr_network: u8,
    pub research_queue_item: u8,
    pub research_queue_target: u8,
    pub research_finish_ts: i64,

    pub small_cargo: u32,
    pub large_cargo: u32,
    pub light_fighter: u32,
    pub heavy_fighter: u32,
    pub cruiser: u32,
    pub battleship: u32,
    pub battlecruiser: u32,
    pub bomber: u32,
    pub destroyer: u32,
    pub deathstar: u32,
    pub recycler: u32,
    pub espionage_probe: u32,
    pub colony_ship: u32,
    pub solar_satellite: u32,
    pub active_missions: u8,
    pub missions: [MissionSlot; 4],
}
```

If this becomes too large, split it into 2-3 PDAs, but keep the same owner:

- `planet_core`
- `planet_research`
- `planet_fleet`

That still allows one owner program to commit all of them in one explicit
instruction.

## Program Layout

Recommended new programs:

- `programs/game-state`
  - owns all unified state PDAs
  - contains delegation and explicit commit instructions
- `programs/gameplay`
  - optional separation if you still want logic split out
  - can mutate the same owned accounts via CPI or direct instructions

The simpler version is one single program that owns state and also executes all
gameplay instructions.

## Instruction Mapping

Current BOLT systems map cleanly to normal Anchor instructions.

### Initialize

Current:

- `system-initialize`
- `InitializeComponent` for 4 components

Unified-state replacement:

- `initialize_planet(name, optional_coords)`
- creates and initializes the single `planet_state` PDA

### Produce

Current:

- `system-produce`

Unified-state replacement:

- `produce(now_ts)`
- settles resources on the unified state account

### Build

Current:

- `system-build` mutates `planet + resources`

Unified-state replacement:

- `start_build(building_idx, now_ts)`
- `finish_build(now_ts)`

Both mutate one unified state PDA, so explicit commit is trivial.

### Research

Current:

- `system-research` mutates `planet + resources + investigation`

Unified-state replacement:

- `start_research(tech_idx, now_ts)`
- `finish_research(now_ts)`

### Shipyard

Current:

- `system-shipyard` mutates `fleet + resources + investigation`

Unified-state replacement:

- `build_ship(ship_type, quantity, now_ts)`

### Launch

Current:

- `system-launch` mutates `fleet + resources`

Unified-state replacement:

- `launch_fleet(args...)`

### Resolve Transport

Current:

- `system-resolve-transport` mutates source fleet and destination resources

Unified-state replacement:

- `resolve_transport(source_planet_state, destination_planet_state, slot, now_ts)`

Both accounts share one owner program, so one instruction can mutate and commit
them together.

### Resolve Colonize

Current:

- create new entity
- initialize colony accounts
- resolve source fleet
- register planet

Unified-state replacement:

- `initialize_colony(...)`
- `resolve_colonize(source_planet_state, new_planet_state, slot, now_ts)`

Or one combined colonize finalization instruction if preferred.

## Commit Model

This is the main benefit of the refactor.

With one owner program, your explicit ER commit helper becomes valid:

```rust
#[ephemeral]
#[program]
pub mod game_state {
    use super::*;

    pub fn commit_selected(ctx: Context<CommitSelected>, mask: u8) -> Result<()> {
        let mut accounts = Vec::new();

        if mask & COMMIT_CORE != 0 {
            accounts.push(&ctx.accounts.planet_state.to_account_info());
        }
        if mask & COMMIT_FLEET != 0 {
            accounts.push(&ctx.accounts.planet_fleet.to_account_info());
        }

        Ok(commit_accounts(
            &ctx.accounts.payer.to_account_info(),
            accounts,
            &ctx.accounts.magic_context.to_account_info(),
            &ctx.accounts.magic_program.to_account_info(),
        )?)
    }
}
```

Now the commit helper is the owner of every account it commits, so the
`InvalidAccountOwner` failure goes away.

## Frontend Impact

The frontend becomes simpler, not harder.

Current:

- derive 4 different component PDAs
- choose different component masks
- load ER state from several accounts
- deserialize 4 account types

Unified-state:

- derive 1 planet-state PDA or a small fixed set under one owner program
- one loader
- one commit helper
- one account shape per planet

Example:

```ts
await client.startBuild(planetStatePda, buildingIdx);
await client.commitPlanetState(planetStatePda);
```

Multi-planet actions stay simple too:

```ts
await client.resolveTransport(sourcePlanetState, destinationPlanetState, slot);
await client.commitTransportState(sourcePlanetState, destinationPlanetState);
```

## Migration Strategy

Do this in phases.

### Phase 1

Create the new unified program without deleting the current BOLT programs.

- add `programs/game-state`
- implement `initialize_planet`
- implement account serialization/deserialization
- implement explicit `commit_selected`

### Phase 2

Port the easiest systems first:

- produce
- build
- research
- shipyard

These are the best first targets because they are single-planet flows.

### Phase 3

Port fleet actions:

- launch
- resolve transport
- resolve colonize

### Phase 4

Switch frontend reads to unified-state accounts.

### Phase 5

Stop writing new gameplay state into the old BOLT components.

Keep registry/account discovery if useful, but point it at unified state.

## What You Keep

You do not need to throw away everything.

You can keep:

- world creation scripts
- registry program
- coordinate registration flow
- client-side formulas and UI concepts
- most of the gameplay logic itself

What changes is where that logic writes state.

## Cost of the Refactor

This is the highest-effort option.

Expected work:

- new account schema
- new instructions
- new IDL and client bindings
- deserializer changes
- possibly migration tooling or world reset

But this is also the only option that cleanly satisfies all of these at once:

- deterministic explicit commit
- staying delegated
- multi-account atomicity under one program owner
- clean frontend integration

## Recommendation

If this game is meant to keep growing, move to unified state.

If you only need a temporary way to persist progress with minimal refactor,
`commit_and_undelegate` is the faster workaround, but it will keep fighting the
UX you want.
