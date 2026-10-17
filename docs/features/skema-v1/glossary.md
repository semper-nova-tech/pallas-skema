# Terms: Pallas Skema
Status: planned

The naming contract for this feature. One row per term that crosses a boundary:
anything exported, published, configured, or sent on the wire. Private helpers stay out.
The technical spec owns shapes; this file owns names, meanings, and surfaces.

| Term | Meaning | Kind | Owner | Model | Functions | Module | Wire name | Status | Alias of |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Manifest | The JSON document that is the only source of truth for one Diagram. | model | backend | ManifestDocument | parseManifest, serializeManifest | manifest | manifest | new | |
| JSON | The text format of a Manifest. | term | backend | | | | application/json | new | |
| SVG | The generated picture of a Manifest. It is disposable and is not edited as source. | artifact | backend | SvgOutput | | svg | image/svg+xml | new | |
| Validation | The check that a Manifest obeys the format rules before an SVG is written. | service | backend | ValidationResult | validateManifest | validation | validateManifest | new | |
| Render | The production of an SVG from a Manifest that has passed Validation. | service | backend | SvgOutput | renderManifest | render | renderManifest | new | |
| Text failure | Text that names the violated Validation or parse rule and the Manifest location when that location is known. | term | backend | TextFailure | formatTextFailure | validation | textFailure | new | |
| Grid | The fixed coordinate system for a Diagram. Positions are grid cells. | model | backend | Grid | | grid | grid | new | |
| Grid position | A Node's coordinates on the Grid stored in the Manifest. | model | backend | GridPosition | | grid | gridPosition | new | |
| Node | A labeled box in a Manifest with a stable id, a type, a size, and a Grid position. | model | backend | Node | | manifest | node | new | |
| Edge | A directed connection from one Node to another. It may carry a label. | model | backend | Edge | | manifest | edge | new | |
| Node type | The kind of Node. v1 values are process, decision, and note. | enum | backend | NodeType | | manifest | type | new | |
| Node size | The predefined size of a Node. v1 values are small, medium, and large. | enum | backend | NodeSize | | manifest | size | new | |
| Diagram | The flow a Manifest describes. It is a picture of a flow, not executable automation. | term | frontend | | | diagram | | new | |
| Editor | The window the Human reviewer uses to move Nodes, connect Edges, and edit labels. The Command-line tool owns it. It reads and writes a Manifest. | client | frontend | EditorSession | loadManifest, saveManifest, refuseInvalidEdit, showTextFailure | editor | editorSession | new | |
| Command-line tool | The local Rust entry point that validates and renders Manifests, owns the Editor window, and accepts close and save requests for that window. | client | backend | CliInvocation | runValidate, runRender, runOpenEditor | cli | skema | new | |
| Human reviewer | The person who reads a Manifest, reviews an SVG, and arranges a Diagram in the Editor. | persona | frontend | | | | | new | |
| Agent | The actor that edits a Manifest as text and produces an SVG without the Editor. | persona | backend | | | | | new | |
| Herdr | An optional local pane host that may show the Editor window. The Editor does not require it. It is not the Pallas Agent Flow worker adapter. | config | infrastructure | | | | | new | |

Rules:

- Discovery adds `existing` rows from exported symbols in scope before anything new is named.
- PRD review adds `new` rows with meaning only; planning fills the surfaces.
- Each requirement, packet, and PR note names the rows it uses.
- A rename edits the row and records the old name in Alias of. A meaning change is a scope change.
- Function names are verb-first sentences. Model names are nouns. Modules are named after the term.
- Terms shared across features are promoted to `product/GLOSSARY.md` at final report.
