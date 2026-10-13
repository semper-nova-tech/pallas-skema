# Current Product State

> This document describes the product as it exists today. Proposed behavior
> belongs in a product PRD, not here.

## Product summary

Pallas Skema is an unbuilt diagram protocol described in this repository. The
repository contains documentation and an MIT license. It does not contain a
runnable product, a checked-in Manifest, or a generated SVG.

## Current capabilities

No product behavior exists yet. There is no validator, renderer, command,
editor, or agent skill in the tree. The YAML sample in
[pallas-skema.md](../pallas-skema.md) is an illustration in a design document,
not a shipped Manifest.

## Current users and roles

No persona is confirmed. [PERSONAS.md](PERSONAS.md) records that gap. The
design documents mention a person who reviews a diagram and an agent that
edits a Manifest. Those mentions are not approved personas.

## Current constraints and known gaps

- The repository is documentation only, so there is no current behavior to
  preserve or migrate.
- [pallas-skema.md](../pallas-skema.md) and
  [brainstorming.md](../brainstorming.md) agree that a YAML Manifest is the
  source of truth and that SVG is a generated artifact.
- Those documents disagree on the first deliverable. The product brief
  describes a desktop editor as the first version. The delivery strategy
  recommends a command-line validator and renderer, plus an agent skill, and
  defers the desktop editor. Neither document is an approved PRD.
- Distribution is the MIT License in [LICENSE](../LICENSE). No hosting,
  account, or collaboration requirement is stated as current fact.

## Source documents

- [README.md](../README.md)
- [pallas-skema.md](../pallas-skema.md) — product brief and technical design
- [brainstorming.md](../brainstorming.md) — delivery strategy
- [LICENSE](../LICENSE)
