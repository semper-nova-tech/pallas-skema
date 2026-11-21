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

Use the first one found. This skill needs `skema` 0.3.3 or newer; check with
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
- `color` (optional, on a Node, Edge, or Separator) is `red`, `yellow`,
  `green`, `cyan`, `blue`, or `magenta`. Each theme supplies its own shade, so
  it stays readable on light and dark themes. Absent means the theme's color.
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

### Shape first

Before placing anything, name the shape the data has. Count the fan at each
Node (one in and many out, many in and one out, one talking to many, a branch
with unequal halves) and pick the matching symmetry. Then place Nodes to make
that symmetry exact. A Diagram placed step by step, wrapping rows when it runs
out of room, reads as a snake; a Diagram whose shape was chosen first reads
at a glance.

| Data | Shape | How to place it |
|---|---|---|
| One source, many targets (fan-out, broadcast, dispatch) | **Triangle**: apex on top, children on one row below | Apex on the axis column; children mirrored in pairs at equal distance from it; every fan Edge `bottom` → `top` so they share one trunk |
| Many sources, one target (fan-in, aggregation, collection) | **Inverted triangle**: sources on top, apex below | Mirror of the above; fan Edges `bottom` → `top` into one trunk |
| One to many and back to one (scatter-gather, map-reduce, orchestrator with a synthesizer) | **Diamond**: triangle over inverted triangle | One axis column; the middle child sits on it; both trunks on the axis |
| One Node that every other talks to, often two-way (broker, coordinator, registry) | **Hub**: center Node, spokes on four sides | Hub on the canvas center; left and right spokes on its center row, top and bottom spokes on its center column, all at one gap; a fifth or sixth spoke on a diagonal joining the nearest trunk for its last cell |
| Stages in time where one stage fans (pipeline, producers → topic → consumers) | **Horizontal triangle or diamond**: apex on the left, children in a column | The row axis; the apex and any collector on the center row; children mirrored above and below; fan Edges `right` → `left` |
| A decision with one cheap branch and one expensive branch or loop | **Bi-modal asymmetry** | Main line straight (in, decide, respond); the cheap branch is one short Edge on the main line; the whole expensive branch sits on one side, in its own column or band; the other side stays empty so the weight is visibly unequal |
| A decision with two branches of equal weight that rejoin | **Diamond**, two children | As the diamond; do not use it for unequal branches, it hides the difference |
| A plain chain of steps | **One straight line** | One row (left to right) or one column (top to bottom). Never wrap a chain into a snake. If neither a row nor a column fits, split at a stage boundary into two rows that both read left to right, one above the other, joined by one Edge from the last Node of the first row (`bottom`) to the first Node of the second (`top`) through the band between them: a Z, not an S |

Pick the axis direction by what the reader follows: time or stage order reads
left to right, so pipelines are horizontal; hierarchy or tiers read top to
bottom (callers above, workers below), so fan-outs and architectures are
vertical. A shape may rotate (the triangle becomes horizontal) but not skew:
mirrored Nodes share a row or column and an equal distance from the axis, and
mirrored Edges take the same route.

### Make the symmetry exact

- **Fix the axis parity before footprints.** Two Nodes joined by one direct
  Edge along an axis must agree in parity across that axis: same column
  parity (both odd widths or both even) on a vertical axis, same row parity
  on a horizontal one, or the Edge jogs half a cell. Decide the spine's
  parity from its largest Node, then size the rest of the spine to match: a
  2×2 store on a 3-wide spine becomes 3×2, a 2×1 step under a 3×1 hub becomes
  3×1 or 1×1. Meaning footprints (below) yield to parity on the spine.
  Across a trunk, parity binds only between the apex and a sibling that sits
  on the axis (an odd number of siblings puts one there): those two must
  agree, or the axis line jogs where the trunk meets the apex. So a store
  facing an odd column of 2×1 siblings is 2×3 or 2×1, not 2×2; a 2×2 apex
  suits an even number of siblings, where no sibling is on the axis. Never
  inflate one sibling to match the apex; mirrored siblings always share one
  footprint.
