# Product Spec: Pallas Skema v0.3 — Expressive Diagrams

Status: Draft for review. Builds on [skema-v1](skema-v1.md), which stays in force
except where a requirement below changes it.

## Changed in this draft

### Added

- Label warnings on stderr, with a strict mode (FR-1 to FR-5)
- Edge label placement that avoids shared segments, with an optional override (FR-6 to FR-9)
- Arrowhead visibility fix (FR-10 to FR-12)
- Node footprints from 1×1 to 3×3 Grid cells (FR-13 to FR-19)
- Separators, solid or dashed (FR-20 to FR-25)
- Edge pulse animation in the SVG, the Editor, and a new GIF output (FR-26 to FR-35)
- Editor Copy menu with PNG, SVG, and GIF (FR-36 to FR-40)
- Skill rules for footprints, separators, pulses, label warnings, and centering (SR-1 to SR-9)
- Glossary terms Footprint, Separator, Pulse, Label warning, Copy menu

### Changed

- Glossary row Node size, which is now a Footprint with keyword shorthands

### Removed

- The requirement that v0.2 Manifests render byte-for-byte the same. There are
  no long-term users yet, so compatibility is not a goal.

### Resolved

- Q-shape, Q-keyword-sizes, Q-gif-size, Q-svg-animation-tech,
  Q-forward-compat, and Q-strict-default (see Risks and open questions)

## Terms

This PRD uses these rows from [the glossary](../GLOSSARY.md): Manifest, SVG,
Grid, Node, Edge, Node size, Footprint, Separator, Pulse, Label warning,
Copy menu, Canvas, Validation, Diagram, Human reviewer, Agent, Editor,
Command-line tool.

## Problem

An Agent drew a three-tier AWS architecture (client, API Gateway with an
authorizer, three services, a shared DB, a search index, and an indexer cron
job). The result was readable, but a review of the PNG found these gaps:

1. **Labels were cut off without any warning.** "Front-end client" rendered as
   "Front-end cli…". The Agent only noticed because it looked at the PNG.
   Validation passed, and stderr said nothing.
2. **Arrowheads into the top of a Node were hidden.** The Node's label is drawn
   on a background rectangle that sits on the top border. That rectangle is
   painted after the Edges and covers any arrowhead that lands on the top port.
   Every downward arrow looked like a flat tick.
3. **Edge labels could not be used on fan-out or fan-in Edges.** A label goes
   on the longest segment. For fanned Edges, that segment is the horizontal
   line they share, so every label lands in the same place.
4. **Every Node had the same proportions.** Only 2×1 and 3×1 boxes exist, so a
   database, a scheduled job, and a service all look alike, and a tall Node
   with a long description is impossible.
5. **There was no way to show a boundary.** The request said "in AWS", but
   nothing could mark what sits inside AWS.
6. **Direction and two-way traffic were hard to read.** The authorizer call
   goes there and back, but it was drawn one way.
7. **The Diagram was off-center.** Four columns were empty on the left while
   Nodes touched the right edge.
8. **Copying was limited.** The Editor copies a PNG with one button, or SVG
   text with a key the Human reviewer has to know about.

## Today

- `skema render` and `skema validate` exit 0 on a legal Manifest and write
  nothing to stderr. A long label or description is silently shortened with an
  ellipsis (`fit_mono` and `wrap_mono` in `crates/skema-core/src/render.rs`).
- An Edge label is drawn on the longest segment of its route (`longest_segment`).
- Edges and their arrowheads are drawn before Nodes. Each Node's label sits on
  the top border over a background rectangle that hides the border behind the
  text, and that rectangle can cover an arrowhead.
- `size` is `small`, `medium`, or `large`. All three are one row tall. Small
  and medium cover 2 cells and large covers 3.
- A Manifest has `canvas`, `nodes`, and `edges`, and nothing else.
- The SVG is static. The PNG is rasterized from the SVG by `resvg`. There is no
  GIF output.
- The terminal Editor (`skema-tui`, using ratatui) has one "⎘ Copy image"
  button (PNG) and the keys `y` (PNG) and `Y` (SVG text). It redraws every
  250 ms, and every 30 ms while a button flash is showing.

## Users and context

Personas are unchanged ([PERSONAS.md](../PERSONAS.md)):

- The **Agent** writes the Manifest and checks the result without seeing it,
  apart from an optional PNG. It needs machine-readable warnings so it doesn't
  depend on spotting problems by eye.
- The **Human reviewer** arranges the Diagram in the Editor and shares it, by
  pasting it into chat, attaching it to a PR, or putting it in docs.

## Desired outcome and success measures

