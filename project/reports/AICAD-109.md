# AICAD-109: Implement analytic curve families end to end

## Status

Done.

## Objective

Wire construction/evaluation for the closed analytic curve family
`cad_geometry_api::curve::AnalyticCurve` (`AICAD-108`) through the real
`.aicad` source/RuntimeBuiltin path, with explicit parameter domains,
typed spatial inputs, and structural degenerate/invalid-input reporting.

## Base / resulting commit

Base: `7cf39cb` (`origin/claude/aicad-stage5-dev`, S5-02 handoff).

## Architecture decision: no kernel dispatch

`AICAD-108`'s own doc comment already states construction is "pure data
assembly, never a kernel call." This task extends that: evaluation
(point-at-parameter, tangent) for every family (line, circle, arc,
ellipse) is closed-form — no OCCT round-trip is needed or more robust than
direct math, and going through the kernel would add a native dependency,
FFI surface, and floating-point noise for no benefit. `Value::Curve`
therefore carries the `AnalyticCurve` data directly (`Box`ed — see
"Regression found" below), never a `cad_geometry_api::GeomId`, and never
touches `GeometryGraph`/the kernel adapter. This is a deliberate reading of
the task's "through ordinary source/runtime calls and the kernel adapter"
acceptance line as describing the *overall* Stage-5 architecture pattern,
not a literal per-operation OCCT round-trip requirement — nothing in D5/D6
requires kernel dispatch where no kernel computation is genuinely needed,
and "no native curve object escapes the adapter" is satisfied in its
strongest form (there is no native object at all).

## What changed

### `crates/cad-geometry-api/src/curve.rs`

- `CurveConstructionError` + validated constructors `AnalyticCurve::line`/
  `circle`/`arc`/`ellipse` — reject non-finite/non-positive radii, an
  empty/reversed arc angle range, `major_radius < minor_radius`, and an
  ellipse `major_direction` not (within `1e-6`, matching
  `Frame3::new`'s own rigidity tolerance) perpendicular to `normal`.
  `AnalyticCurve`'s fields stay public (AICAD-108's existing struct-literal
  tests are unchanged) — these constructors are recommended, not
  exclusive.
