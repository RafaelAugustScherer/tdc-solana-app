# 0002 — Subscription plans

- **Status:** Accepted
- **Author:** Rafael Scherer
- **Related:** [0001](0001-development-harness.md) (harness this builds on);
  first of four — [0003](0003-delegation-and-charging.md),
  [0004](0004-subscriber-spending-caps.md), [0005](0005-variable-pricing.md)

## Context

The repo has a working Anchor harness ([0001](0001-development-harness.md)) and a
placeholder program: [`lib.rs`](../../app/programs/app/src/lib.rs) exposes a single
`initialize` that logs its own program id, [`state.rs`](../../app/programs/app/src/state.rs)
is empty, and [`error.rs`](../../app/programs/app/src/error.rs) holds a stub
`CustomError`. There is no product yet.

The product these four plans build is **recurring merchant charges that the payer
can cancel unilaterally.** Off-chain, cancelling a subscription means asking the
merchant to stop and hoping they comply. On Solana the payer holds the lever
directly: SPL Token's `Approve` grants a capped, revocable spending allowance, and
`Revoke` withdraws it without the merchant's cooperation and without this
program's cooperation. Funds never enter an escrow — the merchant pulls from the
subscriber's own token account under a delegation the subscriber can tear up at
any moment, from any wallet UI, even if this program disappears.

This first plan builds only the **merchant-side catalogue**: publishing a plan and
taking it off sale. No `Subscription` account, no delegation, no money movement.
Those are [0003](0003-delegation-and-charging.md).

*Why split it this way:* the money path deserves a review with nothing else in the
diff. Everything here is inert state a merchant writes about their own plan, which
is a genuinely different risk profile from an instruction that moves a third
party's tokens.

### Sequence

| Plan | Domain |
|---|---|
| **0002** (this) | Merchant catalogue — publish and retire plans |
| [0003](0003-delegation-and-charging.md) | The money path — subscribe, charge, cancel |
| [0004](0004-subscriber-spending-caps.md) | Subscriber-side bounds — per-period cap, allowance top-up |
| [0005](0005-variable-pricing.md) | Merchant price changes — fixed vs variable plans |

## Goals

- A merchant can publish a plan naming a mint, an amount, and a billing period.
- A merchant can take their own plan off sale and put it back on.
- A plan is rejected at creation if its mint is one this program cannot bill
  correctly.
- A merchant declares at publication whether the price is fixed or variable, so it
  is a visible term of the offer before anyone subscribes.

## Non-goals

- **Subscriptions, delegation, or any token transfer.** All of [0003](0003-delegation-and-charging.md).
- **Acting on `price_mode`.** The field is stored but inert; there is no
  `update_price` until [0005](0005-variable-pricing.md), so `amount_per_period` is
  immutable in this plan and in [0003](0003-delegation-and-charging.md) whatever
  the mode says. 0005 adds three further fields to `Plan` to support price changes,
  which is an account layout change — acceptable because the product is devnet-only
  and redeploying discards state. Called out so it is a planned step rather than a
  surprise.
- **Native SOL or wrapped SOL.** Delegation is SPL-only, so the whole product is
  denominated in an SPL mint; wSOL would add `sync_native` and unwrap handling for
  no teaching benefit.
- **Token-2022 mints.** See *Pinning the token program* below.
- **Deleting a plan.** Retiring is `is_active = false`; nothing is closed, because
  0003's subscriptions will reference the plan account.
- **Frontend or indexer.** A later plan.

## Approach

### Account

**`Plan`** — PDA, seeds `[b"plan", merchant, plan_id.to_le_bytes()]`.

```rust
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum PriceMode {
    Fixed,
    Variable,
}

#[account]
pub struct Plan {
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub plan_id: u64,
    pub amount_per_period: u64,
    pub period_seconds: i64,
    pub price_mode: PriceMode,
    pub is_active: bool,
    pub bump: u8,
}
```

`price_mode` is recorded here but does nothing on-chain until
[0005](0005-variable-pricing.md) adds `update_price`; a `Variable` plan behaves
exactly like a `Fixed` one in the meantime.

