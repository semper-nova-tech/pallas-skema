# Technical Requirements
Status: planned

## Terms

Glossary rows this spec introduces surfaces for or relies on: Manifest, JSON, SVG, Grid, Grid position, Node, Edge, Node type, Node size, Validation, Render, Text failure, Diagram, Editor, Command-line tool, Human reviewer, Agent, Herdr. Shapes live here; names and surfaces live in [glossary.md](glossary.md).

## Scope and constraints

- **In scope:** All frontier items from the accepted PRD review for skema-v1; local workspace files only; JSON Manifest; shared Validation before SVG; deterministic Render; Angular Editor (Decision Angular Editor D-001); Rust Validation, Render, and command-line tool (Decision Rust core D-002); JSON Manifest handoff to Editor (Decision JSON handoff D-004); minimal tooling (Decision Minimal tooling D-006).
- **Out of scope:** Hosting, accounts, upload (Decision Webpage later D-005); workflow runner; egui; YAML Manifest as source of truth; executable commands in Manifest; pixel placement and custom sizes or colors.
- **Non-authoritative:** `README.md`, `pallas-skema.md`, `brainstorming.md` do not define v1 behavior.

## Requirement index

| Display name | ID | Type | Owner |
| --- | --- | --- | --- |
| Manifest v1 JSON model | MODEL-manifest-v1 | MODEL | backend |
| Grid cell overlap rules | MODEL-grid-cell-rules | MODEL | backend |
| Validate Manifest contract | CONTRACT-validate-manifest | CONTRACT | backend |
| Render Manifest contract | CONTRACT-render-manifest | CONTRACT | backend |
| Command-line tool surface | CONTRACT-cli-surface | CONTRACT | backend |
| Editor JSON handoff | CONTRACT-editor-json-handoff | CONTRACT | backend |
| Launch Editor in Herdr pane | CONTRACT-launch-editor-pane | CONTRACT | infrastructure |
| Editor Manifest lifecycle | STATE-editor-manifest-lifecycle | STATE | frontend |
| Gate SVG on Validation | BEHAVIOR-validation-gate | BEHAVIOR | backend |
| Deterministic SVG bytes | BEHAVIOR-deterministic-render | BEHAVIOR | backend |
| Agent render without Editor | BEHAVIOR-agent-render | BEHAVIOR | backend |
| SVG after valid Editor save | BEHAVIOR-editor-save-render | BEHAVIOR | frontend |
| Editor grid roundtrip | BEHAVIOR-editor-roundtrip | BEHAVIOR | frontend |
| Refuse invalid Editor edit | BEHAVIOR-refuse-invalid-edit | BEHAVIOR | frontend |
| Editor empty start | BEHAVIOR-empty-start | BEHAVIOR | frontend |
| Repair invalid Manifest | BEHAVIOR-repair-invalid | BEHAVIOR | frontend |
| Read Manifest without SVG | BEHAVIOR-read-manifest | BEHAVIOR | backend |
| No execution from Manifest | BEHAVIOR-no-execution | BEHAVIOR | backend |
| Runnable Skema v1 product | BEHAVIOR-runnable-product | BEHAVIOR | backend |

## Requirements

### Manifest v1 JSON model (`MODEL-manifest-v1`)

- Meaning: The persisted JSON shape for one Diagram is defined and is the only source of truth.
- Today: No normative JSON Manifest exists in the repository; design documents illustrate YAML only (Discovery index DISC-index v1; PRD review).
- After: A single documented JSON object holds `nodes` and `edges` with the field set in Architecture overview; Human reviewer and Agent read and write the same file.
- Type: MODEL
- Owner: backend
- Frontier: JSON Manifest source of truth (FR-json-manifest), Read Manifest without SVG (FR-read-manifest)
- Terms: Manifest, JSON, Node, Edge, Node type, Node size, Grid position
- Inputs/outputs: JSON text ↔ parsed Manifest document
- Invariants and failures: Malformed JSON is not a Manifest; field names and nesting are stable for v1 tests
- Depends on: none
- Acceptance checks: AC-read-manifest field listing; AC-invalid-manifest malformed JSON case
- Evidence: Schema or type definition in backend; golden Manifest fixtures
- Architecture impact: update
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Multiple Diagrams per file; YAML

