# 0001 — Development harness

- **Status:** Implemented
- **Author:** project setup
- **Related:** first PR (`chore/dev-harness`)

## Context

The repository had an Anchor + Docker scaffold but no shared rules for *how* work
gets done: no documented workflow, no engineering standards, no agent roster.
Before feature work begins, the project needs a harness so every contributor —
human or agent — plans, builds, and reviews the same way. This plan documents
that harness and, by existing, models the plan-first workflow it establishes.

## Goals

- A single constitution (`CLAUDE.md`) that points to everything else.
- A written engineering standard, including a strict no-inline-comment policy.
- A documented plan-first workflow with a template and a home for plans.
- A small roster of project sub-agents and two workflow skills.
- Product-agnostic throughout — the harness says nothing about what the app does.

## Non-goals

- Defining the product, its features, or its architecture. That lives in later
  plans.
- Enforcing the workflow with blocking automation (hooks). Enforcement is by
  `CLAUDE.md`, the skills, and PR review.
- Publishing the repo or opening the PR (done by a human once the remote exists).

## Approach

Add documentation and configuration only, in the shapes the repo already uses:

- `CLAUDE.md` — the always-loaded index and the plan-first rule.
- `docs/engineering-guidelines.md`, `docs/workflow.md` — the standard and the
  lifecycle.
- `docs/plans/` — `README.md`, `TEMPLATE.md`, and this plan.
- `.claude/agents/` — `planner`, `researcher`, `implementer`, `reviewer`, and a
  roster `README.md`.
- `.claude/skills/` — `plan-work` and `ship-work`.
- `.claude/settings.json` — a conservative permission allowlist for routine dev
  commands.
- `CONTRIBUTING.md` and a README "Development" section as public-repo entry points.

Packaged as a first PR: the existing scaffold is the baseline commit on `main`;
the harness lands on `chore/dev-harness`.

## Alternatives considered

- **First-party skills under `.agents/` + symlink (matching the vendored
  colosseum-copilot skill).** Rejected: `.agents/` and `skills-lock.json` are for
  externally sourced skills; our own skills live directly under `.claude/skills/`
  so a skills sync can't clobber them.
- **A hook that blocks edits until a plan exists.** Rejected for now: brittle and
  intrusive. Chosen enforcement is process + docs + review.

## Affected areas

New files under the repo root, `docs/`, and `.claude/`. One edit to `README.md`.
No changes to `app/` or the program.

## Test scenarios

Documentation and config, so verification is structural rather than unit-tested:

1. `CLAUDE.md` links resolve to files that exist.
2. Each `.claude/agents/*.md` has valid frontmatter and is discoverable as an
   agent type.
3. Each `.claude/skills/*/SKILL.md` has a name and a triggering description.
4. `.claude/settings.json` is valid JSON.
5. No file names or references the product idea.

## Risks & open questions

- **Risk:** the vendored colosseum-copilot skill is included in the baseline and
  will be public. Mitigation: flagged for the owner to remove before publishing
  if unwanted.
- No open questions.

## Definition of Done

- [x] Constitution, guidelines, and workflow written.
- [x] Plan template and this plan in place.
- [x] Four sub-agents and two skills defined and documented.
- [x] Settings valid; entry-point docs added.
- [x] Product-agnostic throughout.
