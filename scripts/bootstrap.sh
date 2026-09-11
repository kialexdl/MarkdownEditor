#!/usr/bin/env bash
set -euo pipefail

for command_name in git node pnpm cargo rustc; do
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "Missing required command: $command_name" >&2
    exit 1
  fi
done

git --version
node --version
pnpm --version
rustc --version
if [[ -f pnpm-lock.yaml ]]; then
  pnpm install --frozen-lockfile
else
  echo "pnpm-lock.yaml is missing; generating it on this first dependency install." >&2
  pnpm install --no-frozen-lockfile
fi

echo "Dependencies are ready. Run: pnpm tauri dev"
