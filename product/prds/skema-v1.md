# Product Spec: Pallas Skema v1

## Changed in this draft

### Added

- Acceptance criteria (AC-repair-invalid)

### Changed

- Primary journey
- Functional requirements (requirement 14)
- UX and accessibility requirements
- Risks and open questions (Q-open-invalid resolved)
- Definition of done

### Removed

- None

## Terms

This PRD uses these rows from [the glossary](../GLOSSARY.md): Manifest, JSON, SVG, Grid, Node, Edge, Node type, Node size, Validation, Diagram, Human reviewer, Agent, Editor.

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

A Human reviewer can start a Diagram in the Editor, arrange it, save a Manifest, and review an SVG of that same Diagram. An Agent can change that Manifest and produce a new SVG that shows the change. The same Manifest always produces the same SVG. An invalid Manifest fails with a text explanation. Validation writes no new SVG and leaves an existing SVG unchanged.

Success means:

- A person can name the Nodes, Edges, labels, and Grid positions from the Manifest alone.
- The SVG shows those same Nodes, Edges, and labels.
- One label, Edge, or Grid-position change is visible in the Manifest and in the SVG.
- Rendering an unchanged Manifest again produces the same SVG.
- An invalid Manifest yields a text failure, no new SVG, and no change to an existing SVG.
- The Editor can create the first Manifest, and it never saves an invalid one.

## User stories

1. A Human reviewer reads a Manifest and understands the Nodes, Edges, labels, and Grid positions without opening an SVG.
2. An Agent edits a Manifest and produces a new SVG that shows that edit.
3. A Human reviewer opens the SVG and recognizes the same Nodes, Edges, and labels they read in the Manifest.
4. An Agent submits an invalid Manifest and gets a clear failure. No new SVG is written, and an existing SVG is left unchanged.
5. A Human reviewer changes a label, an Edge, or a Grid position and can review a small change in both the Manifest and the SVG.
6. A Human reviewer arranges a Diagram in the Editor by moving Nodes on the Grid, connecting Edges, and editing labels, and the saved Manifest contains those changes.
7. A Human reviewer starts a new Diagram in the Editor when no Manifest exists, and the Editor creates a Manifest with no Nodes and no Edges.

## Primary journey

1. The Human reviewer creates a new Manifest in the Editor, or opens an existing Manifest.
2. A new Manifest has no Nodes and no Edges and passes Validation.
3. The Human reviewer moves Nodes on the Grid, connects Edges, and edits labels.
4. The Editor saves the Manifest only when the result passes Validation.
5. An SVG is produced from that Manifest.
6. The Human reviewer reviews the SVG and can match it to the Manifest.
7. The Agent changes the Manifest as text, Validation accepts it, and a new SVG shows the change.

Alternate path: the Agent creates or edits a Manifest without the Editor, then the Human reviewer reviews the SVG.

Failure path: Validation rejects the Manifest. The actor receives a text failure that names the violated rule and the Manifest location when that location is known. No new SVG is written. An existing SVG is not deleted or modified.

Editor constraint: a move, connection, Node type, or Node size that would make the Manifest invalid is refused. The Editor does not write an invalid Manifest.

Repair path: the Human reviewer opens a Manifest that parses and fails Validation. The Editor shows the Diagram and the text failure. The Editor saves only after Validation passes. A Manifest that does not parse shows the text failure and does not present an editable Diagram. The file on disk stays unchanged until that valid save.

## In scope

- A JSON Manifest as the only source of truth for one Diagram.
- Nodes with a stable id, a label, a Grid position, a Node type, and a Node size.
- Node type values: process, decision, and note.
- Node size values: small, medium, and large.
- Edges from one Node to another, with an optional label.
- Validation that rejects a Manifest before any SVG is written.
- An SVG that matches a valid Manifest and stays the same when the Manifest stays the same.
- The Editor journey: start an empty Diagram, move Nodes on the Grid, connect Edges, edit labels, and save a valid Manifest.
- The Agent journey: edit the Manifest as text and produce an SVG without the Editor.
- Text failures for malformed JSON, duplicate ids, Edges that do not end on a Node, overlapping Nodes, invalid Grid coordinates, and unsupported Node type or Node size values.
- Refusing Editor changes that would fail Validation, without writing an invalid Manifest.
- Leaving an existing SVG unchanged when Validation fails.

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
8. Validation rejects malformed JSON, duplicate ids, Edges that do not end on a Node, overlapping Nodes, invalid Grid coordinates, and unsupported Node type or Node size values.
9. A rejected Manifest produces a text failure naming the violated rule and the Manifest location when that location is known.
10. A rejected Manifest does not produce a new SVG. An existing SVG is not deleted or modified.
11. Opening or saving a Manifest does not run commands contained in the Manifest.
12. The Editor refuses a move, connection, Node type, or Node size that would make the Manifest invalid. It does not write an invalid Manifest. When a saved Manifest already exists, that file stays as it was.
13. The Editor can create a new Manifest when none exists. The new Manifest has no Nodes and no Edges and passes Validation.
14. When a Manifest parses and fails Validation, the Editor opens it, shows the text failure, and does not save until Validation passes. When a Manifest does not parse, the Editor shows the text failure and does not present an editable Diagram. The file on disk stays unchanged until a valid save.

## UX and accessibility requirements

