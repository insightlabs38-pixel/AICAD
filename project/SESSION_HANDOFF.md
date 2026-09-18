# Session Handoff

## Canonical state

**Stage 4 is complete, owner-approved, and merged.**

- Stage-4 merged implementation: `564e6790b67bf2a5b489bca2003a5084a959e937` (PR #13).
- Final Stage-4 task: `AICAD-100A`.
- Final Stage-4 gate: `project/gates/stage-4-gate.md`.
- Owner approval: 2026-09-16, recorded by `project/approvals/STAGE4_OWNER_APPROVAL.md` and `DL-34`.

**The Stage-5 queue is approved and implementation is authorized.**

- Owner approval: 2026-09-18, recorded by `project/approvals/STAGE5_QUEUE_OWNER_APPROVAL.md` and `DL-35`.
- Canonical Stage-5 development branch: `claude/aicad-stage5-dev`, created from the approved merged head `df9334634e09a9793191ea5de40f0aad12032589`.

The active branch is `claude/aicad-stage5-dev`. It is the Stage-5 development branch.

## What the transition finalized

Stage 5 has an exact executable queue, now approved for implementation:

- `AICAD-101..AICAD-130` in `project/TASKS.yaml`;
- fixed batches `S5-00..S5-10` in `project/planning/transitions/stage4-to-stage5/STAGE5_FINAL_BATCHES.md`;
- transition/reconciliation rationale in `TRANSITION_REPORT.md`;
- current public/developer documentation and maintained examples synchronized in `DOCUMENTATION_AND_EXAMPLES_SYNC.md`.

Stage-4 carry-forward work is explicit in S5-00: nested `part` behavior, Area/spatial source construction, remaining source-query vocabulary/lowering, and production-path source-query completeness. Automatic fingerprint recovery remains disabled under D7.

Stage 6 and Stage 7 remain complete but provisional queues; they are not current product commitments.

## Maintained example invariant

The current user-facing example inventory is in `examples/README.md`. Every ACTIVE example is registered in `crates/cad-cli/tests/active_examples.rs`.

Beginning with Stage 5:

- public language/modeling changes update affected ACTIVE examples in the same task/batch;
- each major checkpoint adds/refreshes representative examples;
- stale examples are updated or explicitly archived;
- stress/benchmark fixtures stay out of the primary learning path.

## Stage-5 batch sequence

1. S5-00 / AICAD-101..104 — Stage-4 carry-forward language/query completeness.
2. S5-01 / AICAD-105..106 — runtime/query/tolerance foundations.
3. S5-02 / AICAD-107..108 — programmable feature/provenance + geometry value/IR foundations.
4. S5-03 / AICAD-109..112 — curves + Checkpoint A.
5. S5-04 / AICAD-113..116 — surfaces.
6. S5-05 / AICAD-117..118 — intersection/projection/distance + Checkpoint B.
7. S5-06 / AICAD-119..121 — topology construction/healing/inspection.
8. S5-07 / AICAD-122..124 — controlled raw geometry/edit/adoption.
9. S5-08 / AICAD-125..126 — lineage/reference integrity + Checkpoint C.
10. S5-09 / AICAD-127..129 — realistic/adversarial/example campaign.
11. S5-10 / AICAD-130 — final Stage-5 owner gate.

## Non-negotiable invariants carried forward

D2 functional/value semantics; D5 deterministic equivalence separation; D6 kernel-neutral public semantics; D7 fail-closed references/no automatic fingerprint recovery; D18 ordinary RuntimeBuiltin calls; D20-D25 type/catalogue/safe-raw/query/tolerance/provenance rules. Raw handles remain epoch-bound and never durable identity. Topology-changing Stage-5 operations must emit resolver-consumable lineage.

## Current stop rule and exact next action

**Batch `S5-02` is complete** on `claude/aicad-stage5-dev` (`S5-00`/`S5-01`
were completed and handed off previously — see git history /
`project/reports/AICAD-101.md` through `AICAD-106.md` for those batches'
own record; this section now reflects `S5-02`'s completion):

- `AICAD-107` (resolved `DL-27`: geometry built through a user function
  call, a taken `if`/`match` branch, or a loop iteration now stays visible
  to the feature/dependency/provenance system. New `cad_runtime::
  feature_trace` — `CallPath`/`TraceEntry` — records every Geometry-
  returning `RuntimeBuiltin` call at its own dynamic call-instance
  identity as the interpreter runs, with `binding_refs` resolved *through*
  any number of function-call argument-passing levels back to real
  top-level/`param` bindings. New `cad_feature_graph::trace_graph::
  TraceFeatureGraph` turns a completed trace into a real dependency graph
  reusing the Stage-3 `dirty_set` contract unchanged. `cad-cli::
  ParametricBuildSession::rebuild` — the one production dirty-propagation/
  lineage path — now builds this trace graph from the real interpreter run
  instead of the old purely-static top-level-only `FeatureGraph`. A real
  regression (part-nested named-binding resolution) was found and fixed
  during production wiring, caught by the full `cargo test -p cad-cli` run
  before this task was reported done) — `project/reports/AICAD-107.md`.
- `AICAD-108` (surveyed D22's four geometry-identity tiers first: kernel-
  native object, raw/unsafe handle, persistent reference, and safe value
  all already existed with no gap — `cad_references::raw_handle::
  RawHandle<T>` (`AICAD-093`) already generically covers Stage-5's own raw
  topology tier, so no new raw-handle type was needed. Added the value
  families that genuinely were missing, all pure data, no kernel/source
  wiring: `cad_geometry_api::curve::AnalyticCurve`, `::surface::
  AnalyticSurface`, `::query_result::{QueryOutcome<T>, QueryFailure}`
  (zero/one/many results, numerical failure never silently absorbed into
  an empty result), `::operation_report::OperationReport<T>` (a `'ctx`-
  independent created/modified/deleted/split/merged evidence snapshot),
  `::adoption::{AdoptionOutcome<T>, AdoptionEvidence, AdoptionRejection}`
  (D22's explicit raw-to-safe promotion contract); `cad_kernel_api::
  topology::{TopologyKind, ClassifiedShape}`. Every family's own doc
  comment names exactly which later Stage-5 task owns wiring it) —
  `project/reports/AICAD-108.md`.

All required checks pass: `cargo fmt`, `cargo clippy -D warnings`, the full
`cargo test --workspace` (82 test-result blocks, 0 failed), the frozen
`stage4_resolver_execution` corpus (11/11, unchanged), and the semantic-
reference harness `validate`/`self-test` (both `ok`). Automatic
geometry-fingerprint recovery remains disabled.

Known, explicitly-disclosed limitations carried forward (see each task's
own report for full detail): `AICAD-107`'s `provenance_of` conservative
nested-call approximation (can over-invalidate, never under-invalidate)
and `match`-arm whole-scrutinee provenance attribution; a `param` default
expression's `TraceEntry`s always report an empty `part` scope
(`ParamModel` does not track a `param`'s own enclosing part path).
`AICAD-105`'s `OcctQueryExecutor` dispatches the whole accumulated graph
per query call, not the tightest "minimum required upstream geometry"
subset; the simpler non-incremental `cad_cli::build::build_source` path is
not wired to a real query executor. `AICAD-106`'s `ApproximationTolerance`
has no default constructor. `AICAD-108`'s new value families are
deliberately unwired (no `GeometryOp`/`GeometryQuery` variant, no `.aicad`
source, no kernel dispatch) — each later Stage-5 task's own job per its
own module doc comment.

Per the fixed batch order, the next invocation begins `S5-03`
(`AICAD-109..112`, curves + Checkpoint A).

Do not perform another broad architecture audit during Stage-5 -> Stage-6 promotion unless actual Stage-5 evidence invalidates a material provisional assumption.
