# Implement Rust manifest core
Status: in-progress

Before: the repository has no Manifest schema, Validation, or Render.
After: the `skema-core` Rust library defines Manifest v1, Grid rules, `validate_manifest`, and `render_manifest` with deterministic SVG and validation-gated writes.
See it: run `cargo test` in the workspace root; golden Manifest JSON parses and validates; two renders produce byte-identical SVG; invalid Manifest returns Text failure without changing an existing SVG file.
Files: new Rust workspace at repository root (`Cargo.toml`, `crates/skema-core/**`, golden fixtures under `crates/skema-core/tests/fixtures/`).
Checks: `cargo test` from the repository root — pass.

## Slice

Owns the backend data plane: [Manifest (model)](../architecture-overview.md#manifest-model), [Validation (contract)](../architecture-overview.md#validation-contract), [Render (contract)](../architecture-overview.md#render-contract).

## Glossary rows

Manifest, JSON, SVG, Validation, Render, Text failure, Grid, Grid position, Node, Edge, Node type, Node size.

## Nominal flow

```text
Manifest JSON → validateManifest → pass → renderManifest → SVG bytes
              └→ fail → Text failure (SVG unchanged)
```

## Requirements

MODEL-manifest-v1, MODEL-grid-cell-rules, CONTRACT-validate-manifest, CONTRACT-render-manifest, BEHAVIOR-validation-gate, BEHAVIOR-deterministic-render, BEHAVIOR-read-manifest, BEHAVIOR-no-execution.
