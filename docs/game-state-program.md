# Game-State Program

`game-state` is the unified gameplay owner program for Chained Universe.

It replaces the old split BOLT component/system model with one owner program
that can:

- own the persistent gameplay state directly
- support delegated execution on MagicBlock / ER
- explicitly commit delegated state back to base layer
- authorize gameplay either with the wallet or with a validated session signer

## Main Accounts

### `PlayerProfile`

Per-wallet profile account:

- `authority`
- `planet_count`
- `bump`

### `PlanetState`

Main per-planet gameplay account. It contains:

- identity and coordinates
- buildings and field usage
- local research state and research queue
- resources, production, energy, and caps
- stationed fleet
- active missions
- delegation/commit-compatible unified ownership

### `MissionState`

Embedded mission slot stored inside `PlanetState`:

- transport and colonize mission metadata
- timing
- fleet payload
- resource payload
- `applied` flag for outward-leg resolution

## Initialization Instructions

- `initialize_player`
- `initialize_homeworld`
- `initialize_colony`
- `initialize_planet`

## Gameplay Instructions

Wallet-authorized variants:

- `produce`
- `start_build`
- `finish_build`
- `start_research`
- `finish_research`
- `build_ship`
- `launch_fleet`
- `resolve_transport`
- `resolve_colonize`

Session-authorized ER variants:

- `produce_session`
- `start_build_session`
- `finish_build_session`
- `start_research_session`
- `finish_research_session`
- `build_ship_session`
- `launch_fleet_session`
- `resolve_transport_session`
- `resolve_colonize_session`

## Delegation and Commit Instructions

- `delegate`
- `commit_planet_state`
- `commit_two_planet_states`
- `commit_and_undelegate_planet_state`
- `commit_and_undelegate_two_planet_states`
- `process_undelegation`

## Session Authorization Model

The `*_session` instructions validate a GPL session token owned by the external
session program:

- session token owner must be `KeyspM2ssCJbqUhQ4k7sveSiY4WjnYsrXkC8oDbwde5`
- `session_token.authority` must match the `PlanetState.authority`
- `session_token.target_program` must equal `game-state`
- `session_token.session_signer` must match the provided signer
- expired session tokens are rejected

This enables burner/session signing without always requiring the wallet for
delegated gameplay.

## Delegation Model

`PlanetState` is delegated by the owner program itself. The backend verifies the
PDA seeds from on-chain `PlanetState` data before invoking owner-side
delegation, which is why the program now exposes a real `delegate` instruction.

## Why This Program Exists

The earlier architecture split gameplay state across several BOLT components:

- planet
- resources
- fleet
- investigation / research

That made deterministic explicit commit difficult because the accounts that
needed to be committed together did not share one owner program. `game-state`
fixes that by making one owner responsible for the whole gameplay surface.

## Recommended Developer Workflow

1. Build `game-state`
2. Regenerate and inspect `target/idl/game_state.json`
3. Update clients against the unified IDL
4. Prefer the `*_session` instructions for ER gameplay
5. Use commit instructions after delegated mutations
