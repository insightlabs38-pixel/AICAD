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
- frozen-foundation-plan authority clarification.

The normative cleanup is recorded at
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

## Explicit transition blocker — D20 status conflict

The live repository records D20 as `RESOLVED — DL-21`, and current Stage-3
code follows that model. The owner-provided normative-cleanup directive
states that D20 is open. The cleanup pass does not alter
`project/OWNER_DECISIONS.md`, supersede DL-21, reopen D20, or invent a new
architecture decision.

Before final Stage-4 initialization, the owner must explicitly reconcile
whether DL-21 remains authoritative or D20 is to be reopened/superseded.

## Remaining transition work

Subject to separate owner-reviewed transition instructions, the remaining
major preparation areas are:

1. owner reconciliation of the D20 authority-status conflict;
2. expanded Stage-4 CI/CD preparation;
3. final Stage-4 initialization/authorization.

Normative specification cleanup is complete and is no longer a deferred
transition item. The remaining items are not authorized merely by appearing
here.

## Owner approval required to begin Stage-4 implementation

Yes. Stage-3 approval is complete, but that approval does not itself begin
`AICAD-080`. Stage-4 implementation remains blocked until the transition is
reviewed and a later initialization/authorization explicitly opens the
Stage-4 implementation window.
