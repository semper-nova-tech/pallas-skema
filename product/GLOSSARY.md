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
| Node size | The size of a Node: a Footprint, or the keyword shorthands small, medium (2×1), and large (3×1). | arbitrary dimensions |
| Footprint | The whole Grid cells a Node box covers, 1 to 3 columns by 1 to 3 rows, starting at its grid position. | pixel size, span |
| Canvas | The size of a Diagram in whole Grid cells, set in the Manifest. Every Node box must fit inside it. Without one, the canvas starts at 16×10 and grows to fit. | page, artboard, viewport |
| Validation | The check that a Manifest obeys the format rules before an SVG is written. | lint, schema check |
| Diagram | The flow a Manifest describes. It is a picture of a flow, not executable automation. | workflow, pipeline, script |
| Human reviewer | The person who reads a Manifest, reviews an SVG, and arranges a Diagram in the Editor. | user, author, person |
| Agent | The actor that edits a Manifest as text and produces an SVG without the Editor. | coding agent, LLM |
| Editor | The window the Human reviewer uses to move Nodes, connect Edges, and edit labels. The Command-line tool owns it. It reads and writes a Manifest. | browser tab, GUI as a separate source of truth |
| Command-line tool | The local program that validates a Manifest, renders an SVG, and owns the Editor window. It accepts close and save requests for that window. | browser, hosted service |
| Herdr | An optional local pane host that may show the Editor window. The Editor does not require it. | required shell, Pallas Agent Flow worker |
| Separator | A solid or dashed horizontal or vertical line on Grid lines, with an optional label, marking a boundary or tier. It never crosses a Node. | group, container, swimlane box |
| Accent | An optional named color (`red`, `yellow`, `green`, `cyan`, `blue`, `magenta`) on one Node, Edge, or Separator. Each theme supplies its shade. | hex color, custom style |
| Pulse | An optional animation on an Edge: `forward` (one way) or `both` (back and forth, meaning two-way traffic). | flow animation, marching ants |
| Label warning | A non-blocking stderr message saying a label was shortened, overlaps something, or the Diagram is off-center. `--strict` makes it a failure. | lint error, Validation failure |
| Copy menu | The Editor menu that copies the Diagram as PNG, SVG, or GIF. | export dialog |
