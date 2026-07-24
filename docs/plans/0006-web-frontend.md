# 0006 — Web front-end for the subscription program

- **Status:** Implemented (devnet deploy still outstanding — see Risks)
- **Author:** Rafael Scherer
- **Related:** drives [0002](0002-subscription-plans.md), [0003](0003-delegation-and-charging.md),
  [0004](0004-subscriber-spending-caps.md), [0005](0005-variable-pricing.md) — the on-chain
  program this client talks to. No program changes.

## Context

The Anchor program (`app/programs/app`) implements a full subscription lifecycle —
plans, delegated recurring charges, subscriber spending caps, variable pricing —
and is fully covered by Rust integration tests (`cargo test`). It has **no
client at all**: no TS test harness, no web app. The only way to exercise it
today is by hand-building `@solana/kit`/Anchor calls. This plan adds a web
front-end so a merchant and a subscriber wallet can walk the whole lifecycle
against a running validator.

## Goals

- Merchant: create a plan (fixed or variable price), toggle it active, and
  (variable plans only) update its price.
- Subscriber: browse active plans, subscribe (set allowance + max amount per
  period), adjust allowance/max amount, reauthorize the token delegation, and
  cancel.
- Anyone: trigger a due charge — `charge` takes no signer in the program, so
  this is a demo action, not a role-gated one.
- Show live on-chain state after every action (plan price/active, allowance
  remaining, next charge time, delegation committed total) so the effect of
  each instruction is visible.
- Run against the project's existing local validator (already published via
  `docker-compose.yml` ports `8899`/`8900` "for a host frontend") during
  development, and against devnet for the actual workshop demo. **Not fully
  met:** the devnet half of this goal is blocked by external infra, not by the
  frontend — see Risks.

## Non-goals

- No production concerns: KYC, rate limiting, invoicing, mainnet deployment.
- No automated keeper/cron service to call `charge` on a schedule — the "demo
  charge" button is a manual stand-in; a real merchant's keeper is out of scope.
- No backend/indexer for plan discovery — the browser queries
  `getProgramAccounts` directly.
- No mobile wallet adapter / React Native support — this is a web app.
- No visual design system — functional and clear, not polished.

## Approach

**Stack** (see research below): Vite + React + TypeScript, `@solana/kit` for
all RPC/transaction work, `@solana/kit-plugin-wallet` + `@solana/react` for
wallet connection, and `codama` + `@codama/cli` + `@codama/nodes-from-anchor`
to generate a Kit-compatible TS client from the Anchor IDL.

- **Vite over Next.js:** every write is a wallet-signed instruction and the
  only network dependency is an RPC endpoint — nothing here needs SSR or API
  routes, and a plain SPA is simpler to teach than Next's App Router surface.
  Solana's own frontend docs cover both a Next.js path
  (`solana.com/docs/frontend/nextjs-solana`) and a framework-agnostic React
  path (`solana.com/docs/frontend/react-hooks`); this plan follows the latter.
  Judgment call, not a hard requirement.
- **`@solana/kit-plugin-wallet` over `@solana/wallet-adapter-react`:**
  superseding the earlier decision in this plan, made before checking Solana's
  current official docs. Both `nextjs-solana` and `react-hooks` — the current
  official frontend guides — build wallet connection on `@solana/kit-plugin-wallet`
  (hooks: `useWallets`, `useWalletStatus`, `useConnectedWallet`,
  `useConnect`/`useDisconnect`, from `@solana/kit-plugin-wallet/react`) plus
  `@solana/react` for data hooks (`useRequest`, `useSubscription`,
  `useTrackedData`). A `walletSigner` plugin makes the connected wallet the
  fee payer and transaction signer directly against the `@solana/kit` client
  (`createClient().use(walletSigner({ chain })).use(solanaRpc({ rpcUrl }))`,
  published once via a `ClientProvider` at the app root) — no bridge module
  needed, since the wallet is kit-native from the start. This also drops the
  earlier `@wallet-ui/react` vs. `wallet-adapter-react` trade-off: the docs'
  own path doesn't route through either. Exact package versions (docs
  reference "Kit 7+, `@solana/kit-plugin-*` 0.13+, `@solana/react` 7+") to be
  confirmed against npm at implementation time per this repo's dependency
  rule, since a fetched doc page is not a substitute for checking the
  registry directly.
