# Current Stage

## Lifecycle state

- **Stage 5 — Advanced Programmable Geometry & Robust Topology:** COMPLETE, OWNER APPROVED, AND MERGED.
- **Merged Stage-5 `main` HEAD:** `697847cb2f33f6ab75bdc71911bbbcdd993039ba`.
- **Final Stage-5 task:** AICAD-130 (`99b0777c235b00a06252ea28d7c8c8db5fd6e1a4`).
- **Final Stage-5 gate:** PASS recommended; owner approval is recorded in `project/approvals/STAGE5_OWNER_APPROVAL.md` and `project/DECISION_LOG.md#DL-38`.
- **Stage-5 -> Stage-6 transition:** COMPLETE AND OWNER APPROVED; approval is recorded in `project/approvals/STAGE6_QUEUE_OWNER_APPROVAL.md` and `project/DECISION_LOG.md#DL-39`.
- **Stage 6 — Assemblies, configurations, and kinematics:** **ACTIVE / IN PROGRESS; IMPLEMENTATION AUTHORIZED.** AICAD-131 (batch S6-00) is complete — see `project/reports/AICAD-131.md`. Batch S6-01 (AICAD-132, AICAD-133) is complete — see `project/reports/AICAD-132.md` and `project/reports/AICAD-133.md`. Batch S6-02 (AICAD-134, AICAD-135, AICAD-136) is complete — see `project/reports/AICAD-134.md`, `project/reports/AICAD-135.md`, and `project/reports/AICAD-136.md`. Batch S6-03 (AICAD-137, AICAD-138) is complete — see `project/reports/AICAD-137.md` and `project/reports/AICAD-138.md`. Batch S6-04 (AICAD-139, AICAD-140, AICAD-141 — Checkpoint A) is complete — see `project/reports/AICAD-139.md`, `project/reports/AICAD-140.md`, and `project/reports/AICAD-141.md`. Batch S6-05 (AICAD-142, AICAD-143, AICAD-144) is complete — see `project/reports/AICAD-142.md`, `project/reports/AICAD-143.md`, and `project/reports/AICAD-144.md`. Batch S6-06 (AICAD-145, AICAD-146, AICAD-147, AICAD-148 — Checkpoint B) is complete — see `project/reports/AICAD-145.md`, `project/reports/AICAD-146.md`, `project/reports/AICAD-147.md`, and `project/reports/AICAD-148.md`. Batch S6-07 (AICAD-149, AICAD-150, AICAD-151) is complete — see `project/reports/AICAD-149.md`, `project/reports/AICAD-150.md`, and `project/reports/AICAD-151.md`. AICAD-152 (batch S6-08) is the next executable task.
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

Stage-6 implementation is owner-authorized by `DL-39`. Canonical implementation must occur on `claude/aicad-stage6-dev` created from the exact approved Stage-5 -> Stage-6 transition merge commit on `main`; begin with AICAD-131 and execute the fixed queue in order.

Do not implement Stage 7.
