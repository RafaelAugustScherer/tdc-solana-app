# Plans

Every non-trivial change gets a plan here **before** any code is written. A plan
is a living document: it is written up front, reviewed, and kept current as the
work lands.

## Conventions

- One file per work item: `NNNN-short-slug.md` (zero-padded, incrementing).
- Start from [`TEMPLATE.md`](TEMPLATE.md).
- One plan maps to one PR.
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
