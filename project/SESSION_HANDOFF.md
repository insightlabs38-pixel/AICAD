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

**Batch `S5-08` (`AICAD-125..126`, lineage/reference integrity +
Checkpoint C) is complete** — see `project/reports/AICAD-125.md`/
`AICAD-126.md` for full detail (`S5-07` and earlier retired from this file
per its own "current state, not an appended diary" convention). Summary:

- `AICAD-125` threaded every Stage-5 topology-changing operation's own
  already-captured evidence into the Stage-4 reference-resolution
  machinery: `Sew` (multi-operand `lineage_operand_ids`, reusing its
  already-captured native `Lineage`); a new `cad_geometry_runtime::
  raw_lineage::RawLineageIndex` chain for `remove_face`/`replace_face`/
  `split_edge`/`merge_faces`, classified once `adopt`ed by a new
  `cad_query::feature_lineage::classify_raw_edit_lineage`; `Heal` and an
  untracked raw handle continue to honestly report no evidence
  (`Broken(InsufficientEvidence)`), never a guess. **Found and fixed a
  real bug**: `enter_raw`'s own target dispatches through a *separate*,
  independent kernel construction (`OcctQueryExecutor`'s own call-local
  `dispatch_graph`, per `DL-25`'s demand-materialization contract) from
  the round's own later final dispatch — two independent constructions of
  "the same" geometry are `Shape::is_same` **false** with each other, so a
  raw chain's own "prior entities" must be snapshotted at `enter_raw` time
  from that same call-local dispatch, never re-derived later from the
  round's own `results` table. Fixed in `raw_lineage.rs`'s own doc
  comment/`record_origin` signature.
- `AICAD-126` (Checkpoint C) proved the whole construct → sew → heal →
  inspect → raw-edit → adopt → persistent-reference chain through one real
  `ParametricBuildSession` (`stage5_lineage_checkpoint.rs`), confirmed no
  architecture boundary was bypassed, re-ran the native CTest suite and
  ACTIVE example suite, and recommended **PASS** (owner decision, not
  granted by the agent).

Known, explicitly-disclosed limitation carried forward: `replace_face`'s
own `Modified` evidence has no known constructible *valid* `adopt`-through
fixture (every replacement tried fails kernel/validity checks — `AICAD-
123`'s own disclosed compatibility gap surfacing, not a new one);
`merge_faces`/`split_edge`'s own `List<Raw>` results have no `.aicad`
element-selection syntax to feed into `adopt` at all. The production-path
`Modified` proof uses `Sew`'s own real edge-relabeling instead, exercising
the identical consumer code path. See `AICAD-125.md`/`AICAD-126.md` for
full detail.

All required checks pass as of `AICAD-126`: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
the full `cargo test --workspace` (1811 passed, 0 failed), `python3
scripts/ci/semantic_ref_harness.py validate`/`self-test` (both `ok`), a
standalone native OCCT bridge CMake build + CTest (18/18), and the ACTIVE
example suite (3/3, 14 examples).

Per the fixed batch order, the next invocation begins `S5-09`
(`AICAD-127..129`: realistic/adversarial/example campaign), which depends
on `AICAD-126` (satisfied).

Do not perform another broad architecture audit during Stage-5 -> Stage-6 promotion unless actual Stage-5 evidence invalidates a material provisional assumption.
