# 0003 — Delegation and charging

- **Status:** Implemented
- **Author:** Rafael Scherer
- **Related:** builds on [0002](0002-subscription-plans.md); followed by
  [0004](0004-subscriber-spending-caps.md), [0005](0005-variable-pricing.md)

## Context

[0002](0002-subscription-plans.md) gives merchants a catalogue of plans. Nothing
subscribes to them and no money moves. This plan adds the money path: a subscriber
grants a capped, revocable allowance, and anyone can crank a due charge that pulls
the plan amount from the subscriber's own token account into the merchant's.

This is the security-critical plan of the four. Every other plan writes state; this
one moves a third party's tokens under a delegation, and it is the only plan where
a mistake costs someone money. The account constraints below are therefore part of
the design and are specified in full rather than left to the implementation.

### What the research settled

Confirmed against current SPL Token and Anchor 1.x docs:

- **A token account has exactly one delegate.** Calling `Approve` again replaces
  the previous delegate rather than adding to it. This single constraint shapes
  the whole account layout.
- **`delegated_amount` decrements on each delegated transfer**, and the token
  program clears the delegate once it reaches zero.
- **`Revoke` is signed by the account owner**, clears delegate and allowance
  together, and does not touch this program.
- **`Transfer` is deprecated since SPL Token 4.0.0**; `TransferChecked` is current
  and is what `anchor_spl` exposes.
- **Delegation is a property of a token account, not of a wallet.** A wallet can
  hold any number of token accounts on the same mint, each with its own independent
  delegate and `delegated_amount`. This is the fact the account layout below is
  built around; see *One token account per subscriber per mint*.

## Goals

- A subscriber can start a subscription and grant its allowance in one transaction.
- A subscriber can hold several concurrent subscriptions despite the single
  on-chain delegate slot.
- Anyone can crank a due charge; it moves exactly the plan amount, at most once per
  period, to the plan's merchant and no one else.
- A subscriber can cancel through this program, and can independently kill all
  delegation with a plain SPL `Revoke` this program cannot block.
- Cancelling is reversible: a subscriber can subscribe to the same plan again
  afterwards.

## Non-goals

- **A subscriber-set per-period cap.** [0004](0004-subscriber-spending-caps.md).
  Here a charge is bounded by the plan amount and the remaining allowance only.
- **Topping up an allowance without cancelling.** [0004](0004-subscriber-spending-caps.md).
  Until then, the recovery path after a `Revoke` is `cancel` then `subscribe`
  again, which is why `cancel` closes its account (see *Cancelling is reversible*).
- **Variable pricing.** [0005](0005-variable-pricing.md).
- **Catch-up billing.** A subscription left uncharged for three periods bills
  **once**, not three times. See *Advancing the schedule*.
- **Prorating, refunds, or trials.**
- **Frontend or indexer.** A later plan.

## Approach

### Accounts

**`Subscription`** — PDA, seeds `[b"subscription", plan, subscriber]`.

```rust
#[account]
pub struct Subscription {
    pub plan: Pubkey,
    pub subscriber: Pubkey,
    pub next_charge_at: i64,
    pub allowance_remaining: u64,
    pub bump: u8,
}
```

There is no `is_active` flag: `cancel` closes the account outright, so existence
*is* activity. That removes a state a reader has to reason about and makes
re-subscribing work for free.

**Delegate authority** — a single program PDA, seeds `[b"delegate"]`, holding no
data. It is the pubkey recorded as delegate on every subscriber's token account
and the authority that signs each `transfer_checked`. It is declared
`UncheckedAccount` with `seeds`/`bump` constraints and signs with `ctx.bumps`;
the bump is never accepted as an argument.

### One token account per subscriber per mint

Every instruction that touches a subscriber's tokens requires the **associated
token account** for `(subscriber, plan.mint)`, enforced by Anchor's
`associated_token::` constraints rather than by an owner check.

*Why this is load-bearing and not a convenience:* a delegation lives on a token
account, but the design reasons about a subscriber as if they had one balance per
mint. Those are only the same thing if the program insists on it. A wallet may open
as many token accounts on a mint as it likes, so without this constraint a
`Subscription` is not bound to the account that funded it — `owner` and `mint`
checks alone are satisfied by *any* of that subscriber's accounts on that mint.
A cranker could then charge one subscription against the account a different
subscription was funded from: the subscriber is debited from the wrong pot, the
wrong delegation is drawn down, and the merchant who was cranked for is paid the
right amount from the wrong place.