- An Agent can tell from stderr alone that a label, description, or Edge label
  was shortened or overlaps something.
- Every Edge's arrowhead is fully visible in the SVG, PNG, GIF, and Editor.
- Labels on fanned Edges sit on that Edge's own segment and don't overlap
  each other.
- A Node can be any whole-cell size from 1×1 to 3×3.
- A Diagram can show boundaries with solid or dashed Separators.
- An Edge can pulse one way or both ways. The pulse shows in the SVG in a
  browser, in the Editor, and in a GIF.
- The Human reviewer can copy the Diagram as PNG, SVG, or GIF from one menu.
- Diagrams the Agent writes are centered by default.

## User stories

1. An Agent renders a Manifest whose label doesn't fit. stderr names the Node,
   the full label, the shortened text, and how many characters fit, so the
   Agent can fix it without looking at the picture.
2. An Agent in a CI or check loop passes `--strict`, so any Label warning fails
   the render.
3. A Human reviewer sees an arrowhead at the end of every Edge, including Edges
   that enter a Node from the top.
4. An Agent labels three fan-out Edges (`/a`, `/b`, `/c`), and each label sits
   on its own Edge.
5. An Agent draws a database as a 2×2 Node and a long-description note as a
   3×3 Node.
6. An Agent draws a dashed Separator labeled "AWS" between the client row and
   the gateway row.
7. An Agent marks the authorizer Edge as a two-way pulse and the cron-to-index
   Edge as a one-way pulse. The Human reviewer sees both move in the Editor and
   in the SVG in a browser.
8. A Human reviewer opens the Copy menu and picks GIF to paste an animated
   picture into chat, or SVG to paste markup into a document.
9. An Agent asked for a picture of an animated Diagram writes a GIF instead of
   a PNG.

## In scope

- Label warnings in `render`, `validate`, and the Editor, plus a `--strict` flag
- Edge label placement and an optional `labelPosition` on an Edge
- The arrowhead paint-order fix
- Footprints from 1×1 to 3×3, with `small`, `medium`, and `large` kept as shorthands
- A top-level `separators` list
- An Edge `pulse` field, SVG animation, Editor animation, and `--gif` output
- An Editor Copy menu
- `skills/skema-diagram/SKILL.md` updates (SR-1 to SR-9)

## Out of scope

- Visual shapes other than the rounded box (cylinders, actors, icons). See
  Q-shape.
- Groups or containers that move as one unit in the Editor. Separators are
  lines only.
- Diagonal or free-form Separators
- Animation of anything other than Edge pulses
- Automatic layout. Centering is a skill rule, plus an optional warning (FR-5).
- MP4 or WebM output

## Functional requirements

### Label warnings

1. When `render` or `validate` shortens a Node label, a Node description, a
   Separator label, or an Edge label to fit, it writes one Label warning line
   per item to stderr. The line names the kind of item, its id (for an Edge,
   `from→to`), the full text, the text actually drawn, and the number of
   characters that fit. Example:
   `warning[label-truncated] node "client": "Front-end client" → "Front-end cli…" (fits 13 chars at 2×1)`
2. Each item also gets a Label warning when its drawn label box overlaps a
   Node box, a Separator label, or another Edge label
   (`warning[label-overlap]`).
3. Label warnings never block output. Exit code 0 is unchanged, and the SVG,
   PNG, and GIF are written. With `--strict`, any warning makes the command
   exit non-zero, write no outputs, and leave existing outputs unchanged. This
   follows the same rule as a Validation failure.
4. `--warnings=json` writes the warnings to stderr as one JSON array of
   `{code, kind, id, text, drawn, fits}`, for Agents and CI to parse.
5. `warning[off-center]` is raised when the Diagram's left and right margins
   differ by more than 1 column, or its top and bottom margins differ by more
   than 1 row. This only applies when `canvas` is set. Without `canvas`, the
   Diagram is never flagged.
6. The Editor shows the current Label warnings in its status area. Clicking a
   warning selects the item it names.

### Edge label position

7. A label is placed on the longest segment of an Edge that no other Edge also
   uses. Two segments count as shared when they overlap by more than half a
   cell along the same line. If every segment is shared, the label goes on the
   last segment before the Edge enters its target.
