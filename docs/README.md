# AICAD documentation

AICAD documentation is separated by audience and authority.

## User documentation — `docs/user/`

How to **use AICAD as it exists today**: source language, typed/parametric modeling, persistent semantic references, Stage-5 advanced geometry/topology, CLI workflows, maintained examples, and current limitations. User documentation describes the completed Stage-5 product surface and labels Stage-6 assemblies/configurations as planned rather than implemented.

## Developer documentation — `docs/developer/`

How the current system works: compiler/runtime pipeline, Geometry IR, kernel boundary, `ParamModel`/`FeatureGraph` incremental rebuilds, geometry/runtime contracts, semantic-reference resolution and lineage evidence, testing, and contributor workflow.

## Internal development state/history — `project/`

Active stage/task state, owner decisions, reports, gates, transition records, experiments, and roadmap planning. Historical evidence remains evidence from the time it was written; it is not rewritten to look like current product documentation.

## Frozen foundation plan

`docs/plan/` is retained for traceability because older evidence and source comments reference it. It is historical/foundation planning, not the current user or developer manual. Current behavior is governed by accepted owner decisions/specifications and the implementation, then documented under `docs/user/` and `docs/developer/`.

`docs/site/` is a legacy empty site-source scaffold and is not documentation authority.