Pinning the ATA makes the mapping `(subscriber, mint) → exactly one token account`
total, which is the invariant the pool arithmetic here — and
[0004](0004-subscriber-spending-caps.md)'s `committed_total` — assumes throughout.

The cost is that a subscriber must use their ATA, which every wallet creates by
default, rather than an arbitrary token account. That is a restriction worth taking:
the alternative is storing the chosen token account on `Subscription` and pinning it
with `address =`, which is equally safe here but leaves 0004 without a well-defined
per-subscriber pool to aggregate.

### The delegation is a shared pool, not per-subscription reservations

One global delegate PDA is what the one-delegate-per-account rule forces. It is
worth being precise about what that does and does not give us, because the
comfortable description is wrong:

`allowance_remaining` on each `Subscription` **bounds** what that subscription may
draw. It does not **reserve** anything. There is one pool per subscriber per mint —
the ATA's `delegated_amount` — and every subscription of that subscriber *on that
mint* draws from it first-come-first-served. Subscriptions on different mints are
genuinely independent, because they are delegations on different accounts.

Normally the two agree, because `subscribe` adds to both. They can diverge when
the subscriber acts on the token account directly, which they are entitled to do:
a plain `Approve(delegate_pda, 50)` from a wallet, while two subscriptions hold
100 each, leaves `sum(allowance_remaining) = 200` against `delegated_amount = 50`.
Whichever merchant cranks first is paid; the other gets `DelegateRevoked`.

This is inherent to the single delegate slot, not a defect to engineer around, and
the honest framing matters for the client: program state is an upper bound on what
a merchant can take, never a promise that they can.

### Instructions

**`subscribe(allowance)`** — subscriber signs and pays rent.

Rejects `allowance == 0` (`InvalidAllowance`) and an inactive plan
(`PlanInactive`). Sets `next_charge_at` to the current clock, so the first charge
is immediately due.

Because `Approve` replaces rather than adds, it must read the current delegation
and approve the sum:

```rust
let existing = match subscriber_token_account.delegate {
    COption::Some(current) if current == delegate_authority.key() => {
        subscriber_token_account.delegated_amount
    }
    COption::Some(_) => return err!(SubscriptionError::ForeignDelegate),
    COption::None => 0,
};
let total = existing
    .checked_add(allowance)
    .ok_or(SubscriptionError::AllowanceOverflow)?;
```

Refusing when another program holds the delegate is deliberate: silently
overwriting another protocol's delegation would break it.

> **Superseded by [0004](0004-subscriber-spending-caps.md).** The read-and-add
> arithmetic above is correct only while the token account's `delegated_amount`
> faithfully mirrors program state, which stops being true the moment the
> subscriber approves or revokes externally. 0004 replaces it with a
> `SubscriberDelegation` PDA per `(subscriber, mint)` holding `committed_total`,
> and every approving path writes that figure rather than computing a delta. The
> `ForeignDelegate` guard survives on every approving path **except `cancel`**,
> where refusing would let an unrelated dApp block a subscriber from closing their
> own subscription. Read this section for why the delegation is shared; read 0004
> for the arithmetic that is actually implemented.

**`charge()`** — permissionless crank; the caller pays the transaction fee.

Checks in order: plan active (`PlanInactive`), `clock.unix_timestamp >=
subscription.next_charge_at` (`PeriodNotElapsed`), `allowance_remaining >=
plan.amount_per_period` (`AllowanceExhausted`), and that the delegate is still
this program's PDA with `delegated_amount >= plan.amount_per_period`
(`DelegateRevoked`). Then it CPIs `transfer_checked` signed by the delegate PDA,
decrements `allowance_remaining`, and advances the schedule.

The delegate state is checked explicitly rather than left to the CPI so a
subscriber who revoked gets a named error a client can render as "your approval
was withdrawn" instead of an opaque token program code.

**`cancel()`** — subscriber signs. Closes the `Subscription`, returning rent to
the subscriber. It deliberately does **not** CPI `revoke`, because the subscriber
may hold other live subscriptions sharing the same delegation; killing everything
at once is exactly what a direct SPL `Revoke` does and needs no instruction here.

