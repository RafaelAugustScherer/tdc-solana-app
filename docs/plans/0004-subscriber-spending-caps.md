# 0004 — Subscriber spending caps

- **Status:** Accepted
- **Author:** Rafael Scherer
- **Related:** builds on [0002](0002-subscription-plans.md) and
  [0003](0003-delegation-and-charging.md); followed by [0005](0005-variable-pricing.md)

## Context

After [0003](0003-delegation-and-charging.md) a subscriber has exactly two levers,
and both are blunt. They can pick an allowance once at `subscribe` time, and they
can `cancel`. There is no way to top up a depleted allowance, no way to pause
billing without tearing the subscription down, and — because the allowance is the
only bound — no way to say "bill me monthly, but never more than X in one go".

This plan adds the subscriber's side of the contract:

- **`max_amount_per_period`** — a per-charge cap the subscriber sets and can
  change at any time. With fixed prices it doubles as a pause switch; once
  [0005](0005-variable-pricing.md) lets merchants move prices, it becomes the hard
  bound that makes variable plans safe to subscribe to at all.
- **`set_allowance`** — change the total allowance without cancelling.
- **`reauthorize`** — restore the on-chain delegation after the subscriber has
  revoked or replaced it externally.

It also fixes an accounting gap [0003](0003-delegation-and-charging.md) documented
but deliberately left open: with several subscriptions sharing one delegation, the
program cannot tell how much of the pool belongs to which subscription, so it
cannot restore the pool correctly after an external `Revoke`. 0003's *Alternatives
considered* defers delegation arithmetic to this plan precisely so it can be
designed once, here, for every instruction that touches it.

## Goals

- A subscriber can cap what any single charge may take, and change that cap freely.
- Lowering the cap below the plan price pauses billing without losing the
  subscription; raising it resumes.
- A subscriber can raise or lower their total allowance without cancelling.
- After an external `Revoke` or `Approve`, a subscriber can restore the delegation
  to exactly what their open subscriptions require, in one instruction per wallet
  rather than one per subscription.
- Program state and the token account's `delegated_amount` are reconcilable rather
  than merely correlated.

## Non-goals

- **Variable pricing.** [0005](0005-variable-pricing.md). Here the cap is exercised
  by the subscriber lowering it, not by the merchant raising a price.
- **A merchant-published price ceiling.** Rejected on principle — see *Alternatives*.
- **Automatic top-up.** A depleted allowance stays depleted until the subscriber
  acts. Re-authorising is a deliberate decision, which is the point of the
  allowance.
- **Notifying anyone that a subscription has gone dormant.** No frontend yet; see
  *Risks*.

## Approach

### New account

**`SubscriberDelegation`** — PDA, seeds `[b"delegation", subscriber, mint]`, one per
subscriber **per mint**.

```rust
#[account]
pub struct SubscriberDelegation {
    pub subscriber: Pubkey,
    pub mint: Pubkey,
    pub committed_total: u64,
    pub bump: u8,
}
```

`committed_total` is the sum of `allowance_remaining` across that subscriber's open
subscriptions **on that mint** — the amount the delegation *should* be. It is the
piece [0003](0003-delegation-and-charging.md) lacks.

*Why it is needed:* the approve arithmetic in 0003 reads the token account's
current `delegated_amount` and adds to it, which is correct only while the pool
faithfully reflects program state. Once the subscriber revokes externally the pool
is zero while their subscriptions still expect their shares, and no instruction
that sees only one `Subscription` can work out the right total. With
`committed_total` the answer is a single read.

*Why the mint is in the seeds:* a delegation is a property of one token account, and
[0003](0003-delegation-and-charging.md) pins that account to the ATA for
`(subscriber, plan.mint)`. So a subscriber has one pool **per mint**, not one pool.
Keying this account on the subscriber alone would sum commitments across mints into
a figure that describes no pool that exists: a subscriber committing 100 USDC and
5000 BONK would carry `committed_total = 5100`, and `reauthorize` would approve
5100 on whichever ATA it was handed. Their USDC account would then advertise a 5100
delegation against the 100 they ever agreed to.

That is not merely untidy. The product's whole safety claim is that a subscriber's
exposure is a number their wallet shows them, so a `delegated_amount` inflated by
unrelated subscriptions breaks the one guarantee this design exists to provide —
even though the per-charge checks would still bound what any single merchant takes.
Keying on `(subscriber, mint)` makes the account describe exactly one real pool, and
mirrors 0003's ATA pin one-for-one.

### Creating it

