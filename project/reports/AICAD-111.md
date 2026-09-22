# AICAD-111: Implement curve operations with explicit result and tolerance semantics

## Status

Done.

## Objective

Add curve-local operations (trim, offset, closest-point, interpolation)
over `AnalyticCurve` (`AICAD-109`/`110`), each with typed inputs, explicit
tolerance ownership, and never an arbitrary single answer where the
underlying math has zero, one, or many solutions.

## Base / resulting commit

Base: `aeb35ce` (this branch, `AICAD-110`).

## What changed

### `crates/cad-geometry-api/src/curve.rs`

- **Trim**: new `AnalyticCurve::Trimmed { base, u0, u1 }` variant and
  validated `AnalyticCurve::trim` — restricts (never reparametrizes/
  subdivides) `base` to `[u0, u1]`, rejecting a non-finite/reversed range
  or one that widens an already-bounded curve's own domain. New
  `AnalyticCurve::domain() -> Option<(f64, f64)>` (`None` for
  Line/Circle/Ellipse's unbounded/periodic domain, `Some` for Arc/Bezier/
  BSpline/Trimmed) backs the widening check and is independently useful.
- **Offset**: `AnalyticCurve::offset(distance, normal) -> Result<Self,
  CurveOperationError>` — exact for `Line` (translated along
  `direction.cross(normal)`; `normal` required to disambiguate 3D), and
  for `Circle`/`Arc` (radius ± distance, re-validated through the existing
  `circle`/`arc` constructors). `CurveOperationError::UnsupportedFamily`
  for Ellipse/Bezier/BSpline/Trimmed — exact offsetting is not, in
  general, expressible in the same family (a well-known CAD limitation);
  an honest rejection, never an approximated or silently wrong curve.
- **Closest point**: `AnalyticCurve::closest_point(target) ->
  QueryOutcome<ClosestPointResult>` — exact closed-form for `Line`
  (orthogonal projection) and `Circle` (radial projection via the same
  `Frame3::from_z` reference basis `evaluate` uses); every other family
  uses a bounded numerical search (`numeric_closest_points`: coarse
  sampling to bracket every local minimum of squared distance, then
  golden-section refinement per bracket — `basis_funs`-free, general-
  purpose). **Never** picks one candidate arbitrarily: every local minimum
  found is reported as a separate solution (deduplicated by resulting
  *point*, not parameter — needed at a periodic family's own domain seam,
  where `lo` and `hi` land on the identical point at different
  parameters). A target exactly on a circle/ellipse's own normal axis
  (every point equidistant) is `QueryFailure::Degenerate`, never an
  arbitrary pick.
- **Interpolation**: free function `interpolate(points, tolerance:
  ApproximationTolerance) -> Result<AnalyticCurve, CurveOperationError>` —
  Piegl & Tiller §9.2.1's standard global cubic (degree-3) B-spline
  interpolation: chord-length parameter values, an averaged knot vector,
  and control points solved from the resulting linear system via Gaussian
  elimination with partial pivoting (`basis_funs` computes the nonzero
  B-spline basis values; `solve_linear_system` is a plain, small, O(n³)
  dense solver — no banded-matrix specialization needed at this scale).
  This is *exact interpolation* (the curve passes through every input
  point at its own chord-length parameter), not approximating/least-
  squares fitting to a target error — `tolerance` bounds only the
  achieved *numerical* residual the linear solve itself may leave
  (`CurveOperationError::ToleranceNotAchieved` if a near-singular system —
  duplicate/collinear points — leaves a residual above it), per
  `AGENTS.md`'s "report achieved error... rather than silently widening."
- New `CurveOperationError` enum (`UnsupportedFamily`,
  `DegenerateDirection`, `DegenerateResult`, `TooFewPoints`,
  `DuplicatePoints`, `ToleranceNotAchieved`) and `ClosestPointResult`
  struct (`parameter`, `point`, `distance: Quantity`), both re-exported.
