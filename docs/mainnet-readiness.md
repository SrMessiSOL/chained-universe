# GAMESOL Mainnet Readiness

GAMESOL is the game. ANTIMATTER is the token used by the game economy.

## Current devnet programs

- `game_state`: `FJGxh6SKgNoTVzHj98oBsC2oaEy8ovadVJf8rDUNaEHb`
- `market`: `Dow7f1UqLGKyvs1D2uNR5c6bmAdnKRy2ZDtnsa4UhApp`

These are devnet deployment IDs. Mainnet should use fresh mainnet program
keypairs unless the team intentionally reuses pre-generated IDs.

## Mainnet constants to confirm before deploy

- `programs/game-state/src/lib.rs`: mainnet `game_state` `declare_id!`
- `programs/market/src/lib.rs`: mainnet `market` `declare_id!`
- `programs/game-state/src/constants.rs`: `MARKET_PROGRAM_ID`
- `programs/market/src/constants.rs`: `GAME_STATE_PROGRAM_ID`
- `programs/game-state/src/constants.rs`: `PROTOCOL_AUTHORITY`
- `programs/game-state/src/constants.rs`: `PROTOCOL_ANTIMATTER_MINT`
- `programs/game-state/src/constants.rs`: `STORE_USDC_MINT`

The frontend mainnet build must use the same values through:

- `VITE_GAME_STATE_PROGRAM_ID`
- `VITE_MARKET_PROGRAM_ID`
- `VITE_ANTIMATTER_MINT`
- `VITE_SOLANA_CLUSTER=mainnet`
- `VITE_SOLANA_RPC_ENDPOINT`

## Required checks

Run these before any mainnet deploy:

```powershell
cargo check --workspace --jobs 1
npm.cmd run build
```

Run the Anchor/SBF build with the repo toolchain version (`0.30.1`) and do not
ship while there are unresolved stack-frame diagnostics in game instructions.

```bash
anchor build --program-name game_state --ignore-keys
anchor build --program-name market --ignore-keys
```

## Mainnet deploy sequence

1. Create or choose final mainnet program keypairs.
2. Patch the two `declare_id!` values and cross-program constants.
3. Build both programs with Anchor `0.30.1`.
4. Deploy `game_state` and `market` to mainnet.
5. Initialize `GameConfig` with the final ANTIMATTER mint.
6. Initialize `MarketConfig` with the same ANTIMATTER mint.
7. Build frontend using `.env.mainnet.example` values filled in.
8. Deploy frontend production.
9. Verify the live bundle contains only the mainnet program IDs.

Do not transfer upgrade authority until manual gameplay smoke tests pass.