8. An Edge may set `labelPosition` to `auto` (the default, as in FR-7),
   `start` (the first segment), `middle` (the longest segment, which is
   today's behavior), or `end` (the last segment).
9. If a placed label would still overlap a Node box or another label, it moves
   along its segment until it is clear. If no clear spot exists, it stays at
   the segment midpoint and gets a `label-overlap` warning (FR-2).

### Arrowhead fix

10. Every arrowhead is fully visible. No Node label, label background, Node
    box, or Separator is painted over any part of an arrowhead.
11. When an Edge ends at a top port, comes down from above, and the port's x
    falls inside that Node's label background, the Edge stops at the top of
    the label background and the arrowhead tip sits there, pointing at the
    label. The label is never moved or shortened for this. Arrowheads are
    painted last.
12. The Editor follows the same rule. An arrowhead glyph is never replaced by
    a label character.

### Footprints

13. `size` accepts either a keyword (`small`, `medium`, `large`) or an object
    `{ "columns": c, "rows": r }` where `c` and `r` are each whole numbers from
    1 to 3.
14. The keywords are only shorthands for footprints: `small` and `medium` mean
    `{columns: 2, rows: 1}` and `large` means `{columns: 3, rows: 1}`. They draw
    exactly like the matching object. `small` no longer draws narrower than
    `medium`.
15. A box covers `columns × rows` cells, starting at `gridPosition`, its
    top-left cell. Box pixel size comes from the style. Box width is
    `columns × cell − inset` and box height is `rows × cell − inset`. The
    Technical spec sets `inset`.
16. Validation refuses two footprints that share a cell, and checks Canvas
    bounds against the full footprint. Spacing between Nodes (two empty cells
    side by side, two empty rows stacked) is a skill rule (SR-3), not a
    Validation rule.
17. Ports stay at the midpoint of each side. A Node with an odd number of
    columns has its top and bottom ports half a cell away from those of a Node
    with an even number of columns. SR-2 tells the Agent how to avoid jogged
    lines.
18. Descriptions wrap over the full interior height: at most `2 × rows − 1`
    lines (1, 3, or 5). Anything that doesn't fit is shortened with a Label
    warning.
19. The Editor can resize a selected Node between 1×1 and 3×3, using keys and
    the panel. It refuses a resize that would fail Validation, as it does for
    moves (v1 FR-12).

### Separators

20. A Manifest may have a top-level `separators` list. Each item is
    `{ "id", "from": {x, y}, "to": {x, y}, "style": "solid" | "dashed", "label"? }`.
21. A Separator runs along Grid lines and is either horizontal (same `y`) or
    vertical (same `x`). Validation refuses a diagonal Separator, a
    zero-length Separator, a duplicate id, and a Separator outside the Canvas.
22. A Separator may not pass through a Node footprint. Edges may cross a
    Separator.
23. A Separator draws as a 1.5 px line in a dim line color. A dashed Separator
    uses 6 px dashes with 4 px gaps. It is painted under Edges and Nodes.
24. An optional `label` is drawn at the start of the line, on the line, with a
    background behind it like a Node label. It is checked for Label warnings.
25. In the Editor, a Separator is drawn with `─`/`│` (solid) or `╌`/`╎`
    (dashed). It can be selected, moved, deleted, and added. Adding one means
    clicking its two end points.

### Pulse animation

26. An Edge may set `pulse` to `none` (the default), `forward` (from `from` to
    `to`), or `both` (back and forth, meaning two-way traffic).
27. In the SVG, a pulsing Edge gets an overlay: a short bright dash that moves
    along the Edge path. The movement is done with CSS `@keyframes` on
    `stroke-dashoffset`, in the SVG's own `<style>`, so it plays wherever the
    SVG is shown as an image or inline. SMIL is not used. A `forward`
    pulse travels from source to target and starts again. A `both` pulse
    travels to the target and then back.
28. One loop lasts the same time on every pulsing Edge (1.6 s by default), so
    pulses stay in step.
29. The SVG has a `@media (prefers-reduced-motion: reduce)` rule that stops the
    animation and hides the pulse.
30. The SVG for a given Manifest is still the same bytes on every render
    (v1 FR-4). Animation is markup, not a timestamp.
31. `resvg` doesn't run animation, so the PNG shows no pulse and the static
    Diagram is unchanged.
32. `render --gif <path>` writes an animated GIF at the PNG's scale (2× the
    SVG size), so it is as sharp as the PNG. It draws 16 frames at evenly
    spaced times in the 1.6 s loop (100 ms per frame), with the pulse placed by Rust
    code at that time, not by CSS. Each frame is rasterized with the same
    `resvg` pipeline and fonts, and the GIF loops forever. It works for a
    Diagram with no pulses too, in which case it is a single-frame GIF.
33. GIF output is the same bytes every time for a given Manifest and theme.
    Color reduction uses a fixed palette made from the theme.
34. The terminal Editor animates pulsing Edges. A highlight glyph or color
    moves along the Edge's cells. While any pulse is visible, the Editor
    redraws about every 80 ms. When none is visible, it goes back to the
    250 ms disk poll.
