# Session Handoff

## State

Stage 5 is complete, owner-approved, and merged to `main` at `697847cb2f33f6ab75bdc71911bbbcdd993039ba`. The Stage-5 -> Stage-6 transition and final Stage-6 queue are owner-approved (`project/approvals/STAGE6_QUEUE_OWNER_APPROVAL.md` / `project/DECISION_LOG.md#DL-39`).

Stage 6 batch `S6-00` (`AICAD-131`) is done — see `project/reports/AICAD-131.md`. Both Stage-5 prerequisite gaps are closed:

- `make_edge`/`make_face_on_surface` now materialize Bezier/B-spline curves/surfaces into real kernel topology (`GeometryOp::BezierEdge`/`BSplineEdge`, `SurfaceSpec::Bezier`/`BSpline`, new native `aicad_occt_make_bezier_edge`/`make_bspline_edge`/`make_face_on_bezier_surface`/`make_face_on_bspline_surface`); a trimmed surface (`AnalyticSurface::Trimmed`) unwraps to its own `base` recursively.
- `List<Geometry>` builtin parameters (`make_wire`'s `edges`, `compound`'s `shapes`, etc.) now participate in `geometry_inputs` in both `FeatureGraph` and `TraceFeatureGraph` (`is_geometry_list_type`/`is_geometry_list_type_ref`), with a dedicated incremental-rebuild regression (`crates/cad-cli/tests/stage5_list_geometry_dependency.rs`) proving correct dirty-set/reuse behavior through `ParametricBuildSession`.

`project/benchmarks/stage5_freeform_corpus/held_out/06`/`07` were revisited (not removed) per their own prior "Follow-up" notes; `public/01`-`03` stay intentionally value-level. `project/OWNER_DECISIONS.md`'s two matching non-decision items each carry a "Resolved by `AICAD-131`" note.

## Next executable work

1. Batch `S6-01` (`AICAD-132`, `AICAD-133`) is next: general interfaces/protocols + bounded generics (D27), then distinct deterministic assembly identity primitives (D26).
2. Proceed one bounded task at a time through `AICAD-160` and fixed batches `S6-02`..`S6-12`.
3. Stop at the Stage-6 owner hard gate (`AICAD-160`) before any Stage-7 promotion or implementation.

## Stage-6 queue

- Range: AICAD-131..AICAD-160 (30 tasks)
- Done: AICAD-131 (batch S6-00)
- Next: AICAD-132/133 (batch S6-01)
- Checkpoint A: AICAD-141
- Checkpoint B: AICAD-148
- Checkpoint C: AICAD-156
- Final owner gate: AICAD-160
- Queue detail: `project/planning/transitions/stage5-to-stage6/STAGE6_FINAL.yaml`
- Batches: `project/planning/transitions/stage5-to-stage6/STAGE6_FINAL_BATCHES.md`

## Evidence-sensitive carry-forwards

- Stage-5 advanced curve/surface values and operations, trimmed geometry values, geometric queries, topology construction/healing/inspection (now including Bezier/B-spline/trimmed families, `AICAD-131`), raw geometry, functional editing/adoption, lineage, persistent references, provenance, incremental regeneration (now including `List<Geometry>` dependency edges, `AICAD-131`), and maintained examples are established.
- `Ellipse` curves and periodic B-spline curves/surfaces remain unsupported by `make_edge`/`make_face_on_surface` — a disclosed, narrow, unaffected scope limit, not a new gap.
- Keep Stage-5/6 numerical/resource limitations explicit rather than generalizing tested evidence.

## Authoritative owner decisions

D25-D30 and D11 remain in force. Do not reopen D26-D30 because an older provisional planning document described them as open.

Escalate rather than inventing a new public semantic decision if execution would require identity-domain collapse, solver-defined mate/joint semantics, nondeterministic observable pose, destructive configuration identity loss, path/kernel external-asset identity, or weakening fail-closed cross-instance references.

## Stage 7

Stage 7 remains provisional and non-executable. No final Stage-7 global AICAD IDs exist. See `project/planning/transitions/stage5-to-stage6/STAGE7_RECONCILIATION.md`.
