# Plans

Every non-trivial change gets a plan here **before** any code is written. A plan
is a living document: it is written up front, reviewed, and kept current as the
work lands.

## Conventions

- One file per work item: `NNNN-short-slug.md` (zero-padded, incrementing).
- Start from [`TEMPLATE.md`](TEMPLATE.md).
- One plan maps to one PR.
- Add every new plan to the Index below — that list is the map.
- Keep the plan honest: if the implementation diverges, update the plan in the
  same change — never let the two drift apart.

## Status

Each plan carries a status in its header:

- **Draft** — being written, not yet reviewed.
- **Accepted** — reviewed; implementation may start.
- **Implemented** — merged and verified against its Definition of Done.
- **Superseded** — replaced by a later plan (link it).

## Index

| # | Title | Status |
|---|-------|--------|
| [0001](0001-development-harness.md) | Development harness | Implemented |
| [0002](0002-subscription-plans.md) | Subscription plans | Implemented |
| [0003](0003-delegation-and-charging.md) | Delegation and charging | Implemented |
| [0004](0004-subscriber-spending-caps.md) | Subscriber spending caps | Implemented |
| [0005](0005-variable-pricing.md) | Variable pricing | Implemented |

Plans 0002–0005 build one product — revocable subscriptions — in sequence, and
should be implemented in order. Each is a separate PR.
