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

**Batch `S5-06` is complete** on `claude/aicad-stage5-dev` (`S5-00`
through `S5-05` were completed and handed off previously — see
`project/reports/AICAD-101.md` through `AICAD-118.md`; this section now
reflects `S5-06`'s completion, earlier batches' own summaries retired from
this file per its own "current state, not an appended diary" convention):

- `AICAD-119` (general topology construction — `make_vertex`/`make_edge`/
  `make_wire`/`make_face`/`make_face_on_surface`/`make_shell`/`make_solid`/
  `compound`, plus exact `ValidationReport` validity evidence extended
  with `invalid_shell_count`/`invalid_solid_count`; established the
  "construction success is never proof of validity" evidence pattern this
  whole batch follows) — `project/reports/AICAD-119.md`.
- `AICAD-120` (sewing/healing — `sew`/`heal` with an explicit
  modeling/construction tolerance policy (`RepairPolicy`,
  `project/DECISION_LOG.md#DL-24` domain 2); discovered and disclosed a
  critical finding, that `ShapeFix_Shape` can silently demote an
  unclosable Solid to a trivially-valid Shell, fixed by adding
  `HealReport::kind_changed` evidence rather than trusting bare
  `is_valid_after`) — `project/reports/AICAD-120.md`.
- `AICAD-121` (deterministic topology traversal/inspection —
  `topology_kind_of`, six `*_count` entity-count builtins, four raw-index
  selectors (`topology_face_at`/`topology_edge_at`/`topology_vertex_at`/
  `adjacent_face_at`, the last built on a new `adjacent_face_count`),
  `is_outer_wire`/`is_same_entity`/`is_forward_oriented`/`vertex_point`/
  `classify_point`. Query-category dispatch cannot mint new `GeomId`
  nodes, which rules out a literal `docs/plan`-shaped
  `topology_faces(shape) -> Iterator<FaceRef>`; resolved by pairing a
  Query-category count with Construction-category indexed access instead
  — every enumerable entity kind is still source-reachable, just via two
  primitives instead of one iterator type) — `project/reports/AICAD-121.md`.

All required checks pass: `cargo fmt --all -- --check`, `cargo clippy
--workspace --all-targets --all-features -- -D warnings`, the full `cargo
test --workspace` (1738 passed, 0 failed across every crate), and the
native OCCT bridge CMake build plus CTest (18/18). `cad-query`'s own suite
(98 tests) is unaffected. Automatic geometry-fingerprint recovery remains
disabled.

Known, explicitly-disclosed limitations carried forward from `S5-00`-`S5-05`
(see each task's own report): `AICAD-107`'s conservative nested-call
provenance approximation; `AICAD-105`'s full-graph query redispatch;
`AICAD-106`'s `ApproximationTolerance` has no default constructor;
`AICAD-109`'s arcs cannot wrap through angle zero; `AICAD-110`'s B-splines
cannot be periodic; `AICAD-111`'s `offset_curve` is exact only for
Line/Circle/Arc; `AICAD-113`'s ring-torus-only scope; `AICAD-114`'s
B-spline surfaces cannot be periodic; `AICAD-115`'s trim loops must already
be one closed curve; `AICAD-116`'s offset is `UnsupportedFamily` for
freeform/trimmed surfaces; `AICAD-117`'s `intersect_surfaces` is exact only
for `Plane`-`Plane`/`Plane`-`Sphere`/`Sphere`-`Sphere`. New this batch:
`AICAD-119`'s hand-built independent faces cannot be assembled into a
`BRepCheck`-valid shell by sharing raw edge handles alone (disclosed, not
solved — the box-reassembly-via-`GetFace` case is the positive-path
evidence instead), and `make_face_on_surface` on a quadric can construct
successfully but be geometrically degenerate (zero area) for a wire with
no extent along the surface's own free parameter; `AICAD-120`'s heal has
no per-entity lineage (`ShapeFix_Shape`'s own `History()` does not
reliably populate) and its `sew`/`heal` builtins return only `Geometry`,
not the full `SewReport`/`HealReport`; `AICAD-121`'s `topology_wire_at` is
not source-exposed (the underlying `GetWire` op exists and is tested),
`is_forward_oriented` collapses 4 `TopAbs_Orientation` values to a bool,
and no dedicated fixture exercises a face-with-a-hole's own traversal
counts.

Per the fixed batch order, the next invocation begins `S5-07`
(`AICAD-122..124`: controlled raw geometry/edit/adoption).

Do not perform another broad architecture audit during Stage-5 -> Stage-6 promotion unless actual Stage-5 evidence invalidates a material provisional assumption.
