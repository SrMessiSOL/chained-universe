# GAMESOL Encrypt Integration Plan

Encrypt is the preferred target for the real private-state engine if its FHE
mainnet/alpha security model matures as described. The current pre-alpha docs
state that developer data is not really encrypted yet, so production gameplay
must not depend on it for secrecy today.

## Current Boundary

- `game-state` owns public planet discovery only: owner, name, coordinates, and
  planet index.
- `private-state` owns hidden planet interior metadata: state hash, encrypted
  state hash, category commitments, report nonce, and privacy-engine seal.
- New private states default to `PRIVACY_ENGINE_COMMITMENT_ONLY`.
- Future Encrypt-backed states must use `PRIVACY_ENGINE_ENCRYPT_FHE` and a
  non-default FHE cluster identity.

## Seal Fields

- `privacy_engine`: commitment-only now, Encrypt FHE later.
- `ciphertext_schema`: fixed encrypted state layout version.
- `fhe_cluster`: Encrypt cluster identity used for ciphertext execution.
- `decrypt_policy_hash`: hash of the game rules that decide what may be
  decrypted or re-encrypted.

## First Encrypt Prototype

The first FHE circuit should be espionage because it has a small output and
tests the privacy model without migrating the whole economy.

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
   - stores only report hash/commitment and public metadata

This is the hook where an Encrypt callback authority should replace the dev
resolver. Until Encrypt has real encryption guarantees, the app should treat
this as an integration boundary rather than production privacy.

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

The script publishes deterministic mock encrypted report hashes. It does not
decrypt or reveal private planet state.

Rules:

- Never decrypt the full target state publicly.
- Never require the target owner to be online for a spy report.
- Keep public callback output small enough for Solana transaction limits.
- Store only hashes, commitments, report metadata, and encrypted payload
  references on chain.

## Migration Order

1. Keep current public-only planet shell.
2. Freeze the fixed encrypted state schema.
3. Build espionage as the first Encrypt-backed action.
4. Move build/research/resource ticks into encrypted transitions.
5. Move fleet launches, transport, and attacks into encrypted transitions.
6. Disable commitment-only states for production.
