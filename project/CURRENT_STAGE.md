# Current AICAD Stage

stage: 4
from_stage: 3
to_stage: 4
name: Stage 4 — semantic-topology-reference hard gate
status: in-progress
stage4_readiness: implementation-started
last_completed_batch: S4-00
last_completed_task: AICAD-081
next_batch: S4-01 (AICAD-082, AICAD-083, AICAD-084)

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

## Stage 3 -> Stage 4 transition — complete and merged

The transition branch (`claude/aicad-stage4-transition`) was merged to
`main` (`f587251`, PR #12). Batch S4-00 (`AICAD-080`, `AICAD-081`) treated
this merge, together with this campaign's own explicit instruction to
begin Stage-4 batches starting at `AICAD-080`, as the owner authorization
`project/SESSION_HANDOFF.md`'s prior "Owner flow after reviewing this
transition" step 5 called for. See `project/reports/AICAD-080.md`'s and
`AICAD-081.md`'s own "Base / resulting commit" sections for exact
provenance, and this file's own "Working-branch note" below for a
discrepancy this batch surfaced and did not resolve unilaterally.

## Stage 4 — in progress (Batch S4-00 complete)

`AICAD-080` (stable reference representations) and `AICAD-081` (query
AST/IR) are **done** — see `project/reports/AICAD-080.md` and
`project/reports/AICAD-081.md`. Both are representation-only: no
resolution algorithm, lineage capture, raw-handle epoch, or health report
exists yet. `AICAD-082` (Batch S4-01, next) remains `status: todo`.

The hard gate remains fail-closed:

- `Resolved(exactly one intended entity)` — good;
- `Ambiguous(candidates + evidence)` — good;
- `Broken(reason/evidence)` — acceptable/expected where necessary;
- silent wrong selection — catastrophic and must become a permanent minimized regression.

Never use arbitrary first-candidate selection, raw topology enumeration order, hidden kernel pointer identity, or silent fingerprint recovery as authoritative identity. Fingerprints may be diagnostic evidence, ranking input, or benchmark information only unless a later explicit owner ruling changes D7.

(Historical note: `AICAD-079C` itself, the final transition task, introduced
none of `VertexRef`/`EdgeRef`/`WireRef`/`FaceRef`/`ShellRef`/`SolidRef`, a
query/resolution pipeline, lineage/resolver production behavior, Stage-5
raw geometry/query materialization, interfaces, assemblies, configurations,
an external-asset system, or AICAD-101+ implementation — that boundary was
correct for the transition batch and remains true of it; `AICAD-080`/
`AICAD-081`, the first real Stage-4 implementation tasks, are what
introduced the representation types, per the "Stage 4 — in progress"
section above.)

## Working-branch note (Batch S4-00)

`project/planning/transitions/stage3-to-stage4/STAGE4_READINESS.md`'s own
prior owner-flow text (steps 1-4 below) named `origin/claude/aicad-stage4-dev`
as the canonical Stage-4 development branch, to be created fresh from the
exact merged transition-`main` HEAD. As of this batch, that branch does not
exist on `origin`; this session's own outer harness configuration instead
assigned a differently-named working branch for this repository, which at
the start of this invocation was already exactly at `origin/main` HEAD
(`f587251`) — i.e. content-equivalent to what `claude/aicad-stage4-dev`
would have been had it been created then. Per that harness configuration's
explicit "never push to a different branch without explicit permission"
instruction, Batch S4-00's commits were pushed to the assigned working
branch rather than to a newly-created `claude/aicad-stage4-dev`. This is
recorded here, not silently resolved, so the owner can either rename/adopt
the assigned branch as the canonical Stage-4 branch going forward, or
direct a future invocation to create `claude/aicad-stage4-dev` explicitly
and continue there instead.

## Owner flow after reviewing this transition (historical)

1. Owner reviews `claude/aicad-stage4-transition` and its AICAD-079C evidence.
2. If accepted, owner merges that transition branch to `main`. **Done** — merged as PR #12 (`f587251`).
3. Create `claude/aicad-stage4-dev` from the **exact merged `main` HEAD**. **Not done as literally specified** — see "Working-branch note" above.
4. All sequential Stage-4 agents synchronize to the newest `origin/claude/aicad-stage4-dev`; do not independently recreate Stage-4 work from some other `main` state. **Superseded by the working-branch note above** until the owner resolves the branch-naming discrepancy.
5. Owner authorizes Stage-4 implementation and the first implementation task is AICAD-080. **Treated as satisfied** by this campaign's own explicit instruction to execute Stage-4 batches starting at AICAD-080/081, combined with the completed transition merge.
6. Do not begin AICAD-101+ / Stage 5 until the Stage-4 hard gate is later passed by the owner. **Still in force.**

The Stage-4 development branch is intentionally **not** created by this transition pass so its ancestry remains unambiguous after merge.
