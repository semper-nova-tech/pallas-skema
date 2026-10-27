# Skema Editor — Herdr plugin

Opens the Pallas Skema terminal Editor as a Herdr pane, next to the agent that is
editing the same Manifest.

## Install

From a clone of this repository:

```sh
herdr plugin link /path/to/pallas-skema/herdr-plugin
```

Or from GitHub once published:

```sh
herdr plugin install semper-nova-tech/pallas-skema/herdr-plugin
```

The build step uses `skema` from PATH. Otherwise it downloads the latest
release into `~/.local/bin`, and falls back to `cargo build --release` in the
repository next to the plugin when there is no release to fetch.

## Open a Diagram

```sh
herdr plugin pane open --plugin pallas.skema --entrypoint editor \
  --env SKEMA_MANIFEST=/abs/path/flow.json --focus
```

`SKEMA_SVG` defaults to the Manifest path with `.svg`. Without `SKEMA_MANIFEST`
the pane opens `<workspace>/skema/flow.json`, creating it on first save.

The action `pallas.skema.open` does the same from the workspace menu.

## In tandem with an agent

The agent edits the Manifest JSON and runs `skema render`. The pane notices the
file change and redraws. If the pane has unsaved edits it keeps them and shows
a warning; `r` reloads. Every accepted edit in the pane is saved at once:
Validation, then the Manifest, then the SVG.

Keys: `tab` cycles Nodes then Edges, arrows move the selected Node one Grid
cell, `enter` or double-click edits it, `a` add, `c` then click connects, `x` delete,
`+` `-` zoom, `0` fit, `y` copy image, `T` theme, `s` force a save, `q` quit.
