#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
project_root="$(cd "$script_dir/.." && pwd)"

"$script_dir/bootstrap.sh"
pnpm check
cargo fmt --manifest-path "$project_root/src-tauri/Cargo.toml" --all -- --check
cargo clippy --manifest-path "$project_root/src-tauri/Cargo.toml" --all-targets -- -D warnings
cargo test --manifest-path "$project_root/src-tauri/Cargo.toml"
pnpm tauri build --no-bundle

echo "Release application is under src-tauri/target/release/."
