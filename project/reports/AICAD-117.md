# AICAD-117: Implement multi-solution geometry intersections, projections, and distance queries

## Status

Done.

## Objective

Add curve/curve, curve/surface, and surface/surface intersection, point-to-
surface projection, and curve/curve, curve/surface, surface/surface distance
queries over `AnalyticCurve`/`AnalyticSurface` (`AICAD-109`-`116`), each
returning `QueryOutcome<T>` (`AICAD-108`) — never an arbitrary single answer
where zero, one, or many solutions are mathematically possible.

## Base / resulting commit

Base: `dd94e70` (`origin/claude/aicad-stage5-dev`, `AICAD-116`).

## Architecture: exact where tractable, one general numeric fallback elsewhere

Every query is either **exact** (closed-form) or a **bounded numerical
search** composing two primitives: `AnalyticCurve::closest_point`
(`AICAD-111`) and the new `AnalyticSurface::project_point`. Finding every
local minimum of "distance from a swept point to the other shape" (coarse-
grid bracket + golden-section refinement, `AnalyticCurve::closest_point`'s
own established method generalized) works for *any* curve/curve or
curve/surface pair whose outer side has a bounded parameter domain — a
single general mechanism rather than a hand-derived formula per family
pair.

Exact closed forms, preferred whenever available:
- **Line-Line** (`intersect`/`distance`): the only two curve families with
  no bounded domain, so the general composition cannot run.
- **Line vs. Plane/Sphere/Cylinder** (`intersect`/`distance`) and **Line vs.
  Cone** (`intersect` only): standard line-quadric root-finding.
- **Plane-Plane, Plane-Sphere, Sphere-Sphere** (`intersect_surfaces` only):
  the only surface/surface pairs whose intersection curve is representable
  by `AnalyticCurve`'s closed family set at all — a general quadric/quadric
  intersection is a space curve this crate cannot hold, a structural limit.
  Every other surface/surface pair is `QueryFailure::Unsupported`.

`distance_surface_surface` has no such structural limit (a scalar answer,
not a curve), so it runs the general 2D composition whenever *either*
surface has a bounded `(u, v)` domain (`Sphere`/`Torus`/`Bezier`/`BSpline`);
`Unsupported` only when both sides are unbounded (`Plane`/`Cylinder`/`Cone`)
or either is `Trimmed`.

## `Degenerate` covers "coincident" and "tangent" — deliberately

An infinite-family configuration (identical/overlapping lines, circles,
planes, spheres) and a single-point tangency both report
`QueryFailure::Degenerate` rather than a new tagged-union payload type —
both are genuinely "no well-formed finite answer of this query's own
shape," and both are already unambiguously distinct from every
`QueryOutcome::Solutions` outcome. This satisfies "explicit status, never an
arbitrary representative" without inventing a coincidence/tangency marker
type; each sub-case has its own dedicated test.

## What changed

### `crates/cad-geometry-api/src/surface.rs`

New `AnalyticSurface::project_point(&self, target) -> QueryOutcome<
SurfaceProjectionResult>` — the surface-family counterpart of
`AnalyticCurve::closest_point`. Exact for `Plane`/`Cylinder`/`Cone`/
`Sphere`/`Torus` (every axis-symmetric family's closest point has the
*same angular parameter as the target itself* — a standard result — so
each reduces to a 1D problem in the `(axial, radial)` meridian half-plane,
never a genuine 2D search); a bounded numerical grid-search-plus-refinement
for `Bezier`/`BSpline`; `QueryFailure::Unsupported` for `Trimmed` (disclosed
limit — the search does not yet account for a trimmed boundary).
20 new unit tests, including cross-checks against `evaluate` at the
projection's own returned `(u, v)`.

### `crates/cad-geometry-api/src/query.rs` (new)

`intersect_curves`, `intersect_curve_surface`, `intersect_surfaces`,
`distance_curve_curve`, `distance_curve_surface`, `distance_surface_surface`
— see module doc comment (reproduced above) for the exact/numeric/
`Unsupported` split. New result types `CurveCurveIntersection`,
`CurveSurfaceIntersection`, `DistanceResult`. 41 new unit tests: unique,
zero, multiple (two-circle intersection), tangent, coincident/overlap,
skew/parallel-line distance (including the classic textbook skew-line-
distance and perpendicular-offset cases), and `Trimmed`/unsupported-family
rejection, across every query.