- 24 new unit tests: every construction/operation rejection branch, exact
  trim/offset/closest-point values (including a two-solution ellipse-
  center case and an endpoint-symmetric-minimum case cross-checked to a
  looser, numerically-honest tolerance), and exact interpolation
  (endpoints, every input point reproduced at its own re-derived
  parameter, and determinism across repeated calls with identical input).

### `crates/cad-hir` (type system + catalogue)

- `geometry_types.rs`: new always-seeded `ClosestPointResult { parameter:
  Float, point: Point3, distance: Length }` — `closest_point_on_curve`'s
  own per-solution element type, used as `List<ClosestPointResult>` (only
  possible because of `AICAD-110`'s own `List<T>` widening).
- `builtins.rs`: four new `BuiltinCategory::Value` entries:
  `trim_curve(curve: Curve, u0: Float, u1: Float) -> Curve`,
  `offset_curve(curve: Curve, distance: Length, normal: Vector3<Float>) ->
  Curve` (`normal` always required — no optional-parameter mechanism, per
  `AICAD-110`'s own established convention, even though only `Line`
  actually consumes it), `closest_point_on_curve(curve: Curve, point:
  Point3) -> List<ClosestPointResult>`, `interpolate_curve(points:
  List<Point3>, tolerance: Length) -> Curve`.

### `crates/cad-runtime`

`Interpreter::dispatch_curve_builtin` gained four match arms.
`closest_point_on_curve` is the first builtin in the whole catalogue that
*constructs* a `Value::List` of struct-valued results (rather than
consuming one) — built directly from `Interpreter::point3_value`/
`build_geometry_struct`, no new machinery needed. Two new `RuntimeError`
variants: `CurveOperationFailed` (`RUNTIME-E134`, wraps
`CurveOperationError` — offset/interpolate failures) and
`ClosestPointFailed` (`RUNTIME-E135`, wraps `QueryFailure` — a genuinely
degenerate closest-point query). 8 new source-level tests
(`compiled(...)` + `Interpreter::call_by_name`) proving the real `.aicad`
path end to end, including iterating a returned `List<ClosestPointResult>`
with an ordinary `for` loop (no list-indexing syntax exists yet — not
needed here).

## Verification

```
cargo fmt --all -- --check                                                                    # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings                          # clean
cargo test -p cad-geometry-api -p cad-geometry-runtime -p cad-kernel-api -p cad-occt-bridge    # all ok
cargo test --workspace                                                                        # 82 test-result blocks, 0 failed
cargo test -p cad-cli --test stage4_resolver_execution -- --test-threads=1                    # 11/11, unchanged
python3 scripts/ci/semantic_ref_harness.py validate                                           # ok
python3 scripts/ci/semantic_ref_harness.py self-test                                          # ok
```

New tests: `cad-geometry-api::curve` (+24), `cad-runtime::interp` (+8).

## Limitations / follow-up

- `offset_curve` is exact only for `Line`/`Circle`/`Arc` —
  Ellipse/Bezier/B-spline offsetting is a documented, honest gap (a
  well-known hard problem generally requiring a higher-degree, often
  non-rational, result curve), not attempted approximately.
- `closest_point`'s numerical search (Ellipse/Bezier/BSpline/Trimmed
  families) resolves a parameter only to roughly the square root of
  floating-point epsilon near a numerically flat minimum — an expected,
  inherent limit of any value-comparison-only minimizer, not a
  correctness defect (documented in the relevant test's own comment).
- `interpolate` is fixed at degree 3 with chord-length parametrization and
  no explicit end-tangent/periodic support — `docs/plan`'s own
  `interpolate_curve` mentions an optional `tangents`/`periodic` shape not
  implemented here, a documented narrowing (matching this catalogue's
  established precedent of narrowing optional-shaped plan signatures).
- `project_curve`/curve-to-surface projection and curve/curve, curve/
  surface intersection remain `AICAD-117`/`118`'s own scope ("geometric
  queries"), not this task's — `closest_point` is the bounded,
  curve-local building block those tasks can build on, per this task's
  own acceptance criteria wording.

## Next dependency

`AICAD-112` (Batch S5-03, Checkpoint A) depends on `AICAD-107` and
`AICAD-111` — the final task in this batch.
