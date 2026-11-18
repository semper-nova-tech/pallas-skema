# Blind specs

Three Diagrams described by their data only. No shape, symmetry, footprint,
side, or canvas is named; choosing those is the diagramming agent's job,
following `skills/skema-diagram/SKILL.md`.

Output for spec `<name>` goes to `examples/design-experiments/<name>.json`,
`<name>.svg`, and `<name>.png`. Do not open the Editor. Add no color.

## blind-kafka

A Kafka pipeline. Three producers (Web app, Mobile app, Batch import) write
to one topic, `orders`. Three consumer groups (Fraud check, Fulfilment,
Analytics) read from it. Highlight the three consumer flows.

## blind-orchestrator

An orchestrator takes a user request and dispatches five sub-agents in
parallel: Discovery (map repo), Planner (write plan), Builder (make change),
Reviewer (check diff), Reporter (summarize). A synthesizer merges their
results into one answer for the user. Highlight the builder's path.

## blind-agent-loop

An agent loop. A user message arrives. The agent decides whether it needs
tools. If not, it responds at once. If so, it picks a tool, calls it,
observes the result, and decides again; this may repeat several times before
it responds. Highlight the repeat.

## blind-chain

A deploy pipeline with eight steps in order: Checkout, Install, Lint, Unit
tests, Build image, Push image, Deploy staging, Smoke test. Nothing branches.