- **Codama over `@anchor-lang/core`'s generated client:** Anchor's own TS
  client only supports classic web3.js v1 and is explicitly documented as
  incompatible with `@solana/kit`, which the repo's engineering guidelines
  already mandate for TypeScript. Codama is Solana's documented bridge from an
  Anchor IDL to a Kit-compatible client, and composes directly with the
  `@solana/kit` client the wallet plugin publishes above.
- Since `app/target/` is gitignored (build output), commit the **generated
  Codama client**, not the raw IDL, under `web/src/generated/`. Regeneration is
  two steps across the toolchain boundary: `docker compose exec dev anchor
  build` (inside the container, where the Solana/Anchor toolchain actually
  lives) refreshes `app/target/idl/app.json`; a host-run `yarn codegen` in
  `web/` then runs Codama against that IDL. Document both steps — a plain host
  `anchor build` will not work on this project (no local Solana/Anchor install
  is assumed; see `README.md`). Re-running this pair after any program change
  is manual for now — no CI drift-check is in scope for this plan (flagged as
  a risk below).

**Layout:** new `web/` directory at the repo root, sibling to `app/` — it is
not part of the Anchor workspace. Own `package.json`; same Prettier/yarn
conventions as `app/`.

**Flows** (single-page app, wallet-gated views):

1. **Connect wallet** — a small connect UI built from `@solana/kit-plugin-wallet/react`'s
   `useWallets`/`useConnect`/`useConnectedWallet` hooks (Wallet Standard
   discovery — no extra config needed for Phantom/Solflare/Backpack); RPC
   endpoint switchable between the local validator and devnet via env var
   (below).
2. **Merchant dashboard** — plans owned by the connected wallet
   (`getProgramAccounts` filtered on `Plan.merchant`); create-plan form
   (plan_id, amount, period, price mode); per-plan toggle-active and
   update-price (variable only, shows the *currently applicable* price —
   `previous_amount` while a pending change hasn't taken effect yet, else
   `amount_per_period` — alongside the pending price and its
   `PRICE_CHANGE_NOTICE_SECONDS` effective-at time). `create_plan` bundles an
   idempotent ATA-create for the merchant's own token account ahead of it,
   the same way `subscribe` does for the subscriber (below) — `charge`'s
   `merchant_token_account` also has no `init_if_needed`, so without this the
   first charge against a plan for a mint the merchant has never held would
   fail with no recovery path in the UI.
3. **Plan browser** (subscriber) — active plans across all merchants
   (`getProgramAccounts` filtered on `is_active`), showing the same
   currently-applicable-price logic as the merchant dashboard; subscribe form
   (allowance, max amount per period). `Subscribe`'s `subscriber_token_account`
   has no `init_if_needed` (by design — see 0003) and fails if the ATA doesn't
   already exist, so the frontend bundles an idempotent
   `createAssociatedTokenAccount` instruction ahead of `subscribe` in the same
   transaction whenever the subscriber has no ATA for the plan's mint yet.
4. **My subscriptions** (subscriber) — the connected wallet's subscriptions
   with live allowance/next-charge/max-amount; actions: set allowance, set max
   amount, reauthorize, cancel.
5. **Trigger charge** — shown once a subscription's `next_charge_at` has
   elapsed; callable by any connected wallet (it only pays the tx fee — the
   instruction itself has no signer requirement).

State reads use `@solana/react`'s `useRequest`/`useSubscription` hooks against
the same published client (matching the official React-hooks pattern) rather
than hand-rolled fetch logic, refreshed after each transaction confirms — per
the engineering guidelines' "handle the states a chain forces on you"
guidance (explicit confirmation waits, not silent optimistic updates). Every
write action's `useAction` result also surfaces its `error` through a shared
`ActionError` component, so a failed transaction (insufficient funds, a
program error, a rejected signature) is visible rather than silently leaving
the button re-enabled with no feedback — the same guidance requires this, not
just the confirmation-wait half of it.

