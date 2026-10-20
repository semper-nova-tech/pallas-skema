# Architecture overview: Pallas Skema v1
Status: planned

## Today → After

- **Today:** The repository is documentation only. There is no validator, SVG renderer, command-line tool, Editor, or checked-in Manifest (Discovery index DISC-index v1 at `848e096e`).
- **After:** A Rust command-line tool owns Validation and Render on local Manifest and SVG files. The same Validation outcome gates every SVG write for the Agent path and the Editor save path. open-editor stays running and owns one Editor window for the JSON Manifest handoff. The Angular Editor is shown in that window. Close and save requests go to that process. Herdr may show the window and is not required. The Human reviewer gets SVG after a valid Editor save; the Agent gets SVG by invoking Render without the Editor.

## Manifest (model)

- **Fields:** Top-level `nodes` and `edges` arrays. Each Node has stable `id`, `label`, optional `description`, `type`, `size`, and `gridPosition` with integer `x` and `y`. Each Edge has `from` and `to` Node ids and optional `label`.
- **Invariants:** JSON parses; Node ids unique; Edge endpoints reference existing Node ids; Node `type` ∈ {process, decision, note}; Node `size` ∈ {small, medium, large}; Grid coordinates are non-negative integers with no upper bound; at most one Node per Grid cell (overlap rejected).
- **Implementation (PR-01):** `skema-core` crate modules `manifest` and `grid`; `parse_manifest` / `serialize_manifest` for JSON (`application/json`).

## Validation (contract)

- **Owner:** Backend
- **Input:** Manifest JSON text (file path or buffer)
- **Output:** Success, or Text failure naming the violated rule and Manifest location when known
- **Side effects:** None on SVG; does not execute Manifest content
- **Implementation (PR-01):** `skema-core::validation::validate_manifest`; Text failure via `format_text_failure`.

## Render (contract)

- **Owner:** Backend
- **Input:** Manifest that has passed Validation (same rules as Validation contract)
- **Output:** SVG bytes written to the requested path
- **Invariants:** Same valid Manifest → identical SVG bytes; no execution of Manifest content
- **Implementation (PR-01):** `skema-core::render::render_manifest` runs Validation first, then writes `image/svg+xml` bytes; failed Validation leaves an existing SVG file unchanged.

## Command-line tool (contract)

- **Owner:** Backend
- **Input:** Workspace paths for Manifest and SVG; subcommands validate, render, and open-editor
- **Output:** Exit code zero on success; Text failure on stderr and non-zero exit on errors. open-editor stays running, owns the Editor window, and accepts close and save until the process exits.
- **Invariants:** Render subcommand runs Validation first; Agent path uses the same library Validation and Render as the Editor save path
- **Shipped today:** Binary `skema` in crate `skema` validates, renders, and opens the Editor. `open-editor` runs the terminal Editor (crate `skema-tui`) in the current terminal, which may be a Herdr pane, until the Human reviewer quits. Every edit is validated in-process by `skema-core` before it applies; save validates, writes the Manifest, then renders. The Editor polls the Manifest and reloads when it changes on disk. `--handoff-out` writes the JSON handoff file instead of opening the Editor. A close request from outside the process is specified and not built.

## Editor launch (contract)

- **Owner:** Backend
- **Input:** JSON Manifest (initial or empty template), workspace paths for Manifest and SVG
- **Behavior:** The Rust command-line tool opens one Editor window that it owns and shows the Angular Editor there (Decision Editor window D-003, Decision JSON handoff D-004). A close request, or exiting the process, closes the window and does not write unsaved edits. A save request asks the Editor for the current Diagram, then uses the Editor save path. Herdr may show that window and is not required. No hosted webpage or upload (Decision Local window D-005).

## Editor persistence (contract)

- **Owner:** Frontend
- **Input:** In-memory Diagram edits from `EditorJsonHandoff` (`manifestJson`, `manifestPath`, `svgPath`)
- **Output:** Manifest file write only after Validation passes; Text failure on refusal or repair states; disk unchanged until a valid save
- **Validation and Render transport (PT-002):** The terminal Editor links `skema-core` directly and calls `validate_manifest` and `render_manifest` in-process; no duplicate Validation rules anywhere else. The Angular editor under `skema-editor/` is legacy and unserved.
- **After valid save:** Primary journey invokes shared Render so SVG reflects the saved Manifest; failed Validation writes no SVG and does not modify an existing SVG
- **Implementation (PR-03):** Angular app `skema-editor`; glossary module `editor` exports `EditorSession`, `loadManifest`, `saveManifest`, `refuseInvalidEdit`, and `showTextFailure`

## System flow

Cross-system paths for Validation, Render, and Editor launch.

```mermaid
flowchart LR
  subgraph backend [Rust core]
    CLI[Command-line tool]
    Val[Validation]
    Rend[Render]
    CLI --> Val
    Val -->|pass| Rend
  end
  subgraph frontend [Angular Editor]
    Ed[Editor]
  end
  Win[Editor window]
  Herdr[Herdr pane optional]
  Agent[Agent] -->|edit Manifest text| CLI
  CLI -->|render command| Rend
  Human[Human reviewer] --> CLI
  CLI --> Win
  Win --> Ed
  Herdr -.-> Win
  CLI -->|close or save| Win
  Ed -->|save after Validation| Val
  Ed -->|after valid save| Rend
  Rend --> SVG[SVG file]
  Val -->|fail| TF[Text failure]
```

## Editor Manifest states

Save and repair behavior for valid, invalid, and unparseable Manifests.

```mermaid
stateDiagram-v2
  [*] --> NoManifest: no file
  NoManifest --> ValidEmpty: create empty Manifest
  ValidEmpty --> ValidEditing: edit on Grid
  ValidEditing --> ValidEditing: refuse invalid edit
  ValidEditing --> ValidSaved: save passes Validation
  ValidSaved --> ValidEditing: continue edit
  [*] --> ParseableInvalid: open invalid JSON
  ParseableInvalid --> ValidSaved: repair and save passes Validation
  ParseableInvalid --> ParseableInvalid: save blocked
  [*] --> Unparseable: open corrupt JSON
  Unparseable --> Unparseable: no Diagram
  ValidSaved --> [*]: render SVG after save
```
