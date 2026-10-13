# Product Spec: Pallas Skema v1

## Changed in this draft

### Added

- Terms
- Problem
- Today
- Users and context
- Desired outcome and success measures
- User stories
- Primary journey
- In scope
- Out of scope
- Functional requirements
- UX and accessibility requirements
- Data, permissions, and operational requirements
- Constraints and dependencies
- Risks and open questions
- Acceptance criteria
- Definition of done

### Changed

- None

### Removed

- None

## Terms

This PRD uses these rows from [the glossary](../GLOSSARY.md): Manifest, YAML, SVG, Grid, Node, Edge, Node type, Node size, Validation, Diagram, Human reviewer, Agent, Editor.

## Problem

A person and an agent need to share one Diagram, change it, and see the same result. The picture has to stay a disposable view of a readable Manifest. There is no product that does this today.

## Today

Pallas Skema is documentation only. The repository has no validator, renderer, Editor, or checked-in Manifest. [Current product state](../CURRENT_PRODUCT_STATE.md) records that both design documents treat a YAML Manifest as the source of truth and an SVG as a generated picture, and that neither document is an approved PRD.

## Users and context

Personas are confirmed in [PERSONAS.md](../PERSONAS.md).

- The Human reviewer reads a Manifest, reviews the SVG, and arranges the Diagram in the Editor.
- The Agent edits the Manifest as text and produces an SVG without opening the Editor.

Both actors work on local files in a workspace. No account, role, or shared-server permission is in this PRD.

## Desired outcome and success measures

A Human reviewer can arrange a small Diagram in the Editor, save a Manifest, and review an SVG of that same Diagram. An Agent can change that Manifest and produce a new SVG that shows the change. The same Manifest always produces the same SVG. An invalid Manifest fails with a text explanation and does not become a picture of the invalid content.

Success means:

- A person can name the Nodes, Edges, labels, and Grid positions from the Manifest alone.
- The SVG shows those same Nodes, Edges, and labels.
- One label, Edge, or Grid-position change is visible in the Manifest and in the SVG.
- Rendering an unchanged Manifest again produces the same SVG.
- An invalid Manifest yields a text failure and no SVG of that invalid content.

## User stories

1. A Human reviewer reads a Manifest and understands the Nodes, Edges, labels, and Grid positions without opening an SVG.
2. An Agent edits a Manifest and produces a new SVG that shows that edit.
3. A Human reviewer opens the SVG and recognizes the same Nodes, Edges, and labels they read in the Manifest.
4. An Agent submits an invalid Manifest and gets a clear failure, with no misleading SVG left behind.
5. A Human reviewer changes a label, an Edge, or a Grid position and can review a small change in both the Manifest and the SVG.
6. A Human reviewer arranges a Diagram in the Editor by moving Nodes on the Grid, connecting Edges, and editing labels, and the saved Manifest contains those changes.

## Primary journey

1. The Human reviewer opens a Manifest in the Editor.
2. The Human reviewer moves Nodes on the Grid, connects Edges, and edits labels.
3. The Editor saves the Manifest.
4. Validation accepts the Manifest.
5. An SVG is produced from that Manifest.
6. The Human reviewer reviews the SVG and can match it to the Manifest.
7. The Agent changes the Manifest as text, Validation accepts it, and a new SVG shows the change.

Alternate path: the Agent creates or edits a Manifest without the Editor, then the Human reviewer reviews the SVG.

