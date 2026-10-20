# pallas-skema

AI and human friendly diagram protocol.

- [Product brief and technical design](pallas-skema.md)
- [Brainstorming and delivery strategy](brainstorming.md)

Repository: [https://github.com/semper-nova-tech/pallas-skema](https://github.com/semper-nova-tech/pallas-skema)

JSON > Diagram > Editor > SVG. Bridging the design and comprehension gap

## Build

Requires a Rust toolchain (`rustup`).

```bash
cargo build --release
```

The binary is `target/release/skema`. It has no runtime dependencies.

## Agent loop

The Manifest JSON file is the source of truth. An agent edits it, then renders.
Validation runs first. On failure the text failure goes to stderr, the exit code
is non-zero, and an existing SVG is left byte-for-byte unchanged.

```bash
skema render --manifest flow.json --svg flow.svg
```

`skema validate --manifest flow.json` checks without writing anything.

## Editor (terminal)

The Editor is a terminal app, like Hunk or TTT. It runs in the current
terminal, which can be a Herdr pane, and needs no browser.

```bash
skema open-editor --manifest flow.json --svg flow.svg
```

- A missing Manifest starts an empty Diagram.
- Nodes sit on Grid cells. Arrows move the selected Node one whole cell at a
  time, or drag it with the mouse. A move that would overlap another Node or
  leave the Grid is refused with the Validation text.
- Edges leave and enter at the middle of the facing side of each Node.
- Every Node shares one color and the type is not drawn; decisions have rounded
  corners. The selected Node is drawn filled; others are outlined. A Node has a
  `label` and an optional `description`.
- When an agent writes the Manifest, a clean Editor reloads it on the spot. An
  Editor with unsaved edits keeps them and shows a warning; `r` reloads.
- Every accepted edit is saved at once: the Manifest is written, then the SVG
  is rendered. An edit that fails Validation is refused and writes nothing, so
  the files on disk are always valid. `s` forces a save. Pass `--manual-save`
  to write only on `s`; then `q` asks once if there are unsaved edits.

Keys: `tab` and `shift+tab` cycle through Nodes and then Edges, arrows move the
selected Node, `enter` (or a double-click) opens it for editing, `a` add,
`c` connect (press on the source, then click the target or tab to it and press `c` again), `x` delete,
`+` `-` zoom, `0` fit to pane, `s` force a save, `r` reload, `q` quit.

In the Node editor, up and down pick a field (label, description, size),
`enter` edits it (size cycles), `enter` again saves the field, and `esc` steps
back out. Every field change goes through Validation like a move. The Node
type stays in the Manifest for agents and is not shown in the Editor.

Zooming out stops at the level where the whole Diagram fits the pane, decided
by whichever of width or height runs out first.

## Herdr pane

`herdr-plugin/` opens the Editor as a Herdr pane next to the agent:

```bash
herdr plugin link /path/to/pallas-skema/herdr-plugin
herdr plugin pane open --plugin pallas.skema --entrypoint editor \
  --env SKEMA_MANIFEST=/abs/path/flow.json --focus
```

## Agent plugins

`skills/skema-diagram/SKILL.md` teaches an agent the edit, render, and
open-the-pane loop. It follows the agent skills format, so one file serves both:

- **Claude Code:** this repository is a plugin (`.claude-plugin/plugin.json`).
  Add it as a marketplace from the repo path or URL, or symlink
  `skills/skema-diagram` into `~/.claude/skills/`.
- **OpenAI Codex:** symlink or copy `skills/skema-diagram` into
  `~/.codex/skills/`.

## Legacy web editor

`skema-editor/` holds the earlier Angular editor. It is no longer served by the
CLI and is kept for reference only.
