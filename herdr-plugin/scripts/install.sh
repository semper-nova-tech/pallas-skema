#!/bin/sh
# Make sure a skema binary exists. Prefer PATH; otherwise build the repo next to this plugin.
set -e

if command -v skema >/dev/null 2>&1; then
  echo "skema is already installed: $(command -v skema)"
  exit 0
fi

plugin_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
repo="$plugin_root/.."
if [ -f "$repo/Cargo.toml" ] && command -v cargo >/dev/null 2>&1; then
  echo "Building skema from $repo ..."
  (cd "$repo" && cargo build --release)
  echo "Built $repo/target/release/skema"
  exit 0
fi

echo "skema not found on PATH and no Rust workspace next to this plugin." >&2
echo "Install a Rust toolchain and run 'cargo build --release' in the pallas-skema repo, or put skema on PATH." >&2
exit 1
