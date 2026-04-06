# Game-State Program

`game-state` is the unified gameplay owner program for Chained Universe.

It replaces the old split BOLT component/system model with one owner program
that can:

- own the persistent gameplay state directly
- support delegated execution on MagicBlock / ER
- explicitly commit delegated state back to base layer
- authorize gameplay with the wallet, a validated session signer, or a
  game-managed burner authorization

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

### `AuthorizedBurner`

Per-planet burner authorization account stored as a PDA:

- `authority`
- `burner`
- `planet`
- `expires_at`
- `revoked`
- `bump`

### `PlanetVault`

Per-planet SOL vault stored as a PDA:

- `authority`
- `planet`
- `bump`

This account is intended to be the long-lived, program-owned commit-payer
identity for a planet. It can hold SOL, be delegated by the owner program, and
does not require any off-chain key recovery.

### `BurnerBackup`

Per-wallet, per-planet encrypted burner backup stored as a PDA:

- `authority`
- `planet`
- `burner`
- `version`
- `ciphertext`
- `iv`
- `salt`
- `kdf_salt`
- `updated_at`
- `bump`

The program stores only opaque encrypted bytes. Encryption, decryption, and key
derivation happen entirely client-side.

## Initialization Instructions

- `initialize_player`
- `initialize_homeworld`
- `initialize_colony`
- `initialize_planet`
- `initialize_planet_vault`

## Burner Authorization Instructions

- `register_burner`
- `revoke_burner`
- `extend_burner`

## Burner Backup Instructions

- `upsert_burner_backup`
- `delete_burner_backup`

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

Burner-authorized ER variants:

- `produce_burner`
- `start_build_burner`
- `finish_build_burner`
- `start_research_burner`
- `finish_research_burner`
- `build_ship_burner`
- `launch_fleet_burner`
- `resolve_transport_burner`
- `resolve_colonize_burner`

## Delegation and Commit Instructions

- `delegate`
- `delegate_planet_vault`
- `withdraw_planet_vault`
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

## Burner Authorization Model

The `*_burner` instructions validate a game-owned `AuthorizedBurner` PDA:

- the PDA is derived from `planet` and `burner`
- `authority` must match the owning wallet for the planet
- `burner` must match the provided burner signer
- `planet` must match the target `PlanetState`
- revoked burner records are rejected
- expired burner records are rejected unless `expires_at == 0`

This gives the game program its own burner authorization path without relying
on the external GPL session token lifecycle.

## Burner Backup Model

`BurnerBackup` exists only to help clients recover a burner signer on a new
device without using a Web2 backend:

- the encrypted burner blob is stored on-chain
- the program never sees the plaintext burner secret
- the `planet` field is used as a wallet-scoped recovery namespace key; it does
  not by itself prove that the supplied pubkey is currently a live delegated
  `PlanetState` owned by the authority
- clients should derive the encryption key locally from wallet identity
  material plus a user-chosen PIN/passphrase
- the reconstructed burner pubkey should match the stored `burner` field before
  use

## Planet Vault Model

`PlanetVault` splits commit-payment custody away from the recoverable gameplay
burner:

- the vault is a game-state-owned PDA derived from `planet`
- the wallet can initialize it even after delegation because the program
  verifies delegated planet identity by deserializing `PlanetState` data and
  recomputing its PDA
- the vault can be funded by transferring SOL to its PDA address
- `withdraw_planet_vault` lets the wallet recover excess SOL while preserving
  rent exemption
- the burner keypair remains the recoverable ER gameplay signer; the vault does
  not need secret-key recovery

The current SDK commit helpers in this repo still use an external signer-style
`payer`, so `PlanetVault` establishes the correct on-chain payer identity and
delegation lifecycle first. Rewiring the actual commit call path to PDA signing
is a follow-up integration step.

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
4. Prefer the `*_burner` or `*_session` instructions for ER gameplay
5. Use commit instructions after delegated mutations