35. The Editor has a key and a settings toggle to pause animation. The setting
    is remembered next to the theme. Animation starts paused when
    `SKEMA_REDUCED_MOTION=1` or `NO_MOTION` is set.

### Editor Copy menu

36. The "⎘ Copy image" button becomes "⎘ Copy ▾". It opens a menu with: PNG
    image, SVG markup, and GIF image. GIF image only appears when at least one
    Edge pulses. When none do, it appears greyed out with "no pulses".
37. PNG copies as an image (as today). SVG copies as text, and also as
    `image/svg+xml` where the platform clipboard supports it. GIF copies as
    `image/gif` (wl-copy, xclip, macOS `«class GIFf»`).
38. When the platform can't hold the chosen type (for example Windows with a
    GIF), the Editor writes the file next to the Manifest, copies its path, and
    says so in the status line.
39. The menu works with the mouse and the keyboard (arrow keys and Enter).
    `y` and `Y` keep their meanings, and `g` copies the GIF.
40. Every Copy action saves first, so the copied picture matches the Manifest,
    as today.
41. Every Editor write of the SVG (save, autosave, theme change) marks a
    `.png` or `.gif` with the same name beside the SVG stale, when that file
    already exists. They re-render once, 400 ms after the last change, on one
    background job in the current theme; a newer change cancels a render in
    flight (checked between GIF frames), and files are written through a temp
    file and rename. A GIF is never rendered when no Edge pulses. Copying a GIF
    also writes it beside the SVG, so it can be dragged into apps that won't
    paste one.

## Skill rules (`skills/skema-diagram/SKILL.md`)

- **SR-1 Footprint by meaning.** Use 2×1 for a service or step, 2×2 for a data
  store or queue, 3×1 for a gateway or a wide hub, and 1×1 for a small marker
  or an external actor with a label of up to 5 characters. Use 3×2 or 3×3 for a
  Node with a long description. Don't mix footprints on one row unless the
  meaning needs it.
- **SR-2 Width parity on vertical lines.** Nodes connected top to bottom
  should have the same number of columns (or at least both odd or both even)
  and the same x center, so the Edge is straight.
- **SR-3 Spacing by footprint.** Step side-by-side Nodes by `columns + 3`
  (5 for 2-wide, 6 for 3-wide), and stacked rows by `rows + 2` or more.
- **SR-4 Label budgets.** Keep labels under about 6 characters per column
  (1 column ≈ 5, 2 ≈ 12, 3 ≈ 18). Treat any `label-truncated` warning as a
  defect, fix it, and render again before reporting.
- **SR-5 Read stderr.** After every render, read the warnings (or use
  `--warnings=json`) and fix them, or explain them in the report. Never report
  a Diagram that has unexplained warnings.
- **SR-6 Center by default.** Unless the person placed Nodes themselves,
  center the Diagram on its vertical axis (equal left and right margins) and
  its horizontal axis (equal top and bottom margins), give or take one cell.
  Compute the footprint bounding box and shift every x by
  `(columns − used width) / 2 − left`, and every y the same way. For a
  symmetric fan-out, put the hub on the center column. Treat an `off-center`
  warning as a defect.
- **SR-7 Separators for boundaries.** Use a dashed Separator for a trust or
  cloud boundary ("AWS", "VPC", "public / private"). Use a solid Separator for
  tiers or lanes. Keep Separators in the empty band between rows, never
  through a Node.
- **SR-8 Pulses sparingly.** Pulse only the flows the person asked to
  highlight, or the ones that tell the story: `both` for request and response
  or two-way traffic, and `forward` for a one-way stream or a scheduled push.
  Pulse no more than about a third of the Edges.
- **SR-9 Picture format.** If any Edge pulses, write `--gif` when a picture is
  wanted, and still look at a PNG for the layout check. Otherwise, write a PNG.
  Report the path of every file written.

## UX and accessibility requirements

- Animation never carries meaning by itself. `both` vs `forward` is also
  readable from the Manifest, and two-way Edges are also drawn with an
  arrowhead at each end.
- Reduced-motion settings are respected in the SVG (FR-29) and the Editor
  (FR-35).
- Label warnings are plain text and one line each, and they name the item so
  it can be found in the Manifest.
- The Copy menu works with the keyboard alone.

## Data, permissions, and operational requirements

- New Manifest fields: `size` as an object, `edges[].labelPosition`,
  `edges[].pulse`, and top-level `separators`. All are optional.
- Compatibility with older binaries or older renders is not a goal. There is
  no `version` field and no migration.
