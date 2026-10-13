# Product Personas

Confirmed in the skema-v1 product interview, round 1, on 2026-10-13.
Stories in [prds/skema-v1.md](prds/skema-v1.md) may use only these actors.

## Persona: Human reviewer

- **Role:** Reads a Manifest, reviews the SVG, and arranges the Diagram in the Editor.
- **Context:** A local workspace that contains the Manifest and the generated SVG.
- **Goals:** Understand a Diagram quickly, and change Nodes, Edges, and labels visually.
- **Needs:** A readable Manifest, an SVG that matches it, and an Editor that writes the Manifest.
- **Pain points:** Not stated.
- **Constraints:** Not stated.
- **Priority:** Primary for the Editor journey.

## Persona: Agent

- **Role:** Edits a Manifest as text and produces an SVG without opening the Editor.
- **Context:** Workspace files the Agent can read and modify.
- **Goals:** Make a small Manifest change and obtain an SVG that shows it.
- **Needs:** A plain-text Manifest, Validation, and rendering that do not require the Editor.
- **Pain points:** Not stated.
- **Constraints:** Completes the Agent journey without the Editor.
- **Priority:** Primary for the Agent journey.
