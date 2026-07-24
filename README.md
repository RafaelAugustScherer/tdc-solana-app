# workshop-solana

Anchor/Solana development in Docker — no local Rust, Solana CLI, or Anchor install
needed. The Anchor program lives in [`app/`](app/); editing its source on your host
triggers a rebuild inside the container.

## Hot-reload dev loop

```bash
docker compose up
```

This builds the image on first run (several minutes — it compiles `avm`), then
starts a watcher on `app/programs`. Every time you save a `.rs` or `.toml` file
on your host, it runs `anchor build` inside the container. Stop with `Ctrl-C`.

The watcher polls the filesystem (`--poll`) instead of using inotify, because
inotify events don't cross Docker Desktop's bind mount on macOS.

> The first `anchor build` also downloads the Solana SBF toolchain and compiles
> from scratch, so it's slow. Later builds are incremental. The image is
> `linux/amd64` (Solana ships x86_64-linux only), so on Apple Silicon everything
> runs under Docker Desktop emulation — slower than native.

## Open a shell

While the dev loop is running:

```bash
docker compose exec dev bash
```

Or start a one-off shell (with the validator ports published):

```bash
docker compose run --rm --service-ports dev bash
```

From the shell you can run the usual commands (`anchor build`, `anchor test`,
`anchor deploy`, `solana-test-validator`, …). Ports `8899`/`8900` expose the
local validator's RPC and WebSocket for a host frontend.

## Verify the toolchain

```bash
docker run --rm anchor-dev bash -c \
  "rustc --version && solana --version && anchor --version && node --version"
```

> The Solana CLI is pinned below 3.x in the [`Dockerfile`](Dockerfile):
> `solana-test-validator` on Agave 3.x+ hard-requires `io_uring`, which panics
> under this container's emulated/virtualized kernel. `anchor build`/`anchor
> deploy` still activate a newer release internally for their own SBF
> toolchain — that's expected. If you need to run `solana-test-validator`
> directly, invoke the pinned release explicitly rather than relying on
> `active_release`/`PATH`, e.g.
> `~/.local/share/solana/install/releases/2.1.21/solana-release/bin/solana-test-validator`.

## Web frontend

[`web/`](web/) is a Vite + React client for the program, driven by
`@solana/kit`. It runs on the **host**, not in the Docker container — it needs
no Solana/Anchor toolchain, only Node.

```bash
cd web
yarn install
yarn dev
```

By default it points at a local validator (`http://127.0.0.1:8899`, the same
ports `docker-compose.yml` already publishes). Copy `web/.env.example` to
`web/.env` and follow the comments there to point it at devnet instead — the
program has not been deployed to devnet yet, so that step is a prerequisite,
not a config toggle.

`web/`'s generated client (`web/src/generated/`) is regenerated from the
program's IDL, which only exists after an `anchor build`. The two dev loops
are independent processes but share this one dependency:

```bash
docker compose exec dev anchor build   # regenerates app/target/idl/app.json
cd web && yarn codegen                 # regenerates web/src/generated/ from it
```

See [`web/README.md`](web/README.md) for the rest of `web/`'s commands, and
[`docs/plans/0006-web-frontend.md`](docs/plans/0006-web-frontend.md) for the design.

## Development

This repo runs on a **plan-first** workflow — no non-trivial change starts in
code. Start here:

- [`CLAUDE.md`](CLAUDE.md) — the project constitution and the one rule.
- [`docs/workflow.md`](docs/workflow.md) — research → plan → review → build → verify.
- [`docs/engineering-guidelines.md`](docs/engineering-guidelines.md) — how code is written.
- [`docs/plans/`](docs/plans/) — a documented plan per work item.
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — the short version.
