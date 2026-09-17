# Session Handoff

## Canonical state

**Stage 4 is complete, owner-approved, and merged.**

- Stage-4 merged implementation: `564e6790b67bf2a5b489bca2003a5084a959e937` (PR #13).
- Final Stage-4 task: `AICAD-100A`.
- Final Stage-4 gate: `project/gates/stage-4-gate.md`.
- Owner approval: 2026-09-16, recorded on `main` by `project/approvals/STAGE4_OWNER_APPROVAL.md` at `35547025bbe32f350bcdaf2c1556ed2482a2cada`, and recorded as `DL-34` in `project/DECISION_LOG.md` on this transition branch.

The active branch is the planning/governance branch `claude/aicad-stage5-transition`. It is **not** the Stage-5 development branch.

## What this transition finalized

Stage 5 now has an exact executable queue for owner review:

- `AICAD-101..AICAD-130` in `project/TASKS.yaml`;
- fixed batches `S5-00..S5-10` and checkpoint details in `project/planning/transitions/stage4-to-stage5/STAGE5_FINAL_BATCHES.md`;
- transition/reconciliation rationale in `project/planning/transitions/stage4-to-stage5/TRANSITION_REPORT.md`.

Stage-4/general-language carry-forward work is explicit in `S5-00` rather than buried as limitations: nested `part` semantics, Area/spatial source construction, the remaining source query vocabulary/lowering for already-real Stage-4 predicates, and production-path source-query regression proof. Automatic fingerprint recovery remains disabled under D7.

Stage 6 and Stage 7 have complete but provisional queues:

- `project/planning/transitions/stage4-to-stage5/STAGE6_PROVISIONAL.yaml` — `S6-001..S6-030`, no global AICAD IDs;
- `project/planning/transitions/stage4-to-stage5/STAGE7_PROVISIONAL.yaml` — `S7-001..S7-030`, no global AICAD IDs;
- `project/planning/transitions/stage4-to-stage5/STAGE_PROMOTION_POLICY.md` — short evidence-reconciliation promotion lifecycle.

The Stage-6 queue treats D26-D30 as resolved authoritative decisions. The Stage-7 queue leaves OD-S7-01..OD-S7-05 explicit and unresolved at their latest safe decision points.

## Stage-5 batch sequence

1. `S5-00` — `AICAD-101..104`: Stage-4 carry-forward language/query completeness.
2. `S5-01` — `AICAD-105..106`: runtime/query/tolerance foundations.
3. `S5-02` — `AICAD-107..108`: programmable feature/provenance + geometry value/IR foundations.
4. `S5-03` — `AICAD-109..112`: curves + Checkpoint A.
5. `S5-04` — `AICAD-113..116`: surfaces.
6. `S5-05` — `AICAD-117..118`: geometric queries + Checkpoint B.
7. `S5-06` — `AICAD-119..121`: topology construction/healing/inspection.
8. `S5-07` — `AICAD-122..124`: raw geometry/edit/adoption.
9. `S5-08` — `AICAD-125..126`: lineage/reference integrity + Checkpoint C.
10. `S5-09` — `AICAD-127..129`: realistic/adversarial/example campaign.
11. `S5-10` — `AICAD-130`: final Stage-5 owner gate.

## Non-negotiable invariants carried forward

- D2 functional/value semantics.
- D5 deterministic equivalence remains separate from other tolerance classes.
- D6 kernel-neutral public semantics; no OCCT leakage.
- D7 fail-closed semantic references; no automatic fingerprint recovery.
- D18 ordinary RuntimeBuiltin call semantics.
- D20 type-closed standard environment.
- D21 closed catalogue scaling.
- D22 safe/raw geometry distinction.
- D23 source-visible kernel-backed query execution.
- D24 explicit tolerance taxonomy; no global epsilon.
- D25 feature/provenance visibility through abstraction.
- Raw handles remain ephemeral/epoch-bound and never durable identity.
- Every topology-changing Stage-5 operation must emit resolver-consumable lineage.
- Every ACTIVE example is executable/tested in CI or equivalent automation beginning with Stage 5.

## Current stop rule and exact next action

**Stage-5 implementation has not begun. Do not implement `AICAD-101` on this branch.**

Owner should review the transition branch. If accepted:

1. merge `claude/aicad-stage5-transition` to `main`;
2. create `claude/aicad-stage5-dev` from that exact merged HEAD;
3. begin Stage 5 with `AICAD-101` under the fixed queue/batch plan.

Do not perform another broad architecture audit during Stage-5 -> Stage-6 promotion unless actual Stage-5 evidence invalidates a material provisional assumption.