### `crates/cad-hir`

- `geometry_types.rs`: four new always-seeded structs —
  `CurveIntersectionResult`, `CurveSurfaceIntersectionResult`,
  `SurfaceProjectionResult`, `DistanceResult`.
- `builtins.rs`: seven new `BuiltinCategory::Value` entries —
  `intersect_curves(a: Curve, b: Curve, tolerance: Length) ->
  List<CurveIntersectionResult>`, `intersect_curve_surface(curve: Curve,
  surface: Surface, tolerance: Length) ->
  List<CurveSurfaceIntersectionResult>`, `intersect_surfaces(a: Surface, b:
  Surface, tolerance: Length) -> List<Curve>`, `project_point_to_surface
  (surface: Surface, point: Point3) -> List<SurfaceProjectionResult>`,
  `distance_curve_curve(a: Curve, b: Curve) -> List<DistanceResult>`,
  `distance_curve_surface(curve: Curve, surface: Surface) ->
  List<DistanceResult>`, `distance_surface_surface(a: Surface, b: Surface)
  -> List<DistanceResult>`. Catalogue widened 41 -> 48 entries.

### `crates/cad-runtime`

New `Interpreter::dispatch_geometric_query_builtin` (mirrors
`dispatch_curve_builtin`/`dispatch_surface_builtin`'s own factoring), a new
`RuntimeError::GeometricQueryFailed` (`RUNTIME-E142`, wraps `QueryFailure`)
shared by all seven builtins, and a `distance_result_list` helper shared by
the three distance builtins. 12 new end-to-end interpreter tests proving
the real `.aicad` source path (unique/zero/coincident/unsupported cases per
query family).

### Examples (Checkpoint-adjacent, deferred formally to `AICAD-118`)

Two new ACTIVE examples were authored alongside this task and registered in
`AICAD-118`'s own checkpoint pass — see that report.

## Verification

```
cargo fmt --all -- --check                                                                # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings                      # clean
cargo test -p cad-query -p cad-geometry-api -p cad-geometry-runtime \
  -p cad-kernel-api -p cad-occt-bridge -p cad-cli                                          # all ok
cargo test --workspace                                                                    # 0 failed
```

New tests: `cad-geometry-api::surface` (+20 `project_point`), `cad-geometry-
api::query` (+41), `cad-hir::geometry_types` (+0 new, 1 stale count fixed:
`with_geometry_types_prepends_the_ten_declarations...` -> `...fourteen...`),
`cad-runtime::interp` (+12).

## Limitations / follow-up

- `intersect_surfaces` is exact only for `Plane`-`Plane`/`Plane`-`Sphere`/
  `Sphere`-`Sphere` — every other pair (including the common
  `Cylinder`-`Cylinder`/`Plane`-`Cylinder` cases) is `Unsupported`: a
  structural gap (no space-curve value type exists to hold a general
  quadric/quadric intersection), not a narrowed effort scope.
- `distance_curve_surface`/`intersect_curve_surface` for `Line` vs. `Cone`
  distance (not intersection) is `Unsupported` when the line does not
  actually intersect the cone — the general skew-line-to-cone minimum-
  distance closed form was not derived here (unlike `Cylinder`, whose
  radial-distance objective is provably unimodal, `Cone`'s meridian-plane
  objective was not proven unimodal with the effort available).
- `Line` vs. `Torus`/`Bezier`/`BSpline`/`Trimmed` (intersection and
  distance) and any query touching a `Trimmed` surface: `Unsupported`
  (mirrors `AnalyticSurface::offset`'s own established narrowing).
- Multi-solution ambiguity ("many solutions... deterministic ordering") is
  satisfied by construction (outer-parameter/bracket order, never kernel
  enumeration order — there is no kernel call anywhere in this task), but a
  distance query's *witness point* for a provably flat objective (parallel
  lines, a line parallel to a cylinder's own axis) is a canonical,
  documented choice among infinitely many equally-valid witnesses — the
  *distance* itself is unambiguous.
- `project_point`'s Bezier/BSpline numerical search shares
  `AnalyticCurve::closest_point`'s own inherent flat-minimum resolution
  limit near a numerically shallow minimum.

## Next dependency

`AICAD-118` (Batch S5-05, Checkpoint B) depends on this task — the final
task in this batch.
