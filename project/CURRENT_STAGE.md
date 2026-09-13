# Current AICAD Stage

stage: transition
from_stage: 3
to_stage: 4
name: Stage 3 -> Stage 4 transition and initialization preparation
status: complete-pending-owner-review-and-merge
stage4_readiness: ready-not-implemented
next_task_after_transition_merge: AICAD-080

## Stage 0 — closed

Passed. Owner approval is recorded in `project/DECISION_LOG.md#DL-10`; historical evidence is preserved under the stage-specific report/gate archives.

## Stage 1 — closed

Passed. Owner approval is recorded in `project/DECISION_LOG.md#DL-11`; historical evidence is preserved under the stage-specific report/gate archives.

## Stage 2 — closed

Passed. Owner approval is recorded in `project/DECISION_LOG.md#DL-16`; historical evidence is preserved under the stage-specific report/gate archives.

## Stage 3 — complete, owner-approved, and merged

**Stage 3 has passed.** Owner approval is recorded in `project/DECISION_LOG.md#DL-22`. The accepted Stage-3 implementation was merged to `main` at `15fc5a37e4382de717e426ccc5317a491be264cd`; its lineage includes final incremental-build remediation commit `99fb0d3b17000b0a1c6a1a3c175ea16f0d450180`.

The final gate is `project/gates/stage-3-gate.md`. Stage 3 established the accepted parametric single-part CAD foundation, including typed parameters, `ParamModel`, `FeatureGraph` incremental state/provenance, the remediated `ParametricBuildSession`, current Safe CAD RuntimeBuiltins/spatial values, parts/named outputs, sketch/constraint substrate, modeling operations, exact geometry/STEP integration, and the frozen Stage-4 reference corpus.

## Stage 3 -> Stage 4 transition — complete pending owner review/merge

The canonical transition branch is `claude/aicad-stage4-transition`. Major transition work is complete:

- historical reconciliation/archive preservation;
- current user/developer documentation and root landing-page modernization;
- source/internal-sketch and Stage-3-ID-vs-persistent-reference boundary cleanup;
- canonical specification/RFC cleanup and D5 tolerance taxonomy clarification;
- D20 stale-status correction plus owner recording of D21-D30 as DL-23..DL-32;
- AICAD-079C Stage-4 CI/CD expansion and readiness plumbing: layered PR/integration CI, D5-aware determinism tests, resolver-independent 079A corpus grading infrastructure, permanent silent-misselection regression policy, bounded parser fuzzing, ASan/UBSan scheduling, bounded property invariants, explicit Linux support tier, performance/security/release foundations, failure artifacts, and branch-protection recommendations;
- Stage-4 queue correction: AICAD-079C is the final transition task and AICAD-080 depends on it; AICAD-092/096 metadata is aligned with D7 and the already-frozen 079A corpus.

The detailed readiness record is `project/planning/transitions/stage3-to-stage4/STAGE4_READINESS.md`; task evidence is `project/reports/AICAD-079C.md`.

## Stage 4 — READY, not yet implemented

`AICAD-080` remains `status: todo`. The repository is prepared for Stage 4 but this transition does **not** record Stage-4 owner approval or start implementation.

The hard gate remains fail-closed:

- `Resolved(exactly one intended entity)` — good;
- `Ambiguous(candidates + evidence)` — good;
- `Broken(reason/evidence)` — acceptable/expected where necessary;
- silent wrong selection — catastrophic and must become a permanent minimized regression.

Never use arbitrary first-candidate selection, raw topology enumeration order, hidden kernel pointer identity, or silent fingerprint recovery as authoritative identity. Fingerprints may be diagnostic evidence, ranking input, or benchmark information only unless a later explicit owner ruling changes D7.

No persistent `VertexRef`/`EdgeRef`/`WireRef`/`FaceRef`/`ShellRef`/`SolidRef`, query/resolution pipeline, lineage/resolver production behavior, Stage-5 raw geometry/query materialization, interfaces, assemblies, configurations, external-asset system, or AICAD-101+ implementation is introduced by AICAD-079C.

## Owner flow after reviewing this transition

1. Owner reviews `claude/aicad-stage4-transition` and its AICAD-079C evidence.
2. If accepted, owner merges that transition branch to `main`.
3. Create `claude/aicad-stage4-dev` from the **exact merged `main` HEAD**.
4. All sequential Stage-4 agents synchronize to the newest `origin/claude/aicad-stage4-dev`; do not independently recreate Stage-4 work from some other `main` state.
5. Owner authorizes Stage-4 implementation and the first implementation task is AICAD-080.
6. Do not begin AICAD-101+ / Stage 5 until the Stage-4 hard gate is later passed by the owner.

The Stage-4 development branch is intentionally **not** created by this transition pass so its ancestry remains unambiguous after merge.
