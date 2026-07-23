---
name: ship-work
description: Implement an already-accepted plan end to end — build to match existing patterns with no inline comments, write the tests from the plan's scenarios, run build and lint, self-review against the engineering guidelines, and prepare a PR. Use only once a plan in docs/plans/ is Accepted; if none is, switch to plan-work first.
---

# Ship an accepted plan

This is phase 4–5 of [`docs/workflow.md`](../../../docs/workflow.md): implement →
verify → PR.

## Precondition

An **Accepted** plan exists in `docs/plans/`. If not, stop and run **`plan-work`**
first — implementation never leads.

## Steps

1. **Re-read the plan** and the code it touches. Confirm the approach still holds;
   if reality has moved, update the plan before writing code.
2. **Implement it** with the `implementer` agent (or the relevant `solana-ai-kit`
   specialist for deep Solana work). Match existing patterns, reuse helpers, keep
   functions small and straight-line.
3. **No inline comments.** Only removal-markers (`TODO`/`FIXME`/`HACK`/`XXX`).
   Names and small functions carry the meaning.
4. **Write the tests** from the plan's numbered scenarios alongside the code.
5. **Verify.** Run `anchor build`, `cargo clippy`, the test suite, and
   `yarn lint`. All green.
6. **Self-review.** Run a `reviewer` pass over the diff — clean code, no comments,
   correctness, Solana security, tests. Fix what it finds.
7. **Close the loop.** Tick every item in the plan's Definition of Done, set the
   plan's status to **Implemented**, and update the plans index.
8. **Prepare the PR.** Conventional-commit title, one PR per plan, description
   linking the plan and stating how the Definition of Done was met.

## Guardrails

- Build only what the plan describes; no scope creep. A separate issue found in
  passing is a new plan, not a bundled fix.
- Never commit secrets, keypairs, or build artifacts.
