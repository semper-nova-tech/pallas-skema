#!/bin/sh
# Open the Skema terminal Editor in a Herdr pane.
#
# Inputs (environment):
#   SKEMA_MANIFEST  path to the Manifest JSON. Default: <workspace>/skema/flow.json
#   SKEMA_SVG       path to the SVG. Default: the Manifest path with .svg
#   SKEMA_BIN       path to the skema binary. Default: `skema` on PATH, then the
#                   repo build next to this plugin (target/release/skema)
set -e

json_str() {
  printf '%s' "$HERDR_PLUGIN_CONTEXT_JSON" \
    | grep -o "\"$1\"[[:space:]]*:[[:space:]]*\"[^\"]*\"" \
    | head -1 \
    | sed "s/.*\"$1\"[[:space:]]*:[[:space:]]*\"//" \
    | sed 's/"$//'
}

plugin_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

workspace=""
if [ -n "${HERDR_PLUGIN_CONTEXT_JSON:-}" ]; then
  workspace=$(json_str checkout_path)
  [ -z "$workspace" ] && workspace=$(json_str focused_pane_cwd)
  [ -z "$workspace" ] && workspace=$(json_str workspace_cwd)
fi
[ -z "$workspace" ] && workspace=$(pwd)

manifest="${SKEMA_MANIFEST:-$workspace/skema/flow.json}"
case "$manifest" in
  /*) ;;
  *) manifest="$workspace/$manifest" ;;
esac
svg="${SKEMA_SVG:-${manifest%.json}.svg}"
case "$svg" in
  /*) ;;
  *) svg="$workspace/$svg" ;;
esac

# Actions run headless (no PTY). Re-dispatch to the pane entrypoint, which
# Herdr runs with a PTY. Paths travel through --env, not --cwd, because Herdr
# spawns plugin commands from the plugin root.
if [ -z "${HERDR_PLUGIN_ENTRYPOINT_ID:-}" ] && ! { [ -t 0 ] && [ -t 1 ]; }; then
  exec "${HERDR_BIN_PATH:-herdr}" plugin pane open \
    --plugin pallas.skema \
    --entrypoint editor \
    --env "SKEMA_MANIFEST=$manifest" \
    --env "SKEMA_SVG=$svg" \
    ${SKEMA_BIN:+--env "SKEMA_BIN=$SKEMA_BIN"} \
    --focus
fi

bin="${SKEMA_BIN:-}"
if [ -z "$bin" ] && command -v skema >/dev/null 2>&1; then
  bin=$(command -v skema)
fi
if [ -z "$bin" ] && [ -x "$plugin_root/../target/release/skema" ]; then
  bin="$plugin_root/../target/release/skema"
fi
if [ -z "$bin" ]; then
  echo "skema binary not found. Run 'cargo build --release' in the pallas-skema repo or put skema on PATH." >&2
  echo "Press enter to close." >&2
  read -r _ || true
  exit 1
fi

# The Editor pane should live in the Diagram's directory so Herdr's cwd-following
# and the agent's relative paths agree.
mkdir -p "$(dirname -- "$manifest")"
cd "$(dirname -- "$manifest")"
exec "$bin" open-editor --manifest "$manifest" --svg "$svg"
