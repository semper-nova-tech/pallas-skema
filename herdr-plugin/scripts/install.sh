#!/bin/sh
# Make sure a skema binary exists. Prefer PATH; otherwise download the latest
# release; otherwise build the repository next to this plugin (contributors).
set -e

installer="https://github.com/semper-nova-tech/pallas-skema/releases/latest/download/skema-installer.sh"

if command -v skema >/dev/null 2>&1; then
  echo "skema is already installed: $(command -v skema)"
  exit 0
fi
if [ -x "$HOME/.local/bin/skema" ]; then
  echo "skema is already installed: $HOME/.local/bin/skema"
  exit 0
fi

if command -v curl >/dev/null 2>&1; then
  echo "Installing skema from the latest release ..."
  if curl --proto '=https' --tlsv1.2 -LsSf "$installer" | sh; then
    exit 0
  fi
  echo "Release install failed; trying a source build." >&2
fi

plugin_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
repo="$plugin_root/.."
if [ -f "$repo/Cargo.toml" ] && command -v cargo >/dev/null 2>&1; then
  echo "Building skema from $repo ..."
  (cd "$repo" && cargo build --release)
  echo "Built $repo/target/release/skema"
  exit 0
fi

echo "Could not install skema. Run:" >&2
echo "  curl --proto '=https' --tlsv1.2 -LsSf $installer | sh" >&2
exit 1
