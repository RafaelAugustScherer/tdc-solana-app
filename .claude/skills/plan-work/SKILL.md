---
name: plan-work
description: Turn a feature request, bug, or work item into a documented, reviewed plan before writing any code. Use at the start of any non-trivial change — research prior art in the repo, fill the plan template, define numbered test scenarios and a Definition of Done, resolve open questions, and stop for review. Do not implement from this skill.
---

# Plan a work item

This is phase 1–3 of [`docs/workflow.md`](../../../docs/workflow.md): research →
documented plan → review. It ends **before** any code is written.

## Steps

1. **Scope it.** Restate the request in one or two sentences. If it's genuinely
   trivial (a typo, a version bump), say so and skip the plan.
2. **Research.** Read the code, specs, and existing plans it touches. Search for
   prior art — a helper, a pattern, a past decision — and prefer reusing it. For
   an unfamiliar library or approach, use the `researcher` agent; don't guess.
3. **Draft the plan.** Copy `docs/plans/TEMPLATE.md` to
   `docs/plans/NNNN-short-slug.md` (next number). Fill every section — especially
   **non-goals** (to hold scope) and numbered **test scenarios** (mapped to tests
   you'll write).
4. **List alternatives** you considered and why you rejected them.
5. **Surface open questions.** Any unresolved question blocks acceptance — flag
   it, don't paper over it.
6. **Stop for review.** A human or a fresh `reviewer` pass accepts the plan
   before implementation. Update the plan's status to **Accepted** once it is.

## Guardrails

- Write only under `docs/plans/`. No source changes in this phase.
- Keep it product-agnostic unless the work item itself defines the product.
- A good plan is small and specific enough that a reviewer finds problems here
  instead of in the diff.

Once accepted, continue with the **`ship-work`** skill.
