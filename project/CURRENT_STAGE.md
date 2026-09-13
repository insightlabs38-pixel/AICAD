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

The transition branch is:

`claude/aicad-stage4-transition`

Completed transition preparation now includes:

- import/preservation of the frozen post-100 audit as non-normative planning;
- archive preservation of Stage-0..3 evidence with compatibility pointers;
- the `docs/user/` / `docs/developer/` / `project/` information architecture;
- current user/developer documentation;
- the already-issued Stage-3 approval record (DL-22);
- source/internal sketch-profile boundary cleanup;
- Stage-3 identity versus Stage-4 persistent-reference wording cleanup;
- restoration of the canonical `specs/language/{grammar,semantics,types,diagnostics}` set;
- accepted-RFC/current-decision reconciliation while preserving exact pre-cleanup RFC snapshots under `rfcs/history/stage0/`;
- D5 determinism/equivalence and tolerance-category clarification;
- frozen-foundation-plan authority clarification;
- correction of stale transition-brief wording that incorrectly treated D20 as open; DL-21 remains authoritative and AICAD-076A implements it;
- owner recording of D21-D30 as resolved future-stage semantic baselines in DL-23 through DL-32, with implementation details explicitly deferred.

The normative/governance cleanup is recorded at
`project/planning/transitions/stage3-to-stage4/NORMATIVE_SPEC_CLEANUP.md`.

## Stage 4 — not started

`AICAD-080` remains `status: todo` in `project/TASKS.yaml`.

No Stage-4 semantic-reference implementation is authorized by the current
transition work. In particular, the repository must not yet implement or
claim as current:

- persistent `VertexRef` / `EdgeRef` / `WireRef` / `FaceRef` / `ShellRef` /
  `SolidRef` semantics;
- the Stage-4 query/resolution pipeline;
- Stage-4 lineage/resolver production behavior.

Stage-3 named outputs (`--name <binding>[.<field>]`) remain exact source-
name selection, not persistent topology identity.

## Owner decisions relevant to future stages

D20 remains resolved by `project/DECISION_LOG.md#DL-21`; AICAD-076A
implements that ruling.

D21-D30 are now resolved by DL-23 through DL-32. They establish semantic
baselines for future RuntimeBuiltin scaling, safe/raw geometry, source-visible
kernel queries, tolerance domains, feature/provenance preservation,
assembly identity, interfaces/protocols, assembly relation/pose semantics,
configurations, and external-asset identity. Recording these rulings does
**not** authorize their Stage-5/6 implementations and does not begin Stage 4.
Implementation/representation details explicitly deferred by those rulings
remain listed in `DEFERRED_TRANSITION_ITEMS.md`.

## Remaining transition work

Subject to separate owner-reviewed transition instructions, the remaining
major preparation areas are:

1. expanded Stage-4 CI/CD preparation;
2. final Stage-4 initialization/authorization.

Normative specification cleanup and the D21-D30 owner-decision recording
pass are complete. The remaining items are not authorized merely by appearing
here.

## Owner approval required to begin Stage-4 implementation

Yes. Stage-3 approval and the D21-D30 future-stage rulings do not themselves
begin `AICAD-080`. Stage-4 implementation remains blocked until the
transition is reviewed and a later initialization/authorization explicitly
opens the Stage-4 implementation window.
