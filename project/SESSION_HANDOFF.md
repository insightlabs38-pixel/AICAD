# Session Handoff

## State

Stage 5 is complete, owner-approved, and merged to `main` at `697847cb2f33f6ab75bdc71911bbbcdd993039ba`. AICAD-130 is the final Stage-5 task and its gate recommends PASS. Owner approval is durably recorded in `project/approvals/STAGE5_OWNER_APPROVAL.md` and `project/DECISION_LOG.md#DL-38`.

The Stage-5 -> Stage-6 transition and final Stage-6 queue are owner-approved. Stage 6 is now the active, in-progress stage and implementation is authorized by `project/approvals/STAGE6_QUEUE_OWNER_APPROVAL.md` / `project/DECISION_LOG.md#DL-39`. No Stage-6 implementation task is marked complete by this approval.

## Next executable work

1. use `claude/aicad-stage6-dev` created from the exact approved transition merge commit on `main`;
2. execute AICAD-131 first;
3. proceed one bounded task at a time through AICAD-160 and fixed batches S6-00..S6-12;
4. stop at the Stage-6 owner hard gate before any Stage-7 promotion or implementation.

## Stage-6 queue

- Range: AICAD-131..AICAD-160 (30 tasks)
- Checkpoint A: AICAD-141
- Checkpoint B: AICAD-148
- Checkpoint C: AICAD-156
- Final owner gate: AICAD-160
- Queue detail: `project/planning/transitions/stage5-to-stage6/STAGE6_FINAL.yaml`
- Batches: `project/planning/transitions/stage5-to-stage6/STAGE6_FINAL_BATCHES.md`

## Evidence-sensitive carry-forwards

- Stage-5 advanced curve/surface values and operations, trimmed geometry values, geometric queries, supported topology construction/healing/inspection, raw geometry, functional editing/adoption, lineage, persistent references, provenance, incremental regeneration, and maintained examples are established.
- Bezier/B-spline curves and surfaces, plus trimmed surfaces, cannot yet be converted into real kernel topology through `make_edge` / `make_face_on_surface`; AICAD-131 must implement supported conversion paths while preserving deterministic structured failure for invalid inputs.
- `List<Geometry>` builtin parameters are currently invisible to `geometry_inputs` in `FeatureGraph` / `TraceFeatureGraph`; AICAD-131 must make them first-class geometry dependencies and prove dirty-set/incremental invalidation before later Stage-6 tasks rely on them.
- Keep Stage-5 numerical/resource limitations explicit rather than generalizing tested evidence.

## Authoritative owner decisions

D25-D30 and D11 remain in force. Do not reopen D26-D30 because an older provisional planning document described them as open.

Escalate rather than inventing a new public semantic decision if execution would require identity-domain collapse, solver-defined mate/joint semantics, nondeterministic observable pose, destructive configuration identity loss, path/kernel external-asset identity, or weakening fail-closed cross-instance references.

## Stage 7

Stage 7 remains provisional and non-executable. No final Stage-7 global AICAD IDs exist. See `project/planning/transitions/stage5-to-stage6/STAGE7_RECONCILIATION.md`.
