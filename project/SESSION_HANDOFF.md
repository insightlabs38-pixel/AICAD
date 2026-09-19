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

**Batch `S5-03` is complete** on `claude/aicad-stage5-dev` (`S5-00`
through `S5-02` were completed and handed off previously — see
`project/reports/AICAD-101.md` through `AICAD-108.md`; this section now
reflects `S5-03`'s completion, batches `S5-00`-`S5-02`'s own summaries
retired from this file per its own "current state, not an appended diary"
convention):

- `AICAD-109` (analytic curve families — Line/Circle/Arc/Ellipse — wired
  end to end: validated constructors plus exact closed-form evaluation on
  `cad_geometry_api::curve::AnalyticCurve` (`AICAD-108`), reached from real
  `.aicad` source via `line_curve`/`circle_curve`/`arc_curve`/
  `ellipse_curve`/`evaluate_curve` through a new third
  `BuiltinCategory::Value` — pure value computation, no `GeometryGraph`
  node, no kernel call at all, since every family's evaluation is
  closed-form) — `project/reports/AICAD-109.md`.
- `AICAD-110` (extended the same enum with Bezier/B-spline/NURBS, sharing
  one exact de Boor's-algorithm evaluation core in homogeneous coordinates
  — a Bezier curve evaluates as the equivalent clamped B-spline, not a
  separate implementation. Widened `cad_hir::typeck::CheckedType::List`
  from a `Value`-only element type to any `CheckedType` (contained to one
  file), needed for `List<Point3>` control-point arguments — recorded as
  an architecture decision for independent review, judged non-escalating)
  — `project/reports/AICAD-110.md`.
- `AICAD-111` (curve-local operations: trim (generic domain-restriction
  wrapper), offset (exact for Line/Circle/Arc, honest
  `UnsupportedFamily` elsewhere), closest-point (exact for Line/Circle,
  golden-section search elsewhere, reporting every local minimum found,
  never an arbitrary one), and exact global cubic B-spline interpolation
  via a Gaussian-elimination linear solve, checked against its own
  achieved residual) — `project/reports/AICAD-111.md`.
- `AICAD-112` (Checkpoint A — passed, no architecture conflict: a
  production-path vertical slice reproduces an exact textbook geometric
  identity with direct evidence of zero kernel calls; D21/D23/D24/D25
  re-audited with new targeted tests, not merely re-cited; two new ACTIVE
  examples; native OCCT CMake+CTest run directly, 18/18) —
  `project/reports/AICAD-112.md`.

All required checks pass: `cargo fmt`, `cargo clippy -D warnings`, the full
`cargo test --workspace` (83 test-result blocks, 0 failed), the frozen
`stage4_resolver_execution` corpus (11/11, unchanged), the semantic-
reference harness `validate`/`self-test` (both `ok`), and the native OCCT
bridge CMake build + CTest suite (18/18). Automatic geometry-fingerprint
recovery remains disabled.

Known, explicitly-disclosed limitations carried forward (see each task's
own report for full detail): `AICAD-107`'s conservative nested-call
provenance approximation; `AICAD-105`'s full-graph query redispatch;
`AICAD-106`'s `ApproximationTolerance` has no default constructor;
`AICAD-108`'s value families remain progressively wired by each dependent
task. New this batch: `AICAD-109`'s arcs cannot wrap through angle zero;
`AICAD-110`'s B-splines cannot be periodic (closed/wrapping), and
`weights`/other plan-spelled optional parameters are mandatory
(empty-list-means-absent) since the catalogue has no optional-parameter
mechanism; `AICAD-111`'s `offset_curve` is exact only for Line/Circle/Arc,
`closest_point`'s numerical branch resolves to only `~sqrt(f64 epsilon)`
near a flat minimum (an inherent numerical-method limit), and
`interpolate_curve` is fixed at degree 3 with no tangent/periodic option.
No `make_edge`/topology-construction path from a `Curve` exists yet —
curves remain pure values until a later Stage-5 topology task bridges
them.

Per the fixed batch order, the next invocation begins `S5-04`
(`AICAD-113..116`, surfaces).

Do not perform another broad architecture audit during Stage-5 -> Stage-6 promotion unless actual Stage-5 evidence invalidates a material provisional assumption.
