# AICAD-120: Implement topology sewing and healing with explicit bounded policy

## Status

Done. Second task of batch S5-06.

## Objective

Add `sew`/`heal` with an explicit modeling/construction tolerance policy
(`project/DECISION_LOG.md#DL-24` domain 2), structured evidence, and
lineage where the underlying OCCT call can actually provide it — building
on `AICAD-119`'s `make_shell`/`make_solid`, whose own report disclosed
they never sew/heal on their own.

## Base / resulting commit

Base: `141b617` (`origin/claude/aicad-stage5-dev`, `AICAD-119`).

## Critical finding: healing can silently change a shape's own kind

While writing `Shape::heal`'s test for an unclosable solid (one built from
a single standalone face — `AICAD-119`'s own disclosed non-manifold-shell
scenario), `ShapeFix_Shape::Perform` returned `Standard_Boolean` "done",
and `BRepCheck_Analyzer` reported the **result** valid — verified
empirically, not assumed. Investigating why: `fixer.Shape()` was no longer
a `TopoDS_Solid` at all, but a bare `TopoDS_Shell` (`solid_count: 0 ->
shell_count: 1`, confirmed via `aicad_occt_shape_solid_count`/
`shape_shell_count`). `ShapeFix_Shape` cannot invent the missing faces a
real closure would need, so it silently demoted the shape to a lesser
topological kind that trivially validates (a Shell has no closure
requirement). A bare `is_valid_after: true` would misrepresent this as a
genuine repair — directly the failure mode this task's own acceptance line
("healing is never an invisible fallback... safe outputs are produced only
when post-operation validity/adoption requirements are satisfied") forbids.

Fix: `aicad_heal_report_t`/`HealReport` gained `kind_changed` (`fixed.
ShapeType() != kind_before`, computed natively). `is_valid_after == true`
together with `kind_changed == true` means healing gave up and returned a
different, lesser kind — documented as never-a-genuine-repair in every
layer's own doc comment, with a dedicated regression test at the
`cad-occt-bridge` level (`healing_an_unclosable_solid_reports_valid_
after_only_alongside_kind_changed`).

## `ShapeFix_Shape` history does not reliably populate (investigated, not assumed)

Per-entity Generated/Modified lineage for `heal` was investigated via
`ShapeFix_Shape::Context()->History()` (`BRepTools_History`, the same
Generated/Modified/IsRemoved shape `AICAD-086`'s boolean lineage already
uses). Empirically: an orientation-only correction on a hand-built
inconsistently-oriented box shell never populates it at all (`Generated`/
`Modified` both empty for every face, despite `Perform()` reporting
`changed`). `heal` therefore reports only coarse `changed`/before/after
validity evidence (plus `kind_changed`), not per-entity lineage — disclosed
here rather than faked; `AICAD-125` should investigate whichever finer-
grained OCCT mechanism (e.g. each individual `ShapeFix_Wire`/`ShapeFix_
Face` sub-tool's own `Context()`) actually proves reliable, if that
granularity turns out to matter for reference resolution.

`sew` (`BRepBuilderAPI_Sewing`) does NOT have this problem: its own
`IsModified`/`Modified` (single-result, not a list) reliably reports which
face/edge was relabeled, so `OcctContext::sew` captures real per-entity
lineage into the **same** `Lineage`/`aicad_lineage_handle_t` table
`Shape::union_with_lineage` etc. (`AICAD-086`) already use — directly
consumable by whatever `AICAD-125` builds, with zero new lineage-consumer
code needed for sew specifically.

## What changed

- `native/occt_bridge/CMakeLists.txt`: new `OCCT_HEALING_LIBS` (`TKShHealing`
  — confirmed via `nm -D` symbol lookup across every installed `TK*.so`,
  not guessed, that it provides both `BRepBuilderAPI_Sewing` and
  `ShapeFix_Shape`).
- `native/occt_bridge/include/aicad_occt_bridge.h` /
  `src/aicad_occt_bridge.cpp`: `aicad_occt_sew` (`BRepBuilderAPI_Sewing`,
  captures lineage via a bespoke `CaptureSewLineage`, since `Sewing`'s API
  shape differs from `BRepAlgoAPI_BooleanOperation`'s), `aicad_occt_heal`
  (`ShapeFix_Shape`, `kind_changed` detection); `aicad_sew_report_t`/
  `aicad_heal_report_t`.
- `crates/cad-occt-bridge/src/{ffi.rs,lib.rs}`: matching declarations;
  `OcctContext::sew` (returns `(Shape, Lineage, SewReport)`), `Shape::heal`
  (returns `(Shape, HealReport)`); `SewReport`/`HealReport`; 5 new unit
  tests (deterministic-repeat sewing, edge-adjacent-square merge with
  exact area, empty-list rejection, already-valid-box heal, and the
  `kind_changed` regression above).
- `crates/cad-validation/src/healing.rs` (new): `RepairPolicy` — the
  kernel-neutral typed modeling/construction tolerance policy
  (`ConstructionTolerance`-backed, `AICAD-106`) a `sew`/`heal` caller
  picks from; no `cad-occt-bridge`/`cad-kernel-api` dependency, matching
  this crate's own established `ComparisonProfile` precedent. 3 new
  tests. `Cargo.toml` gained a `cad-units` production dependency (its
  first).
- `crates/cad-geometry-api/src/ir.rs`: `GeometryOp::{Sew,Heal}` (plain
  `Quantity` tolerance, matching every other dimensioned op field — the
  typed `RepairPolicy` stays a caller-facing/documentation-level concept,
  not an IR value) with `push_op` validation; 4 new unit tests.
- `crates/cad-geometry-runtime/src/dispatch.rs`: `dispatch_op` arms
  (discarding lineage/report, matching `Union`'s own precedent);
  `dispatch_op_with_lineage`'s dedicated `Sew` arm (captures real
  lineage); `op_input_ids` arms; 3 new tests (edge-adjacent merge through
  real dispatch, sew-with-lineage capture via `dispatch_graph_incremental_
  with_lineage`, heal-of-already-valid-solid).
- `crates/cad-hir/src/builtins.rs`: `BuiltinFnId::{Sew,Heal}`,
  `Construction` category, `sew(shapes: List<Geometry>, tolerance:
  Length) -> Geometry` / `heal(shape: Geometry, tolerance: Length) ->
  Geometry` (deliberately narrower than `docs/plan/05...`'s own
  `non_manifold`/`profile`/`max_tolerance` parameters — same "escalate
  rather than guess" precedent as `Transform`); catalogue 56 -> 58.
- `crates/cad-runtime/src/interp.rs`: dispatch arms; 3 new end-to-end
  interpreter tests (sew/heal build a `Geometry` value; sew rejects an
  empty list).
- `examples/topology/topology_construction_basics.aicad` (extended, not a
  new file — Examples Policy's "prefer a small number of strong teaching
  examples"): a second edge-adjacent square, `sew`'d with the first, then
  `heal`'d.
- `crates/cad-cli/tests/stage5_sewing_healing.rs` (new): production-path
  proof — exact merged-square area after `sew`, exact preserved volume
  after `heal`, real-kernel-shape evidence for both.

## Verification

```
cargo fmt --all -- --check                                            # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings  # clean
cargo test -p cad-kernel-api -p cad-occt-bridge -p cad-geometry-runtime \
  -p cad-validation                                                   # all passed
cargo test --workspace                                                # 1707 passed, 0 failed
ctest (native/occt_bridge build, RelWithDebInfo)                       # 18/18 passed
```

## Limitations / follow-up

- Per-entity heal lineage is not offered (see the History investigation
  above) — `AICAD-125` inherits this as an open question, not a solved one.
- `sew`/`heal` builtins return only `Geometry`, discarding `SewReport`/
  `HealReport`'s own richer evidence (`free_edge_count`, `kind_changed`,
  ...) at the source-language layer — matches `AICAD-119`'s own identical
  "full `ValidationReport` breakdown isn't source-exposed either, only
  `is_valid`" precedent. A caller must call `is_valid`/`area`/`volume`
  before and after as the source-level substitute; the full reports remain
  reachable only from Rust (`cad-occt-bridge`) or a future task that adds
  a source-level report struct type.
- `sew`'s `non_manifold` override and `heal`'s `profile`/`max_tolerance`
  parameters (`docs/plan/05...`'s own fuller signatures) are not exposed —
  narrower scope, not a missing kernel capability.
- No new sewing/healing-specific example beyond extending the existing
  `AICAD-119` teaching example — deliberate, per the Examples Policy.

## Next dependency

`AICAD-121` (deterministic topology traversal/inspection) is independent
of this task (`depends_on: AICAD-119` only) and can proceed without
further sewing/healing work. Final batch task of `S5-06`.
