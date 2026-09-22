# AICAD-114: Implement Bezier, B-spline, and NURBS surface families

## Status

Done.

## Objective

Extend `cad_geometry_api::surface::AnalyticSurface` with tensor-product
Bezier/B-spline/NURBS surfaces, wired through the real `.aicad` source/
RuntimeBuiltin path — the surface-family counterpart of `AICAD-110`'s
curve work.

## Base / resulting commit

Base: `e7c93d9` (`origin/claude/aicad-stage5-dev`, `AICAD-113`).

## Architecture decision: reuse `crate::curve`'s NURBS core, don't duplicate it

`crate::curve`'s de Boor/derivative/knot-expansion machinery
(`Homogeneous`, `to_homogeneous`, `expand_knots`, `clamped_bezier_knots`,
`find_span`, `de_boor`, `derivative_control_points`) already implements
exactly the per-direction math a tensor-product surface needs twice (once
per parametric direction). These six items were widened from private to
`pub(crate)` and reused directly by a new `crate::surface::
tensor_bspline_evaluate` rather than re-implemented — so the curve and
surface numerical cores cannot silently diverge, and any future fix to one
automatically benefits the other.

A tensor-product surface is separable: holding `v` fixed, `S(u, v0)` is an
ordinary B-spline curve in `u` whose control points are each control-net
row evaluated at `v0`; holding `u` fixed is the symmetric construction.
`tensor_bspline_evaluate` computes exactly those two curves' worth of
homogeneous data and reuses `de_boor`/`derivative_control_points`
unchanged for both the point and each partial derivative, applying the
identical rational quotient rule `crate::curve::nurbs_evaluate` already
uses for a plain curve's tangent, once per direction.

## What changed

### `crates/cad-geometry-api/src/curve.rs`

- `Homogeneous`/`to_homogeneous`/`expand_knots`/`clamped_bezier_knots`/
  `find_span`/`de_boor`/`derivative_control_points` widened from private to
  `pub(crate)`, documented as reused by `crate::surface`.

### `crates/cad-geometry-api/src/surface.rs`

- `AnalyticSurface` gains `Bezier { control_points: Vec<Vec<Point3>>,
  weights: Option<Vec<Vec<f64>>> }` and `BSpline { degree_u, degree_v,
  control_points, knots_u, multiplicities_u, knots_v, multiplicities_v,
  weights, periodic_u, periodic_v }` — `control_points[i][j]`, outer index
  along `u`, inner along `v`. `AnalyticSurface` is no longer `Copy` (mirrors
  `AnalyticCurve`'s own identical `AICAD-110` change); every prior call
  site relying on an implicit copy now clones.
- `SurfaceConstructionError` gains `TooFewControlPoints`/`RaggedControlNet`/
  `MismatchedWeightShape`/`NonPositiveWeight`/`MismatchedKnotArrays`/
  `NonIncreasingKnots`/`InvalidMultiplicity`/`KnotControlPointCountMismatch`/
  `UnsupportedPeriodic` — validated constructors `AnalyticSurface::bezier`/
  `bspline`, sharing `validate_rectangular_net`/`validate_weight_shape`/
  `validate_knot_direction` helpers (the latter applied once per direction,
  mirroring `AnalyticCurve::bspline`'s own validation exactly).
  `periodic_u`/`periodic_v: true` is constructible-only-to-reject, matching
  `AnalyticCurve::BSpline`'s own scope limitation.
- `AnalyticSurface::evaluate` gains `Bezier`/`BSpline` arms, both reducing
  to `tensor_bspline_evaluate` (Bezier via the equivalent clamped knot
  vector per direction, exactly `AnalyticCurve::Bezier`'s own precedent).
- 15 new unit tests: a hand-derived bidegree-(1,1) bilinear control net
  checked against a closed-form `point(u,v) = (u, v, u*v)` (point *and*
  both derivatives independently verified), corner-interpolation, a
  uniform-weight rational/non-rational equivalence check (a strong
  correctness proof for the rational quotient rule without hand-deriving a
  new rational closed form), Bezier/equivalent-B-spline agreement across a
  parameter grid, and construction/domain rejection (ragged net, too few
  control points, weight shape mismatch, mismatched knot arrays, periodic,
  out-of-domain).

### `crates/cad-hir/src/builtins.rs`

- `list_of_ref(elem: HirTypeRef) -> HirTypeRef` — `list_of`'s general form,
  needed for a `List<List<Point3>>` control-net parameter `list_of`'s
  `&str`-only signature cannot express.
- Two new `BuiltinFnId`s (`BuiltinCategory::Value`): `bezier_surface(
  control_points: List<List<Point3>>, weights: List<List<Float>>) ->
  Surface` and `bspline_surface(degree_u: Int, degree_v: Int,
  control_points: List<List<Point3>>, knots_u: List<Float>,
  multiplicities_u: List<Int>, knots_v: List<Float>, multiplicities_v:
  List<Int>, weights: List<List<Float>>, periodic_u: Bool, periodic_v:
  Bool) -> Surface`. `ALL`/`catalogue` extended (37 -> 39 entries).

### `crates/cad-runtime/src/interp.rs`

- `dispatch_surface_builtin`'s `surface` closure changed from `Ok(**s)` to
  `Ok((**s).clone())` (`AnalyticSurface` is no longer `Copy`).
- New `point_grid`/`optional_weight_grid` closures (`List<List<_>>` ->
  `Vec<Vec<_>>`, one level of element conversion per row) alongside the
  existing scalar/list closures, plus the two new match arms dispatching to
  `AnalyticSurface::bezier`/`bspline`.
- 4 new end-to-end interpreter tests, including nested list literals
  (`[[Point3(...), Point3(...)], [...]]`) reaching a real `bezier_surface`/
  `bspline_surface` call — confirms `List<List<T>>` source syntax works
  with zero new grammar (an existing `AICAD-056`/`AICAD-110` capability,
  not previously exercised two levels deep).

## Verification

```
cargo test -p cad-geometry-api --lib surface::   # 44 passed (29 + 15 new)
cargo fmt --all -- --check                       # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings  # clean
cargo test -p cad-geometry-api -p cad-geometry-runtime -p cad-kernel-api \
  -p cad-occt-bridge                             # all passed, incl. native OCCT tests
cargo test --workspace                           # all passed, 0 failed
```

## Limitations / follow-up

- Trimmed surfaces (`AICAD-115`) and generalized offset/derivative
  operations across every family (`AICAD-116`) remain out of scope.
- `periodic_u`/`periodic_v` remain constructible-only-to-reject, exactly
  like `AnalyticCurve::BSpline::periodic` — a closed/wrapping B-spline
  surface genuinely differs in control-net/knot relationship, not merely a
  parameter-domain restriction on the same math.
- `tensor_bspline_evaluate` uses the straightforward double-de-Boor
  algorithm (not the most locally-optimized tensor evaluation), matching
  this crate's established "small control nets, correctness over
  micro-efficiency" precedent (`crate::curve::solve_linear_system`'s own
  doc comment).
- No active example added yet — deferred to Checkpoint B (after
  `AICAD-117`/`118`), per the same precedent `AICAD-113`'s report recorded.

## Next

Per the fixed batch order, `AICAD-115` (trimmed-surface semantic model).
