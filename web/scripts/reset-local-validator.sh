#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../"

VALIDATOR_BIN="/root/.local/share/solana/install/releases/2.1.21/solana-release/bin/solana-test-validator"

docker compose exec dev bash -c '
  for p in /proc/[0-9]*; do
    if [ -r "$p/cmdline" ] && grep -q "solana-test-validator" "$p/cmdline" 2>/dev/null; then
      kill -9 "${p#/proc/}" 2>/dev/null || true
    fi
  done
' || true

docker compose exec -d dev bash -c "cd /workspace/app && rm -rf test-ledger && $VALIDATOR_BIN --reset > /tmp/validator.log 2>&1"

for _ in $(seq 1 30); do
  if docker compose exec dev bash -c "solana slot --url http://127.0.0.1:8899" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

docker compose exec dev bash -c '
  mkdir -p ~/.config/solana
  [ -f ~/.config/solana/id.json ] || solana-keygen new --no-bip39-passphrase -o ~/.config/solana/id.json >/dev/null
  solana config set --url http://127.0.0.1:8899 --keypair ~/.config/solana/id.json >/dev/null
  solana airdrop 50 >/dev/null
  cd /workspace/app && anchor deploy --provider.cluster localnet
'