### Advancing the schedule

This is the subtlest part of the plan and the one most likely to be got wrong.

Two properties are wanted at once, and they pull against each other:

1. **No drift.** Billing on the 3rd should stay on the 3rd, even if a crank runs
   late. That argues for `next_charge_at += period`.
2. **No catch-up.** A subscription left uncharged for ten periods should bill once
   when someone finally cranks, not ten times.

A bare `next_charge_at += period` satisfies (1) and **breaks (2) dangerously**.
After ten idle periods it leaves `next_charge_at` still in the past, so a second
and third `charge` in the *same transaction* pass the due check. Since the crank
is permissionless and the merchant is the party with an incentive to run it, a
merchant can simply wait and then harvest the whole backlog atomically — bounded
only by the allowance, which is sized for many periods.

`max(next_charge_at + period, now + period)` does not fix it: a charge can never
run before it is due, so `next_charge_at <= now` always holds and the `max`
collapses to `now + period` every time, silently reintroducing drift.

The fix is to advance by whole periods until the result is in the future:

```rust
let elapsed = now
    .checked_sub(subscription.next_charge_at)
    .ok_or(SubscriptionError::ScheduleOverflow)?;
let periods_to_advance = elapsed
    .checked_div(plan.period_seconds)
    .ok_or(SubscriptionError::ScheduleOverflow)?
    .checked_add(1)
    .ok_or(SubscriptionError::ScheduleOverflow)?;
subscription.next_charge_at = subscription
    .next_charge_at
    .checked_add(
        periods_to_advance
            .checked_mul(plan.period_seconds)
            .ok_or(SubscriptionError::ScheduleOverflow)?,
    )
    .ok_or(SubscriptionError::ScheduleOverflow)?;
```

`period_seconds > 0` is guaranteed by [0002](0002-subscription-plans.md), and
`elapsed >= 0` by the due check, so `periods_to_advance >= 1`. The result
satisfies `now < next_charge_at <= now + period_seconds` and stays congruent to
the original date modulo the period — both properties, and a second `charge` in
the same transaction now fails on `PeriodNotElapsed`.

### Cancelling is reversible

`Subscription` is a PDA on `[b"subscription", plan, subscriber]`, so its address
is fixed for a given pair. If `cancel` merely flagged it inactive, `init` would
fail forever afterwards and that subscriber could never subscribe to that plan
again. Closing the account instead makes re-subscription a plain `init` and
returns the rent.

This also supplies the recovery path this plan would otherwise lack. After a
subscriber sends a plain SPL `Revoke`, every one of their subscriptions is stuck:
`charge` fails with `DelegateRevoked` and nothing here re-approves. With a closing
`cancel` the answer is `cancel` then `subscribe` again, which re-approves as part
of the normal path. [0004](0004-subscriber-spending-caps.md) adds a direct
top-up so this is not the only route.

### Account constraints

For this program the constraints *are* the design. Stated in full for `charge`,
which is the instruction that can lose someone money:

```rust
#[derive(Accounts)]
pub struct Charge<'info> {
    #[account(
        seeds = [b"plan", plan.merchant.as_ref(), &plan.plan_id.to_le_bytes()],
        bump = plan.bump,
    )]
    pub plan: Account<'info, Plan>,

    #[account(
        mut,
        seeds = [b"subscription", plan.key().as_ref(), subscription.subscriber.as_ref()],
        bump = subscription.bump,
        has_one = plan,
    )]
    pub subscription: Account<'info, Subscription>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = subscription.subscriber,
    )]
    pub subscriber_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = merchant_token_account.owner == plan.merchant
            @ SubscriptionError::WrongMerchantAccount,
        constraint = merchant_token_account.mint == plan.mint
            @ SubscriptionError::WrongMint,
    )]
    pub merchant_token_account: Account<'info, TokenAccount>,

    #[account(address = plan.mint)]
    pub mint: Account<'info, Mint>,

    /// CHECK: seeds-derived delegate authority; signs the transfer, holds no data
    #[account(seeds = [b"delegate"], bump)]
    pub delegate_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}
```

Each line earns its place, and the failure it prevents:

- **`associated_token::mint = mint, associated_token::authority =
  subscription.subscriber`** — the one that matters most, and the reason an owner
  check is not enough. A single global delegate PDA means the delegation no longer
  identifies who is paying, so `charge` must derive the paying account rather than
  accept one. Anchor computes the ATA address from the two constraints and compares
  it to the passed key, which pins the account exactly. Two distinct attacks are
  closed by this single line: passing *another wallet's* token account (Bob's cheap
  `Subscription` with Alice's account — Alice is debited, Bob's merchant is paid),
  and passing *the same subscriber's other* account on the same mint, which an
  `owner == subscription.subscriber` check would happily accept while draining a
  pot earmarked for a different merchant. See *One token account per subscriber per
  mint*.
- **`merchant_token_account.owner == plan.merchant`** — otherwise a cranker
  redirects the payment. Not pinned to the ATA: the merchant is only ever a
  recipient, holds no delegation, and may legitimately want payments landing in a
  treasury account of their choosing.
- **`address = plan.mint` on the mint** — `transfer_checked` reads `decimals` from
  the passed mint account, so the mint must be the plan's. Note this is pinning the
  *decimals source*, not preventing a wrong-mint drain on its own: the token
  program independently rejects a mint that disagrees with the source account. The
  constraint's real value is failing early with a named Anchor error instead of an
  opaque token-program code, and feeding `mint` to the ATA derivation above.
- **`seeds`/`bump = *.bump` on `plan` and `subscription`** — re-derives both from
  stored bumps rather than trusting a passed address, and `has_one = plan` stops
  plan A's price and period being applied to plan B's subscription.

`subscribe` carries the matching ATA constraints, with
`associated_token::authority = subscriber` since the subscriber signs there. The
account must already exist — no `init_if_needed`. A subscriber needs a funded ATA
for the subscription to ever charge, so requiring it is not a real restriction, and
it keeps the `init-if-needed` feature (and its reinitialization footguns) out of
the program. The subscriber signs `subscribe`, so a mistake there is self-harm
rather than theft, but an unconstrained account would produce a subscription that
can never charge.

## Alternatives considered

- **Escrow vault** — subscriber pre-funds a program-owned vault the merchant draws
  from. Rejected: it takes custody, which is worse on the exact axis this product
  is about, and it makes revocation depend on this program staying alive and
  correct. Delegation keeps the money in the user's wallet.
- **One dedicated token account per subscription** — sidesteps the single-delegate
  constraint entirely and would make `allowance_remaining` a true reservation.
  Rejected: rent and a funding step per plan, and a balance split across accounts
  no wallet UI shows sensibly. The shared pool is the cost of a single ATA.
- **Storing the subscriber's chosen token account on `Subscription`** and pinning it
  with `address = subscription.token_account`, instead of requiring the ATA. Equally
  safe for this plan, and more permissive. Rejected because it leaves a subscriber
  with several token accounts per mint, each carrying an independent delegation,
  and [0004](0004-subscriber-spending-caps.md) needs to aggregate a subscriber's
  commitments into one figure per pool. Requiring the ATA makes that pool
  well-defined; storing the account would push a second key into every seed there.
- **Merchant-only `charge`** — restrict the crank to the merchant's signature.
  Rejected: it buys nothing, since `charge` can only ever move the plan amount to
  the plan merchant's own account on or after the due date. A stranger calling it
  is paying the merchant's fee for them. Note this is only true *given* the
  constraints above; without them the permissionless crank is the delivery
  mechanism for the theft in finding 2.
- **`cancel` also revoking the delegation** — friendlier teardown. Rejected: it
  would clear the delegate for the whole token account, killing every sibling
  subscription. Reducing it by this subscription's share instead means a `charge`
  on another plan and a `cancel` in the same slot read and write `delegated_amount`
  through two paths, and getting the ordering wrong silently mis-approves.
  [0004](0004-subscriber-spending-caps.md) owns delegation arithmetic and can
  design it once.
- **`is_active` on `Subscription` instead of closing it** — mirrors `Plan`.
  Rejected: it permanently bricks re-subscription, since the PDA address is fixed
  and `init` would always fail. See *Cancelling is reversible*.

## Affected areas

- [`app/programs/app/src/state.rs`](../../app/programs/app/src/state.rs) — adds
  `Subscription`.
- [`app/programs/app/src/instructions/`](../../app/programs/app/src/instructions/) —
  `subscribe`, `charge`, `cancel`.