### Grid cell overlap rules (`MODEL-grid-cell-rules`)

- Meaning: Grid position domain and overlap are defined for Validation messages and Editor refusal.
- Today: No Grid implementation (Discovery index DISC-index v1).
- After: Grid coordinates are non-negative integers with no maximum; fractional or negative coordinates are invalid Grid coordinates; two Nodes overlap when they share the same Grid cell (`x`, `y` pair); overlap is rejected.
- Type: MODEL
- Owner: backend
- Frontier: Shared Validation before SVG (FR-validation-gate), Editor refuse invalid edit (FR-refuse-invalid-edit)
- Terms: Grid, Grid position, Node
- Inputs/outputs: Grid position values on Nodes
- Invariants and failures: Text failure names invalid coordinate or overlap with location when known
- Depends on: MODEL-manifest-v1
- Acceptance checks: AC-invalid-manifest invalid Grid coordinate and overlapping Nodes cases; AC-refuse-invalid-edit for move into occupied cell
- Evidence: Validation tests with adjacent and colliding coordinates
- Architecture impact: update
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Pixel hit-testing; bounding-box overlap

### Validate Manifest contract (`CONTRACT-validate-manifest`)

- Meaning: One Validation entry point evaluates every Manifest rule before SVG.
- Today: No Validation implementation (Discovery index DISC-index v1).
- After: Editor and command-line tool call the same Validation; outcome is pass or Text failure; no SVG write on failure.
- Type: CONTRACT
- Owner: backend
- Frontier: Shared Validation before SVG (FR-validation-gate)
- Terms: Validation, Text failure, Manifest
- Inputs/outputs: Manifest JSON in → pass or Text failure out
- Invariants and failures: Rejects malformed JSON, duplicate ids, Edges not ending on a Node, overlapping Nodes, invalid Grid coordinates, unsupported Node type or size; existing SVG untouched on failure
- Depends on: MODEL-manifest-v1, MODEL-grid-cell-rules
- Acceptance checks: AC-invalid-manifest (all rule cases); AC-preserve-svg
- Evidence: Shared test vectors invoked from CLI and Editor integration tests
- Architecture impact: update
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Different rules per actor

### Render Manifest contract (`CONTRACT-render-manifest`)

- Meaning: SVG is produced only from a Manifest that passed Validation.
- Today: No renderer (Discovery index DISC-index v1).
- After: Render writes SVG showing Manifest Nodes, Edges, and labels; invalid Manifest never reaches Render.
- Type: CONTRACT
- Owner: backend
- Frontier: Deterministic SVG render (FR-deterministic-render), Shared Validation before SVG (FR-validation-gate)
- Terms: Render, SVG, Manifest, Validation
- Inputs/outputs: Validated Manifest in → SVG bytes out
- Invariants and failures: Render does not run if Validation fails; does not mutate existing SVG on Validation failure
- Depends on: CONTRACT-validate-manifest
- Acceptance checks: AC-svg-matches; AC-deterministic; AC-preserve-svg
- Evidence: Double-render byte comparison tests; SVG content assertions
- Architecture impact: update
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Editing SVG as source

### Command-line tool surface (`CONTRACT-cli-surface`)

