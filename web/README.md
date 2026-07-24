# web

A Vite + React + TypeScript client for the subscription program in
[`../app`](../app). Runs on the host — unlike `app/`, it needs no Solana/Anchor
toolchain, so it doesn't run inside the Docker dev container.

See the repo root [`README.md`](../README.md) for how the two dev loops fit
together, and [`docs/plans/0006-web-frontend.md`](../docs/plans/0006-web-frontend.md)
for the design.

## Commands

```bash
yarn dev          # start the app (defaults to the local validator, see .env.example)
yarn codegen      # regenerate src/generated from app/target/idl/app.json
yarn build        # type-check and build
yarn lint         # check formatting and lint
yarn test:e2e     # reset the local validator, redeploy, run the Playwright suite
```

`test:e2e` requires the Docker dev container (`docker compose up -d` from the
repo root) — its `pretest:e2e` hook resets and redeploys to a fresh local
validator inside it (`scripts/reset-local-validator.sh`) before every run, so
the suite is safe to rerun repeatedly.

Copy `.env.example` to `.env` and adjust `VITE_RPC_URL` / `VITE_CHAIN` to point
at a devnet RPC instead of the local validator.
