# Example design specs

Nine Diagrams. Three subjects, each drawn under three different symmetries,
so the same content can be compared shape against shape. Each spec names the
subject, the symmetry to use, the Nodes and Edges, and what a good result
looks like. Layout (exact `gridPosition`, footprints, sides, canvas) is the
diagramming agent's job, following `skills/skema-diagram/SKILL.md`.

Output for spec `<name>` goes to `examples/design-experiments/<name>.json`,
`<name>.svg`, and `<name>.png`. Render the PNG beside the Manifest so a
reviewer can open it. Do not open the Editor.

## Symmetry vocabulary

- **Symmetric triangle**: one apex Node at the top, children spread evenly on a
  row below, mirror-symmetric about the apex's center column. Fan-out.
- **Inverted triangle**: the mirror image. Many Nodes on top converge to one
  apex at the bottom. Fan-in.
- **Diamond**: a symmetric triangle stacked on an inverted one. Fan-out, then
  fan-in to a single collector. Common for scatter-gather.
- **Hub**: one Node at the center, spokes placed around it on all four sides
  (above, below, left, right, or on diagonals), symmetric about both axes.
  Spokes talk to the hub, not to each other.
- **Horizontal triangle**: the triangle rotated 90°. One apex at the left,
  children stacked in a column to the right, mirror-symmetric about the apex's
  center row. Reads left to right as the main flow direction.
- **Bi-modal asymmetry**: a decision with two unequal branches. The short
  branch (one or two Nodes) and the long branch (several Nodes or a loop) sit
  on opposite sides of the main line. The visual weight is deliberately
  unequal; the eye should see at once which path is the cheap one.

A good Diagram makes the symmetry visible at a glance: Nodes that mirror each
other share a row or column and an equal distance from the axis, Edges on
mirrored branches take the same route, and the whole figure is centered on
the canvas.

---

## 1. kafka-horizontal-triangle

**Subject**: Kafka pipeline. **Symmetry**: horizontal triangle, twice. Three
producers fan in to one topic (apex on the right of the first triangle), then
the topic fans out to three consumers (apex on the left of a second triangle).
Together they form a horizontal diamond read left to right.

Nodes:
- `web`, `mobile`, `batch`: producers (Web app, Mobile app, Batch import).
- `topic`: Kafka topic `orders`, 2×2, the center of the figure.
- `fraud`, `fulfil`, `analytics`: consumer groups (Fraud check, Fulfilment,
  Analytics).

Edges: each producer → `topic` (label "produce" on one only), `topic` → each
consumer (labels "fraud-cg", "fulfil-cg", "analytics-cg"). Pulse `forward` on
the three topic → consumer Edges only.

Good: `topic` sits on the center row and center column. The producer column
and consumer column are equidistant from the topic. The middle producer and
middle consumer are straight horizontal lines through the topic.

## 2. kafka-inverted-triangle

**Subject**: Kafka pipeline. **Symmetry**: inverted triangle. Four event
sources on the top row converge through a topic and a stream processor to a
single warehouse at the bottom.

Nodes:
- `clicks`, `orders`, `payments`, `returns`: sources (Clickstream, Orders
  service, Payments, Returns).
- `topic`: Kafka topic `events`, 2×2 or 3×1, below the sources.
- `flink`: Stream processor (Flink job, "join and enrich").
- `warehouse`: Data warehouse, 2×2, the apex at the bottom.

Edges: each source → `topic`, `topic` → `flink`, `flink` → `warehouse`. Pulse
`forward` on `flink` → `warehouse` only. Add one dashed Separator between the
sources and the topic labeled "Kafka cluster", spanning only the Nodes it
divides.

Good: the four sources are evenly spaced and centered over the topic. The
vertical spine topic → flink → warehouse is one straight line (same column
parity). The figure narrows steadily top to bottom.

## 3. kafka-hub

**Subject**: Kafka pipeline. **Symmetry**: hub. The broker cluster is the
center. Producers sit on the left and top, consumers on the right and
bottom, so the hub has traffic on all four sides.

Nodes:
- `broker`: Kafka cluster, 3×2 or 3×3, the hub.
- `api`: API gateway (left).
- `cron`: Scheduled jobs (top).
- `search`: Search indexer (right).
- `lake`: Data lake sink (bottom).
- `schema`: Schema registry, 1×1 or 2×1, placed on a diagonal (top right),
  connected to `broker` with a `both` pulse.

Edges: `api` → `broker`, `cron` → `broker`, `broker` → `search`, `broker` →
`lake`, `schema` ↔ `broker` (`pulse: both`). Labels: "produce" on the api
Edge, "consume" on the search Edge, "validate" on schema.

Good: the hub is on the exact center of the canvas. Left and right spokes
share the hub's center row; top and bottom spokes share its center column.
Every spoke is the same distance from the hub. No spoke Edge bends.

## 4. orchestrator-triangle

**Subject**: an orchestrator that fans out sub-agents. **Symmetry**: diamond
(symmetric triangle over an inverted triangle). The orchestrator is the top
apex, five sub-agents form the wide middle row, and a single synthesizer is
the bottom apex.

Nodes:
- `user`: User request, 1×1 or 2×1, above the orchestrator.
- `orch`: Orchestrator, 3×1, the top apex.
- `discover`, `plan`, `build`, `review`, `report`: sub-agents (Discovery,
  Planner, Builder, Reviewer, Reporter), one row.
- `synth`: Synthesizer ("merge results"), 3×1, the bottom apex.
- `answer`: Final answer, 1×1 or 2×1, below the synthesizer.