- Meaning: Local entry point for Validation, Render, and opening the Editor.
- Today: No command-line tool (Discovery index DISC-index v1).
- After: Human reviewer and Agent invoke validate, render, and open-editor commands against workspace paths; Agent completes journey without Editor.
- Type: CONTRACT
- Owner: backend
- Frontier: Agent render without Editor (FR-agent-render), Runnable diagram product (FR-runnable-product)
- Terms: Command-line tool, Validation, Render
- Inputs/outputs: File paths and subcommands; stdout/stderr Text failure on errors
- Invariants and failures: Render subcommand runs Validation first; non-zero exit on Validation failure
- Depends on: CONTRACT-validate-manifest, CONTRACT-render-manifest
- Acceptance checks: AC-agent-render; AC-invalid-manifest from CLI path
- Evidence: CLI integration tests in workspace temp dirs
- Architecture impact: update
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Workflow runner; hosted API

### Editor JSON handoff (`CONTRACT-editor-json-handoff`)

- Meaning: Opening the Editor passes the current Manifest JSON to the Angular client (Decision JSON handoff D-004).
- Today: No Editor or handoff (Discovery index DISC-index v1).
- After: Command-line tool supplies Manifest JSON (including empty valid template) so Editor can start without a separate source of truth.
- Type: CONTRACT
- Owner: backend
- Frontier: JSON Manifest source of truth (FR-json-manifest), Runnable diagram product (FR-runnable-product)
- Terms: Manifest, JSON, Editor, Command-line tool
- Inputs/outputs: JSON Manifest payload + paths → Editor session
- Invariants and failures: Payload matches on-disk Manifest when opening existing file; empty start uses valid empty Manifest shape
- Depends on: MODEL-manifest-v1, CONTRACT-cli-surface
- Acceptance checks: AC-empty-start; AC-editor-roundtrip after relaunch
- Evidence: Handoff contract test between CLI stub and Editor bootstrap
- Architecture impact: update
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Web upload of Manifest

### Launch Editor in Herdr pane (`CONTRACT-launch-editor-pane`)

- Meaning: One Editor instance opens in a Herdr pane from the command-line tool (Decision Pane-hosted Editor D-003, assumed).
- Today: No pane launch integration (Discovery index DISC-index v1).
- After: Human reviewer opens Editor from CLI; no second terminal canvas; local pane only.
- Type: CONTRACT
- Owner: infrastructure
- Frontier: Runnable diagram product (FR-runnable-product)
- Terms: Editor, Command-line tool
- Inputs/outputs: Launch request from CLI → Editor pane running
- Invariants and failures: Single Editor instance per launch; failure surfaces as Text failure if pane cannot open
- Depends on: CONTRACT-cli-surface, CONTRACT-editor-json-handoff
- Acceptance checks: Manual or automated smoke: CLI open-editor shows Editor in pane with handoff
- Evidence: Local Herdr launch script or test harness log
- Architecture impact: decision
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Desktop packaging; browser-hosted Editor in v1

### Editor Manifest lifecycle (`STATE-editor-manifest-lifecycle`)

- Meaning: Editor states for empty start, valid editing, parseable invalid repair, and unparseable open.
- Today: No Editor (Discovery index DISC-index v1).
- After: States match PRD repair and refusal rules; disk unchanged until valid save from repair or normal edit.
- Type: STATE
- Owner: frontend
- Frontier: Editor repair invalid Manifest (FR-repair-invalid), Editor empty start (FR-empty-start), Editor refuse invalid edit (FR-refuse-invalid-edit)
- Terms: Editor, Diagram, Text failure, Manifest
- Inputs/outputs: File open/save events ↔ UI mode and persisted file
- Invariants and failures: Unparseable → Text failure only, no Diagram; parseable invalid → Diagram visible, save blocked until Validation passes
- Depends on: CONTRACT-validate-manifest, CONTRACT-editor-json-handoff
- Acceptance checks: AC-repair-invalid; AC-empty-start; AC-refuse-invalid-edit
- Evidence: Frontend state tests per transition in architecture state diagram
- Architecture impact: update
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Optimistic save of invalid Manifest

### Gate SVG on Validation (`BEHAVIOR-validation-gate`)

