# Project constitution

This repository holds a full-stack Solana application that targets **devnet**.
The specific product scope is defined per work item in [`docs/plans/`](docs/plans/),
not here — keep this file about *how* we build, not *what* we build.

The global rules in the user's `~/.claude/CLAUDE.md` also apply. This file adds
the project-specific workflow, standards, and roster.

## The one rule: plan first, then build

No non-trivial change starts in code. The order is always:

1. **Research** the problem and prior art in this repo.
2. **Write a plan** — a numbered doc under [`docs/plans/`](docs/plans/), from
   [the template](docs/plans/TEMPLATE.md).
3. **Get the plan reviewed** and accepted.
4. **Implement** it, matching existing patterns.
5. **Verify** against the plan's Definition of Done, then open a PR.

*Why:* a cheap redirect on a plan beats a wrong-shape rewrite. The plan is also
the durable record of *why* — source comments never carry that (see below).

The full lifecycle lives in [`docs/workflow.md`](docs/workflow.md). Two skills
drive it: **`plan-work`** (steps 1–3) and **`ship-work`** (steps 4–5).

## Quality bar

Full spec: [`docs/engineering-guidelines.md`](docs/engineering-guidelines.md).
The non-negotiables:

- **Match the surrounding code.** Reuse existing helpers, names, and structure
  before inventing new ones.
- **No inline or explanatory comments.** Names and small functions explain
  *what*; the plan and PR explain *why*. The only allowed comments are markers
  that announce their own removal: `TODO`, `FIXME`, `HACK`, `XXX`.
- **Solve the case in front of you.** No speculative options, generic
  interfaces, or "what if we need X later" branches.
- **Small, well-named functions over clever abstraction.**
- **Tests are part of the change**, not a follow-up.
- **Vet dependencies before adding them** — current version, known advisories,
  and that the API you call still exists.
- **Never commit secrets or keypairs.**

## Stack & layout

- `app/` — Anchor v1 workspace (`yarn`, `prettier`).
  - `app/programs/app/` — the on-chain program (Rust).
  - `app/migrations/`, `app/tests` — deploy + TypeScript integration tests.
- Docker-based dev loop (no local Rust/Solana/Anchor needed) — see
  [`README.md`](README.md).
- `docs/` — specs, workflow, and plans.
- `.claude/` — sub-agents, skills, and shared settings.

### Commands

```bash
docker compose up                 # hot-reload: rebuilds on program source changes
docker compose exec dev bash      # shell into the running container
```

Inside the container: `anchor build`, `anchor test`, `anchor deploy`. Lint the
TypeScript with `yarn lint` / `yarn lint:fix` from `app/`. Deploy to devnet with
`anchor deploy --provider.cluster devnet` (needs a funded devnet wallet).

## Sub-agents

Roster and when to use each: [`.claude/agents/README.md`](.claude/agents/README.md).
Default flow: **planner** → **researcher** (as needed) → **implementer** →
**reviewer**. For deep Solana work, defer to the `solana-ai-kit` specialists
(e.g. `anchor-engineer`, `solana-frontend-engineer`, `solana-qa-engineer`).

## Commits & PRs

- Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`).
- One PR maps to one accepted plan. Keep it small and reviewable.
- The PR description links its plan and states how the Definition of Done was met.