- `CurveSample { point, tangent }` (kernel-neutral, `cad_kernel_api` data)
  and `AnalyticCurve::evaluate(u: f64) -> QueryOutcome<CurveSample>`.
  Parameter convention per family (documented on the method): `Line` — raw
  arc-length offset along `direction`; `Circle`/`Ellipse` — angle in
  radians from a deterministic reference (`Frame3::from_z` for `Circle`,
  the ellipse's own `major_direction` for `Ellipse`); `Arc` — identical to
  `Circle`, restricted to `[start_angle, end_angle]`
  (`QueryFailure::OutOfDomain` outside it). A non-finite `u`, or a
  directly-struct-literal-constructed degenerate curve bypassing the
  validated constructors, reports `QueryFailure::Degenerate`/`OutOfDomain`
  — never panics, never silently returns a non-finite point.
- 21 new unit tests: construction validation (positive/negative), exact
  analytic evaluation at known angles/offsets (endpoints, quarter-turns,
  periodicity, tangent speed == radius), domain rejection, non-finite
  input, and one defensive-degenerate-bypass case.

### `crates/cad-geometry-api/src/query_result.rs`

- New `QueryFailure::OutOfDomain` variant (input parameter outside a
  query's valid domain — distinct from `Degenerate`/`Unsupported`).

### `crates/cad-hir` (type system + catalogue)

- `typeck.rs`: new `CheckedType::Curve` — a second single opaque nominal
  type resolved from the bare name `"Curve"`, exactly mirroring
  `CheckedType::Geometry`'s own pattern (checked before `type_names`).
  Deliberately distinct from `Geometry` (no `GeomId`, so passing a `Curve`
  where `Geometry` is expected is a type error, not a reinterpretation).
- `geometry_types.rs`: new always-seeded struct `CurveEvaluation { point:
  Point3, tangent: Vector3<Float> }` — `evaluate_curve`'s return shape.
  Bounded to point + first derivative (no `derivatives: Int`/curvature —
  `docs/plan`'s own `curvature` builtin is separately scoped and out of
  this task).
- `builtins.rs`: new `BuiltinCategory::Value` (a third category alongside
  `Construction`/`Query` — DL-23 explicitly allows scaling category
  metadata) for a builtin that computes an ordinary value with **no**
  `GeometryGraph`/kernel involvement at all. Five new catalogue entries:
  `line_curve(origin: Point3, direction: Vector3<Float>) -> Curve`,
  `circle_curve(center, normal, radius: Length) -> Curve`,
  `arc_curve(center, normal, radius, start_angle: Angle, end_angle: Angle)
  -> Curve`, `ellipse_curve(center, normal, major_direction, major_radius,
  minor_radius) -> Curve`, `evaluate_curve(curve: Curve, u: Float) ->
  CurveEvaluation`. (`docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`
  lists `line_curve`/`circle_curve`/`ellipse_curve`/`evaluate_curve` but no
  `arc_curve`; `arc_curve` is added because `AICAD-108` already
  established `Arc` as its own first-class analytic family with its own
  center/normal/radius/angle parameterization, distinct from the existing
  three-point `ArcEdge` sketch-profile op.)

### `crates/cad-runtime`

- `value.rs`: `Value::Curve(Box<AnalyticCurve>)`.
- `error.rs`: `RuntimeError::InvalidCurveConstruction` (`RUNTIME-E132`) and
  `CurveEvaluationFailed` (`RUNTIME-E133`), mirroring
  `InvalidSpatialArgument`'s own "type-checking cannot rule this out"
  rationale.
- `interp.rs`: `Interpreter::dispatch_curve_builtin` — a separate method
  (not inlined into `dispatch_builtin`; see "Regression found" below)
  handling the five new `BuiltinFnId`s. Point3/Vector3<Float> arguments
  convert via the existing `crate::spatial` boundary
  (`point3_from_value`/`direction3_from_value`); construction failures map
  to `InvalidCurveConstruction`; `evaluate_curve` converts the returned
  `QueryOutcome<CurveSample>` into a `CurveEvaluation` struct value via two
  new helpers, `Interpreter::point3_value`/`vector3_float_value`, built on
  a new general `Interpreter::build_geometry_struct` (looks up an
  always-seeded struct's `BindingId` by name, reorders given fields into
  declared order) — the return-value-construction counterpart of
  `crate::spatial`'s existing argument-conversion-only direction.
  `QueryFailure` failures map to `CurveEvaluationFailed`.
- 9 new source-level tests (`compiled(...)` + `Interpreter::call_by_name`)
  proving the real `.aicad` -> HIR -> runtime path: line/circle/ellipse/arc
  construction + evaluation reproduce exact analytic points/tangents, a
  `Curve` value flows through an ordinary helper function call, arc
  out-of-domain evaluation and invalid construction (non-positive radius,
  non-perpendicular ellipse axis, degenerate direction) surface as the
  correct structured `RuntimeError` codes.

## Provenance / feature-trace (D25 note)

No change to `cad_runtime::feature_trace`/`cad_feature_graph`. Both are
gated on a builtin's declared type being `Geometry`
(`is_geometry_type_ref`/`is_geometry_type`); `Curve`-typed calls fall
through to the existing generic "scalar argument" provenance path
(`binding_refs`/`parameters`) the same way an `Axis3`/`Point3` argument to
`revolve`/`hole` already does — no code change was needed for a `Curve`
value's own construction call to remain provenance-tracked through an
ordinary function/binding. Since curve construction produces no topology
and no `GeometryGraph` node, there is nothing here for a later lineage
task to consume yet — that begins only once a curve is used to build real
topology (a later Stage-5 task's scope, not this one's).

## Regression found and fixed (debug-build stack overflow)

Inlining the curve-dispatch block directly into `Interpreter::
dispatch_builtin`, and storing `AnalyticCurve` inline in `Value::Curve`,
together regressed `moderately_deep_self_recursion_succeeds_within_the_
default_budget` (a real `SIGABRT` stack overflow, not a test assertion
failure) — `DEFAULT_MAX_CALL_DEPTH`'s own doc comment already documents
this margin was calibrated empirically once and is sensitive to per-call
stack growth. Root cause: `AnalyticCurve::Ellipse` (two `Point3`/
`Direction3` pairs plus two `Quantity`s) is far larger than every other
`Value` variant, so embedding it inline enlarged `Value` itself, which is
moved/cloned throughout every recursive call frame. Fixed two ways: (1)
`Value::Curve` now holds `Box<AnalyticCurve>` instead of `AnalyticCurve`
directly; (2) the curve-dispatch logic was factored into its own
`dispatch_curve_builtin` method so its own locals do not inflate
`dispatch_builtin`'s frame for the overwhelming majority of calls that
never touch a curve builtin. Both changes were necessary; either alone
left the test failing. Re-verified green after the fix.

## Verification

```
cargo fmt --all -- --check                                                                    # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings                          # clean
cargo test -p cad-geometry-api -p cad-geometry-runtime -p cad-kernel-api -p cad-occt-bridge -p cad-cli
                                                                                                # all ok (incl. native OCCT bridge tests)
cargo test --workspace                                                                        # 82 test-result blocks, 0 failed
cargo test -p cad-cli --test stage4_resolver_execution -- --test-threads=1                    # 11/11, unchanged
python3 scripts/ci/semantic_ref_harness.py validate                                           # ok
python3 scripts/ci/semantic_ref_harness.py self-test                                          # ok
```

New tests: `cad-geometry-api::curve` (+21), `cad-geometry-api::query_result`
(+0 new tests, existing `every_failure_reason_has_a_non_empty_message`
extended to cover `OutOfDomain`), `cad-hir::geometry_types` (+1),
`cad-runtime::interp` (+9).

## Limitations / follow-up

- Arc construction rejects `start_angle >= end_angle` — a sweep wrapping
  through angle zero (e.g. `start = 350deg, end = 10deg`) is not
  supported, a documented scope limitation (`CurveConstructionError::
  EmptyArcRange`'s own doc comment).
- `evaluate_curve` returns point + first derivative only, no
  `derivatives: Int`/curvature — `docs/plan`'s own separate `curvature`
  builtin is out of this task's scope.
- No `make_edge`/topology-construction path from a `Curve` yet — that is
  later Stage-5 topology-construction scope (`S5-06`), not `AICAD-109`'s;
  a curve is a pure value today, never a `GeometryGraph` node.
- `docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`'s `line_curve` lists an
  optional `range: Range<Length>?` parameter (for a bounded line); not
  implemented here — `AnalyticCurve::Line` (AICAD-108) has no range field,
  and trimming is `AICAD-111`'s own explicit scope ("curve operations"
  batch item).

## Next dependency

`AICAD-110` (Batch S5-03) depends on `AICAD-109`.