`subscribe` creates the account with `init_if_needed`, subscriber pays rent; every
other instruction requires it to exist already and re-derives it from the seeds.

`init_if_needed` needs the `init-if-needed` feature on `anchor-lang`, and its usual
hazard is that a second call does *not* reset the fields. That hazard is the desired
behaviour here — a subscriber's second subscription on a mint must find the running
`committed_total`, not a zeroed one — and the arithmetic below only ever adjusts the
field by a delta, never assumes it starts at zero. `subscriber` and `mint` are
written on every call, which is a no-op after the first.

### Changed account

`Subscription` gains one field:

```rust
pub max_amount_per_period: u64,
```

The subscriber now owns two independent bounds, and the distinction matters:

- **`max_amount_per_period`** caps a *single* charge — "how much per month?"
- **`allowance_remaining`** caps the *total* across all future charges, and is
  what the delegation mirrors — "how much before I re-authorise?"

Neither subsumes the other. A high allowance with a low cap is a long-running cheap
subscription; a low allowance with a high cap is a short leash on an expensive one.

### Revised delegation arithmetic

Every instruction that changes what a subscriber owes now updates
`committed_total` and then approves exactly that figure, rather than computing a
delta against whatever the token account happens to say:

| Instruction | `committed_total` | Approves |
|---|---|---|
| `subscribe` | `+= allowance` | `committed_total` |
| `set_allowance` | `+= new − old` | `committed_total` |
| `charge` | `-= amount_per_period` | — (the CPI decrements it) |
| `cancel` | `-= allowance_remaining` | `committed_total`, best-effort |
| `reauthorize` | unchanged | `committed_total` |

Every arithmetic step is checked: `subscribe` fails with `AllowanceOverflow` if
`committed_total + allowance` would exceed `u64::MAX`, and the decrementing paths
saturate at zero rather than wrapping.

This replaces 0003's read-and-add approach. It is a deliberate revision of that
plan's accounting, not an addition alongside it — 0003 should be read as superseded
on this point once this lands, and its `subscribe` changes shape accordingly.

### `cancel` must never be blocked

The `ForeignDelegate` guard from 0003 stays on every approving path — `subscribe`,
`set_allowance`, `reauthorize` — where refusing to overwrite another protocol's
delegation is right, and where the subscriber can simply not perform the action.

**`cancel` is the exception, and applying the guard there would be a bug.** A
subscriber may hand their token account's delegate slot to an unrelated dApp at any
time with a plain `Approve`, which needs no cooperation from this program. If
`cancel`'s re-approve then failed with `ForeignDelegate`, the whole instruction
would revert: the `Subscription` would stay open, its rent locked, and
`committed_total` unreduced. The subscriber would be unable to close their own
subscription through the program until they first went and fixed the delegate
elsewhere — turning 0003's unilateral escape hatch into something that depends on
external state.

So `cancel` always performs its bookkeeping — close the `Subscription`, reduce
`committed_total` — and only attempts the re-approve when the delegate is still this
program's PDA. When it is foreign, the foreign delegation is left untouched and the
instruction still succeeds. Nothing is lost by skipping it: a foreign delegate means
this program's delegation is already gone, so there is no allowance to reduce, and
`charge` was already failing with `DelegateRevoked`.

This is the one place where "refuse rather than overwrite" and "the subscriber can
always get out" disagree, and getting out wins.

### Instructions

**`subscribe(allowance, max_amount_per_period)`** — revised from
[0003](0003-delegation-and-charging.md), which took `allowance` alone. Rejects
`max_amount_per_period == 0` (`InvalidMaxAmount`) and, before granting any
delegation, a plan already priced above the offered cap
(`PriceAboveSubscriberMax`) — subscribing to something you have already said you
will not pay for is a client bug, and failing loudly beats creating a subscription
that can never charge. It then adds `allowance` to `committed_total` and approves
the result.

**`set_max_amount(new_max)`** — subscriber signs, `has_one = subscriber`. Rejects
zero (`InvalidMaxAmount`); otherwise writes the field. It deliberately permits a
cap *below* the plan's current price: that is a subscriber saying "not at this
price", and it pauses charging without cancelling.

**`set_allowance(new_allowance)`** — subscriber signs. Rejects zero
(`InvalidAllowance`), adjusts `committed_total` by the signed difference, and
approves the result. Lowering below `allowance_remaining` is permitted and is how
a subscriber reduces exposure without cancelling.