It is stored at publication anyway, and deliberately, because it is a **term of the
offer rather than a mechanism**. The product requirement is that a subscriber can
see which kind of plan they are agreeing to *before* they agree, and a plan's
pricing kind must not be something the merchant can change after subscribers have
committed. Recording it at `create_plan` — the one instruction that writes it — is
what makes it immutable; adding it later in 0005 alongside the fields that give it
behaviour would mean the first subscribers in 0003 subscribed to an offer with no
stated pricing kind at all. The inert field is the guarantee.

`period_seconds` is `i64` rather than `u64` so it adds directly to the `i64`
timestamps 0003 works in, without a cast at every call site. `plan_id` is `u64`
and is part of the seeds, letting one merchant publish many plans on one mint.

### Instructions

**`create_plan(plan_id, amount_per_period, period_seconds, price_mode)`** —
merchant signs and pays rent. Rejects `amount_per_period == 0` (`InvalidAmount`)
and `period_seconds <= 0` (`InvalidPeriod`). Stores `ctx.bumps.plan`.

Re-using a `plan_id` fails on the `init` constraint, which is the behaviour we
want: plan addresses are deterministic, so a client can derive one and a merchant
cannot silently overwrite a plan subscribers are already paying into.

**`set_plan_active(is_active)`** — merchant signs, `has_one = merchant`. Flips the
flag. 0003's `charge` and `subscribe` both refuse an inactive plan, so this is the
merchant's off-switch for new and existing subscriptions alike.

### Pinning the token program

The plan's mint must be a **plain SPL Token** mint, and `create_plan` enforces it
by requiring `Program<'info, Token>` (which pins `spl_token::ID`) and
`Account<'info, Mint>` rather than the `Interface`/`InterfaceAccount` pair.

*Why reject Token-2022:* a mint with the transfer-fee extension would have 0003's
`charge` debit the subscriber `amount_per_period` while the merchant receives less,
and the program would report success — a silent shortfall on every single charge.
A transfer-hook mint can fail or reenter in ways this design has not been reviewed
for. Using the concrete `Token` type makes the non-goal true by construction
rather than by a constraint someone can forget, and validating at `create_plan`
means a bad mint is rejected once at publication instead of on every charge.

Revisiting this is a deliberate future decision, not an oversight — supporting
Token-2022 properly means reading the fee configuration and charging gross so the
merchant nets the plan amount.

### Dependency

Add `anchor-spl` to [`app/programs/app/Cargo.toml`](../../app/programs/app/Cargo.toml),
pinned to **1.1.2** to match the `anchor-lang` **1.1.2** already resolved in
`Cargo.lock` — `anchor-spl` depends on `anchor-lang` with an exact `=`
requirement, so the two must move together.

The `idl-build` feature must be extended in the same change:

```toml
idl-build = ["anchor-lang/idl-build", "anchor-spl/idl-build"]
```

*Why:* omitting `anchor-spl/idl-build` breaks IDL generation as soon as an
`anchor-spl` type appears in an account struct, and the failure is reported
somewhere unhelpful.

## Alternatives considered

- **Fold this into the subscription plan** — one plan covering plans, delegation,
  and charging. Rejected: it puts inert merchant state in the same diff as the
  instruction that moves other people's tokens. A first review of this program's
  money path should have nothing else competing for attention.
- **`Interface<TokenInterface>` accepting Token-2022 as well** — the more general
  account type. Rejected: it accepts precisely the mints the non-goals exclude,
  making the restriction a comment rather than a constraint. See *Pinning the
  token program*.
- **A plan counter on a merchant account instead of a caller-supplied `plan_id`** —
  auto-incrementing ids. Rejected: it adds a second account and a write-lock on
  every `create_plan` to save the client from choosing a number it already knows.
- **Closing a retired plan to reclaim rent** — `close` rather than a flag.
  Rejected: 0003's `Subscription` accounts reference the plan and read its price
  and period; a closed plan would strand them.

## Affected areas

- [`app/programs/app/src/state.rs`](../../app/programs/app/src/state.rs) — `Plan`
  (currently empty).
- [`app/programs/app/src/instructions/`](../../app/programs/app/src/instructions/) —
  `create_plan`, `set_plan_active`; `initialize.rs` is removed along with its entry
  in [`lib.rs`](../../app/programs/app/src/lib.rs) and
  [`instructions.rs`](../../app/programs/app/src/instructions.rs).
