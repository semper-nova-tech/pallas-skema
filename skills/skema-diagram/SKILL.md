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
5. Report the Nodes and Edges you changed, the Validation outcome, and both
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
