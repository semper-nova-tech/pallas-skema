# Pallas Skema

## Product brief

Pallas Skema is an agent-native flow design tool for simple flow charts.

The goal is:

> Very simple flow chart style small box / medium box lines / arrows / a default restrictive snap to grid that generates a manifest or pseudo code that is easy for agents to interpret so an agent could edit the manifest and get something back.

The core idea is:

```text
YAML = source of truth
SVG = generated artifact
GUI = constrained YAML editor
```

The human GUI is a restrictive renderer/editor. It should make it easy to create and adjust a small flow diagram while producing a stable, readable manifest. An agent should be able to edit the YAML and rerender the diagram.

The product should produce cheap artifacts like SVG, then allow the YAML to be edited and rendered again.

## Design decisions

- Use YAML as the canonical manifest format.
- Generate SVG from YAML.
- Do not treat SVG as the source of truth.
- The GUI writes YAML; it does not manually edit SVG.
- Use a default restrictive snap-to-grid.
- Use predefined small, medium, and large box sizes.
- Use simple lines and arrows.
- Keep the diagram easy for agents to interpret and modify.
- Keep the visual graph separate from executable automation.
- Do not put arbitrary shell commands or executable code directly in the manifest.
- If the graph later drives automation, use typed actions with explicit validation and approval gates.
- The renderer should be deterministic.
- The first version should be intentionally limited rather than becoming a general-purpose diagram editor.

## Intended workflow

```text
Human creates or edits a flow in the GUI
        ↓
GUI saves YAML
        ↓
Validator checks the manifest
        ↓
Renderer generates SVG
        ↓
Human reviews the visual artifact
        ↓
Agent can edit the YAML
        ↓
Renderer produces the updated SVG
```

An agent should be able to make a small edit such as:

```yaml
- from: review
  to: revise
  label: "needs changes"
```

Then rerender the SVG deterministically.

## Example manifest

```yaml
version: 1

grid:
  unit: 24

nodes:
  - id: intake
    type: process
    label: Receive request
    position: [1, 2]
    size: medium

  - id: review
    type: decision
    label: Approved?
    position: [5, 2]
    size: small

edges:
  - from: intake
    to: review

  - from: review
    to: done
    label: "yes"

  - from: review
    to: revise
    label: "no"
```

## GUI behavior

The GUI should support:

- Small, medium, and large predefined boxes.
- Dragging nodes to grid positions.
- Connecting valid ports.
- Editing labels and edge conditions.
- Preventing overlap.
- Restricting colors and shapes.
- Saving YAML.
- Live YAML validation.
- SVG export.
- A grid-style canvas.

The GUI should not require arbitrary pixel positioning. Node positions should be grid coordinates, not freeform coordinates.

The renderer should enforce:

- Valid node types.
- Valid node sizes.
- Stable node IDs.
- No duplicate IDs.
- No dangling edges.
- No overlapping nodes.
- Valid grid coordinates.
- Valid edge endpoints.

## Agent-native requirements

The manifest must be:

- Plain text.
- Searchable.
- Stable under formatting.
- Easy to diff.
- Easy for an agent to edit.
- Validatable without launching the GUI.
- Renderable without launching the GUI.

The interface should provide commands such as:

```bash
flow validate flow.yaml
flow render flow.yaml --output flow.svg
flow preview flow.yaml
```

The source format should remain readable to agents even when the diagram is viewed only as text.

The product should work well with agentic workflows such as:

```text
planner → implementer → tests → reviewer → human gate
```

