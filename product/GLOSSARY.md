# Product Glossary

Use these terms consistently in product documents and acceptance criteria.
Prefer one canonical term for each concept.

Rows below are the terms this product uses. A PRD records a term here when it introduces one.
Actors are confirmed in [PERSONAS.md](PERSONAS.md).

| Term | Meaning | Avoid / related terms |
| --- | --- | --- |
| Manifest | The JSON document that is the only source of truth for one diagram. | flow file, source SVG, diagram XML |
| JSON | The text format of a Manifest. | YAML, yml |
| SVG | The generated picture of a Manifest. It is disposable and is not edited as source. | image, render output |
| Grid | The fixed coordinate system for a diagram. Positions are grid cells. | pixel position, freeform canvas |
| Node | A labeled box in a Manifest with a stable id, a type, a size, and a grid position. | box |
| Edge | A directed connection from one Node to another. It may carry a label. | arrow, line, connector |
| Node type | The kind of Node. The design documents name process, decision, and note. | shape |
| Node size | The predefined size of a Node. The design documents name small, medium, and large. | arbitrary dimensions |
| Validation | The check that a Manifest obeys the format rules before an SVG is written. | lint, schema check |
| Diagram | The flow a Manifest describes. It is a picture of a flow, not executable automation. | workflow, pipeline, script |
| Human reviewer | The person who reads a Manifest, reviews an SVG, and arranges a Diagram in the Editor. | user, author, person |
| Agent | The actor that edits a Manifest as text and produces an SVG without the Editor. | coding agent, LLM |
| Editor | The window the Human reviewer uses to move Nodes, connect Edges, and edit labels. The Command-line tool owns it. It reads and writes a Manifest. | browser tab, GUI as a separate source of truth |
| Command-line tool | The local program that validates a Manifest, renders an SVG, and owns the Editor window. It accepts close and save requests for that window. | browser, hosted service |
| Herdr | An optional local pane host that may show the Editor window. The Editor does not require it. | required shell, Pallas Agent Flow worker |