- [`app/programs/app/src/error.rs`](../../app/programs/app/src/error.rs) — replaces
  the stub `CustomError`.
- [`app/programs/app/src/constants.rs`](../../app/programs/app/src/constants.rs) —
  the `b"plan"` seed; replaces the stub `SEED`.
- [`app/programs/app/Cargo.toml`](../../app/programs/app/Cargo.toml) — `anchor-spl`,
  the `idl-build` feature, and LiteSVM token helpers under `dev-dependencies`
  (versions to be checked against the registry at implementation time, per the
  guidelines' dependency rule).
- [`app/programs/app/tests/`](../../app/programs/app/tests/) — new tests;
  `test_initialize.rs` is removed with the instruction it covers.

**Risky:** deleting `initialize` changes the IDL and removes the repo's only test.
That is intended — it is scaffolding, not product — but it should be a deliberate
line in the PR rather than a silent deletion.

## Test scenarios

Rust + LiteSVM, matching [`test_initialize.rs`](../../app/programs/app/tests/test_initialize.rs).

1. Given a merchant and an SPL Token mint, when `create_plan` runs, then the `Plan`
   PDA holds the given merchant, mint, `plan_id`, amount, period, and `price_mode`,
   `is_active` is true, and `bump` matches the derived bump.
2. Given `amount_per_period == 0`, when `create_plan` runs, then it fails with
   `InvalidAmount`.
3. Given `period_seconds == 0`, when `create_plan` runs, then it fails with
   `InvalidPeriod`.
4. Given `period_seconds < 0`, when `create_plan` runs, then it fails with
   `InvalidPeriod`.
5. Given an existing plan, when the same merchant calls `create_plan` with the same
   `plan_id`, then it fails and the stored plan is unchanged.
6. Given the same `plan_id` used by a *different* merchant, when `create_plan` runs,
   then it succeeds and derives a distinct PDA.
7. Given a merchant with an existing plan, when they call `create_plan` on the same
   mint with a different `plan_id`, then it succeeds and both plans coexist with
   independent amounts and periods.
8. Given a Token-2022 mint, when `create_plan` runs, then it fails rather than
   publishing a plan this program cannot bill correctly.
9. Given a published plan, when its merchant calls `set_plan_active(false)`, then
   `is_active` is false.
10. Given a retired plan, when its merchant calls `set_plan_active(true)`, then
    `is_active` is true again.
11. Given a published plan, when a non-merchant signer calls `set_plan_active`,
    then it fails with a `has_one` constraint error and the flag is unchanged.
12. Given a `Plan` address that does not match the seeds for the passed merchant
    and `plan_id`, when `set_plan_active` runs, then it fails with a seeds
    constraint error.

## Risks & open questions

- **`amount_per_period` is immutable here but mutable in 0005.** A reader of this
  plan alone could reasonably treat it as permanently fixed and build a client that
  caches it. Named here so 0005's change is expected rather than surprising; no
  code consequence in this plan.
- **Rejecting Token-2022 excludes real mints.** Some tokens a merchant might want
  to bill in are Token-2022. Accepted deliberately: billing them correctly needs
  fee-aware arithmetic, and shipping silent under-payment would be worse than not
  supporting them. Recorded as a future decision above.
- **The Anchor CLI and crate versions disagree, pre-existing.** The
  [`Dockerfile`](../../Dockerfile) pins the Anchor CLI to **1.0.2** while
  `app/Cargo.lock` resolves `anchor-lang` to **1.1.2**. This has not bitten the
  placeholder program, but adding `anchor-spl` and a real IDL raises the odds of a
  build or IDL-generation mismatch. De-risk by running `anchor build` in the
  container as the first implementation step, before writing instructions; if it
  breaks, align the versions in a separate change rather than folding it into this
  one. Tracked separately.

No open questions.

## Definition of Done

- [ ] Goals met; non-goals respected.
- [ ] Tests for the scenarios above pass (`cargo test`).
- [ ] `anchor build` succeeds in the container and generates an IDL containing both
      instructions.
- [ ] Build and lint green (`cargo fmt`, `cargo clippy`).
- [ ] `/audit-solana` and `/diff-review` run clean.
- [ ] A Token-2022 mint cannot be used to publish a plan — covered by a test.
- [ ] No inline comments; no secrets committed.
- [ ] This plan updated to **Implemented**.
