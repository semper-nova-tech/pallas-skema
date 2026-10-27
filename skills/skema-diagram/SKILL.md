---
name: skema-diagram
description: >-
  Create or change a Pallas Skema diagram by editing its JSON Manifest and
  rendering the SVG with the skema CLI, and open the terminal Editor in a Herdr
  pane so a person can arrange it alongside you. Use when the user asks to
  diagram, sketch a flow, add or move a node, connect steps, or open Skema.
  Do not hand-draw SVG or invent a layout engine; the CLI owns rendering.
---

# Skema diagram

One JSON Manifest is the source of truth for one Diagram. The SVG is a
generated picture. You change the Manifest; `skema render` validates it and
writes the SVG. A person may have the same Manifest open in the Editor pane,
which redraws when the file changes.

## Preflight

```bash
command -v skema || ls target/release/skema
```

If neither exists, build it: `cargo build --release` in the pallas-skema repo,
then use `target/release/skema`. Do not continue without the binary.

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
  canvas starts at 16×10 and grows to fit. When it is set, every Node box must
  fit inside it, padding included, or Validation refuses the Manifest. Keep a
  canvas a person picked in the Editor; to place a Node past it, ask first or
  pick a larger canvas.

## Layout

Validation only proves a Diagram is legal. These rules make it readable; follow
them when you place Nodes or add Edges.

- **One layer or stage per row.** Put the parts of one tier (callers and core,
  the orchestration flow, the workers) on one row, top to bottom in the order
  things happen. Flows that run in steps read left to right inside their row.
- **Even steps.** A cell is 56 px; Nodes are 104 (small), 128 (medium), and
  156 (large) px wide, so a large Node spans about three cells. Step side-by-side
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
`~/.config/skema/theme`), so do not pass `--theme` unless they ask for a
specific one.

## Open the Editor pane

Only when the user asks to see or arrange the Diagram, and only inside Herdr:

```bash
test "${HERDR_ENV:-}" = 1 && herdr plugin list | grep -q pallas.skema
```

If the plugin is missing: `herdr plugin link <repo>/herdr-plugin`. Then open it
split beside your own pane, so it lands in the user's current workspace:

```bash
herdr plugin pane open --plugin pallas.skema --entrypoint editor \
  --target-pane "$HERDR_PANE_ID" --placement split --direction right \
  --env "SKEMA_MANIFEST=$(realpath <path>.json)" --focus
```

The pane autosaves: every edit the person makes is written to the Manifest and
SVG at once, so re-read the file before you edit it again.

Do not run `skema open-editor` in your own pane; it is a full-screen TUI.

## While the pane is open

- Keep editing the file. The pane reloads a clean Editor automatically.
- If the person has unsaved edits, the pane warns them instead of overwriting.
  Tell the user you changed the file and that `r` in the pane reloads it.
- Never edit the SVG. Never put commands or scripts in the Manifest.