- Meaning: No new SVG is written unless Validation passes; failed Validation leaves prior SVG byte-for-byte unchanged.
- Today: unknown (no SVG writes).
- After: Every SVG write path runs Validation first; AC-preserve-svg holds for Editor and Agent.
- Type: BEHAVIOR
- Owner: backend
- Frontier: Shared Validation before SVG (FR-validation-gate)
- Terms: Validation, SVG, Text failure
- Inputs/outputs: Manifest change attempt → SVG or Text failure
- Invariants and failures: AC-stale-svg resolved behavior via Text failure and unchanged SVG
- Depends on: CONTRACT-validate-manifest, CONTRACT-render-manifest
- Acceptance checks: AC-preserve-svg; AC-invalid-manifest
- Evidence: Tests that mutate Manifest to invalid and assert SVG hash unchanged
- Architecture impact: none
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Auto-delete stale SVG

### Deterministic SVG bytes (`BEHAVIOR-deterministic-render`)

- Meaning: Unchanged valid Manifest always yields identical SVG file bytes.
- Today: unknown (no renderer).
- After: Two renders of the same Manifest produce identical output (AC-deterministic).
- Type: BEHAVIOR
- Owner: backend
- Frontier: Deterministic SVG render (FR-deterministic-render)
- Terms: Render, SVG, Manifest
- Inputs/outputs: Same Manifest twice → identical SVG bytes
- Invariants and failures: No timestamps or random ids in SVG output
- Depends on: CONTRACT-render-manifest
- Acceptance checks: AC-deterministic; AC-svg-matches
- Evidence: Byte-for-byte compare in render tests
- Architecture impact: none
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Non-deterministic themes

### Agent render without Editor (`BEHAVIOR-agent-render`)

- Meaning: Agent edits Manifest as text and obtains SVG via command-line Render without opening Editor.
- Today: unknown.
- After: AC-agent-render and AC-reviewable-change for label, Edge, or Grid position via CLI only.
- Type: BEHAVIOR
- Owner: backend
- Frontier: Agent render without Editor (FR-agent-render)
- Terms: Agent, Command-line tool, Render, Manifest
- Inputs/outputs: Text edit to Manifest file → SVG update on successful Validation
- Invariants and failures: Same Validation and Render as Editor path
- Depends on: CONTRACT-cli-surface, BEHAVIOR-validation-gate
- Acceptance checks: AC-agent-render; AC-reviewable-change
- Evidence: CLI scenario test: single-field diff in Manifest and SVG
- Architecture impact: none
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Agent opens Editor

### SVG after valid Editor save (`BEHAVIOR-editor-save-render`)

- Meaning: Primary Human reviewer journey produces SVG after a valid Editor save using the same Render as the Agent (closes Q-svg-trigger).
- Today: unknown.
- After: Successful Editor save persists Manifest, then invokes backend Render to write SVG; Agent continues to use explicit render command without Editor.
- Type: BEHAVIOR
- Owner: frontend
- Frontier: Runnable diagram product (FR-runnable-product), Editor grid roundtrip (FR-editor-roundtrip)
- Terms: Editor, Render, Human reviewer, Command-line tool
- Inputs/outputs: Valid save event → SVG file updated
- Invariants and failures: Save blocked when Validation fails; Render not called on refused edit
- Depends on: CONTRACT-render-manifest, STATE-editor-manifest-lifecycle, BEHAVIOR-validation-gate
- Acceptance checks: AC-editor-roundtrip; primary journey step 5 observation in integrated test
- Evidence: End-to-end Editor save then SVG content match
- Architecture impact: update
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: SVG generation without valid save in Editor path

### Editor grid roundtrip (`BEHAVIOR-editor-roundtrip`)