- [`app/programs/app/src/error.rs`](../../app/programs/app/src/error.rs) — adds
  `InvalidAllowance`, `PlanInactive`, `PeriodNotElapsed`, `AllowanceExhausted`,
  `AllowanceOverflow`, `ForeignDelegate`, `DelegateRevoked`, `ScheduleOverflow`,
  `WrongMerchantAccount`, `WrongMint`. There is no `WrongSubscriberAccount`: the
  subscriber's account is derived by the `associated_token::` constraints, so
  Anchor's own constraint error is the failure, and a custom code would only be
  reachable if the derivation were duplicated by hand.
- [`app/programs/app/src/constants.rs`](../../app/programs/app/src/constants.rs) —
  `b"subscription"` and `b"delegate"` seeds.
- [`app/programs/app/tests/`](../../app/programs/app/tests/) — new tests.

**Risky:** `charge` is the only instruction in the product that moves someone
else's tokens. Review it against the constraint table above line by line.

## Test scenarios

Rust + LiteSVM. Time-dependent cases warp the clock rather than sleeping.

**Happy path**

1. Given an active plan, when a subscriber calls `subscribe`, then the
   `Subscription` PDA holds the plan, subscriber, and allowance, and their token
   account's delegate is the program delegate PDA with `delegated_amount` equal to
   the allowance.
2. Given a fresh subscription, when `charge` runs immediately, then the merchant's
   balance increases by exactly `amount_per_period` and the subscriber's falls by
   the same.
3. Given a subscription charged once, when the clock advances one full period and
   `charge` runs, then it succeeds and `next_charge_at` has advanced by exactly
   `period_seconds` from its previous value, not from the current time.
4. Given a live subscription, when the subscriber calls `cancel`, then the account
   is closed and the rent is returned to them.
5. Given a cancelled subscription, when the same subscriber calls `subscribe` to
   the same plan again, then it succeeds.

**Schedule**

6. Given a subscription charged once, when `charge` runs again before the period
   elapses, then it fails with `PeriodNotElapsed` and no tokens move.
7. Given a subscription three periods overdue, when a single `charge` runs, then
   exactly one `amount_per_period` moves and `next_charge_at` lands strictly in
   the future.
8. Given a subscription three periods overdue, when **two `charge` instructions
   are submitted in one transaction**, then the second fails with
   `PeriodNotElapsed` and only one charge is taken.
9. Given a subscription ten periods overdue, when `charge` runs, then
   `next_charge_at` is still congruent to the original billing date modulo
   `period_seconds` — no drift.

**Delegation**

10. Given a subscriber with a live subscription, when they `subscribe` to a second
    plan, then `delegated_amount` equals the sum of both allowances and both plans
    charge successfully.
11. Given a subscriber whose second `subscribe` would push the delegation past
    `u64::MAX`, when it runs, then it fails with `AllowanceOverflow`.
12. Given a token account whose delegate is some unrelated pubkey, when the owner
    calls `subscribe`, then it fails with `ForeignDelegate` and the existing
    delegation is untouched.
13. Given a live subscription, when the subscriber sends a plain SPL `Revoke` and a
    charge falls due, then `charge` fails with `DelegateRevoked`.
14. Given a revoked subscriber, when they `cancel` and `subscribe` again, then
    charging works once more.
15. Given two subscriptions of 100 each and an external `Approve` of 50, when both
    merchants crank, then the first succeeds and the second fails with
    `DelegateRevoked` — the pool is drawn first-come-first-served.
16. Given an allowance covering exactly one charge, when that charge runs, then it
    succeeds, `allowance_remaining` is zero, and the token program has cleared the
    delegate for the whole account.
17. Given the account in scenario 16 with a sibling subscription still open, when
    the sibling's charge falls due, then it fails with `DelegateRevoked`.

**Account substitution — the theft cases**

18. Given Alice's token account and Bob's `Subscription`, when a third party calls
    `charge`, then it fails on the associated-token constraint and Alice is not
    debited.
19. Given a subscriber holding **two** token accounts on the plan's mint — their ATA
    and a second, non-associated account they also delegated — when `charge` is
    called with the second account, then it fails on the associated-token constraint
    and that account is not debited. This is the same-owner substitution an
    `owner ==` check would have allowed.