- **Straighten every Edge that can be straight, not only the spine.** An Edge
  whose two Nodes face each other (`bottom` → `top` with one above the other,
  or `right` → `left` with one beside the other) draws straight only when
  their centers coincide across the Edge: center x = `x + columns / 2` for a
  vertical Edge, center y = `y + rows / 2` for a horizontal one. Otherwise it
  jogs: a bend halfway along a line that had no reason to turn. Check every
  such Edge, branches included, and fix it in this order:
  1. Parity already agrees (both widths odd, or both even) but the centers
     differ: move one Node, usually the one with fewer other Edges.
  2. Parity differs: grow the smaller Node by one cell across the Edge (a 2×1
     over a 3×2 becomes 3×1; a 2×1 beside a 2×2 becomes 2×2), then align.

  Leave the jog only when the fix would overlap a footprint, break a mirrored
  sibling set or the apex rule above, run past a fixed canvas, leave a band
  between rows or columns with no empty cell, or lay one Edge on top of
  another. That last one is easy to miss: a Node moved under another Node's
  port can pull a third Node's Edge onto the same line, so render and look.
  When you move a Node along a chain, move the Nodes after it by the same
  amount so the steps stay even; a gap left behind reads as a break. Never
  shrink a Node below its label budget to straighten an Edge. Some bends are
  real: an Edge that leaves
  `left` and enters `top`, or joins Nodes in different rows and columns, turns
  by design; this rule is for the ones that need not.
  Example: a 2×1 decision at (2, 7) over a 3×2 step at (2, 10) has centers 3
  and 3.5, so the `bottom` → `top` Edge jogs. Make the decision 3×1 at (2, 7).
  A 3×2 step at y = 10 with `right` → `left` into a 2×2 at y = 13 has row
  centers 11 and 14; parity agrees, so move the 2×2 to y = 10.
- **Footprint by meaning, within parity and alignment.** 2×1 for a service or
  step; 2×2 for a data store or queue; 3×1 for a gateway or wide hub; 1×1 for
  a small marker or an external actor with a label of five characters or
  fewer; 3×2 or 3×3 for a hub or a Node with a long description. Meaning sets
  the smallest footprint; a Node may grow by one cell to straighten an Edge
  (see above). Keep one footprint per row unless meaning needs another.
- **Step siblings so their center lands on the axis.** `k` siblings of width
  `w` stepped by `s` have their center at `x0 + ((k − 1)·s + w) / 2`. Choose
  `s` so that equals the spine's center: a 3-wide spine wants an odd total
  width, so four 2-wide siblings step by 4 (not 5) and two 3-wide siblings
  step by 7 (not 6). The step is `columns + 3` by default and may move by one
  either way to hit the axis; stacked rows step by `rows + 2` or more. Uneven
  steps make jogged lines; an off-axis center makes a lopsided figure.
- **Center the figure, and pick a canvas that can.** Unless the person placed
  Nodes themselves, center the Diagram on both axes: take the footprint
  bounding box (min x to max x + columns, min y to max y + rows) and shift
  every Node and Separator by `(canvas columns − 1 − used width) / 2 − min x`
  (rows the same way). That division must come out whole, so a figure of even
  width needs an odd column count and an odd width an even one; 24×15 cannot
  center a 3-wide hub, 24×14 can. Size the canvas to the figure plus one or
  two empty cells on each side; do not pick a portrait canvas and leave half
  of it empty. Render reports `off-center` when a fixed canvas is lopsided;
  treat it as a defect.
- **Even steps.** A cell is 56 px; a box is its columns × 56 + 16 px wide and
  rows × 56 − 4 px tall, centered on its cells.

### Route Edges

- **Choose sides when the facing rule would cross something.** Leave
  `fromSide`/`toSide` out for simple neighbors. Pin them when an Edge would
  cut through a box, share a port with another Edge, or double back:
  - Fan-out or fan-in: every Edge on the same side (`bottom` → `top`, or
    `right` → `left` for a horizontal fan), so they merge into one trunk. A
    trunk is the one case where many Edges share a port, and it is the right
    one: it draws as one line that splits, with one arrowhead per target.
  - A long hop to another row: leave by `right`, enter by `top`, so the line
    runs in the empty band between rows.
  - Loops and detours: same side on both ends (`top` → `top`, `left` → `left`).
    The line brackets half a cell out and goes around, not through.
  - Never let two Edges from different directions end on the same port with
    their own arrowheads, and never put a loop return on a port a forward
    Edge uses. A hub with more than four spokes puts the extra spoke on a
    diagonal and pins it to join the nearest spoke's trunk for its last cell.
