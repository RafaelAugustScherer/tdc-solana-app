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
