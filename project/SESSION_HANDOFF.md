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

**Batch `S5-04` is complete** on `claude/aicad-stage5-dev` (`S5-00`
through `S5-03` were completed and handed off previously — see
`project/reports/AICAD-101.md` through `AICAD-112.md`; this section now
reflects `S5-04`'s completion, earlier batches' own summaries retired from
this file per its own "current state, not an appended diary" convention):

- `AICAD-113` (analytic surface families — Plane/Cylinder/Cone/Sphere/ring
  Torus — wired end to end on `cad_geometry_api::surface::AnalyticSurface`
  (`AICAD-108` stub): validated constructors, and `evaluate(u, v)` giving
  point/`du`/`dv`/normal, all closed-form, no kernel call. The normal is
  always `du.cross(dv)` normalized, never a family-specific shortcut — this
  is what makes a genuine parametrization singularity (a sphere's own
  pole, a cone's own apex) surface as `QueryFailure::Degenerate` for free.
  Reached from `.aicad` source via `plane_surface`/`cylinder_surface`/
  `cone_surface`/`sphere_surface`/`torus_surface`/`evaluate_surface`
  through the same `BuiltinCategory::Value` curves already established) —
  `project/reports/AICAD-113.md`.
- `AICAD-114` (extended the enum with tensor-product Bezier/B-spline/NURBS
  surfaces, reusing `crate::curve`'s own de Boor/derivative/knot-expansion
  core directly — widened to `pub(crate)` rather than re-implemented — since
  a tensor-product surface is separable into two ordinary curve evaluations
  per direction. `bezier_surface`/`bspline_surface` take `List<List<Point3>>`
  control nets, the first two-levels-deep nested list literal this language
  has exercised) — `project/reports/AICAD-114.md`.
- `AICAD-115` (trimmed surfaces — `AnalyticSurface::Trimmed`: a base surface
  plus an outer `TrimLoop` and hole `TrimLoop`s, each loop exactly one
  already-closed `AnalyticCurve` read in the base's own `(u, v)` plane
  (composite multi-segment loops are a disclosed, not-yet-supported scope
  limit). Closure/planarity/orientation validated at construction, and
  point-in-region membership at evaluation, both via one shared Green's-
  theorem/winding-number numerical core over exact analytic tangents —
  proven against a closed-form circle area to `1e-6`. Found and root-fixed
  a test-helper bug (`binding_named` matched by name only, so a
  `RuntimeBuiltin` parameter named `base` could shadow a same-named
  top-level test binding)) — `project/reports/AICAD-115.md`.
- `AICAD-116` (bounded surface offset — exact for Plane/Cylinder/Cone/
  Sphere/Torus, `UnsupportedFamily` for Bezier/B-spline/Trimmed; the cone
  case derives a same-half-angle apex-shift identity, cross-checked in a
  unit test against the original surface's own point+normal, not merely
  internal self-consistency. Also adds a central-finite-difference
  independent check of `du`/`dv` against `evaluate`'s own analytic values
  across every constructible family) — `project/reports/AICAD-116.md`.

All required checks pass: `cargo fmt --all -- --check`, `cargo clippy
--workspace --all-targets --all-features -- -D warnings`, the full `cargo
test --workspace` (0 failed across every crate), and the native OCCT
bridge tests reached via `cargo test -p cad-occt-bridge` (concurrency/
adversarial/Stage-1-bracket suites all green). `cad-query`'s own suite
(98 tests) is unaffected. Automatic geometry-fingerprint recovery remains
disabled.

Known, explicitly-disclosed limitations carried forward from `S5-00`-`S5-03`
(see each task's own report): `AICAD-107`'s conservative nested-call
provenance approximation; `AICAD-105`'s full-graph query redispatch;
`AICAD-106`'s `ApproximationTolerance` has no default constructor;
`AICAD-109`'s arcs cannot wrap through angle zero; `AICAD-110`'s B-splines
cannot be periodic; `AICAD-111`'s `offset_curve` is exact only for
Line/Circle/Arc. New this batch: `AICAD-113`'s ring-torus-only scope
(`minor_radius < major_radius` required at construction); `AICAD-114`'s
B-spline surfaces cannot be periodic in either direction (mirrors
`AICAD-110`'s curve limitation); `AICAD-115`'s trim loops must already be
one closed curve (no composite multi-segment loops yet), and
`TrimLoop::contains` re-samples on every call (no caching); `AICAD-116`'s
offset is `UnsupportedFamily` for every freeform/trimmed surface family
(mirrors `AICAD-111`'s curve-offset narrowing). No `make_face`/topology-
construction path from a `Surface` exists yet — surfaces remain pure
values until a later Stage-5 topology task (`AICAD-119`+) bridges them,
exactly like curves.

Per the fixed batch order, the next invocation begins `S5-05`
(`AICAD-117..118`: multi-solution geometric queries, then Checkpoint B).
No active example has been added since `AICAD-112`'s own two — Checkpoint
B's own acceptance requires at least one new representative example
drawing on the curves+surfaces+queries surface together, so example work
is deliberately deferred to it rather than split piecemeal across
`AICAD-113`-`118`.

Do not perform another broad architecture audit during Stage-5 -> Stage-6 promotion unless actual Stage-5 evidence invalidates a material provisional assumption.
