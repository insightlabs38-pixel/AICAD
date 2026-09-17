# AICAD documentation

AICAD documentation is separated by audience and by authority.

## User documentation — `docs/user/`

How to **use AICAD as it exists today**: source language, supported modeling operations, persistent semantic references, CLI workflows, maintained examples, and troubleshooting. Current user documentation describes the completed Stage-4 surface and labels Stage-5 work as planned rather than implemented.

## Developer documentation — `docs/developer/`

How the current system works: compiler/runtime pipeline, Geometry IR, kernel boundary, `ParamModel`/`FeatureGraph` incremental rebuilds, semantic-reference resolution and lineage evidence, testing, and contributor workflow.

## Internal development state/history — `project/`

Active stage/task state, owner decisions, reports, gates, transition records, experiments, and roadmap planning. Historical evidence remains evidence from the time it was written; it is not rewritten to look like current documentation.

## Frozen foundation plan

`docs/plan/` is retained for traceability because older evidence and source comments reference it. It is historical/foundation planning, not the current user or developer manual. Current behavior is governed by accepted owner decisions/specifications and the implementation, then documented under `docs/user/` and `docs/developer/`.

`docs/site/` is a legacy empty site-source scaffold and is not documentation authority.