- New dependency: a GIF encoder (the `gif` crate) in `skema-core`. Nothing
  else is added.
- The Editor still never writes an invalid Manifest.

## Constraints and dependencies

- The same Manifest gives the same SVG, PNG, and GIF bytes (v1 FR-4, extended).
- A pulse frame for the GIF must be computed by the Rust renderer, because
  `resvg` doesn't run CSS or SMIL.
- The TUI uses whole terminal cells, so a pulse moves one cell per step. That
  is enough to show direction.

## Risks and open questions

### Q-shape — resolved

"Shape" means footprint dimensions (1×1 to 3×3). Visual shapes such as a
cylinder or an actor icon are out of scope.

### Q-keyword-sizes — resolved

The keywords are plain shorthands (FR-14). `small` draws the same as `medium`.

### Q-gif-size — resolved

The GIF is drawn at the PNG's 2× scale, with 16 frames at 100 ms each. An
early 512 px cap made text unreadable. Frames are drawn in parallel, and each
frame after the first stores only the rectangle that changed, so a 24×14
Canvas is about 1 MB and renders in a few seconds.

### Q-svg-animation-tech — resolved

CSS `@keyframes` only. It supports `prefers-reduced-motion`, and it keeps the
markup simple. Still to check during build: whether it plays in GitHub, the
VS Code preview, and Slack. If a viewer doesn't play it, that viewer shows the
static Diagram, which is acceptable.

### Q-forward-compat — resolved

Not addressed. There are no long-term users, so there's no versioning,
`deny_unknown_fields`, or migration.

### Q-strict-default — resolved

The skill does not pass `--strict`. The skill reads
warnings and fixes them (SR-5), and `--strict` is for CI.

## Acceptance criteria

- **AC-warn-truncate:** Rendering the AWS example with the label
  "Front-end client" on a 2×1 Node exits 0, writes the SVG, and prints exactly
  one `label-truncated` line naming `client`. With `--strict`, it exits
  non-zero and the existing SVG keeps its bytes.
- **AC-warn-json:** `--warnings=json` prints a JSON array that parses, with
  one object per warning.
- **AC-arrow-visible:** For every Edge in the AWS example, the arrowhead
  polygon doesn't overlap any Node label background rectangle painted after
  it (checked by a test on the SVG geometry). A PNG of the example shows nine
  arrowheads on top ports.
- **AC-fanout-labels:** Three fan-out Edges from one Node, labeled `/a`, `/b`,
  `/c`, each have their label on their own last segment, and no
  `label-overlap` warning is raised.
- **AC-footprint:** A Manifest with Nodes at 1×1, 2×2, and 3×3 passes
  Validation and draws boxes of the matching size. Overlapping footprints are
  refused, with a message naming both Nodes.
- **AC-examples:** Every Manifest in `examples/` and `docs/` passes Validation
  and renders again with no `label-truncated` or `off-center` warnings. Fix
  the Manifests where needed.
- **AC-separator:** A dashed horizontal Separator labeled "AWS" renders as a
  dashed line with its label. A Separator through a Node is refused.
- **AC-pulse-svg:** An Edge with `pulse: "both"` gives SVG that animates back
  and forth with CSS in Chrome and Firefox, and stops under reduced motion. Rendering
  twice gives the same bytes.
- **AC-pulse-gif:** `render --gif` on that Manifest writes a looping GIF at
  the PNG's size, with 16 frames where the pulse moves along the
  Edge. Rendering twice gives the
  same bytes.
- **AC-pulse-tui:** In the Editor, the pulse moves along the Edge, the pause
  toggle stops it, and the redraw rate goes back to 250 ms when nothing pulses.
- **AC-copy-menu:** The Copy menu offers PNG, SVG, and GIF. Each puts the
  right type on the Linux clipboard (wl-copy and xclip) and macOS clipboard.
  The Windows GIF fallback copies a file path.
- **AC-skill-center:** Asked to "diagram a client, gateway, and three
  services", the Agent produces a Manifest with no `off-center` warning and no
  `label-truncated` warnings.

## Definition of done

- All FRs implemented in `skema-core`, `skema`, and `skema-tui`, with tests
  for each AC.
- `SKILL.md` updated with SR-1 to SR-9, and the Manifest shape section
  showing `size` objects, `labelPosition`, `pulse`, and `separators`.
- `examples/` has one Diagram that uses Footprints, a Separator, and both
  pulse modes, with its SVG and GIF checked in.
- The glossary and [CURRENT_PRODUCT_STATE.md](../CURRENT_PRODUCT_STATE.md)
  updated after release.
