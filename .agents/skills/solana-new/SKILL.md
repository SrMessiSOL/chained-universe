---
name: solana-new
description: Solana idea-to-launch workflow for founders and builders. Use when Codex needs to help with Solana project ideation, MVP scoping, Anchor or Solana build plans, launch readiness, pitch decks, investor messaging, competitive framing, or marketing assets in the style of the Solana.new setup flow.
---

# Solana New

Use this skill as a lightweight operating mode for Solana founder work: move from idea to launch with a bias toward concrete output.

## Core workflow

1. Start from the current phase:
- `idea`: clarify the wedge, target user, why-now, and what should be built first
- `build`: define the MVP, architecture, contracts, client flows, and execution plan
- `launch`: tighten positioning, investor narrative, pitch materials, demo framing, and marketing assets

2. Default to shipping artifacts, not just advice:
- product thesis
- MVP scope
- technical plan
- investor pitch
- deck outline
- demo narrative
- marketing video script

3. Keep Solana-specific recommendations practical:
- prefer fast feedback loops
- favor clear ownership and account models
- call out signer, PDA, token, and rent implications explicitly
- avoid speculative token design unless the user asks for it

## Phase guidance

### Idea

- Define the product in one sharp sentence.
- Identify the retention loop or core repeated behavior.
- Explain why Solana improves the product instead of being incidental.
- Reduce the first version to the smallest believable MVP.

### Build

- Prefer Anchor for on-chain programs unless the repo already chose otherwise.
- Break the system into: accounts, instructions, assets, users, and flows.
- For games, separate:
  - player state
  - economy or resources
  - progression
  - market or settlement
- Flag mismatches between deployed program IDs, PDA seeds, signer assumptions, and client constants early.

### Launch

- Write investor materials with a clear market gap, technical edge, execution proof, and ask.
- Write demo scripts around what already works today, not hypothetical future features.
- For marketing assets, produce:
  - hook
  - narrative arc
  - scene list
  - voiceover
  - CTA

## Output patterns

### Investor pitch

- Open with the category claim.
- State the market gap.
- Explain the technical or product advantage.
- Prove execution with what exists now.
- End with roadmap and opportunity.

### Demo pitch

- Show the product loop in plain language.
- Focus on what a user does, not internal implementation.
- Keep under 90 seconds unless asked otherwise.

### Build planning

- Give concrete milestones.
- Name the risky parts first.
- Prefer implementation sequencing over abstract architecture talk.

## References

- Read [references/solana-new-reference.md](references/solana-new-reference.md) when you need the exact Solana.new-style framing, modes, and output formats.
