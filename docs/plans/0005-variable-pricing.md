# 0005 — Variable pricing

- **Status:** Implemented
- **Author:** Rafael Scherer
- **Related:** builds on [0002](0002-subscription-plans.md),
  [0003](0003-delegation-and-charging.md), [0004](0004-subscriber-spending-caps.md)

## Context

[0002](0002-subscription-plans.md) records a `price_mode` on every plan but does
nothing with it: `amount_per_period` is immutable whatever the mode says. This
plan makes it real — a merchant who published a `Variable` plan can change its
price, and a `Fixed` plan is guaranteed never to change for the life of the plan.

The subscriber-side protection already exists.
[0004](0004-subscriber-spending-caps.md) gives every subscription a
`max_amount_per_period` the merchant cannot exceed, and pausing-on-cap already
falls out of the existing checks. So this plan is narrower than it looks: it adds
the merchant's ability to move a price, and the rules governing *when* a new price
starts applying.

## Goals

- A merchant can change the price of a plan they published as `Variable`.
- A plan published as `Fixed` can never have its price changed.
- A price change never affects a period the subscriber has already paid for.
- A price change cannot take effect less than a day after it is announced.
- Crank timing does not affect what a subscriber is billed.
- `charge` does not become a point of contention between subscribers on the same
  plan.

## Non-goals

- **Changing a plan's period, mint, or price mode after publication.** Only the
  price of a `Variable` plan is mutable. Anything else means a new plan.
- **Prorating or refunds.** A change takes effect whole at a period boundary.
- **Notifying subscribers.** An event is emitted for indexers to consume; the
  client work is a later plan.
- **Migrating existing plan accounts.** Three fields are added to `Plan`, which is a
  layout change; devnet state is discarded on redeploy, as
  [0002](0002-subscription-plans.md) anticipates.

## Approach

### Changed account

`Plan` gains three fields:

```rust
pub previous_amount: u64,
pub previous_effective_at: i64,
pub amount_effective_at: i64,
```

Together with the existing `amount_per_period` these describe two price *intervals*
— the current price and the one before it, each with the instant it took force —
rather than a current price plus a pending change. At `create_plan` all three are
initialised so the plan reads as a constant price: `previous_amount =
amount_per_period`, and both timestamps `0`.

### The applicable price is a pure function

```rust
pub fn applicable_amount(plan: &Plan, at: i64) -> Result<u64> {
    if at >= plan.amount_effective_at {
        Ok(plan.amount_per_period)
    } else if at >= plan.previous_effective_at {
        Ok(plan.previous_amount)
    } else {
        err!(SubscriptionError::PriceHistoryUnavailable)
    }
}
```

This is the whole mechanism, and it is deliberately a *read*. `charge` calls it
and never writes to `Plan`.

*Why it returns a `Result` rather than a `u64`:* the plan stores two intervals, but
`charge` asks it about `subscription.next_charge_at`, which
[0003](0003-delegation-and-charging.md) allows to sit arbitrarily far in the past —
a subscription nobody cranks, or one [0004](0004-subscriber-spending-caps.md) has
paused on its cap. If two price changes have both matured since that date, the
older price is gone and no honest answer exists.

Returning `previous_amount` anyway would be the tempting bug, and it is a real one:
with period one day, a plan at 100, a charge due at `T` that nobody cranks, a rise
to 120 announced at `T+1d`, and a further rise to 150 at `T+2.5d`, a crank at `T+4d`
would bill **120** for a period that began at `T` — an amount that did not exist as
a price until two days after that period started. The subscriber overpays for a
window the merchant never charged them for at the time.

So the third branch fails with `PriceHistoryUnavailable` instead. Failing loudly
keeps the guarantee this plan is built on — you are billed the price in force when
your period began, or you are not billed — rather than trading it for an answer that
is merely available. Recovery is in *Risks*.

