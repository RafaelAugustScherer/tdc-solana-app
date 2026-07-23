# Development workflow

Every non-trivial change moves through five phases in order. Do not skip ahead —
the point is to be redirected on a cheap plan, not an expensive diff.

```
research → plan (documented) → review → implement → verify → PR
```

Trivial changes (a typo, a version bump, a one-line fix) skip the plan; use
judgment, and when in doubt, write the plan.

## 1. Research

Understand the problem and what already exists here before proposing anything.

- Read the relevant code, existing plans, and specs.
- Search for prior art in the repo — a helper, a pattern, a past decision.
- When the change depends on a library or approach you're unsure about, hand it
  to the **researcher** agent: it returns options with trade-offs and a
  recommendation. Don't ship the first thing that works.

## 2. Plan (documented)

Write the plan as a numbered doc in [`docs/plans/`](plans/), copied from
[`TEMPLATE.md`](plans/TEMPLATE.md). Name it `NNNN-short-slug.md`.

A plan is complete when it states:

- the problem and the goals (and explicit **non-goals**),
- the chosen approach and the alternatives you rejected (and why),
- the files/areas it will touch,
- numbered **test scenarios**,
- risks and open questions,
- a **Definition of Done**.

The **`plan-work`** skill walks through this. The **planner** agent can produce
the plan for you; it stops here and does not write code.

## 3. Review

The plan is a gate, not a formality. It must be read and accepted — by a human,
or by a fresh **reviewer** pass — before implementation starts. Resolve every
open question first; an unanswered question is a reason to pause, not to guess.

## 4. Implement

Build exactly what the accepted plan describes.

- Match existing patterns; reuse helpers; follow
  [`engineering-guidelines.md`](engineering-guidelines.md).
- **No inline comments.** Names and small functions carry the meaning.
- Write the tests from the plan's scenarios alongside the code.
- If reality diverges from the plan, update the plan first, then keep going —
  the plan and the code never drift apart.

The **`ship-work`** skill drives this. The **implementer** agent does the work
against an already-accepted plan.

## 5. Verify → PR

- Run build, tests, and lint; all green.
- Check every item in the plan's Definition of Done.
- Run a **reviewer** pass over the diff (clean code, no comments, correctness,
  tests, Solana security).
- Open a PR that links its plan and states how the Definition of Done was met.

## Definition of Done (baseline)

A change is done when **all** of these hold:

- [ ] It does what its plan said, and nothing it said it wouldn't.
- [ ] Tests for the plan's scenarios exist and pass.
- [ ] Build and lint are green (`anchor build`, `cargo clippy`, `yarn lint`).
- [ ] No inline comments; no secrets or artifacts committed.
- [ ] The plan doc is current and marked implemented.
- [ ] The PR links the plan.