- Meaning: Human reviewer moves Nodes, connects Edges, edits labels; saved Manifest stores Grid positions and content; SVG reflects arrangement.
- Today: unknown.
- After: AC-editor-roundtrip and AC-grid-positions pass.
- Type: BEHAVIOR
- Owner: frontend
- Frontier: Editor grid roundtrip (FR-editor-roundtrip)
- Terms: Editor, Grid, Grid position, Node, Edge, Diagram
- Inputs/outputs: Pointer edits → Manifest fields → SVG view
- Invariants and failures: Predefined Node sizes only; Grid-aligned positions
- Depends on: STATE-editor-manifest-lifecycle, MODEL-manifest-v1
- Acceptance checks: AC-editor-roundtrip; AC-grid-positions; AC-svg-matches
- Evidence: UI or integration test recording Manifest after save
- Architecture impact: none
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Pixel-freeform canvas

### Refuse invalid Editor edit (`BEHAVIOR-refuse-invalid-edit`)

- Meaning: Moves, connections, Node type, or Node size that would fail Validation are refused with Text explanation; valid on-disk Manifest stays valid.
- Today: unknown.
- After: AC-refuse-invalid-edit; Diagram unchanged on refusal.
- Type: BEHAVIOR
- Owner: frontend
- Frontier: Editor refuse invalid edit (FR-refuse-invalid-edit)
- Terms: Editor, Text failure, Validation
- Inputs/outputs: Invalid edit attempt → refusal Text failure, no file write
- Invariants and failures: Already-invalid on-disk file unchanged by refused actions
- Depends on: CONTRACT-validate-manifest, MODEL-grid-cell-rules, STATE-editor-manifest-lifecycle
- Acceptance checks: AC-refuse-invalid-edit
- Evidence: Tests for overlap move, bad type, bad size, dangling Edge attempt
- Architecture impact: none
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Save-then-fail pattern

### Editor empty start (`BEHAVIOR-empty-start`)

- Meaning: With no Manifest file, Human reviewer creates a valid empty Manifest (zero Nodes, zero Edges) from Editor.
- Today: unknown.
- After: AC-empty-start; empty SVG on render.
- Type: BEHAVIOR
- Owner: frontend
- Frontier: Editor empty start (FR-empty-start)
- Terms: Editor, Manifest, Human reviewer
- Inputs/outputs: No file → empty valid Manifest on first save
- Invariants and failures: Empty Manifest passes Validation
- Depends on: STATE-editor-manifest-lifecycle, MODEL-manifest-v1
- Acceptance checks: AC-empty-start
- Evidence: Create-new flow test
- Architecture impact: none
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Template Nodes on create

### Repair invalid Manifest (`BEHAVIOR-repair-invalid`)

- Meaning: Parseable invalid Manifest opens with Diagram and Text failure; save only after Validation passes; unparseable Manifest shows Text failure without editable Diagram; disk unchanged until valid save.
- Today: unknown.
- After: AC-repair-invalid.
- Type: BEHAVIOR
- Owner: frontend
- Frontier: Editor repair invalid Manifest (FR-repair-invalid)
- Terms: Editor, Text failure, Manifest, Diagram
- Inputs/outputs: Open file → repair UI or parse failure UI → valid save persists
- Invariants and failures: Q-open-invalid resolved behavior
- Depends on: STATE-editor-manifest-lifecycle, CONTRACT-validate-manifest
- Acceptance checks: AC-repair-invalid
- Evidence: Fixtures for invalid-but-parseable and corrupt JSON open flows
- Architecture impact: none
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Auto-repair without Human reviewer action

### Read Manifest without SVG (`BEHAVIOR-read-manifest`)

- Meaning: All Node and Edge fields needed for review are explicit in JSON text.
- Today: unknown (no schema).
- After: Human reviewer can list ids, types, sizes, labels, Grid positions, Edge endpoints and labels from Manifest alone (AC-read-manifest).
- Type: BEHAVIOR
- Owner: backend
- Frontier: Read Manifest without SVG (FR-read-manifest)
- Terms: Manifest, Node, Edge, Human reviewer
- Inputs/outputs: Manifest JSON → human-readable field set
- Invariants and failures: No required field hidden only in SVG
- Depends on: MODEL-manifest-v1
- Acceptance checks: AC-read-manifest
- Evidence: Documented field list matches golden Manifest sample
- Architecture impact: none
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Binary Manifest encoding

