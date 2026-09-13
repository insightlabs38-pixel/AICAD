# AICAD documentation

AICAD documentation is intentionally separated by audience.

## User documentation — `docs/user/`

How to **use AICAD** as it exists today: source language, current modeling operations, CLI workflows, examples, and troubleshooting. User documentation must not present planned Stage-4+ capabilities as implemented.

## Developer documentation — `docs/developer/`

How AICAD **works** and how to develop, extend, test, or integrate the current system: compiler/runtime pipeline, geometry IR, kernel boundary, feature DAG, constraints, APIs, testing, and contributor workflow.

## Internal development history — `project/`

How AICAD **was/is being developed**: active task/stage state, owner decisions, decision logs, implementation reports, gate packets, transition records, experiments, architecture audits, and future-roadmap drafts. Historical development evidence is intentionally not part of normal public documentation navigation.

## Legacy foundation plan

`docs/plan/` remains at its historical path during the Stage-3→4 transition because it is a frozen foundation plan referenced pervasively by Stage-0..3 evidence and source comments. Treat it as **internal historical/foundation planning**, not as the current user manual or developer architecture guide. Current behavior is documented under `docs/user/` and `docs/developer/` and ultimately governed by accepted decisions/specifications plus implementation.

`docs/site/` is a legacy empty site-source scaffold and is not documentation authority.