- The Editor shows the Diagram on a Grid and uses the predefined Node sizes.
- Node labels and Edge labels are text the Human reviewer can read and edit.
- Validation failures are text, including when they are raised from the Editor.
- A refused edit leaves the Diagram unchanged and explains the refusal in text.
- A parseable invalid Manifest opens in the Editor with its text failure visible. An unparseable Manifest shows the text failure and does not present an editable Diagram.
- Assumption for this draft: the Editor is operated with a pointer. Labels and failures are still available as text so the Manifest remains readable without the picture.

## Data, permissions, and operational requirements

- The Manifest is the only source of truth. The SVG is generated from it.
- The Human reviewer and the Agent read and write the same Manifest in the workspace.
- The Editor persists changes by writing the Manifest.
- The Editor writes a Manifest only when Validation accepts it.
- No account, network service, or shared permission model is required for the journeys in this PRD.
- A successful Validation is what allows a new SVG to be written.
- A failed Validation leaves any existing SVG byte-for-byte unchanged.
- Local file access is the only data boundary in this PRD.

## Constraints and dependencies

- The Manifest format is JSON.
- The generated picture is an SVG.
- There is no existing product behavior to migrate. See [current product state](../CURRENT_PRODUCT_STATE.md).
- No ticket is attached to this PRD.
- This PRD binds observable behavior. Language, toolkit, and packaging choices are left to implementation.

## Risks and open questions

- The Editor and the Agent can drift if they enforce different Validation rules. Both journeys use one Validation outcome.
- After a failed Validation, the previous SVG can disagree with the Manifest now on disk. The text failure is how the actor knows that SVG is not a picture of the current Manifest. This is the resolved behavior for Q-stale-svg.

### Q-invalid-edit

Resolved in round 2. The Editor refuses the change and leaves the saved Manifest valid.

### Q-stale-svg

Resolved in round 2. Validation writes no new SVG and does not delete or modify an existing SVG.

### Q-empty-start

Resolved in round 2. The Editor can create a new Manifest when none exists. That Manifest has no Nodes and no Edges.

### Q-open-invalid

When a Manifest already on disk fails Validation, which behavior is required?

- The Editor shows the text failure and does not open the Manifest for editing. The file is corrected as text.
- The Editor opens the Manifest so the Human reviewer can repair it, and the Editor still refuses to save until the result passes Validation.

Resolved in round 3. The Editor opens a Manifest that parses and fails Validation so the Human reviewer can change it, and it saves only after Validation passes. A Manifest that does not parse shows the text failure and does not present an editable Diagram. The file on disk stays unchanged until a valid save.

## Acceptance criteria

- **AC-read-manifest.** Given a valid Manifest, a Human reviewer can list every Node id, Node type, Node size, label, and Grid position, and every Edge's endpoints and label, without using the SVG.
- **AC-agent-render.** Given an Agent change to a label, an Edge, or a Grid position, the resulting SVG shows that change and the Agent did not need the Editor.
- **AC-svg-matches.** Given a valid Manifest, the SVG shows the same Nodes, Edges, and labels as the Manifest.
- **AC-deterministic.** Given an unchanged Manifest, two renders produce the same SVG.
- **AC-editor-roundtrip.** Given a Human reviewer moving a Node on the Grid, connecting an Edge, or editing a label in the Editor, the saved Manifest contains that change and a rendered SVG shows it.
- **AC-invalid-manifest.** Given a Manifest with malformed JSON, a duplicate id, an Edge that does not end on a Node, overlapping Nodes, an invalid Grid coordinate, or an unsupported Node type or Node size, Validation fails with text that names the rule, and no SVG of that invalid Manifest is written.
- **AC-preserve-svg.** Given an existing SVG and a Manifest that then fails Validation, the existing SVG is still present and unchanged, and no new SVG is written.
- **AC-reviewable-change.** Given one label, Edge, or Grid-position change, the Manifest and the SVG both show that change.
- **AC-grid-positions.** Given a Node moved in the Editor, the Manifest records a Grid position for that Node.
- **AC-no-execution.** Given a Manifest, Validation and rendering do not execute commands from its contents.
- **AC-refuse-invalid-edit.** Given a move, connection, Node type, or Node size that would fail Validation, the Editor refuses it, explains the refusal in text, and does not write that change. A Manifest that was valid stays valid. A Manifest that was already invalid stays unchanged.
- **AC-empty-start.** Given no Manifest, the Human reviewer can create one from the Editor. The new Manifest has no Nodes and no Edges, passes Validation, and renders to an SVG with no Nodes and no Edges.
- **AC-repair-invalid.** Given a Manifest that parses and fails Validation, the Editor opens it, shows the text failure, and does not save until Validation passes. Given a Manifest that does not parse, the Editor shows the text failure and does not present an editable Diagram. The file on disk stays unchanged until a valid save.

## Definition of done

- AC-read-manifest, AC-agent-render, AC-svg-matches, AC-deterministic, AC-editor-roundtrip, AC-invalid-manifest, AC-preserve-svg, AC-reviewable-change, AC-grid-positions, AC-no-execution, AC-refuse-invalid-edit, AC-empty-start, and AC-repair-invalid pass.
- The Human reviewer and the Agent can each complete their journey on one shared Manifest.
- Out-of-scope items are not required to call this version done.