**`reauthorize()`** — subscriber signs. Changes no program state; re-approves
`committed_total` on the token account. This is the recovery path after a plain
SPL `Revoke`, and it is one instruction per wallet rather than one per
subscription, because `committed_total` already aggregates them.

### `charge` gains one check

Before the transfer, after the due check:

```rust
require!(
    plan.amount_per_period <= subscription.max_amount_per_period,
    SubscriptionError::PriceAboveSubscriberMax
);
```

The comparison and the transfer must always read the **same** value.
[0005](0005-variable-pricing.md) replaces `plan.amount_per_period` here with a
historical price, and must move both together.

### Account constraints on the revised instructions

0003 states that for the money path the constraints *are* the design, so the
changes it makes here are given in full rather than described.

`Charge` keeps every constraint from [0003](0003-delegation-and-charging.md) —
including the `associated_token::` pin on `subscriber_token_account` — and adds:

```rust
    #[account(
        mut,
        seeds = [b"delegation", subscription.subscriber.as_ref(), plan.mint.as_ref()],
        bump = subscriber_delegation.bump,
        has_one = subscriber,
        has_one = mint,
    )]
    pub subscriber_delegation: Account<'info, SubscriberDelegation>,
```

`Cancel` is the instruction that changes shape most. In 0003 it touched no token
state at all; it now needs the token account, the mint, the delegation account and
the token program, because it re-approves the reduced figure:

```rust
#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(mut)]
    pub subscriber: Signer<'info>,

    #[account(
        seeds = [b"plan", plan.merchant.as_ref(), &plan.plan_id.to_le_bytes()],
        bump = plan.bump,
    )]
    pub plan: Account<'info, Plan>,

    #[account(
        mut,
        close = subscriber,
        seeds = [b"subscription", plan.key().as_ref(), subscriber.key().as_ref()],
        bump = subscription.bump,
        has_one = plan,
        has_one = subscriber,
    )]
    pub subscription: Account<'info, Subscription>,

    #[account(
        mut,
        seeds = [b"delegation", subscriber.key().as_ref(), plan.mint.as_ref()],
        bump = subscriber_delegation.bump,
        has_one = subscriber,
        has_one = mint,
    )]
    pub subscriber_delegation: Account<'info, SubscriberDelegation>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = subscriber,
    )]
    pub subscriber_token_account: Account<'info, TokenAccount>,

    #[account(address = plan.mint)]
    pub mint: Account<'info, Mint>,

    /// CHECK: seeds-derived delegate authority; named as the delegate, not a signer
    #[account(seeds = [b"delegate"], bump)]
    pub delegate_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}
```

Note what `delegate_authority` is doing on the approving instructions, because it is
not what it does on `charge`. SPL `Approve` is signed by the token account's
**owner**, so on `subscribe`, `set_allowance`, `cancel` and `reauthorize` the
subscriber's own signature authorises the CPI and the PDA is merely the pubkey being
named as delegate — it signs nothing. Only `charge` needs the PDA to sign, because
`transfer_checked` under a delegation is signed by the delegate. Passing it as an
`UncheckedAccount` with seed constraints covers both roles; the seeds are what stop
a caller naming some other pubkey as the delegate.

`Subscribe` gains the same `subscriber_delegation` account with `init_if_needed,
payer = subscriber` and the `system_program`. `SetAllowance` and `Reauthorize`
carry the same set as `Cancel` minus the `close`, and `Reauthorize` takes no
`Subscription` at all — it operates on the wallet's pool for one mint, which is
exactly why it needs no per-subscription accounts. It takes the `mint` explicitly,
since without a `Plan` there is nothing else to derive the pool from.

`SetMaxAmount` touches no token state: `Subscription` and the signer only.

### Pausing falls out of the existing checks

When the price exceeds a subscriber's cap, `charge` fails and — per 0003's
schedule rule — `next_charge_at` does not advance. The subscription sits dormant,
costing nothing, and resumes the moment either side moves.

Because the period has long since elapsed by then, the first charge after the
impasse lands immediately; and because 0003 collapses a backlog to exactly one
charge, it lands **once**, not once per period spent paused. No new state is needed
to express "paused" — it is the absence of a successful charge.

That interaction is worth stating explicitly because it is exactly the shape of
bug 0003's schedule arithmetic exists to prevent: a subscription that sat dormant
for ten periods must not bill ten times when it wakes.

## Alternatives considered