**TypeScript:** `strict` mode is on across all three of `web/`'s tsconfigs
(`tsconfig.app.json`, `tsconfig.node.json`, and `tsconfig.tests.json`, the
last covering `tests/` and `playwright.config.ts`, which the app/node configs
don't include) per the engineering guidelines.

**RPC config:** `web/.env` with `VITE_RPC_URL` and `VITE_CHAIN` (the Wallet
Standard chain identifier used for wallet discovery, independent of the RPC
target), defaulting to the local validator (`http://127.0.0.1:8899`, matching
the ports the existing `docker-compose.yml` already exposes) for day-to-day
development, switched to a devnet endpoint for the workshop demo.
`web/.env.example` documents both. No separate `VITE_PROGRAM_ID` is needed —
the program address is fixed in the generated Codama client (from the IDL's
`declare_id!`), not something this frontend switches between.

**Devnet deploy:** this program has never been deployed to devnet in this
repo's history, and still hasn't been as of this plan's implementation —
see Risks for why (an external infra block, not a design gap) and the
Definition of Done for the concrete unblock path.

**Testing:** E2E tests (`web/tests/`, `@playwright/test`, run via
`yarn test:e2e`) drive the real UI end to end against a local validator, with
no on-chain mocking. Two Wallet Standard mock wallets are injected per test
(`tests/fixtures/mockWallet.ts`, one named "Merchant Mock Wallet" and one
"Subscriber Mock Wallet") rather than one wallet playing both roles — the
program's `Charge` instruction rejects a transaction where
`merchant_token_account` and `subscriber_token_account` resolve to the same
account (`ConstraintDuplicateMutableAccount`), which a same-wallet test setup
hits immediately. Each mock wallet is a real Ed25519 keypair (Node's
`crypto.generateKeyPairSync`) signing through the browser's native
`crypto.subtle` — no browser extension involved, and no bridge to
`@solana/wallet-adapter-react` needed since it speaks the Wallet Standard
protocol directly (`wallet-standard:register-wallet` /
`wallet-standard:app-ready`, `solana:signTransaction`). `tests/fixtures/testChain.ts`
funds both wallets and creates a fresh SPL mint per test run directly via
`@solana/kit` (bypassing the UI, since this is test setup, not part of what's
under test). Because on-chain state persists across runs, a
`pretest:e2e` hook (`scripts/reset-local-validator.sh`) resets the local
validator and redeploys the program before every `yarn test:e2e` — without it
the suite is order-dependent and only passes once per validator lifetime.

**Docker:** `web/` runs on the host with plain `yarn dev`, not inside the
Anchor container — the container exists only because the Solana/Anchor
toolchain needs `linux/amd64`; a JS frontend has no such constraint. Document
the two parallel dev loops side by side in `README.md`.

## Alternatives considered

- **Next.js** — rejected: no SSR/API-route need; adds complexity a
  workshop-teaching app doesn't need. (Solana's own docs cover a Next.js path
  too — `nextjs-solana` — this plan follows the React-only guide instead.)
- **`@solana/wallet-adapter-react`** — this plan's first draft chose it for
  its adoption numbers, before checking Solana's current official frontend
  docs; both official guides now build on `@solana/kit-plugin-wallet`
  instead, so this plan follows that. See Approach.
- **`@wallet-ui/react`** — moot once `@solana/kit-plugin-wallet` (what it's
  built on) is used directly, per the official docs.
- **`@anchor-lang/core`'s generated TS client** — rejected: incompatible with
  `@solana/kit`.
- **Backend indexer for plan discovery** — rejected: an extra service to run
  for a workshop demo; direct `getProgramAccounts` is sufficient at this scale.

## Affected areas

- New `web/` directory: React/Vite app (`web/src/`), generated Codama client
  under `web/src/generated/`, `.env.example`, Playwright E2E suite
  (`web/tests/`) and its hermetic-reset script (`web/scripts/reset-local-validator.sh`).
- `README.md` — document the `web/` dev loop alongside the existing Docker one,
  including that `web/`'s codegen depends on an `anchor build` run inside the
  container first (cross-directory build dependency).
- `docs/solana-toolchain.md` — add `@playwright/test` as the Verify-phase E2E
  tool for this program's first web client (currently undocumented there).
- `Dockerfile` — pin the Solana CLI install to `v2.1.21` (was tracking
  "stable"): Agave 3.x+'s `solana-test-validator` hard-requires `io_uring`,
  which panics under this container's virtualized kernel, so a local
  validator (needed for the E2E suite) couldn't start at all on "stable".
  `anchor build`/`anchor deploy` still activate a newer release internally for
  their own SBF toolchain regardless of this pin — that's a separate,
  unaffected mechanism.
- No changes to `app/programs/app` — the program itself is untouched. No root
  `.gitignore` change was needed after all: its existing generic `node_modules/`
  and `dist/` patterns already cover `web/node_modules` and `web/dist` without
  a `web/`-specific entry.
