---
name: skema-diagram
description: >-
  Create or change a Pallas Skema diagram by editing its JSON Manifest and
  rendering the SVG with the skema CLI, and open the terminal Editor beside
  you (a Herdr pane, a tmux split, or a second terminal) so a person can
  arrange it alongside you. Use when the user asks to
  diagram, sketch a flow, add or move a node, connect steps, or open Skema.
  Do not hand-draw SVG or invent a layout engine; the CLI owns rendering.
---

# Skema diagram

One JSON Manifest is the source of truth for one Diagram. The SVG is a
generated picture. You change the Manifest; `skema render` validates it and
writes the SVG. A person may have the same Manifest open in the Editor,
which redraws when the file changes.

## Preflight

```bash
command -v skema || ls ~/.local/bin/skema || ls target/release/skema
```

Use the first one found. This skill needs `skema` 0.2.1 or newer; check with
`skema --version`. A binary that rejects `--version` is older than that.

If none exists, or it is older, tell the user and offer the one-line install,
which needs no Rust and also upgrades an old binary in place:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/semper-nova-tech/pallas-skema/releases/latest/download/skema-installer.sh | sh
```

On Windows, in PowerShell:
`powershell -ExecutionPolicy Bypass -c "irm https://github.com/semper-nova-tech/pallas-skema/releases/latest/download/skema-installer.ps1 | iex"`.
Run it only after they agree.
Inside a pallas-skema checkout, `cargo build --release` also works. Do not
continue without the binary.

## Manifest shape

```json
{
  "canvas": { "columns": 16, "rows": 10 },
  "nodes": [
    { "id": "intake", "label": "Receive request", "description": "optional longer text",
      "type": "process", "size": "medium", "gridPosition": { "x": 0, "y": 0 } }
  ],
  "edges": [
    { "from": "intake", "to": "review", "label": "optional",
      "fromSide": "right", "toSide": "left" }
  ]
}
```

Rules the validator enforces. Design to them instead of discovering them by failure:

- `id` is unique and stable. Keep ids when relabeling or moving.
- `label` is short and required. `description` is optional and shows under the
  label when the box has room; the type is never drawn, so say what matters in these two.
- `type` is `process`, `decision`, or `note`. `size` is `small`, `medium`, or `large`.
- `gridPosition` is whole non-negative Grid cells. No pixels, no fractions.
- One Node per cell. Leave at least two empty cells between Nodes side by side
  and two rows between Nodes stacked, so boxes and edge labels have room.
- Every Edge endpoint names an existing Node id. `fromSide` and `toSide`
  (`left`, `right`, `top`, `bottom`) are optional; leave them out and the
  renderer picks the side that faces the other Node. Set them only when a
  specific side matters, or keep ones a person set in the Editor.
  Two Edge ends on the same side (`left` to `left`) draw as a bracket that
  steps half a cell out.
- `canvas` is optional: a fixed size in whole Grid cells. Leave it out and the
  canvas starts at 16×10 and grows to fit. The numbers count dots: a Node
  covers its size rounded to whole cells (small and medium 2×1, large 3×1)
  plus the closing dot, so on 16×10 a medium Node fits up to x = 13, y = 8.
  When `canvas` is set, a Node past it makes Validation refuse the Manifest. Keep a
  canvas a person picked in the Editor; to place a Node past it, ask first or
  pick a larger canvas.

## Layout

Validation only proves a Diagram is legal. These rules make it readable; follow
them when you place Nodes or add Edges.

- **One layer or stage per row.** Put the parts of one tier (callers and core,
  the orchestration flow, the workers) on one row, top to bottom in the order
  things happen. Flows that run in steps read left to right inside their row.
- **Even steps.** A cell is 56 px; Nodes are 104 (small), 128 (medium), and
  156 (large) px wide, centered on 2, 2, and 3 cells. Step side-by-side
  Nodes by 5 columns and stacked rows by 3 or 4. Uneven steps make jogged lines.
- **Matching sizes on a vertical line.** Nodes that connect top to bottom
  should share an x and a size, so their center lines match and the Edge is
  straight. A medium under a large at the same x jogs a few pixels.
- **Choose sides when the facing rule would cross something.** Leave
  `fromSide`/`toSide` out for simple neighbors. Pin them when an Edge would
  cut through a box, share a port with another Edge, or double back:
  - Fan-out to a row below: every Edge `bottom` → `top`, so they share one trunk.
  - A long hop to another row: leave by `right`, enter by `top`, so the line
    runs in the empty band between rows.
  - Loops and detours: same side on both ends (`top` → `top`, `left` → `left`).
    The line brackets half a cell out and goes around, not through.
  - Never let two Edges end on the same port of one Node. If a loop and a
    forward Edge both want `delegate`'s top, move one to a bracket on the left.
- **Leave room for labels.** An Edge label sits on its longest segment. Keep
  labels to two or three words, and leave the empty band between rows clear.
- **Pick a canvas for a finished Diagram.** 24×15 suits a three-tier
  architecture; 16×10 suits a short flow. Portrait (10×16, 15×24) suits a tall
  pipeline.

## Loop

1. Read the Manifest. If it does not exist, start from `{"nodes":[],"edges":[]}`.
2. Make the requested change as the smallest JSON diff that does it.
3. Render:
   ```bash
   skema render --manifest <path>.json --svg <path>.svg
   ```
   Exit 0 means the Manifest was accepted and the SVG now matches it. Non-zero
   means Validation refused it; stderr names the rule and location. Fix the
   Manifest and render again. A refused render leaves the old SVG untouched, so
   never describe the SVG as updated after a failure.
4. Confirm the change landed with `grep aria-label <path>.svg`, which lists
   every Node label in the SVG.
5. Look at it. Render a PNG to a scratch path (not next to the Manifest unless
   asked) and open it with your image viewer or Read tool:
   ```bash
   skema render --manifest <path>.json --svg <path>.svg --png <scratch>/check.png
   ```
   Check for lines through boxes, two arrowheads on one port, Edges running
   along each other, and labels on top of lines or boxes. Fix each with the
   Layout rules and render again. Expect a second pass on any new Diagram of
   more than a few Nodes; do not report a Diagram you have not looked at.
6. Report the Nodes and Edges you changed, the Validation outcome, and both
   file paths.

`skema validate --manifest <path>.json` checks without writing anything. Add
`--png <path>.png` to `render` when the user wants a picture to paste or attach.
Renders use the theme the person chose in the Editor (kept in
`~/.config/skema/theme`); with no choice it is Match terminal, which is
Phosphor until the Editor has cached the terminal's colors. Do not pass
`--theme` unless they ask for a specific one.

## Open the Editor

Open the Editor by default after you create a new Diagram, so the person sees
it while you work. Skip it when they said not to, when it is already open, or
for small follow-up edits (an open Editor reloads by itself).

The Editor is a full-screen terminal app: never run `skema open-editor` in your
own pane or shell. Give it its own pane, tab, or window, using absolute paths:

```bash
skema open-editor --manifest <abs>.json --svg <abs>.svg
```

Pick the first host that applies.

**Herdr** (`HERDR_ENV=1`). Open it split beside your own pane:

```bash
herdr plugin pane open --plugin pallas.skema --entrypoint editor \
  --target-pane "$HERDR_PANE_ID" --placement split --direction right \
  --env "SKEMA_MANIFEST=<abs>.json" --focus
```

If `herdr plugin list` does not show `pallas.skema`, offer
`herdr plugin install semper-nova-tech/pallas-skema/herdr-plugin --yes` and
run it only after the user agrees.

**tmux** (`TMUX` is set). Split the current window:

```bash
tmux split-window -h -c "$(dirname <abs>.json)" \
  "skema open-editor --manifest <abs>.json --svg <abs>.svg"
```

**Any other terminal.** Open a new tab or window in the user's terminal app
(`TERM_PROGRAM` and the OS tell you which) running the command above. If you
cannot, give the user the command to run in a second tab.

Whichever host runs it, the Editor autosaves: every edit the person makes is
written to the Manifest and SVG at once, so re-read the file before you edit it
again.

## While the Editor is open

- Keep editing the file. A clean Editor reloads it automatically.
- If the person has unsaved edits, the Editor warns them instead of overwriting.
  Tell the user you changed the file and that `r` in the Editor reloads it.
- Never edit the SVG. Never put commands or scripts in the Manifest.