- **A merchant-published ceiling on the plan** instead of a subscriber-set cap.
  Rejected: it is only as meaningful as the merchant chooses to make it — nothing
  stops a plan advertising 10 with a ceiling of 10,000, which is bounded on paper
  and useless in practice, and it pushes the real check into client UI copy. A
  subscriber-set cap needs no good faith from the merchant.
- **Both ceilings.** Rejected: the subscriber's cap binds strictly tighter in
  every case that matters, so a plan ceiling adds a field, a validation, and an
  error path that can never be the operative constraint.
- **Deriving `committed_total` by scanning the subscriber's subscriptions.**
  Rejected: unbounded account reads in an instruction, and the client would have to
  pass every subscription to `reauthorize`. One `u64` maintained incrementally is
  cheaper and has no size limit.
- **A dedicated token account per subscription**, making `allowance_remaining` a
  true reservation and this account unnecessary. Rejected in 0003 and still
  rejected: rent and a funding step per plan, and a balance split across accounts
  no wallet UI shows sensibly.
- **`reauthorize` taking an explicit amount.** Rejected: the subscriber would have
  to compute the sum of their open allowances by hand, and any mistake silently
  under- or over-approves. The program already knows the number.

## Affected areas

- [`app/programs/app/src/state.rs`](../../app/programs/app/src/state.rs) — adds
  `SubscriberDelegation`; adds `max_amount_per_period` to `Subscription`.
- [`app/programs/app/src/instructions/`](../../app/programs/app/src/instructions/) —
  adds `set_max_amount`, `set_allowance`, `reauthorize`; **revises** `subscribe`,
  `charge`, and `cancel` to maintain `committed_total`.
- [`app/programs/app/src/error.rs`](../../app/programs/app/src/error.rs) — adds
  `InvalidMaxAmount`, `PriceAboveSubscriberMax`.
- [`app/programs/app/src/constants.rs`](../../app/programs/app/src/constants.rs) —
  the `b"delegation"` seed.
- [`app/programs/app/Cargo.toml`](../../app/programs/app/Cargo.toml) — enables
  `anchor-lang`'s `init-if-needed` feature for `SubscriberDelegation`.

**Risky:** this changes instructions [0003](0003-delegation-and-charging.md) already
shipped. The `subscribe` approve path is rewritten, and `charge`/`cancel` gain a
write to a new account. 0003's delegation scenarios must be re-run, not assumed.

## Test scenarios

Continuing the numbering style of the earlier plans; these are additions.

**The cap**

1. Given `subscribe` with a cap, when the `Subscription` is read, then
   `max_amount_per_period` holds it.
2. Given `max_amount_per_period == 0`, when `subscribe` runs, then it fails with
   `InvalidMaxAmount`.
3. Given a plan priced above the cap a subscriber offers, when they `subscribe`,
   then it fails with `PriceAboveSubscriberMax` and no delegation is granted.
4. Given a live subscription, when the subscriber calls `set_max_amount` below the
   plan price and a charge falls due, then it fails with `PriceAboveSubscriberMax`
   and no tokens move.
5. Given that dormant subscription, when the subscriber raises the cap above the
   price, then the next `charge` succeeds immediately.
6. Given that dormant subscription left through three further periods, when the cap
   is raised, then exactly **one** charge lands, not three.
7. Given a non-subscriber signer, when they call `set_max_amount` on someone else's
   subscription, then it fails.
8. Given `set_max_amount(0)`, when it runs, then it fails with `InvalidMaxAmount`.

**Allowance**

9. Given a live subscription, when the subscriber calls `set_allowance` above the
   current figure, then `allowance_remaining`, `committed_total`, and
   `delegated_amount` all rise by the same difference.
10. Given a live subscription, when the subscriber calls `set_allowance` below the
    current figure, then all three fall by the same difference.
11. Given `set_allowance(0)`, when it runs, then it fails with `InvalidAllowance`.
12. Given a non-subscriber signer, when they call `set_allowance`, then it fails.
13. Given a subscription whose allowance is exhausted, when the subscriber calls
    `set_allowance` and a charge falls due, then it succeeds.

**`committed_total` reconciliation**

14. Given a subscriber with two subscriptions, when both are created, then
    `committed_total` equals the sum of both allowances and matches
    `delegated_amount`.
15. Given a charge on one of them, when it settles, then `committed_total` and
    `delegated_amount` fall by the same amount and remain equal.
16. Given one subscription cancelled, when it closes, then `committed_total` falls
    by that subscription's `allowance_remaining` and the delegation is re-approved
    to the reduced figure.
17. Given a subscriber who sends a plain SPL `Revoke`, when they call
    `reauthorize`, then `delegated_amount` is restored to `committed_total` and
    **both** subscriptions charge successfully again.