### No execution from Manifest (`BEHAVIOR-no-execution`)

- Meaning: Validation and Render treat Manifest as data only; they do not run commands embedded in labels or other strings.
- Today: unknown.
- After: AC-no-execution with command-like strings in Manifest.
- Type: BEHAVIOR
- Owner: backend
- Frontier: No execution from Manifest (FR-no-execution)
- Terms: Validation, Render, Manifest
- Inputs/outputs: Manifest containing shell-like strings → no side effects beyond validate/render
- Invariants and failures: No process spawn driven by Manifest content parsing
- Depends on: CONTRACT-validate-manifest, CONTRACT-render-manifest
- Acceptance checks: AC-no-execution
- Evidence: Tests with malicious strings in labels; assert no subprocess
- Architecture impact: none
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Sandboxing the Editor OS process beyond this rule

### Runnable Skema v1 product (`BEHAVIOR-runnable-product`)

- Meaning: The repository delivers an integrated local product instead of documentation-only intent.
- Today: Documentation only (Discovery index DISC-index v1; PRD § Today).
- After: Human reviewer and Agent each complete PRD journeys on one shared Manifest in a workspace; all PRD acceptance criteria pass in an integrated scenario.
- Type: BEHAVIOR
- Owner: backend
- Frontier: Runnable diagram product (FR-runnable-product)
- Terms: Command-line tool, Editor, Validation, Render
- Inputs/outputs: Workspace with Manifest and SVG paths → both journeys succeed
- Invariants and failures: Definition of done in PRD satisfied
- Depends on: All other requirements in this index
- Acceptance checks: Full AC list in coverage matrix
- Evidence: Integrated workspace scenario test script or checklist run
- Architecture impact: update
- Durable docs/ADRs: none
- Domain drafts: none
- Non-goals: Shipping hosted service in v1

## Dependency graph

```text
MODEL-manifest-v1
MODEL-grid-cell-rules → MODEL-manifest-v1
CONTRACT-validate-manifest → MODEL-manifest-v1, MODEL-grid-cell-rules
CONTRACT-render-manifest → CONTRACT-validate-manifest
CONTRACT-cli-surface → CONTRACT-validate-manifest, CONTRACT-render-manifest
CONTRACT-editor-json-handoff → MODEL-manifest-v1, CONTRACT-cli-surface
CONTRACT-launch-editor-pane → CONTRACT-cli-surface, CONTRACT-editor-json-handoff
STATE-editor-manifest-lifecycle → CONTRACT-validate-manifest, CONTRACT-editor-json-handoff
BEHAVIOR-validation-gate → CONTRACT-validate-manifest, CONTRACT-render-manifest
BEHAVIOR-deterministic-render → CONTRACT-render-manifest
BEHAVIOR-agent-render → CONTRACT-cli-surface, BEHAVIOR-validation-gate
BEHAVIOR-editor-save-render → CONTRACT-render-manifest, STATE-editor-manifest-lifecycle, BEHAVIOR-validation-gate
BEHAVIOR-editor-roundtrip → STATE-editor-manifest-lifecycle, MODEL-manifest-v1
BEHAVIOR-refuse-invalid-edit → CONTRACT-validate-manifest, MODEL-grid-cell-rules, STATE-editor-manifest-lifecycle
BEHAVIOR-empty-start → STATE-editor-manifest-lifecycle, MODEL-manifest-v1
BEHAVIOR-repair-invalid → STATE-editor-manifest-lifecycle, CONTRACT-validate-manifest
BEHAVIOR-read-manifest → MODEL-manifest-v1
BEHAVIOR-no-execution → CONTRACT-validate-manifest, CONTRACT-render-manifest
BEHAVIOR-runnable-product → all above
```

## Cross-area contracts

