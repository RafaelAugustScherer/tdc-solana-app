# Anchor / Solana dev environment — versions track
# https://www.anchor-lang.com/docs/installation
# amd64 pinned: Solana's CLI + SBF build tools ship x86_64-linux only
# (emulated on Apple Silicon via Docker Desktop).
# trixie base (glibc 2.41): avm's prebuilt anchor needs glibc >= 2.39.
FROM --platform=linux/amd64 rust:1-slim-trixie

ENV DEBIAN_FRONTEND=noninteractive

# Build deps for the Solana + Anchor toolchain
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        libssl-dev \
        libudev-dev \
        llvm \
        libclang-dev \
        protobuf-compiler \
        curl \
        ca-certificates \
        gnupg \
        git \
    && rm -rf /var/lib/apt/lists/*

# Node.js + Yarn (Anchor's TypeScript tests)
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && npm install -g yarn \
    && rm -rf /var/lib/apt/lists/*

# Solana CLI
RUN sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
ENV PATH="/root/.local/share/solana/install/active_release/bin:${PATH}"

# Anchor via avm (pinned; bump with `avm install <ver> && avm use <ver>`)
RUN cargo install --git https://github.com/solana-foundation/anchor avm --force \
    && avm install 1.0.2 \
    && avm use 1.0.2
ENV PATH="/root/.avm/bin:${PATH}"

# watchexec — file watcher for the hot-reload dev loop
RUN curl -fsSL -o /tmp/watchexec.deb \
        https://github.com/watchexec/watchexec/releases/download/v2.5.1/watchexec-2.5.1-x86_64-unknown-linux-gnu.deb \
    && dpkg -i /tmp/watchexec.deb \
    && rm /tmp/watchexec.deb

WORKDIR /workspace
CMD ["bash"]
