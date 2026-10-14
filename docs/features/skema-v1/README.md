# Pallas Skema v1
Status: planned

Pallas Skema v1 turns the approved product spec into a runnable local product: a JSON Manifest remains the only source of truth for one Diagram, a shared Rust Validation and Render path produces deterministic SVG, and an Angular Editor lets the Human reviewer arrange the Diagram on a Grid while the Agent edits Manifest as text and renders without the Editor.

## Systems touched

- **Backend:** JSON Manifest model, Validation, SVG Render, and a Rust command-line tool that validates, renders, and opens the Editor with a JSON Manifest handoff.
- **Frontend:** Angular Editor for empty start, Grid roundtrip, refuse-invalid-edit, and repair-invalid flows; Editor save runs Validation before persisting and triggers the same Render the Agent uses via the command-line tool.
- **Infrastructure (local only):** Herdr pane launch so the command-line tool opens one Editor instance in a pane—no second terminal canvas, no hosting or upload in this version.

## End state

The Human reviewer and Agent complete their PRD journeys on one shared Manifest in a workspace. Invalid Manifests yield Text failure, no new SVG, and unchanged existing SVG. Valid Manifests render to byte-stable SVG. Validation and Render never execute commands from Manifest content.

## Published pack

- [Technical Requirements](technical-spec.md) — named requirements, acceptance checks, coverage matrix
- [Architecture overview](architecture-overview.md) — today → after, models, contracts, diagrams
- [Feature glossary](glossary.md) — boundary terms and surfaces

PR sequence and per-PR notes are added after requirements approval.
