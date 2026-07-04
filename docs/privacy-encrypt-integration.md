# GAMESOL Privacy Migration Plan

GAMESOL is the game. ANTIMATTER is the token. The current privacy migration is
client-encrypted private state on Solana devnet, with an on-chain request and
resolver boundary for espionage. Encrypt/FHE remains a future engine option once
its security model is ready for production gameplay.

## Current Boundary

- `game-state` owns public planet discovery only: owner, name, coordinates, and
  planet index.
- `private-state` owns hidden planet interior metadata: encrypted snapshot,
  snapshot nonce, state hash, encryption key hash, category commitments, report
  nonce, and privacy-engine seal.
- New private states use `PRIVACY_ENGINE_CLIENT_AES_GCM`.
- The current on-chain maximum encrypted private snapshot is 1536 bytes.
- The current on-chain maximum encrypted spy report is 1024 bytes.
- Future Encrypt-backed states should use `PRIVACY_ENGINE_ENCRYPT_FHE` and a
  non-default FHE cluster identity only after the engine is production ready.

## Seal Fields

- `privacy_engine`: client AES-GCM now, Encrypt FHE later.
- `ciphertext_schema`: fixed encrypted state layout version.
- `fhe_cluster`: zeroed for the current client-encrypted engine, Encrypt cluster
  identity later.
- `decrypt_policy_hash`: hash of the game rules that decide what may be
  decrypted or re-encrypted.

## First Private Prototype

The first private action is espionage because it has a small output and tests
the privacy boundary without migrating the whole economy.

Inputs:

- encrypted target resources/buildings/research/fleet/defense summary
- attacker espionage probe count
- attacker spy/computer/research modifiers
- target defense/research modifiers
- reveal policy and target epoch

Outputs:

- updated encrypted target state, if probes are destroyed or counters change
- encrypted spy report for the spy authority
- public report commitment and ciphertext hash
- reveal level, target epoch, and nonce

## Implemented Request Boundary

`private-state` now has a two-step spy report flow:

1. `request_spy_report`
   - creates a `SpyReportRequest`
   - pins target planet, spy authority, resolver, target epoch, nonce,
     reveal cap, encrypted input hash, and request commitment
   - increments the target planet report nonce
2. `publish_spy_report`
   - requires the stored resolver to sign
   - requires the original spy authority and target planet
   - closes the request account
   - stores encrypted report bytes, report nonce, report commitment, encrypted
     report hash, and public metadata

This is the hook where a production resolver, TEE, or Encrypt callback authority
can replace the dev resolver. Until that authority model is finalized, the app
should treat this as devnet-ready privacy plumbing rather than a final mainnet
privacy guarantee.

## Dev Resolver

For local/devnet testing before Encrypt callbacks are available, use:

```powershell
$env:SOLANA_RPC_URL="https://api.devnet.solana.com"
$env:SOLANA_KEYPAIR="C:\path\to\resolver-keypair.json"
node scripts\dev-resolve-private-spy-report.cjs
```

The frontend currently sets the resolver to the spy wallet for dev requests, so
the script only resolves requests where the local keypair is both resolver and
spy authority. This is intentional: it avoids adding a hidden server authority
while proving the on-chain request-to-report flow.

Optional:

```powershell
$env:SPY_REQUEST="<request account pubkey>"
node scripts\dev-resolve-private-spy-report.cjs
```

The script publishes deterministic encrypted report payloads for devnet testing.
It does not decrypt or reveal private planet state.

Rules:

- Never decrypt the full target state publicly.
- Never require the target owner to be online for a spy report.
- Keep public callback output small enough for Solana transaction limits.
- Store encrypted payloads, hashes, commitments, and report metadata on chain.

## Migration Order

1. Keep current public-only planet shell.
2. Freeze the compact encrypted state schema.
3. Deploy `private-state` client-encrypted snapshots and encrypted spy reports
   on devnet.
4. Move build/research/resource ticks into private transitions.
5. Move fleet launches, transport, and attacks into private transitions.
6. Replace the dev resolver with the production privacy authority.
7. Switch to Encrypt/FHE only after the engine has real production secrecy.