- One-time devnet deploy of the existing program — attempted, currently
  blocked; see Risks.

## Test scenarios

E2E flows in `web/tests/subscriptions.spec.ts`, driven via `@playwright/test`
against a freshly reset and redeployed local validator (see Testing above),
so they're reproducible without depending on devnet state or on the order
runs happen in.

1. Given a connected merchant wallet, when they submit the create-plan form
   with a valid amount/period, then a new Plan appears in "My plans" with
   `is_active = true`.
2. Given an active fixed-price plan, when a subscriber submits the subscribe
   form, then a Subscription and SubscriberDelegation account appear, and the
   subscriber's token account shows a delegate approval for the committed
   allowance.
3. Given a subscription whose `next_charge_at` has elapsed, when any wallet
   clicks "Trigger charge", then the merchant's token balance increases by the
   plan's applicable amount and `next_charge_at` advances.
4. Given a variable-price plan, when the merchant updates the price, then the
   UI shows both the current price and the pending price with its
   effective-at time.
5. Given a subscriber lowers their allowance below its current value, when
   they submit "Set allowance", then the delegation's committed total
   decreases by the difference and the token account's delegated amount is
   re-approved to match (the subscriber's other subscriptions on the same
   mint, if any, are unaffected).
6. Given a subscriber cancels, when the action confirms, then the Subscription
   account is closed (rent returned to the subscriber) and the delegation
   shrinks to the remaining committed total across any other plans.
7. Given a program id with no on-chain Plan accounts yet, when the frontend
   loads, then it shows a "no plans found" empty state rather than an error.

## Risks & open questions

- **Risk (materialized):** devnet deploy is blocked, not just undone. The
  public devnet faucet returned a hard daily-quota error
  (`"You've either reached your airdrop limit today or the airdrop faucet has
  run dry"`) from this environment's outbound IP, re-checked hours apart with
  the same result. A proof-of-work faucet fallback (`devnet-pow`) exists but
  itself needs a small SOL balance to pay transaction fees before it can mine
  — which bootstraps through the same blocked airdrop endpoint, a dead end.
  **Unblock path:** fund the deployer keypair from an unblocked network (a
  human, or CI on a non-datacenter IP) via `https://faucet.solana.com`, or
  retry later once the daily quota resets, then run `anchor deploy
  --provider.cluster devnet` from inside the container.
- **Risk:** `@solana/kit-plugin-wallet` is a young package (early 0.x, per the
  docs' own version guidance) with low adoption so far — but it's first-party
  and is what Solana's current official frontend docs build on, which this
  plan treats as the safer signal over raw download counts. Mitigated by
  following the documented pattern exactly rather than improvising around it.
- **Risk:** committing a generated Codama client instead of regenerating it in
  CI can silently drift from the on-chain program if someone forgets
  `yarn codegen` after a program change. No CI drift-check is in scope here —
  worth a follow-up plan.
- **Risk:** unindexed `getProgramAccounts` scans won't scale past
  devnet/workshop usage. Acceptable given the stated non-goals; flag if this
  ever needs to survive real traffic.
- **Risk:** the E2E suite's `pretest:e2e` hook shells out to `docker compose
  exec` directly, so it only runs where the Docker dev container is available
  and already built (`docker compose up -d` from the repo root) — it cannot
  run from a bare `web/` checkout without `app/`'s Docker setup. Acceptable
  given this repo's Docker-first design throughout; worth revisiting if `web/`
  is ever split into its own repo.

## Definition of Done

- [x] Goals met; non-goals respected — except the devnet half of the last
      Goal (see Risks).
- [x] Tests for the scenarios above pass (Playwright, against a freshly reset
      and redeployed local validator, twice in a row to confirm the reset
      makes the suite rerunnable).
- [x] Build and lint green (`yarn build`, `yarn lint` in `web/`), `strict`
      mode on throughout.
- [x] No inline comments; no secrets or `.env` values committed.
- [x] `README.md` documents the `web/` dev loop and its cross-directory
      codegen dependency on `app/`.
- [x] `docs/solana-toolchain.md` documents `@playwright/test` as the
      Verify-phase E2E tool.
- [ ] Program deployed to devnet; `web/.env.example` points at it. **Not
      done** — blocked by external infra, see Risks for the unblock path.
- [x] This plan updated to **Implemented**.
