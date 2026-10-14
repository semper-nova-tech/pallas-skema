# Implement Rust command-line tool
Status: in-progress

Before: Validation and Render exist only as library calls with no local product entry point.
After: a Rust CLI exposes validate, render, and open-editor with JSON Manifest handoff; the Agent completes edit-and-render without the Editor.
See it: edit Manifest text in a temp workspace, run `cargo test -p skema` or invoke `skema render --manifest … --svg …`; run `skema open-editor --handoff-out …` and inspect handoff JSON (Editor may be stubbed until PR-03).
Files: `crates/skema/**`, workspace `Cargo.toml`, handoff integration tests.
Checks: `cargo test` including CLI integration tests — pass.

## Slice

Extends [System flow](../architecture-overview.md#system-flow) with the Command-line tool and Agent path.

## Glossary rows

Command-line tool, Agent.

## Nominal flow

```text
Agent → CLI render → validateManifest → renderManifest → SVG
Human → CLI open-editor → JSON handoff payload (pane wiring in PR-04)
```

## Requirements

CONTRACT-cli-surface, CONTRACT-editor-json-handoff, BEHAVIOR-agent-render, BEHAVIOR-runnable-product (Agent and CLI portions; full PRD checklist completes after PR-04).

## PT-002

Handoff shape fixed: `EditorJsonHandoff` with `manifestJson`, `manifestPath`, `svgPath` (camelCase JSON). Editor-to-Rust Validation transport remains frontend-owned in PR-03.
