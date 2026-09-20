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

**Batch `S5-06` (`AICAD-119..121`, topology construction/healing/
inspection) is complete** — see `project/reports/AICAD-119.md` through
`AICAD-121.md` for full detail (retired from this file per its own
"current state, not an appended diary" convention now that `S5-07` is the
active batch). Carried-forward disclosed limitations from `S5-00`-`S5-06`
(see each task's own report, not repeated here): `AICAD-105`'s full-graph
query redispatch; `AICAD-107`'s conservative nested-call provenance
approximation; `AICAD-109`'s arcs cannot wrap through angle zero;
`AICAD-110`'s B-splines cannot be periodic; `AICAD-111`'s `offset_curve`
exact only for Line/Circle/Arc; `AICAD-113`'s ring-torus-only scope;
`AICAD-114`'s B-spline surfaces cannot be periodic; `AICAD-115`'s trim
loops must already be one closed curve; `AICAD-116`'s offset is
`UnsupportedFamily` for freeform/trimmed surfaces; `AICAD-117`'s
`intersect_surfaces` exact only for Plane-Plane/Plane-Sphere/
Sphere-Sphere; `AICAD-119`'s hand-built independent faces cannot be
assembled into a `BRepCheck`-valid shell by sharing raw edge handles
alone; `AICAD-120`'s heal has no per-entity lineage and its `sew`/`heal`
builtins return only `Geometry`, not the full report; `AICAD-121`'s
`topology_wire_at` is not source-exposed and `is_forward_oriented`
collapses 4 orientation values to a bool.

**`S5-07` (`AICAD-122..124`, raw/unsafe geometry tier) is in progress —
`AICAD-122` and `AICAD-123` are done, `AICAD-124` remains:**

- `AICAD-122` (controlled raw/unsafe geometry tier with epoch-bound
  handles) — `cad_geometry_api::raw::RawGeometry`
  (`RawHandle<ClassifiedShape>`, reusing `AICAD-093`'s/`108`'s existing
  epoch/classification primitives with no new mechanism), a fourth opaque
  `CheckedType::Raw`, `enter_raw` (the sole entry point) and
  `raw_topology_kind_of`. Adversarial evidence: stale-epoch, wrong-context,
  dropped/rebuilt-owner, handle-reuse rejection; type-level rejection of
  `Raw`/`Geometry` cross-use. See `project/reports/AICAD-122.md`.
- `AICAD-123` (functional raw topology editing with lineage/change
  evidence) — 4 new native ops (`remove_face`/`replace_face`/`split_edge`/
  `merge_faces`, `BRepTools_ReShape`/`ShapeUpgrade_UnifySameDomain`-backed)
  and matching builtins (`BuiltinCategory::Raw`, extending `AICAD-122`'s
  own category). **Found and fixed a real bug along the way**: `enter_raw`'s
  own minted handle was already stale by the time any later kernel call
  tried to resolve it (a call-local `GraphResults` table released its own
  native slot the instant `OcctQueryExecutor::execute` returned) — fixed by
  a new `cad_geometry_runtime::raw_registry::RawShapeRegistry`, owned by
  `ParametricBuildSession` for a whole rebuild round and cleared alongside
  `EpochCounter::advance`, that both `OcctQueryExecutor` and the new
  `OcctRawEditExecutor` retain every result shape through. Real
  `OperationReport<ClassifiedShape>` evidence per operation (not yet
  source-exposed, matching `AICAD-120`'s own disclosed precedent). See
  `project/reports/AICAD-123.md`.

Neither task modified `examples/topology/topology_construction_basics.aicad`:
every raw-tier builtin needs a real kernel call, and `AICAD-121` already
established that such builtins are proven in a dedicated
`crates/cad-cli/tests/` file (`stage5_raw_geometry.rs`,
`stage5_raw_editing.rs`), not the structurally-only-built ACTIVE example.

All required checks pass as of `AICAD-123`: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo test -p cad-kernel-api -p cad-occt-bridge -p cad-geometry-runtime
-p cad-references` (and `AICAD-122`'s own `-p cad-references -p
cad-kernel-api -p cad-occt-bridge -p cad-runtime -p cad-cli`), the full
`cargo test --workspace` (1778 passed, 0 failed), and a standalone native
OCCT bridge CMake build (all targets) + CTest (18/18, pre-existing native
suite, unaffected).

Per the fixed batch order, the next invocation resumes `S5-07` at
`AICAD-124` (explicit raw-to-safe validation and adoption — depends on
`AICAD-123`, satisfied), completing the batch. It reuses
`cad_geometry_api::adoption::{AdoptionOutcome, AdoptionEvidence,
AdoptionRejection}` (`AICAD-108`, already defined, not yet wired to real
validation) and `Shape::resolve`/`RawShapeRegistry` (`AICAD-122`/`123`).
After `AICAD-124`, `S5-07` is complete and the next invocation begins
`S5-08` (`AICAD-125..126`: lineage/reference integrity + Checkpoint C).

Do not perform another broad architecture audit during Stage-5 -> Stage-6 promotion unless actual Stage-5 evidence invalidates a material provisional assumption.
