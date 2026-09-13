# Current AICAD Stage

stage: transition
from_stage: 3
to_stage: 4
name: Stage 3 -> Stage 4 transition and initialization preparation
status: active

## Stage 0 — closed

Passed. Owner approval is recorded in `project/DECISION_LOG.md#DL-10`.
Completed implementation/gate evidence is preserved under
`project/reports/archive/stage0/` and `project/gates/archive/stage0/`.

## Stage 1 — closed

Passed. Owner approval is recorded in `project/DECISION_LOG.md#DL-11`.
Completed implementation/gate evidence is preserved under
`project/reports/archive/stage1/` and `project/gates/archive/stage1/`.

## Stage 2 — closed

Passed. Owner approval is recorded in `project/DECISION_LOG.md#DL-16`.
Completed implementation/gate evidence is preserved under
`project/reports/archive/stage2/` and `project/gates/archive/stage2/`.

## Stage 3 — closed, owner-approved, and merged

**Stage 3 has passed.** Owner approval is recorded once in
`project/DECISION_LOG.md#DL-22`.

The accepted Stage-3 implementation was merged to `main` at:

`15fc5a37e4382de717e426ccc5317a491be264cd`

That merge contains the final incremental-build remediation lineage,
including commit:

`99fb0d3b17000b0a1c6a1a3c175ea16f0d450180`

The final gate is `project/gates/stage-3-gate.md`; completed Stage-3 task
and checkpoint evidence is preserved under `project/reports/archive/stage3/`
and `project/gates/archive/stage3/`.

Stage 3 established the accepted parametric single-part CAD foundation:
typed parameters/derived expressions; `ParamModel`; a modeling
`FeatureGraph` with dependency-aware dirty propagation, cache keys, and
source provenance; the remediated in-process `ParametricBuildSession`
production path; the current Safe CAD RuntimeBuiltin surface; spatial
values; `part` and named outputs; sketch/constraint IR and solving; solved
profile lowering; high-level modeling operations; exact geometry/STEP
integration; and the frozen Stage-4 reference benchmark.

## Active transition state

The repository is in an explicit **Stage-3 -> Stage-4 transition**, not in
Stage-4 implementation.

Current transition work separates and improves current documentation,
preserves completed development evidence, reconciles governance and future
planning, and prepares later specification/CI initialization work without
changing production semantics.

The transition branch is:

`claude/aicad-stage4-transition`

The first reconciliation pass imported the frozen post-100 audit into
`project/planning/roadmap/post100/`, archived Stage-0..3 evidence while
retaining compatibility pointers, and established the `docs/user/` /
`docs/developer/` / `project/` information architecture. The second pass
populates the current user/developer documentation and records the already-
made Stage-3 owner approval.

## Stage 4 — not started

`AICAD-080` remains `status: todo` in `project/TASKS.yaml`.

No Stage-4 semantic-reference implementation is authorized by the current
documentation transition pass. In particular, the repository must not yet
implement or claim as current:

- persistent `VertexRef` / `EdgeRef` / `WireRef` / `FaceRef` / `ShellRef` /
  `SolidRef` semantics;
- the Stage-4 query/resolution pipeline;
- automatic topology-reference recovery;
- Stage-4 lineage/resolver production behavior.

Stage-3 named outputs (`--name <binding>[.<field>]`) remain exact source-
name selection, not persistent topology identity.

## Remaining transition work after the documentation pass

Subject to separate owner-reviewed transition instructions, the remaining
major preparation areas are:

1. normative specification/decision cleanup already identified by the
   post-100 audit, without silently resolving open architecture choices;
2. expanded Stage-4 CI/CD preparation;
3. final Stage-4 initialization/authorization.

Those are not authorized merely by appearing here. The current pass stops
after documentation/governance reconciliation and owner review.

## Owner approval required to begin Stage-4 implementation

Yes. Stage-3 approval is complete, but that approval does not itself begin
`AICAD-080`. Stage-4 implementation remains blocked until the transition is
reviewed and a later initialization/authorization explicitly opens the
Stage-4 implementation window.