Failure path: Validation rejects the Manifest. The actor receives a text failure that names the violated rule and the Manifest location when that location is known. No SVG is written that depicts the invalid Manifest. What happens to an older SVG is [Q-stale-svg](#q-stale-svg).

Empty start: whether the Human reviewer can create a Diagram with no existing Manifest is [Q-empty-start](#q-empty-start).

## In scope

- A YAML Manifest as the only source of truth for one Diagram.
- Nodes with a stable id, a label, a Grid position, a Node type, and a Node size.
- Node type values: process, decision, and note.
- Node size values: small, medium, and large.
- Edges from one Node to another, with an optional label.
- Validation that rejects a Manifest before any SVG of that Manifest is written.
- An SVG that matches a valid Manifest and stays the same when the Manifest stays the same.
- The Editor journey: move Nodes on the Grid, connect Edges, edit labels, and save the Manifest.
- The Agent journey: edit the Manifest as text and produce an SVG without the Editor.
- Text failures for malformed YAML, duplicate ids, Edges that do not end on a Node, overlapping Nodes, invalid Grid coordinates, and unsupported Node type or Node size values.

Assumption carried from both design documents: the Node type and Node size values above are the v1 set.

## Out of scope

Assumption carried from the design documents, except the Editor, which this PRD includes:

- Executable commands or other automation stored in the Manifest.
- Pixel placement, custom Node sizes, and custom colors or shapes.
- More than one diagram language.
- Collaboration, accounts, comments, or hosted sharing.
- A workflow runner.
- Packaging for a specific desktop environment.
- An editor that is a separate source of truth from the Manifest.

## Functional requirements

1. A Human reviewer can identify every Node id, Node type, Node size, label, and Grid position, and every Edge's endpoints and label, from the Manifest alone.
2. An Agent can change a label, an Edge, or a Grid position in the Manifest and produce an SVG that shows that change without opening the Editor.
3. A valid Manifest produces an SVG of the same Nodes, Edges, and labels.
4. Rendering the same Manifest twice produces the same SVG.
5. The Editor lets the Human reviewer move a Node to a Grid position, connect an Edge between Nodes, and edit a Node or Edge label.
6. The Editor writes those changes into the Manifest. Grid positions are stored as Grid coordinates.
7. After a successful Editor save, an SVG can be produced that shows the Diagram the Human reviewer arranged.
8. Validation rejects malformed YAML, duplicate ids, Edges that do not end on a Node, overlapping Nodes, invalid Grid coordinates, and unsupported Node type or Node size values.
9. A rejected Manifest produces a text failure naming the violated rule and the Manifest location when that location is known.
10. A rejected Manifest does not produce an SVG of the invalid content.
11. Opening or saving a Manifest does not run commands contained in the Manifest.
12. The Editor's response when a move, connection, or size choice would make the Manifest invalid follows [Q-invalid-edit](#q-invalid-edit).

## UX and accessibility requirements

- The Editor shows the Diagram on a Grid and uses the predefined Node sizes.
- Node labels and Edge labels are text the Human reviewer can read and edit.
- Validation failures are text, including when they are raised from the Editor.
- Assumption for this draft: the Editor is operated with a pointer. Labels and failures are still available as text so the Manifest remains readable without the picture.

## Data, permissions, and operational requirements

- The Manifest is the only source of truth. The SVG is generated from it.
- The Human reviewer and the Agent read and write the same Manifest in the workspace.
- The Editor persists changes by writing the Manifest.
- No account, network service, or shared permission model is required for the journeys in this PRD.
- A successful Validation is what allows an SVG to be written.
- Local file access is the only data boundary in this PRD.

## Constraints and dependencies

- The Manifest format is YAML.
- The generated picture is an SVG.
- There is no existing product behavior to migrate. See [current product state](../CURRENT_PRODUCT_STATE.md).
- No ticket is attached to this PRD.
- This PRD binds observable behavior. Language, toolkit, and packaging choices are left to implementation.

## Risks and open questions

- The Editor and the Agent can drift if they enforce different Validation rules. Both journeys use one Validation outcome.
- An older SVG can be mistaken for the current Manifest after a failed edit. See [Q-stale-svg](#q-stale-svg).

### Q-invalid-edit

When the Human reviewer moves a Node onto another Node, connects an Edge to a Node that is not in the Manifest, or chooses an unsupported Node type or Node size, which behavior is required?

- The Editor refuses the change and leaves the saved Manifest valid.
- The Editor saves the change and shows the Validation failure.

The design documents describe both prevention in the Editor and rejection by Validation. They do not choose one product behavior.

### Q-stale-svg

Story 4 requires no misleading SVG after an invalid Manifest. When an earlier SVG exists and Validation then fails, which behavior is required?

- Leave the earlier SVG unchanged.
- Remove or replace the earlier SVG so it cannot be read as a picture of the current Manifest.

[brainstorming.md](../../brainstorming.md) says Validation runs before an SVG is created or replaced. That keeps the earlier file on disk.

### Q-empty-start

The primary journey opens an existing Manifest. Can the Human reviewer also start a new Diagram when no Manifest exists yet?

- Yes. The Editor can create a new Manifest.
- No. v1 only opens an existing Manifest.

## Acceptance criteria

- **AC-read-manifest.** Given a valid Manifest, a Human reviewer can list every Node id, Node type, Node size, label, and Grid position, and every Edge's endpoints and label, without using the SVG.
- **AC-agent-render.** Given an Agent change to a label, an Edge, or a Grid position, the resulting SVG shows that change and the Agent did not need the Editor.
- **AC-svg-matches.** Given a valid Manifest, the SVG shows the same Nodes, Edges, and labels as the Manifest.
- **AC-deterministic.** Given an unchanged Manifest, two renders produce the same SVG.
- **AC-editor-roundtrip.** Given a Human reviewer moving a Node on the Grid, connecting an Edge, or editing a label in the Editor, the saved Manifest contains that change and a rendered SVG shows it.
- **AC-invalid-manifest.** Given a Manifest with malformed YAML, a duplicate id, an Edge that does not end on a Node, overlapping Nodes, an invalid Grid coordinate, or an unsupported Node type or Node size, Validation fails with text that names the rule, and no SVG of that invalid Manifest is written.
- **AC-reviewable-change.** Given one label, Edge, or Grid-position change, the Manifest and the SVG both show that change.
- **AC-grid-positions.** Given a Node moved in the Editor, the Manifest records a Grid position for that Node.
- **AC-no-execution.** Given a Manifest, Validation and rendering do not execute commands from its contents.

## Definition of done

- AC-read-manifest, AC-agent-render, AC-svg-matches, AC-deterministic, AC-editor-roundtrip, AC-invalid-manifest, AC-reviewable-change, AC-grid-positions, and AC-no-execution pass.
- Q-invalid-edit, Q-stale-svg, and Q-empty-start are resolved, and any acceptance criterion they add is in a later draft.
- The Human reviewer and the Agent can each complete their journey on one shared Manifest.
- Out-of-scope items are not required to call this version done.