18. Given a subscriber who externally approves this program's PDA for less than
    `committed_total`, when they call `reauthorize`, then the delegation is
    restored to `committed_total`.
19. Given a token account whose delegate is an unrelated program, when the owner
    calls `reauthorize`, then it fails with `ForeignDelegate` and the foreign
    delegation is untouched.
20. Given a subscriber with no open subscriptions, when they call `reauthorize`,
    then `committed_total` is zero and the instruction is a no-op rather than an
    error.
21. Given a subscriber whose `committed_total + allowance` would exceed `u64::MAX`,
    when they `subscribe`, then it fails with `AllowanceOverflow` and no
    `Subscription` is created.

**`cancel` is never blocked**

22. Given a subscriber with a live subscription who then hands the delegate slot to
    an unrelated program with a plain `Approve`, when they call `cancel`, then it
    **succeeds**: the `Subscription` closes, rent is returned, `committed_total`
    falls by the cancelled allowance, and the foreign delegation is left exactly as
    it was.
23. Given the same subscriber with a second subscription still open, when they later
    call `reauthorize` after clearing the foreign delegate, then the delegation is
    restored to the reduced `committed_total` — the figure cancel left behind, not
    the pre-cancel one.

**One pool per mint**

24. Given a subscriber with subscriptions on two different mints, when both are
    created, then two distinct `SubscriberDelegation` PDAs exist and each
    `committed_total` equals only that mint's allowance.
25. Given that subscriber, when they call `reauthorize` for one mint, then only that
    mint's ATA delegation changes and the other mint's `delegated_amount` is
    untouched.
26. Given that subscriber, when a charge settles on one mint, then the other mint's
    `committed_total` and `delegated_amount` are unchanged.

## Risks & open questions

- **A dormant subscription is silent.** Nothing on-chain tells the subscriber their
  cap has stopped their billing, or the merchant why revenue dropped. Both sides
  believe they have a live subscription until someone reads the accounts. The fix
  belongs in the frontend plan, where the client should compare each subscription's
  cap against its plan's price and surface the mismatch; recorded here so it is
  designed rather than discovered.
- **A subscriber can set a cap so high it is not a cap.** Symmetrical to the
  merchant-ceiling problem this design rejects, but materially different: the number
  is the subscriber's own, their exposure is still bounded by `allowance_remaining`,
  and they can revoke. A client should default the cap to the plan's current price
  and treat raising it as a deliberate action.
- **`committed_total` can still diverge from the pool**, because the subscriber may
  always act on their own token account. What changes is that divergence is now
  *recoverable* in one instruction instead of requiring cancel-and-resubscribe.
  Program state remains an upper bound on what a merchant can take, never a promise.
- **This plan revises 0003's instructions.** The churn is deliberate and was
  anticipated there, but it means 0003 must not be treated as frozen once merged.
- **`SubscriberDelegation` is never closed.** Once a subscriber has held a
  subscription on a mint, the account and its rent stay put even after
  `committed_total` returns to zero, unlike `Subscription`, which `cancel` closes.
  Accepted rather than fixed: a close path would have to be gated on
  `committed_total == 0` and would then be re-created by the next `subscribe`, which
  is churn for one rent-exempt account per subscriber per mint. Noted so the
  inconsistency with `Subscription` is a decision rather than an oversight.

No open questions.

## Definition of Done

- [ ] Goals met; non-goals respected.
- [ ] Tests for the scenarios above pass, **and** 0003's delegation scenarios still
      pass against the revised arithmetic.
- [ ] No charge can exceed the subscriber's own `max_amount_per_period` — covered
      by scenarios 3 and 4.
- [ ] A subscription paused by its cap resumes on exactly one charge, not one per
      period paused — covered by scenario 6.
- [ ] `reauthorize` restores every open subscription of a wallet **on one mint** in
      one instruction — covered by scenario 17.
- [ ] A subscriber's `delegated_amount` on a mint never reflects commitments made on
      any other mint — covered by scenarios 24, 25 and 26.
- [ ] `cancel` succeeds whatever the token account's delegate currently is —
      covered by scenario 22.
- [ ] Build and lint green (`anchor build`, `cargo fmt`, `cargo clippy`).
- [ ] `/audit-solana` and `/diff-review` run clean.
- [ ] No inline comments; no secrets committed.
- [ ] This plan updated to **Implemented**, and 0003 annotated where this
      supersedes its delegation arithmetic.