Edges: `user` → `orch`; `orch` → each sub-agent, `bottom` → `top`, labeled
with a two-word task each ("map repo", "write plan", "make change", "check
diff", "summarize"); each sub-agent → `synth`, `bottom` → `top`; `synth` →
`answer`. Pulse `forward` on `orch` → `build` and `build` → `synth` only (the
critical path).

Good: `build` is the center sub-agent, directly under `orch` and over `synth`,
with a straight vertical spine user → orch → build → synth → answer. The
other four sub-agents mirror in pairs. Fan-out Edges share one trunk below
`orch`; fan-in Edges share one trunk above `synth`.

## 5. orchestrator-hub

**Subject**: an orchestrator that fans out sub-agents. **Symmetry**: hub.
The orchestrator is the center. Four sub-agents sit on the four sides. The
orchestrator exchanges messages with each (task out, result back).

Nodes:
- `orch`: Orchestrator, 3×2, the hub.
- `research`: Research agent (top).
- `code`: Coding agent (right).
- `test`: Test agent (bottom).
- `docs`: Docs agent (left).
- `human`: Human, 1×1, on a diagonal (top left), connected to `orch`.

Edges: `orch` ↔ each sub-agent with `pulse: both` (four Edges). `human` →
`orch` labeled "goal", `orch` → `human` not drawn (the `both` pulse is for
sub-agents only). Edge labels on the spokes: "research", "implement",
"verify", "document".

Good: four spokes at equal distance, each a straight line with an arrowhead
at both ends. The human sits off-axis and reads as the outside actor. The
figure is centered on both axes.

## 6. orchestrator-horizontal-triangle

**Subject**: an orchestrator that fans out sub-agents. **Symmetry**:
horizontal triangle. The orchestrator is the left apex. Sub-agents are a
column to its right. Each sub-agent writes to a shared result store on the
far right, which forms a second, mirrored horizontal triangle (apex right).

Nodes:
- `orch`: Orchestrator, on the left, center row.
- `a1`, `a2`, `a3`, `a4`: sub-agents (Frontend build, Backend build, Infra
  build, Ponytail review), a column.
- `store`: Run state, 2×2, on the right, center row.
- `final`: Final report, to the right of `store`.

Edges: `orch` → each sub-agent (labels "frontend", "backend", "infra",
"review"); each sub-agent → `store`; `store` → `final`. Pulse `forward` on
`store` → `final`.

Good: `orch` and `store` share the center row. With four sub-agents there is
no middle Node, so the column straddles the center row with two above and
two below. Edges from `orch` leave by `right` and enter the column by `left`;
edges to `store` leave by `right` and enter by `left`, so no line crosses a
box.

## 7. agent-loop-bimodal

**Subject**: an agent loop that decides to skip or loop, then responds.
**Symmetry**: bi-modal asymmetry. The main line runs top to bottom: receive
message, decide, respond. The skip branch is a single short Edge on one side.
The loop branch is a chain of tool work on the other side that returns to the
decision.

Nodes:
- `msg`: User message (top).
- `decide`: Decide, `type: decision`, "need tools?".
- `respond`: Respond (bottom).
- `pick`: Pick tool; `call`: Call tool; `observe`: Observe result. These three
  form the long branch, placed on the right of the main line.

Edges: `msg` → `decide`; `decide` → `respond` labeled "skip" (the short
branch, straight down); `decide` → `pick` labeled "loop"; `pick` → `call`;
`call` → `observe`; `observe` → `decide` labeled "again", routed as a bracket
so it does not cross the forward Edges. Pulse `forward` on `observe` →
`decide` only.

Good: the short branch is visibly short and straight. The long branch is
visibly long and sits entirely on one side. The loop return re-enters
`decide` on a side no other Edge uses. Nothing on the left of the main line,
so the asymmetry is obvious.

## 8. agent-loop-triangle

**Subject**: the same agent loop. **Symmetry**: diamond, forced. `decide` is
the top apex, the two branches sit side by side on the row below with equal
weight, and both rejoin at `respond`, the bottom apex. This is the
"symmetric even when the branches are unequal" treatment, for comparison
with spec 7.

Nodes:
- `msg`: User message (top).
- `decide`: Decide, `type: decision`.
- `skip`: Draft reply directly (left of the axis).
- `tools`: Run tool loop, 3×2 with description "pick, call, observe; repeat
  until done" (right of the axis).
- `respond`: Respond (bottom apex).

Edges: `msg` → `decide`; `decide` → `skip` labeled "skip"; `decide` → `tools`
labeled "loop"; `skip` → `respond`; `tools` → `respond`; `tools` → `tools`
is not representable, so instead `tools` → `decide` labeled "again" as a
bracket on the right.

Good: `skip` and `tools` are equidistant from the axis. The spine msg →
decide → respond is straight. The loop return is a clean bracket on the far
right that does not touch the other Edges.

## 9. agent-loop-horizontal-bimodal

**Subject**: the same agent loop. **Symmetry**: bi-modal asymmetry, rotated.
The main line runs left to right: message, decide, respond. The skip Edge
goes straight along the row. The loop branch is a chain above the main row
that returns to `decide`.

Nodes:
- `msg`, `decide` (decision), `respond`: one row, left to right.
- `pick`, `call`, `observe`: the loop chain on the row above, running left to
  right, starting over `decide`.

Edges: `msg` → `decide`; `decide` → `respond` labeled "skip"; `decide` →
`pick` labeled "loop", leaving `top`; `pick` → `call` → `observe`; `observe`
→ `decide` labeled "again", entering `decide` on a free side. Pulse `forward`
on `observe` → `decide` only.

Good: the main row is a straight line with one short labeled hop. The loop
sits entirely above it in its own band. The return Edge comes around without
crossing `pick` → `call` → `observe`, and lands on a port no other Edge uses.
Below the main row is empty, so the asymmetry reads immediately.
