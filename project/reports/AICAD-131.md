# AICAD-131: Stabilize Stage-5 compatibility and close prerequisite geometry/dependency gaps

## Status

Done. Batch S6-00, only task.

## Objective

Close the two disclosed Stage-5 prerequisite gaps Stage 6 assembly work
depends on, without reopening D26-D30: (1) Bezier/B-spline curve/surface
values could not be converted into real kernel topology through
`make_edge`/`make_face_on_surface`; (2) `List<Geometry>` builtin
parameters (`make_wire`'s `edges`, `compound`'s `shapes`, ...) were
invisible to `geometry_inputs` in both `FeatureGraph` and
`TraceFeatureGraph`, so dirty-set/incremental invalidation could not see
through them.

## Base / resulting commit

Base: `d9f0d4e` (`origin/claude/aicad-stage6-dev`, the Stage-6 transition
commit). Working tree change only in this invocation; not yet committed
at report-write time (committed immediately after).

## Part A — freeform curve/surface topology construction

Added four native ABI functions (`native/occt_bridge`): `aicad_occt_
make_bezier_edge`/`make_bspline_edge` (`Geom_BezierCurve`/
`Geom_BSplineCurve` -> `BRepBuilderAPI_MakeEdge`) and `aicad_occt_
make_face_on_bezier_surface`/`make_face_on_bspline_surface`
(`Geom_BezierSurface`/`Geom_BSplineSurface`, reusing the existing
`MakeFaceOnSurface<Surface>` template via its `Handle(Geom_Surface)`
instantiation — no new template needed). Wired through `cad-occt-bridge`'s
FFI layer and safe `OcctContext`/`Shape` wrappers, then into
`cad-geometry-api::ir` as new `GeometryOp::BezierEdge`/`BSplineEdge` and
`SurfaceSpec::Bezier`/`BSpline` variants, dispatched in
`cad-geometry-runtime::dispatch`.

`cad-runtime::interp`'s `curve_to_edge_op`/`surface_to_spec` now dispatch
`AnalyticCurve`/`AnalyticSurface::Bezier`/`BSpline` into these ops instead
of `UnsupportedTopologyConstruction`. `AnalyticSurface::Trimmed` unwraps
to its own `base` recursively (`surface_to_spec(base)`): a trimmed
surface's boundary is supplied separately as `make_face_on_surface`'s own
explicit `outer`/`holes` kernel wires, so its embedded `TrimLoop` — a
value-level domain restriction for `evaluate_surface`/`trim_surface` — is
never a second topology boundary. `Ellipse` remains unsupported
(unaffected scope); periodic B-spline curves/surfaces remain unreachable
(`AnalyticCurve::bspline`/`AnalyticSurface::bspline` already reject
`periodic: true` at construction) but are defensively guarded again here.

Native validation happens before OCCT construction (finite control
points/weights, positive weights, strictly-increasing distinct knots,
degree/control-point-count consistency) — `native/occt_bridge/tests/
freeform_topology_test.cpp` (new, 25 checks) proves both positive and
adversarial paths directly against the ABI.

`project/benchmarks/stage5_freeform_corpus/held_out/06`/`07` are revisited
per their own prior "Follow-up" notes (not removed — the interface-level
rejection they proved no longer exists): `06`'s 0.001mm-sliver
`make_face_on_surface` call now builds structurally but is measured
`is_valid() == false` (OCCT's own `BRepCheck_Analyzer`, Stage-1 kernel
policy #14); `07`'s ordinary hook `bspline_curve` now builds a valid
`make_edge` edge. `public/01`-`03` stay intentionally value-level (frozen
checked evidence unchanged); their stale "not yet supported" comments were
corrected. `README.md` gained a "Capability gap closed by `AICAD-131`"
section recording exactly what changed. `project/OWNER_DECISIONS.md`'s
matching non-decision item now has a "Resolved by `AICAD-131`" note.

## Part B — `List<Geometry>` dependency tracking

Added `is_geometry_list_type`/`is_geometry_list_type_ref` (exact
`List<Geometry>` match) alongside the existing `is_geometry_type`/
`is_geometry_type_ref` in `cad_feature_graph::graph` and
`cad_runtime::interp`. A `List<Geometry>`-typed parameter now contributes
each element as its own `geometry_inputs` edge, in declared order:

- `FeatureGraph` (static): resolves each element of a direct list-literal
  argument (`make_wire([e0, e1, ...])`, the only form any current call
  site uses) the same way a scalar `Geometry` argument resolves. An
  indirect `List<Geometry>`-typed variable is a structured
  `UnresolvedGeometryInput` error rather than silently falling back to an
  opaque scalar parameter — consistent with this module's existing
  fail-closed convention, and not exercised by any current source.
- `TraceFeatureGraph` (dynamic, production): `Interpreter::call` now
  iterates the evaluated `Value::List` and pushes each `Value::Geometry`
  element's id, instead of treating the whole list as a scalar
  `parameters`/`binding_refs` entry.

`crates/cad-cli/tests/stage5_inspectability_fixture.rs`'s own
self-documenting assertion (`square.geometry_inputs.is_empty()`, with an
explicit "update this if fixed" comment) now asserts the four real edge
ids instead, exactly as it anticipated. New `crates/cad-cli/tests/
stage5_list_geometry_dependency.rs` proves the production incremental-
rebuild path end to end (`ParametricBuildSession`): editing a param
`compound`'s middle element alone depends on dirties exactly that element
plus the `compound` node, reuses the other two siblings (literal same
kernel `Shape` handles across the rebuild), and the rebuilt volume
reflects the edit. `project/OWNER_DECISIONS.md`'s matching non-decision
item has a "Resolved by `AICAD-131`" note.

## Verification

```
cmake --build (RelWithDebInfo) + ctest                         # 19/19 (native/occt_bridge, +1 new file)
cargo fmt --all -- --check                                     # clean
cargo clippy --workspace --all-targets --all-features -Dwarnings  # clean
cargo test --workspace                                         # all green (no regressions)
python3 scripts/ci/semantic_ref_harness.py validate             # {"cases":10,"status":"ok"}
python3 scripts/ci/semantic_ref_harness.py self-test            # {"status":"ok"}
cargo test -p cad-cli --test active_examples                    # 3/3
cargo test -p cad-cli --test stage5_freeform_corpus             # 7/7
cargo test -p cad-cli --test stage5_inspectability_fixture      # 4/4
cargo test -p cad-cli --test stage5_list_geometry_dependency    # 1/1 (new)
```

## Invariants preserved

No public syntax/semantics changed; no kernel type crossed the adapter
boundary (native ABI stays plain C types); no fail-closed reference/
validity guarantee weakened (construction success is still never treated
as validity evidence); no existing test weakened — `06`/`07` were revised
exactly as their own frozen documentation anticipated, not silently
re-authored. D26-D30 untouched (no assembly identity domain exists yet).

## Limitations

- Freeform topology construction does not attempt any repair/healing of a
  near-degenerate result (matches existing `make_face_on_*` precedent).
- `List<Geometry>` resolution in the static `FeatureGraph` only recognizes
  a direct list-literal argument; no current source needs more.
- Periodic B-spline curves/surfaces remain out of scope entirely (an
  existing Stage-5 limitation, not touched here).

## Next dependency

`AICAD-132` (general interfaces/protocols) may now proceed — both
prerequisite repairs are complete and evidenced.
