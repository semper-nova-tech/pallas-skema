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

Use the first one found. This skill needs `skema` 0.3.0 or newer; check with
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
      "type": "process", "size": "medium", "gridPosition": { "x": 0, "y": 0 } },
    { "id": "db", "label": "Orders DB", "type": "process",
      "size": { "columns": 2, "rows": 2 }, "gridPosition": { "x": 5, "y": 0 } }
  ],
  "edges": [
    { "from": "intake", "to": "review", "label": "optional",
      "fromSide": "right", "toSide": "left",
      "labelPosition": "auto", "pulse": "forward" }
  ],
  "separators": [
    { "id": "aws", "from": { "x": 0, "y": 2 }, "to": { "x": 15, "y": 2 },
      "style": "dashed", "label": "AWS" }
  ]
}
```

Rules the validator enforces. Design to them instead of discovering them by failure:

- `id` is unique and stable. Keep ids when relabeling or moving.
- `label` is short and required. `description` is optional and shows under the
  label when the box has room; the type is never drawn, so say what matters in these two.
- `type` is `process`, `decision`, or `note`. It is never drawn; use `process`
  unless the Node really is a branch point or an annotation.
- `size` is a Footprint: `{ "columns": c, "rows": r }` with c and r each 1 to 3,
  or a shorthand: `small` and `medium` are 2×1, `large` is 3×1.
- `gridPosition` is whole non-negative Grid cells. No pixels, no fractions.
- Footprints may not share a cell; Validation refuses an overlap and names both
  Nodes. Spacing is yours to keep (see Layout).
- Every Edge endpoint names an existing Node id. `fromSide` and `toSide`
  (`left`, `right`, `top`, `bottom`) are optional; leave them out and the
  renderer picks the side that faces the other Node. Set them only when a
  specific side matters, or keep ones a person set in the Editor.
  Two Edge ends on the same side (`left` to `left`) draw as a bracket that
  steps half a cell out.
- `labelPosition` (optional) is `auto` (default), `start`, `middle`, or `end`:
  which segment of the Edge carries its label.
- `pulse` (optional) is `none` (default), `forward` (one way), or `both`
  (back and forth: two-way traffic). A `both` Edge has an arrowhead at each end.
- `separators` (optional) are lines on Grid lines: `from` and `to` share an x
  (vertical) or a y (horizontal), `style` is `solid` (default) or `dashed`,
  `label` is optional. A Separator may not touch a Node footprint; Edges may
  cross it.
- `canvas` is optional: a fixed size in whole Grid cells. Leave it out and the
  canvas starts at 16×10 and grows to fit. The numbers count dots: a Node
  covers its size rounded to whole cells (small and medium 2×1, large 3×1)
  plus the closing dot, so on 16×10 a 2×1 Node fits up to x = 13, y = 8.
  When `canvas` is set, a Node past it makes Validation refuse the Manifest. Keep a
  canvas a person picked in the Editor; to place a Node past it, ask first or
  pick a larger canvas.

## Layout

Validation only proves a Diagram is legal. These rules make it readable; follow
them when you place Nodes or add Edges.

- **One layer or stage per row.** Put the parts of one tier (callers and core,
  the orchestration flow, the workers) on one row, top to bottom in the order
  things happen. Flows that run in steps read left to right inside their row.
- **Center by default.** Unless the person placed Nodes themselves, center the
  Diagram on both axes: equal empty columns left and right, equal empty rows
  top and bottom, within one cell. Take the footprint bounding box
  (min x to max x + columns, min y to max y + rows) and shift every Node and
  Separator by `(canvas columns − 1 − used width) / 2 − min x` (rows the same
  way). For a symmetric fan-out, put the hub on the center column. Render
  reports `off-center` when a fixed canvas is lopsided; treat it as a defect.
- **Footprint by meaning.** 2×1 for a service or step; 2×2 for a data store or
  queue; 3×1 for a gateway or wide hub; 1×1 for a small marker or an external
  actor with a label of five characters or fewer; 3×2 or 3×3 for a Node with a
  long description. Keep one footprint per row unless meaning needs another.
- **Even steps.** A cell is 56 px; a box is its columns × 56 + 16 px wide and
  rows × 56 − 4 px tall, centered on its cells. Step side-by-side Nodes by
  `columns + 3` (5 for 2-wide, 6 for 3-wide) and stacked rows by `rows + 2` or
  more. Uneven steps make jogged lines.
- **Width parity on a vertical line.** Nodes that connect top to bottom should
  share an x center and have the same number of columns (or both odd or both
  even), so the Edge is straight. A 2-wide under a 3-wide jogs half a cell.
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
- **Label budgets.** About six characters per column fit on a Node label:
  1 column ≈ 5, 2 ≈ 12, 3 ≈ 18. Longer text goes in `description`, which wraps
  over the box height. Edge labels: two or three words.
- **Leave room for labels.** An Edge label sits on its longest segment that no
  other Edge shares, so fan-out labels each land on their own branch. Set
  `labelPosition` only when a label still collides. Keep the empty band
  between rows clear.
- **Separators for boundaries.** A dashed Separator marks a trust or cloud
  boundary ("AWS", "VPC", "public / private"); a solid one marks tiers or
  lanes. Run it through the empty band between rows or columns, never through
  a Node.
- **Pulses sparingly.** Pulse only flows the person asked to highlight, or the
  few that tell the story: `both` for request and response or two-way traffic,
  `forward` for a one-way stream or a scheduled push. Pulse no more than about
  a third of the Edges.
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

   Then read stderr for Label warnings, even on exit 0:
   `warning[label-truncated]` (text was shortened), `warning[label-overlap]`
   (a label sits on a box or another label), `warning[off-center]`. Fix each
   one and render again; `--warnings=json` gives the same list as JSON. Never
   report a Diagram with a warning you have not fixed or explained. Do not
   pass `--strict`; that flag is for CI.
4. Confirm the change landed with `grep aria-label <path>.svg`, which lists
   every Node label in the SVG.
5. Look at it. Render a PNG to a scratch path (not next to the Manifest unless
   asked) and open it with your image viewer or Read tool:
   ```bash
   skema render --manifest <path>.json --svg <path>.svg --png <scratch>/check.png
   ```
   Check for lines through boxes, two arrowheads on one port, Edges running
   along each other, labels on top of lines or boxes, and that every Edge ends
   in a visible arrowhead. Fix each with the
   Layout rules and render again. Expect a second pass on any new Diagram of
   more than a few Nodes; do not report a Diagram you have not looked at.
6. Report the Nodes and Edges you changed, the Validation outcome, and both
   file paths.

`skema validate --manifest <path>.json` checks without writing anything. When
the user wants a picture to paste or attach: if any Edge pulses, add
`--gif <path>.gif` (as sharp as the PNG, looping; the PNG cannot show motion) and still look
at a PNG for the layout check; otherwise add `--png <path>.png`. The SVG
animates by itself in a browser. Report the path of every file written.
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