It can produce artifacts for workflows such as [Pallas Agent Flow](https://github.com/semper-nova-tech/pallas-agent-flow), where state, control flow, contracts, verification, and recovery are externalized into durable artifacts.

The tool should not depend on Cursor CLI being able to connect to a local model. It should produce files and commands that any capable agent can read and modify.

## Technical section

### Recommended architecture

```text
Rust application
├── YAML model
├── validator
├── grid layout engine
├── egui canvas editor
└── SVG exporter
```

The core pipeline is:

```text
YAML → validate → layout → SVG
```

The YAML model and renderer should be independent from the GUI so agents can modify and render diagrams without launching the application.

### Language and GUI

Use Rust for the complete application.

Use `egui` with `eframe` for the first GUI:

- `eframe` provides the desktop application window.
- `egui` provides the interface widgets and drawing canvas.
- Both are written for Rust.
- They support Linux and Wayland.
- They can handle click-and-drag nodes, snap-to-grid movement, restricted node shapes and sizes, dragging connections, selection, property editing, live YAML validation, and SVG export.

Alternatives considered:

- **Slint**: more polished declarative UI, good if this becomes a product.
- **GTK4**: strongest Linux-native accessibility, but more platform-specific.
- **Tauri**: Rust backend with a web frontend; unnecessary if the GUI is intentionally simple.

For a vibe-coded prototype, `egui` is the shortest path.

### Suggested Rust components

Use Rust components for:

- YAML parsing and serialization.
- Schema validation.
- Grid and overlap rules.
- Orthogonal edge routing.
- Deterministic SVG generation.
- CLI commands.
- Snapshot tests for generated SVG.

Likely supporting pieces include:

- `serde` for the data model.
- A YAML parser.
- `clap` for the CLI.
- Snapshot tests for generated SVG.

The exact dependency choices are implementation details. Avoid a general-purpose layout engine until the restrictive grid model needs one.

### MVP scope

The first version should be:

```text
Load YAML
  → Render grid and nodes
  → Drag node, snapping to grid
  → Connect nodes
  → Edit labels
  → Validate
  → Save YAML
  → Export SVG
```

The first renderer should support only:

- `process` nodes.
- `decision` nodes.
- `note` nodes.
- Fixed grid coordinates.
- Small, medium, and large sizes.
- Straight or right-angle edges.

Do not start with:

- A general-purpose diagram editor.
- Arbitrary pixel placement.
- Arbitrary executable actions.
- Multiple diagram languages.
- Complex collaborative editing.
- A workflow execution engine.

## Validation and safety

Vibe coding is appropriate for the prototype, but validation must not be vibe-coded away.

The renderer should fail clearly for:

- Malformed YAML.
- Duplicate IDs.
- Missing node references.
- Dangling edges.
- Overlapping nodes.
- Invalid coordinates.
- Unsupported shapes or sizes.

Keep executable behavior out of the visual manifest. If actions are added later, use typed actions, explicit schemas, validation, and approval gates rather than arbitrary commands.

## Omarchy integration

Pallas Skema can be made into an Omarchy plugin.

The cleanest design is:

```text
Omarchy plugin
├── Rust/egui diagram application
├── YAML manifests
├── SVG renderer
└── Omarchy integration
    ├── bar/menu launcher
    └── start/stop/update commands
```

The Rust application remains a normal standalone GUI. The Omarchy plugin installs and launches it.

The recommended first integration is simple:

```text
Omarchy menu → Pallas Skema → opens Rust egui app
```

Deep shell integration can be added later with a Quickshell/QML panel component that displays current diagrams or provides controls. The QML layer would call the Rust binary for operations.

Do not make the entire egui canvas a Quickshell component unless that complexity becomes necessary. Keep the project in its own repository and install it as an Omarchy plugin, for example:

```bash
omarchy plugin add https://github.com/semper-nova-tech/pallas-skema --enable
```

The plugin layer is for launcher, bar integration, notifications, shell controls, installation, and updates. The Rust application is the actual diagram editor.

## Existing tools considered

### Mermaid

Mermaid is text-based, renders nicely, and agents can read and edit the source directly. It is useful for quick diagrams and Markdown documentation.

Mermaid Chart’s Visual Editor supports:

- Dragging nodes.
- Dragging connections.
- Creating nodes visually.
- Automatic alignment/snapping.
- A grid-style canvas.
- Two-way synchronization between the canvas and Mermaid code.

However, Mermaid should not be the canonical format for this product. Its layout is mostly automatic and it does not provide strong control over grid coordinates. Mermaid can be used for simple documentation, but the custom YAML format is better for a restrictive renderer.

### Draw.io

Draw.io has visual editing, but its XML is noisy and less agent-friendly than YAML.

### Excalidraw and tldraw

These support freeform visual editing, but their files are generally less readable to coding agents than a constrained YAML manifest.

### Graphviz and other layout engines

They can generate SVG, but automatic layout does not match the desired restrictive snap-to-grid GUI. They may be useful later if the renderer needs more complex graph layout.

## Product boundary

Pallas Skema is a constrained visual editor and artifact generator, not initially a workflow runner.

The primary artifact is:

```text
flow.yaml
```

The generated artifact is:

```text
flow.svg
```

The GUI is an editor for the YAML-backed model. Agents can work directly with the YAML and invoke the renderer through the CLI.