| Contract | Producer | Consumer | Payload |
| --- | --- | --- | --- |
| Validate Manifest | backend | frontend Editor, CLI | Manifest JSON → pass or Text failure |
| Render Manifest | backend | CLI, Editor after save | Validated Manifest → SVG path |
| Editor JSON handoff | backend CLI | frontend Editor | JSON Manifest + file paths |
| Launch Editor in pane | infrastructure | frontend Editor | Pane session with handoff |

## Facts to verify before build

| Fact | Status | Owner | Checkpoint |
| --- | --- | --- | --- |
| Pane-hosted Editor: command-line tool opens one Angular Editor in a Herdr pane without a second terminal canvas (Decision Pane-hosted Editor D-003) | assumed | infrastructure | technical-requirements approval checkpoint |
| Herdr pane launch mechanism and env vars for local dev are available on builder machines | assumed | infrastructure | technical-requirements approval checkpoint |
| Editor save invokes backend Render in-process or via CLI wrapper (same Render entry as Agent) | assumed | frontend | technical-requirements approval checkpoint |

## Architecture documentation plan

Published [architecture-overview.md](architecture-overview.md) holds models, contracts, and diagrams. No separate ADR in v1 unless launch pane decision needs one during build.

### Diagram plan

- **Purpose:** Show shared Validation and Render across Agent CLI path and Human reviewer Editor path, plus Herdr pane launch.
- **Type:** flowchart
- **Home:** architecture-overview.md § System flow
- **Source requirements:** BEHAVIOR-runnable-product, BEHAVIOR-agent-render, BEHAVIOR-editor-save-render, CONTRACT-launch-editor-pane
- **Convention and validation:** Manual syntax review per diagram policy; repository has no Mermaid renderer.

- **Purpose:** Clarify Editor states for empty start, valid edit, parseable invalid repair, and unparseable open.
- **Type:** stateDiagram-v2
- **Home:** architecture-overview.md § Editor Manifest states
- **Source requirements:** STATE-editor-manifest-lifecycle, BEHAVIOR-repair-invalid, BEHAVIOR-empty-start, BEHAVIOR-refuse-invalid-edit
- **Convention and validation:** Manual syntax review per diagram policy; repository has no Mermaid renderer.

## Migration and rollout

Greenfield: no migration. Rollout is local install/build of command-line tool and Editor in developer workspace.

## Local and hosted configuration

- **Local:** Workspace paths for Manifest and SVG supplied to CLI and Editor handoff; Herdr pane configuration for Editor launch on developer machines.
- **Hosted:** none (Decision Webpage later D-005).

## Coverage matrix

| Frontier item | Requirement IDs | Acceptance check |
| --- | --- | --- |
| Runnable diagram product (FR-runnable-product) | BEHAVIOR-runnable-product, CONTRACT-cli-surface, CONTRACT-launch-editor-pane, CONTRACT-editor-json-handoff | Integrated workspace scenario: all PRD AC pass; CLI and Editor present |
| JSON Manifest source of truth (FR-json-manifest) | MODEL-manifest-v1, CONTRACT-editor-json-handoff | AC-read-manifest; valid JSON round-trip; malformed JSON → AC-invalid-manifest |
| Shared Validation before SVG (FR-validation-gate) | CONTRACT-validate-manifest, BEHAVIOR-validation-gate | AC-invalid-manifest; AC-preserve-svg; same Text failure from CLI and Editor |
| Deterministic SVG render (FR-deterministic-render) | CONTRACT-render-manifest, BEHAVIOR-deterministic-render | AC-deterministic; AC-svg-matches |
| Agent render without Editor (FR-agent-render) | BEHAVIOR-agent-render, CONTRACT-cli-surface | AC-agent-render; AC-reviewable-change via CLI only |
| Editor grid roundtrip (FR-editor-roundtrip) | BEHAVIOR-editor-roundtrip, BEHAVIOR-editor-save-render | AC-editor-roundtrip; AC-grid-positions; AC-svg-matches after save |
| Editor refuse invalid edit (FR-refuse-invalid-edit) | BEHAVIOR-refuse-invalid-edit, MODEL-grid-cell-rules | AC-refuse-invalid-edit |
| Editor empty start (FR-empty-start) | BEHAVIOR-empty-start, STATE-editor-manifest-lifecycle | AC-empty-start |
| Editor repair invalid Manifest (FR-repair-invalid) | BEHAVIOR-repair-invalid, STATE-editor-manifest-lifecycle | AC-repair-invalid |
| Read Manifest without SVG (FR-read-manifest) | MODEL-manifest-v1, BEHAVIOR-read-manifest | AC-read-manifest |
| No execution from Manifest (FR-no-execution) | BEHAVIOR-no-execution | AC-no-execution |

