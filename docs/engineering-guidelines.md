# Engineering guidelines

How code in this repository is written. Each rule carries a *why* so you can use
judgment at the edges instead of matching the letter of the rule.

## Principles

- **Match the surrounding code's patterns.** *Why:* consistency beats local
  cleverness. If a helper already exists, use it — don't add a second one that
  does almost the same thing.
- **Solve the case in front of you.** *Why:* speculative options bags, generic
  interfaces, and "what if we need X later" branches are the most common form of
  overengineering and are hard to remove later. Three known inputs today → handle
  three.
- **Prefer straight-line code over clever abstraction.** *Why:* a short `if/else`
  usually reads faster than a lookup driven by a config object. Reach for an
  abstraction once there are 3+ real callers, not in anticipation of them.
- **When a choice is unclear, stop and ask.** *Why:* a 5-second clarification is
  cheaper than a wrong-shape rewrite. If two paths lead to materially different
  outcomes, surface both in the plan instead of silently picking one.

## Naming

- Name things for what they hold or do: `activeUser`, not `canonicalUserRecord`;
  `dbUrl`, not `primaryDatabaseConnectionString`. *Why:* plain names read faster.
- Booleans read as predicates: `isFunded`, `hasAuthority`.
- No abbreviations that a newcomer would have to decode.

## Functions & structure

- Small functions with one job. If you need a comment to explain a block, extract
  it into a well-named function instead.
- Keep nesting shallow — prefer early returns over deep `if` pyramids.
- Pure logic separate from I/O (RPC calls, disk, network) so it can be tested.

## Comments

- **No explanatory comments.** *Why:* well-named identifiers and small functions
  explain *what*; the plan and PR explain *why*. Comments rot the moment the code
  around them changes. If a reader couldn't follow the code without prose, the
  code is wrong — rename, restructure, or split until they can.
- The **only** allowed comments are markers that announce their own removal:
  `TODO`, `FIXME`, `HACK`, `XXX` — and each should say when it comes out, e.g.
  `// TODO: drop once the devnet faucet limit is lifted`.
- Doc comments on **public, exported APIs** are allowed when they document the
  contract (params, returns, errors) — not when they narrate the implementation.

## Error handling

- Fail loudly and early; never swallow an error to keep going.
- Error messages name what failed and the input that caused it.
- On the client, handle the states a chain forces on you: retries, confirmation
  waits, and recovery. *Why:* robust client code is the real product moat —
  chains don't have loading spinners; the app does.

## TypeScript

> Deep rules: **solana-ai-kit** `.claude/rules/typescript.md`. Defer to it.

- `strict` mode; no `any` — reach for `unknown` and narrow.
- No `console.log` in committed code; use a logger or remove it.
- Prefer `@solana/kit` and generated (Codama/Anchor) clients over hand-rolled
  request building.
- Format with the repo's Prettier config (`yarn lint:fix` in `app/`).

## Rust / Anchor

> Authoritative, current deep rules live in the **solana-ai-kit** plugin —
> `.claude/rules/anchor.md`, `rust.md`, `pinocchio.md`. They track Anchor 1.0 /
> Solana 3.x (`@anchor-lang/core`, `transfer_checked`, LiteSVM/Surfpool). Defer
> to them; the bullets here are the always-on essentials.

- Let Anchor's macros do the account checks (ownership, signer, seeds, rent)
  rather than re-implementing them by hand.
- Validate every account and input; never trust a passed account's contents.
- Keep instructions small; push shared logic into helpers in the program crate.
- Run `cargo fmt` and `cargo clippy`; treat clippy warnings as errors.

## Solana security

> Run the **solana-ai-kit** security gates on every change — `/diff-review` and
> `/audit-solana` (they wrap the Trail of Bits scanner and safe-solana-builder).
> The checklist below is the minimum you verify by hand.

- Check `is_signer` / ownership on every account an instruction mutates.
- Derive PDAs, don't accept them as input; verify the bump.
- Guard against arithmetic overflow (checked math) and missing-rent-exemption.
- Never log or commit private keys, seeds, or `.env` values.

## Testing

- Every change ships with tests that map to the plan's test scenarios.
- Program logic: integration tests via the framework the repo is configured for.
  Don't hardcode a runner here — Anchor 1.0 defaults to LiteSVM/Surfpool; `app/`
  config and the kit's rules are the source of truth.
- Prefer a failing test first, then the code that makes it pass.
- A change isn't done until the test suite is green.

## Dependencies

- **Check the current state before adding or upgrading anything.** *Why:* model
  knowledge lags reality. Confirm the latest version, scan for known advisories,
  and verify the API you'll call still exists in that version's docs. Use the
  `solana-dev` and `context7` MCP servers for current Solana and library docs
  instead of relying on memory.
- Prefer the ecosystem default with real adoption over the clever niche pick.
- No dependency for something the standard library or an existing dep already does.

## Formatting & lint

- TypeScript: Prettier (`yarn lint` / `yarn lint:fix`).
- Rust: `cargo fmt` + `cargo clippy`.
- CI-equivalent checks must pass locally before a PR is opened.

## Git hygiene

- Conventional Commits; imperative subject under ~72 chars.
- Small, focused commits; one PR per accepted plan.
- Never commit build artifacts, secrets, or wallet keypairs (see `.gitignore`).