20. Given two subscriptions of the same subscriber on the same mint to different
    merchants, when merchant A's `charge` is submitted with every account correct,
    then exactly `plan_a.amount_per_period` leaves the subscriber's ATA and lands in
    merchant A's account — there is no account either merchant can substitute to be
    paid from a different pot.
21. Given a `merchant_token_account` not owned by the plan's merchant, when
    `charge` runs, then it fails with `WrongMerchantAccount`.
22. Given a `mint` account other than `plan.mint`, when `charge` runs, then it
    fails with an address constraint error.
23. Given a `merchant_token_account` for a different mint than the plan's, when
    `charge` runs, then it fails with `WrongMint`.
24. Given a `Subscription` belonging to a different plan than the passed `plan`,
    when `charge` runs, then it fails on the `has_one`/seeds constraint.
25. Given a `Plan` address not matching the seeds for its stored merchant and
    `plan_id`, when `charge` runs, then it fails on the seeds constraint.

As in [0002](0002-subscription-plans.md), scenario 25 turns out not to be
independently reachable and has no test: the `seeds` constraint re-derives from the
plan's own stored `merchant` and `plan_id`, so a legitimately created plan always
matches its address, and substituting a *different* plan is caught by `has_one =
plan` on the subscription first — scenario 24. Scenario 17 likewise has no test of
its own; the sibling-subscription case it describes is exercised by scenario 15,
which drains the shared pool and asserts the second merchant gets
`DelegateRevoked`.

**Guards and authorisation**

26. Given `allowance == 0`, when `subscribe` runs, then it fails with
    `InvalidAllowance`.
27. Given an inactive plan, when a subscriber calls `subscribe`, then it fails with
    `PlanInactive` and no delegation is granted.
28. Given a plan the merchant retires after subscription, when `charge` falls due,
    then it fails with `PlanInactive`.
29. Given a subscription whose `allowance_remaining` is below `amount_per_period`,
    when `charge` falls due, then it fails with `AllowanceExhausted`.
30. Given a subscriber whose token **balance** is below `amount_per_period` while
    the allowance is sufficient, when `charge` falls due, then it fails cleanly.
31. Given a non-subscriber signer, when they call `cancel` on someone else's
    subscription, then it fails and the account still exists.
32. Given a duplicate `subscribe` for a plan the subscriber already holds, when it
    runs, then it fails on the `init` constraint.
33. Given a subscriber with no associated token account for the plan's mint, when
    they call `subscribe`, then it fails on the associated-token constraint rather
    than creating one.

## Risks & open questions

- **The shared pool cannot be made into reservations.** Documented above rather
  than fixed; the alternative costs an extra token account per subscription. The
  client must present `allowance_remaining` as a ceiling, not a guarantee.
- **A charge that fails on token balance is a poor error.** Scenario 28 pins that
  it fails, but the message comes from the token program. Pre-checking the balance
  would add a named error at the cost of a check the token program repeats.
  Accepted as-is; revisit if the client cannot render it usefully.
- **`charge` writes `Subscription` only**, so subscribers on the same plan do not
  contend with each other. Worth preserving — [0005](0005-variable-pricing.md)
  reads the price from `Plan` without writing it, specifically to keep this
  property, and any future change to the charge path must not regress it.
- **Requiring the ATA excludes subscribers who keep funds in a non-associated token
  account.** Accepted deliberately: it is what makes "one pool per subscriber per
  mint" an invariant rather than an assumption, and
  [0004](0004-subscriber-spending-caps.md)'s accounting depends on it. Every wallet
  creates the ATA by default, so the excluded case is rare and self-inflicted.

No open questions.

## Definition of Done

- [ ] Goals met; non-goals respected.
- [ ] Tests for the scenarios above pass (`cargo test`).
- [ ] Every `charge` debits exactly the associated token account of its own
      subscription's subscriber for the plan's mint, and no other account — covered
      by scenarios 18, 19 and 20. Stated this way rather than as "an account owned
      by the subscriber", which is a weaker property that scenario 19 violates.
- [ ] N periods of backlog collapse to exactly one charge, including within a
      single transaction — covered by scenarios 7, 8 and 9.
- [ ] Build and lint green (`anchor build`, `cargo fmt`, `cargo clippy`).
- [ ] `/audit-solana` and `/diff-review` run clean.
- [ ] No inline comments; no secrets committed.
- [ ] This plan updated to **Implemented**.