| Requirement ID | Acceptance check |
| --- | --- |
| MODEL-manifest-v1 | AC-read-manifest lists all fields from golden JSON; malformed JSON fails Validation |
| MODEL-grid-cell-rules | AC-invalid-manifest for negative, fractional, and overlapping Grid positions |
| CONTRACT-validate-manifest | AC-invalid-manifest all rule cases; shared vectors with Editor |
| CONTRACT-render-manifest | AC-svg-matches; Render rejects without prior Validation success |
| CONTRACT-cli-surface | AC-agent-render; validate/render/open-editor subcommands succeed in workspace test |
| CONTRACT-editor-json-handoff | AC-empty-start bootstrap; open existing Manifest matches file JSON |
| CONTRACT-launch-editor-pane | Smoke: CLI opens Editor in Herdr pane with handoff |
| STATE-editor-manifest-lifecycle | AC-repair-invalid, AC-empty-start, AC-refuse-invalid-edit state transitions |
| BEHAVIOR-validation-gate | AC-preserve-svg; no SVG write on Validation failure |
| BEHAVIOR-deterministic-render | AC-deterministic byte-identical output |
| BEHAVIOR-agent-render | AC-agent-render; AC-reviewable-change without Editor |
| BEHAVIOR-editor-save-render | After valid Editor save, SVG matches Manifest (AC-editor-roundtrip) |
| BEHAVIOR-editor-roundtrip | AC-editor-roundtrip; AC-grid-positions |
| BEHAVIOR-refuse-invalid-edit | AC-refuse-invalid-edit |
| BEHAVIOR-empty-start | AC-empty-start |
| BEHAVIOR-repair-invalid | AC-repair-invalid |
| BEHAVIOR-read-manifest | AC-read-manifest |
| BEHAVIOR-no-execution | AC-no-execution |
| BEHAVIOR-runnable-product | Full PRD AC checklist in one integrated scenario |

## Numbered decisions and consequences

| Decision | Consequence in this spec |
| --- | --- |
| Angular Editor (D-001) | Editor owner frontend; Angular implements STATE and BEHAVIOR Editor requirements |
| Rust core (D-002) | Validation, Render, CLI owner backend |
| Pane-hosted Editor (D-003) | CONTRACT-launch-editor-pane owner infrastructure; recorded as assumed fact |
| JSON handoff (D-004) | CONTRACT-editor-json-handoff |
| Webpage later (D-005) | No hosting; handoff keeps Editor able to accept JSON for a later skin |
| Minimal tooling (D-006) | Nineteen requirements cover full frontier without extra products |

## Open questions

- **Q-svg-trigger:** Closed. Primary journey: SVG after valid Editor save via shared Render. Agent: explicit render command without Editor. Same Rust Render entry (BEHAVIOR-editor-save-render, BEHAVIOR-agent-render).
- **Q-grid-domain:** Closed. Non-negative integers; unbounded maximum; reject negatives and non-integers (MODEL-grid-cell-rules).
- **Q-overlap-rule:** Closed. Same Grid cell occupancy; reject overlap; no pixel hit-testing (MODEL-grid-cell-rules).