*Why this shape rather than a pending value promoted on charge:* the obvious
design stores `pending_amount`/`pending_effective_at` and has `charge` promote
them once mature. That has two defects. First, `charge` would take a write lock on
`Plan`, so every subscriber to a popular plan serialises against every other — one
charge per plan per slot, for no reason. [0003](0003-delegation-and-charging.md)
deliberately keeps `charge` writing only `Subscription`, and this plan must not
regress that. Second, a promotion inside a charge that later fails is rolled back
with it, so a matured price could sit unpromoted indefinitely and off-chain
readers would see a stale `amount_per_period`. Deriving the price from the clock
has neither problem and needs no crank.

### Billed at the price in force when the period began

`charge` evaluates the price at **`subscription.next_charge_at`** — the moment the
period being billed started — not at `now`:

```rust
let amount = applicable_amount(plan, subscription.next_charge_at)?;
require!(
    amount <= subscription.max_amount_per_period,
    SubscriptionError::PriceAboveSubscriberMax
);
```

Both lines matter, and they must move together. [0004](0004-subscriber-spending-caps.md)
compares `plan.amount_per_period` against the subscriber's cap; this plan replaces
that with `amount`, the same value the transfer uses. Leaving the comparison on the
live price while transferring the historical one lets the two diverge — the merchant
lowers the price below a subscriber's cap, and an overdue period still billed at the
old, higher price sails past a check that was reading the new one. That would move
more tokens than the subscriber authorised, which is exactly the invariant 0004
exists to hold. Check what you charge, not what the plan currently advertises.

This matters because the crank is permissionless and the merchant is the party
with an incentive to run it. Evaluating at `now` would let a merchant let a due
date pass uncranked, announce a rise, wait out the notice day, then crank: the
subscriber pays the new price for a window that started days earlier, and gets
less coverage for more money. Pinning the price to the billing date makes crank
timing irrelevant to what anyone is charged, which removes the incentive to game
it entirely.

Note this composes with [0003](0003-delegation-and-charging.md)'s backlog
collapse: an overdue subscription bills once, at the price in force on its
original due date.

### Instruction

**`update_price(new_amount)`** — merchant signs, `has_one = merchant`. Rejects a
`Fixed` plan (`PlanPriceFixed`) and a zero amount (`InvalidAmount`).

```rust
let now = Clock::get()?.unix_timestamp;
if now >= plan.amount_effective_at {
    plan.previous_amount = plan.amount_per_period;
    plan.previous_effective_at = plan.amount_effective_at;
}
plan.amount_per_period = new_amount;
plan.amount_effective_at = now
    .checked_add(PRICE_CHANGE_NOTICE_SECONDS)
    .ok_or(SubscriptionError::ScheduleOverflow)?;
```

The condition is what makes a second call during the notice window correct. Two
cases, and only one of them has any history to record:

- **The pending change has matured** (`now >= amount_effective_at`). The current
  price really did take force at `amount_effective_at`, so that interval moves into
  the `previous_*` pair before being overwritten.
- **Still inside the notice window.** The announced amount never applied to anyone,
  so there is nothing to remember — it is discarded and `previous_*` is left alone,
  still describing the price genuinely in force. Overwriting it here would be the
  bug: it would record an interval that never happened and lose the one that did.

An announcement superseded before it matures therefore leaves no trace, which is the
correct behaviour and is why the plan account is not a price history.

It emits an event carrying the plan, both amounts, and `amount_effective_at`, so
an indexer or client has something to subscribe to rather than polling and diffing
plan accounts.

### Why a day, and why prepaid billing does most of the work

Billing is **prepaid**: a charge at `T` covers `[T, T+period)`. A price change
therefore cannot reach a period the subscriber has already paid for — "applies
from the next payment" is how the schedule already behaves, not something built
here.

That leaves one narrow gap, and it is the only thing the notice window closes: a
merchant raising the price seconds before a charge falls due, so the subscriber is
billed the new amount with no practical warning. `PRICE_CHANGE_NOTICE_SECONDS` is
a flat day, which is enough.

Deliberately *not* tied to `period_seconds`: on an annual plan that would hold a
price change for a year, which no merchant would accept. A flat day is predictable
for merchants and sufficient for subscribers, given
[0004](0004-subscriber-spending-caps.md)'s cap already makes the worst case a
number the subscriber chose. On a plan with a period shorter than a day the window
outlasts the period, which only slows the merchant and never harms the subscriber
— an acceptable asymmetry rather than a case worth branching on.

