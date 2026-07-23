# Sub-agents

The roster for this project. The default flow follows the
[workflow](../../docs/workflow.md): **planner → researcher (as needed) →
implementer → reviewer**.

## Project agents

| Agent | Use it to… | Writes code? |
|-------|------------|--------------|
| [`planner`](planner.md) | Turn a request into a documented, reviewed plan under `docs/plans/`. | No (plans only) |
| [`researcher`](researcher.md) | Compare libraries/approaches and recommend one, before a plan commits. | No |
| [`implementer`](implementer.md) | Build an **accepted** plan: code, tests, build, lint. | Yes |
| [`reviewer`](reviewer.md) | Check a plan or diff against the guidelines; report findings. | No |

## When to reach past this roster

- **Deep Solana work** — defer to the `solana-ai-kit` specialists instead of the
  generic `implementer`: `anchor-engineer` (programs), `solana-frontend-engineer`
  (wallet/UX), `solana-qa-engineer` (test infra, CU), `defi-engineer`,
  `token-engineer`, `solana-architect`. They carry ecosystem-specific expertise;
  they still follow this repo's workflow and guidelines.
- **Broad codebase search** — the built-in `Explore` agent.
- **Open-ended investigation** — the built-in `general-purpose` agent.

## solana-ai-kit commands vs. these agents

The kit ships `/plan-feature` and `/diff-review`, which overlap `planner` and
`reviewer`. They don't replace the gate — they feed it:

- `/plan-feature` is a Solana-specific spec aid (account/PDA design). Use it to
  *fill* the documented plan the `planner` owns in `docs/plans/`; the plan doc and
  its acceptance are still the gate.
- `/diff-review` and `/audit-solana` are the mechanism the `reviewer` gate runs.

Full phase → command map: [`../../docs/solana-toolchain.md`](../../docs/solana-toolchain.md).

## How they fit the rules

The split exists to enforce **plan-first**: the `planner` cannot write code, and
the `implementer` refuses to start without an accepted plan. Keep that boundary —
it's the cheapest place to catch a wrong direction.
