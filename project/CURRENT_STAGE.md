# Current Stage

## Lifecycle state

- **Stage 5 — Advanced Programmable Geometry & Robust Topology:** COMPLETE, OWNER APPROVED, AND MERGED.
- **Merged Stage-5 `main` HEAD:** `697847cb2f33f6ab75bdc71911bbbcdd993039ba`.
- **Final Stage-5 task:** AICAD-130 (`99b0777c235b00a06252ea28d7c8c8db5fd6e1a4`).
- **Final Stage-5 gate:** PASS recommended; owner approval is recorded in `project/approvals/STAGE5_OWNER_APPROVAL.md` and `project/DECISION_LOG.md#DL-38`.
- **Stage-5 -> Stage-6 transition:** COMPLETE on `claude/aicad-stage6-transition`, pending owner review/merge.
- **Stage 6 — Assemblies, configurations, and kinematics:** FINAL QUEUE PREPARED, **NOT YET AUTHORIZED FOR IMPLEMENTATION** until this transition is owner-merged.
- **Stage 7:** PROVISIONAL / NON-EXECUTABLE.

## Stage-6 queue

The final Stage-6 queue contains 30 tasks, **AICAD-131 through AICAD-160**, in fixed batches S6-00 through S6-12. AICAD-131 is the mandatory compatibility-and-remediation prerequisite: it must close the Stage-5 freeform/trimmed-to-kernel-topology construction gap and the `List<Geometry>` dependency/invalidation gap before AICAD-132/133. Both repairs are part of AICAD-131 itself, not deferred follow-up tasks. Checkpoints are AICAD-141, AICAD-148, and AICAD-156; AICAD-160 is the final Stage-6 owner hard gate.

Canonical transition detail:

- `project/planning/transitions/stage5-to-stage6/STAGE6_FINAL.yaml`
- `project/planning/transitions/stage5-to-stage6/STAGE6_FINAL_BATCHES.md`
- `project/planning/transitions/stage5-to-stage6/TRANSITION_REPORT.md`

## Governing invariants

D25-D30 and D11 remain authoritative. In particular:

- assembly definition, instance, occurrence/path, configuration/variant, external-asset, semantic-topology, and BOM/purchasing identities remain distinct;
- mechanical interfaces use general interface/protocol semantics;
- mates/joints are AICAD-owned and solver-neutral;
- observable pose/DOF/conflict behavior is deterministic and structured;
- configurations are immutable overlays preserving identity/provenance;
- external assets are content/provenance identified, not path/kernel identified.

## Execution boundary

Do **not** execute AICAD-131 or create the Stage-6 development branch while this transition branch is awaiting owner merge.

After the owner merges this transition to `main`, create `claude/aicad-stage6-dev` from that exact merged `main` HEAD and begin AICAD-131.

Do not implement Stage 7.