## Alternatives considered

- **`pending_amount` promoted inside `charge`.** Rejected — write contention on
  `Plan` and rollback of the promotion when the surrounding charge fails. See *The
  applicable price is a pure function*.
- **A separate permissionless `promote_price` crank.** Fixes contention but adds an
  instruction someone must remember to call, and leaves the stored price stale
  until they do. Deriving from the clock needs no crank at all.
- **Notice window of one full period.** Guarantees one more charge at the old
  price, but an annual plan would hold a change for a year. Rejected in favour of
  a flat day.
- **Immediate price changes, no window.** Defensible now that the subscriber's cap
  is a hard bound — a merchant still cannot charge above what the subscriber
  authorised. Rejected because a cap set months ago and forgotten is common, and a
  price moving from 5 to 20 under a cap of 20 is legitimate but surprising.
- **Evaluating the price at `now`.** Simpler, and wrong: it makes what a subscriber
  pays depend on when a merchant chooses to crank. See *Billed at the price in
  force when the period began*.
- **A merchant-published ceiling constraining `update_price`.** Rejected in
  [0004](0004-subscriber-spending-caps.md); the subscriber's own cap binds tighter
  and needs no good faith from the merchant.

## Affected areas

- [`app/programs/app/src/state.rs`](../../app/programs/app/src/state.rs) — adds
  `previous_amount`, `previous_effective_at` and `amount_effective_at` to `Plan`.
- [`app/programs/app/src/instructions/`](../../app/programs/app/src/instructions/) —
  adds `update_price`; **revises** `create_plan` to initialise the new fields and
  `charge` to price from `applicable_amount` and to compare the subscriber's cap
  against that amount rather than `plan.amount_per_period`.
- [`app/programs/app/src/error.rs`](../../app/programs/app/src/error.rs) — adds
  `PlanPriceFixed`, `PriceHistoryUnavailable`.
- [`app/programs/app/src/constants.rs`](../../app/programs/app/src/constants.rs) —
  `PRICE_CHANGE_NOTICE_SECONDS`.

**Risky:** `charge` changes how it determines the amount. Every charging scenario
from [0003](0003-delegation-and-charging.md) and
[0004](0004-subscriber-spending-caps.md) must be re-run.

## Test scenarios

**Mode enforcement**

1. Given a `Fixed` plan, when the merchant calls `update_price`, then it fails with
   `PlanPriceFixed` and the price is unchanged.
2. Given a `Variable` plan, when a non-merchant signer calls `update_price`, then
   it fails and the price is unchanged.
3. Given `new_amount == 0`, when `update_price` runs, then it fails with
   `InvalidAmount`.

**The notice window**

4. Given a fresh plan, when `applicable_amount` is evaluated at any time, then it
   returns `amount_per_period` — an untouched plan is a constant price.
5. Given a `Variable` plan whose price the merchant has just raised, when a charge
   falls due within the notice day, then the subscriber is billed the **old**
   amount.
6. Given the same plan, when the clock passes `amount_effective_at` and a charge
   falls due, then the subscriber is billed the new amount.
7. Given a pending change, when the merchant calls `update_price` again before it
   lands, then the price still in force is preserved as `previous_amount`, the
   newest amount becomes upcoming, and the day restarts.
8. Given two `update_price` calls a day apart, when charges fall due between and
   after them, then each is billed the amount in force at its own due date.
9. Given a merchant who lowers the price, when the notice day passes and a charge
   falls due, then the subscriber is billed the lower amount.

**Crank timing cannot be gamed**

10. Given a charge due at `T` that nobody cranks, when the merchant raises the
    price at `T+2d` and cranks at `T+3d`, then the subscriber is billed the price
    in force at `T` — the **old** amount.
11. Given the same setup, when the charge settles, then `next_charge_at` has
    advanced from `T` by whole periods and the subscriber has not been billed twice.
12. Given a subscription three periods overdue spanning a price change, when a
    single `charge` runs, then exactly one charge is taken, at the price in force
    on its original due date.

**Price history that cannot answer**

