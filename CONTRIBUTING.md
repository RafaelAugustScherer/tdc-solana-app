# Contributing

Thanks for building here. This project has one hard rule: **plan first, then
build.** No non-trivial change starts in code.

## The flow

1. **Plan.** Write a numbered plan under [`docs/plans/`](docs/plans/) from the
   [template](docs/plans/TEMPLATE.md). State the goals, non-goals, approach,
   test scenarios, and a Definition of Done.
2. **Review.** The plan is accepted before implementation begins. Resolve every
   open question first.
3. **Build.** Implement exactly what the plan describes, matching existing
   patterns. Follow [`docs/engineering-guidelines.md`](docs/engineering-guidelines.md).
4. **Verify.** Build, tests, and lint green; every Definition-of-Done item met.
5. **PR.** One PR per plan; link the plan in the description.

The full lifecycle is in [`docs/workflow.md`](docs/workflow.md). If you're using
Claude Code, the `plan-work` and `ship-work` skills drive steps 1–3 and 4–5.

## Non-negotiables

- **No inline or explanatory comments** — only `TODO`/`FIXME`/`HACK`/`XXX`
  markers that announce their own removal.
- **Match the surrounding code.** Reuse helpers; don't add near-duplicates.
- **Tests ship with the change**, mapped to the plan's scenarios.
- **Vet dependencies** before adding them.
- **Never commit secrets or keypairs.**

## Running it locally

```bash
docker compose up
```

See [`README.md`](README.md) for the full dev loop, shell access, and toolchain
checks.
