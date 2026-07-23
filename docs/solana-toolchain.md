# Solana toolchain — solana-ai-kit

This harness defines *how* we work. It does not reimplement Solana expertise —
that comes from the **solana-ai-kit** plugin (Superteam Brasil's open-source kit:
specialist agents, build/test/deploy/audit commands, current language rules, and
docs/RPC MCP servers). The harness orchestrates it.

## Install

Install it as a **plugin** — it namespaces everything (`solana-ai-kit:anchor-engineer`,
`solana-ai-kit:audit-solana`, …), so it can't collide with this repo's own
`CLAUDE.md`, agents, or skills:

```
/plugin marketplace add solanabr/solana-ai-kit
/plugin install solana-ai-kit@stbr
```

Do **not** run the kit's `install.sh` in this repo — it copies files into
`.claude/` and would fight the harness. Use the plugin. API keys (e.g.
`HELIUS_API_KEY`) go in `.env`; wire the MCP servers with `/setup-mcp`.

## Phase → tool map

Each [workflow](workflow.md) phase has a concrete tool. The harness owns the
*process*; these are the *tools* it calls.

| Phase | Reach for |
|-------|-----------|
| Research | `solana-dev` + `context7` MCP (current docs); the `solana-researcher` agent |
| Plan | `/plan-feature` as a spec aid (account/PDA design) — capture the result in the documented plan under [`plans/`](plans/) |
| Build | specialists: `anchor-engineer`, `solana-frontend-engineer`, `defi-engineer`, `token-engineer`, `pinocchio-engineer`; commands: `/scaffold`, `/build-program`, `/build-app`, `/generate-idl-client` |
| Verify | `/test-and-fix`, `/test-rust`, `/test-ts`; `/diff-review`, `/audit-solana`, `/audit-infra`; `/benchmark`, `/profile-cu` |
| Deploy | `/deploy` (devnet first, then mainnet) |
| Env / CI | `/doctor`, `/setup-mcp`, `/setup-ci-cd` |

## MCP servers — the source of truth for "current"

Prefer these over model memory whenever an API, version, or doc detail matters
(the [engineering guidelines](engineering-guidelines.md) require it):

| Server | Use for |
|--------|---------|
| `solana-dev` | Canonical Solana docs (mcp.solana.com) |
| `context7` | Up-to-date library / SDK docs |
| `helius` | RPC, DAS, wallet / asset / transaction reads |
| `surfpool` | Local mainnet-fork to test against real state |

## Language rules

The kit's `.claude/rules/` (`anchor.md`, `rust.md`, `pinocchio.md`,
`typescript.md`) are the authoritative, current deep rules — they track Anchor 1.0
/ Solana 3.x. This repo's [`engineering-guidelines.md`](engineering-guidelines.md)
carries only the always-on essentials and defers to these for specifics.