13. Given a charge due at `T` that nobody cranks, when the merchant raises the price
    twice such that **both** changes mature before `T` is cranked, then `charge`
    fails with `PriceHistoryUnavailable`, no tokens move, and `next_charge_at` is
    unchanged. Without the third branch this case silently bills the intermediate
    price.
14. Given the same plan, when a *different* subscriber whose `next_charge_at` falls
    after the second change is cranked, then their charge succeeds at the current
    price — one subscriber's stale schedule does not break the plan for everyone.
15. Given two price changes where only the first has matured, when a charge due
    before either runs, then it succeeds at the original price — one matured change
    is always answerable.

**Interaction with the subscriber cap**

16. Given a `Variable` plan raised above a subscriber's cap, when a charge falls due
    after the notice window, then it fails with `PriceAboveSubscriberMax`, no
    tokens move, and `next_charge_at` is unchanged.
17. Given that dormant subscription, when the subscriber raises their cap, then the
    next charge succeeds at the new price.
18. Given a `Variable` plan raised above one subscriber's cap but within another's,
    when both charges fall due, then the first fails and the second succeeds.
19. Given a merchant who **lowers** the price below a subscriber's cap while an
    overdue period is still priced above it, when `charge` runs, then it fails with
    `PriceAboveSubscriberMax` — the cap is compared against the amount actually
    transferred, not the plan's current price.

**No regression**

20. Given two subscribers to the same plan, when both charges are submitted in one
    transaction, then both succeed and neither `Plan` field changes. This checks the
    weaker claim that `charge` needs no `Plan` write; the parallel-scheduling
    property it enables is verified by review of the generated IDL, where `plan`
    must not be marked writable.
21. Given a charge that fails after the price was read, when the transaction rolls
    back, then `Plan` is unchanged and the next charge still prices correctly.

## Risks & open questions

- **The plan remembers two price intervals, and a charge older than both is
  refused.** This is the deliberate trade in *The applicable price is a pure
  function*, and it has a liveness cost worth stating plainly: a subscription whose
  `next_charge_at` falls before `previous_effective_at` can never be charged again,
  because a failed charge does not advance the schedule and a further `update_price`
  only pushes the window forward. The subscriber's recovery is `cancel` then
  `subscribe` — the same route 0003 already prescribes after an external `Revoke` —
  and the merchant's cost is the revenue they declined to crank for.

  Reaching it requires a merchant to leave a charge uncranked across two full notice
  windows *and* change the price twice in that time, so the party who causes it is
  the party who loses by it. A merchant cranking on schedule can never produce it.
  Recorded as an accepted bound rather than fixed with a longer history, which would
  buy a rarer version of the same cliff for a bigger account and more arithmetic.
- **An announcement superseded before it matures leaves no trace**, by design. The
  plan account is not a price log; an indexer consuming the `update_price` event is
  the source of history.
- **A subscription dormant on its cap is silent to both parties.** Carried over
  from [0004](0004-subscriber-spending-caps.md); the `update_price` event gives the
  merchant side a signal, and the subscriber side needs the client to compare each
  cap against its plan's price.
- **The layout change discards devnet state.** Anticipated in
  [0002](0002-subscription-plans.md). Any demo plans and subscriptions must be
  recreated after deploying this.

No open questions.

## Definition of Done

- [ ] Goals met; non-goals respected.
- [ ] Tests for the scenarios above pass, **and** every charging scenario from 0003
      and 0004 still passes.
- [ ] A `Fixed` plan's price cannot change — covered by scenario 1.
- [ ] What a subscriber is billed does not depend on when the crank runs — covered
      by scenarios 10 and 12.
- [ ] No subscriber is ever billed an amount that was not in force when their period
      began; where that cannot be determined the charge fails — covered by
      scenarios 13, 14 and 15.
- [ ] The subscriber's cap is compared against the amount actually transferred —
      covered by scenario 19.
- [ ] `charge` still writes no `Plan` state — covered by scenario 20 and by
      confirming `plan` is not writable in the generated IDL.
- [ ] Build and lint green (`anchor build`, `cargo fmt`, `cargo clippy`).
- [ ] `/audit-solana` and `/diff-review` run clean.
- [ ] No inline comments; no secrets committed.
- [ ] This plan updated to **Implemented**.