- **Budget a decision's four sides.** A branch point with an input, a cheap
  exit, an expensive exit, and a loop return uses all four ports, so decide
  the ports before placing the branches. Give the return the side on the
  expensive branch's half of the Diagram; if that side is taken, move the
  input (bring it in from the left instead of the top) or let both exits
  leave by one side as a trunk, so the side you want stays free. A return
  that lands on the empty side of a bi-modal figure destroys the asymmetry.
  When both exits share a trunk, place the first Node of the expensive
  branch within one step of the decision, so the trunk splits beside the
  decision and not across the Diagram.
- **An actor at both ends is drawn twice.** When the same party starts and
  ends a flow (the user sends a request and receives the answer), give it a
  Node at each end of the spine rather than a return Edge around the figure.
- **Parity on every facing Edge.** Nodes that connect top to bottom share an
  x center and a column parity; Nodes that connect side to side share a y
  center and a row parity, so a 1-row step beside a 2-row store jogs. Same
  parity is not enough: one cell of offset jogs too (see Straighten every
  Edge).

### Labels, Separators, pulses

- **Label budgets.** About six characters per column fit on a Node label:
  1 column ≈ 5, 2 ≈ 12, 3 ≈ 18. Longer text goes in `description`, which wraps
  over the box height. Edge labels: two or three words. An Edge label sits on
  its longest segment that no other Edge shares; a horizontal segment fits
  about one character per 9 px, so a label on a short stub truncates. Widen
  the step or shorten the label.
- **The center branch of a fan has no segment of its own.** It runs straight
  through the trunk junction, so its label lands at the junction on top of the
  trunk, and `labelPosition` cannot move it. Leave that Edge unlabeled and put
  the text in the child's `description`, or label only the outer branches.
- **Leave room for labels.** Set `labelPosition` only when a label still
  collides. Keep the empty band between rows clear.
- **Separators for boundaries.** A dashed Separator marks a trust or cloud
  boundary ("AWS", "VPC", "public / private"); a solid one marks tiers or
  lanes. Run it through the empty band between rows or columns, never through
  a Node, and not on the row a trunk runs along (a trunk on the Separator's
  line hides it; give the band one more row). Span only the Nodes it divides,
  plus one cell: start one cell before the first Node's footprint and end one
  cell after the last (Nodes at x = 3 to 20 → `from.x` 2, `to.x` 21), not the
  full canvas. Its label sits at `from`, so a long line pushes the label away
  from what it names.
- **Pulses sparingly.** Pulse only flows the person asked to highlight, or the
  few that tell the story: `both` for request and response or two-way traffic,
  `forward` for a one-way stream or a scheduled push. Pulse no more than about
  a third of the Edges, except a hub whose point is two-way traffic, where
  every spoke may pulse `both`.

## Loop

1. Read the Manifest. If it does not exist, start from `{"nodes":[],"edges":[]}`.
2. For a new Diagram, name its shape (see Shape first) and the axis parity
   before writing any Node. For a change, make the smallest JSON diff that
   does it and keeps the existing shape. Then, for every Edge whose Nodes
   face each other, compare the two centers across the Edge and straighten
   any that differ (see Straighten every Edge).
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
   Check first that the shape you chose is visible: the axis is straight,
   mirrored Nodes sit at equal distances, and a bi-modal figure has its
   weight on one side. Then check for a bend in an Edge between two Nodes
   that sit in one column or one row, lines through boxes, two arrowheads on
   one port, Edges running along each other, labels on top of lines or boxes,
   and that every Edge ends in a visible arrowhead. Fix each with the
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

Leave `color` off by default: the theme's own colors already keep Nodes and
Edges apart. After the first render, you may offer once, in one line, to
personalize: switch the theme (`phosphor`, `nord`, `tokyo`, `mocha`,
`contrast`, `latte`, `tokyo-day`; the person picks in Editor Settings or
with `T`) or color a few elements, such as one path or one tier. Add `color`
only when they say yes or ask for it, and keep colors they set in the Editor.

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
