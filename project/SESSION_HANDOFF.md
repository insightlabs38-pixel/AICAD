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

**Batch `S5-05` is complete** on `claude/aicad-stage5-dev` (`S5-00`
through `S5-04` were completed and handed off previously — see
`project/reports/AICAD-101.md` through `AICAD-116.md`; this section now
reflects `S5-05`'s completion, earlier batches' own summaries retired from
this file per its own "current state, not an appended diary" convention):

- `AICAD-117` (multi-solution curve/curve, curve/surface, surface/surface
  intersection, point-to-surface projection, and distance queries —
  `cad_geometry_api::query` (new module) plus a new `AnalyticSurface::
  project_point`. Exact closed forms where cheap and structurally possible
  (Line-Line; Line vs. Plane/Sphere/Cylinder/Cone; Plane-Plane/Plane-
  Sphere/Sphere-Sphere for `intersect_surfaces` — the only surface pairs
  whose intersection curve `AnalyticCurve`'s closed family set can even
  represent); one general bounded-numerical-search composition
  (`AnalyticCurve::closest_point`/`AnalyticSurface::project_point` reused
  as the "distance from a swept point" objective, bracket-scanned for every
  local minimum) covering every other curve/curve and curve/surface pair,
  and `distance_surface_surface` whenever either side has a bounded
  `(u, v)` domain. A coincident/overlapping or tangent configuration is
  `QueryFailure::Degenerate` (deliberately not a new tagged-union payload
  type — see that module's own doc comment); genuinely zero solutions is a
  real, successful empty list, never an error) — `project/reports/
  AICAD-117.md`.
- `AICAD-118` (Checkpoint B — new `crates/cad-cli/tests/
  stage5_queries_checkpoint.rs` proves the real production path
  (`ParametricBuildSession`) with independent exact checks and direct
  "never touches the kernel" evidence; two new ACTIVE examples
  (`examples/surfaces/surface_query_basics.aicad`,
  `examples/surfaces/pipe_clearance_check.aicad`); the ambiguity/
  cardinality campaign is `AICAD-117`'s own 61 new unit tests, each family
  covering unique/zero/multiple/tangent/coincident/unsupported cases with
  independently-computed expected values) — `project/reports/AICAD-118.md`.

All required checks pass: `cargo fmt --all -- --check`, `cargo clippy
--workspace --all-targets --all-features -- -D warnings`, the full `cargo
test --workspace` (0 failed across every crate), the native OCCT bridge
CMake build plus CTest (18/18), the Stage-4 resolver production-path
regression suite (11/11, unchanged), and `scripts/ci/semantic_ref_harness.py
validate`/`self-test`. `cad-query`'s own suite (98 tests) is unaffected.
Automatic geometry-fingerprint recovery remains disabled.

Known, explicitly-disclosed limitations carried forward from `S5-00`-`S5-04`
(see each task's own report): `AICAD-107`'s conservative nested-call
provenance approximation; `AICAD-105`'s full-graph query redispatch;
`AICAD-106`'s `ApproximationTolerance` has no default constructor;
`AICAD-109`'s arcs cannot wrap through angle zero; `AICAD-110`'s B-splines
cannot be periodic; `AICAD-111`'s `offset_curve` is exact only for
Line/Circle/Arc; `AICAD-113`'s ring-torus-only scope; `AICAD-114`'s
B-spline surfaces cannot be periodic; `AICAD-115`'s trim loops must already
be one closed curve; `AICAD-116`'s offset is `UnsupportedFamily` for
freeform/trimmed surfaces. New this batch: `AICAD-117`'s
`intersect_surfaces` is exact only for `Plane`-`Plane`/`Plane`-`Sphere`/
`Sphere`-`Sphere` (a structural limit — no other pair's intersection curve
is representable by `AnalyticCurve` at all); `distance_curve_surface`/
`intersect_curve_surface` for a non-intersecting `Line` vs. `Cone` is
`Unsupported` (the general skew-line-to-cone minimum-distance closed form
was not derived); any query touching `AnalyticSurface::Trimmed`, or a
`Line` vs. `Torus`/`Bezier`/`BSpline`, is `Unsupported`. No `make_face`/
topology-construction path from a `Surface` exists yet — surfaces remain
pure values until `AICAD-119`+ bridges them, exactly like curves.

Per the fixed batch order, the next invocation begins `S5-06`
(`AICAD-119..121`: topology construction/healing/inspection).

Do not perform another broad architecture audit during Stage-5 -> Stage-6 promotion unless actual Stage-5 evidence invalidates a material provisional assumption.
