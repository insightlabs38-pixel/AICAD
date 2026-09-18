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

Stage-5 implementation is authorized and in progress on `claude/aicad-stage5-dev`, batch `S5-00`.

Done so far in `S5-00`:

- `AICAD-101` (nested `part`-in-`part` recurses to unbounded depth) — `project/reports/AICAD-101.md`, `DL-36`.
- `AICAD-104A` (owner-requested narrow remediation: part-body `param`s now modeled by `ParamModel`, found by `AICAD-101`'s own limitation sweep) — `project/reports/AICAD-104A.md`, `DL-37`. Not part of the original four-task S5-00 definition; added to `project/TASKS.yaml` alongside it and completed in the same batch.

Still open in `S5-00`, in dependency order: `AICAD-102` (Area/spatial source-value construction — investigation already done, see below), `AICAD-103` (complete `.aicad` query vocabulary lowering, depends on 101+102), `AICAD-104` (production-path proof, depends on 103).

`AICAD-102` investigation findings (not yet implemented): `Dimension::Area` and `Length * Length -> Area` dimensional arithmetic already work today (`crates/cad-units/src/dimension_vector.rs`, `crates/cad-units/src/arithmetic.rs`); what's missing is purely an `Area`-dimensioned unit-literal suffix in `crates/cad-units/src/registry.rs`'s frozen `UNITS` table (no `mm2`/`m2` entries exist — the lexer already fuses arbitrary identifier suffixes with zero validation, so no lexer change is needed). `Point3`/`Vector3`/`Point2`/`Vector2`/`Axis3`/`Frame3`/`Plane` are already real, always-seeded standard source-level `struct` types (`crates/cad-hir/src/geometry_types.rs`'s `GEOMETRY_TYPES_SOURCE`, seeded by `crates/cad-hir/src/lower.rs::seed_standard_types`) with real source construction syntax already exercised by `crates/cad-runtime/src/spatial.rs`'s own tests — the genuine gap is that `crates/cad-cli/src/query_lowering.rs`'s query-clause mini-grammar has no way to spell a nested struct-literal/point argument for `area(...)`/`nearest_to(...)`/`farthest_from(...)` (its own doc comment already discloses this), and `cad_query::predicate::SpatialTarget`/`Frame3` there are a separate, simpler plain-data type from `cad_hir::geometry_types`'s struct, needing a lowering/bridging layer regardless of clause-grammar extension. This is `AICAD-102`'s (unit literal) and `AICAD-103`'s (clause grammar + bridging) work respectively.

Work exactly one fixed batch per invocation; do not reorder tasks, combine batches, or begin later-stage work early. Per explicit owner instruction, do not begin `S5-01` until a future invocation is asked to.

Do not perform another broad architecture audit during Stage-5 -> Stage-6 promotion unless actual Stage-5 evidence invalidates a material provisional assumption.
