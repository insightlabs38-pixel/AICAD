# AICAD-113: Implement analytic surface families end to end

## Status

Done.

## Objective

Wire construction/evaluation for the closed analytic surface family
`cad_geometry_api::surface::AnalyticSurface` (`AICAD-108` stub) through the
real `.aicad` source/RuntimeBuiltin path — the surface-family counterpart
of `AICAD-109`'s curve work, with explicit parameter domains, typed
spatial inputs, and structural singular/degenerate reporting.

## Base / resulting commit

Base: `ac50640` (`origin/claude/aicad-stage5-dev`, S5-03/Checkpoint A
handoff).

## Architecture decision: no kernel dispatch (mirrors AICAD-109)

Every family (plane, cylinder, cone, sphere, ring torus) is closed-form:
point, both partial derivatives, and the normal are all elementary
trigonometric expressions. No OCCT round-trip is needed or more accurate.
`Value::Surface` therefore carries `AnalyticSurface` data directly (boxed,
mirroring `Value::Curve`), never a `GeomId`, and never touches
`GeometryGraph`/the kernel adapter.

## Architecture decision: normal is always `du x dv`, normalized

`evaluate` never derives a family-specific closed-form normal (e.g. "radial
direction from center" for a sphere). It always computes
`du.cross(dv).normalize()`. This is what makes the acceptance criterion
("singular/degenerate parameter locations are reported structurally") true
for free: at a sphere's own pole (`v = +-pi/2`) or a cone's own apex
(`v = 0`), longitude `u` is ambiguous and `du` is the zero vector, so the
cross product is zero and normalizing it correctly yields `None` ->
`QueryFailure::Degenerate`. A hand-derived closed-form normal would stay
well-defined at exactly these points (the underlying point/tangent-plane is
not geometrically singular) and would silently paper over the genuine
parametrization singularity the acceptance line is about. No extra
per-family singularity check was needed anywhere as a result.

## What changed

### `crates/cad-geometry-api/src/surface.rs`

- `SurfaceConstructionError` + validated constructors `AnalyticSurface::
  plane`/`cylinder`/`cone`/`sphere`/`torus` — reject a non-finite/
  non-positive radius, a cone `half_angle` outside `(0, pi/2)`, and a torus
  `minor_radius >= major_radius` (Stage-5's scope is the non-self-
  intersecting "ring" torus only, mirroring `AnalyticCurve::bspline`'s own
  "constructible-only-to-reject" precedent for `periodic: true`). `plane`
  has no failure mode (a `Direction3` can never itself be degenerate).
- `SurfaceSample { point, du, dv, normal }` and `AnalyticSurface::
  evaluate(u, v) -> QueryOutcome<SurfaceSample>`. Parameter convention per
  family (documented on the method): `Plane` — raw in-plane offsets via
  `Frame3::from_z`; `Cylinder` — angle around axis (periodic) / height
  along axis (unbounded); `Cone` — angle (periodic) / distance from apex,
  `v >= 0` required; `Sphere` — longitude (periodic, referenced against
  `Frame3::WORLD` since a sphere has no orientation of its own) / latitude,
  required in `[-pi/2, pi/2]`; `Torus` — angle around the main axis
  (periodic) / angle around the tube (periodic). Every family validated at
  construction (`Plane`, `Cylinder` given `major_radius > minor_radius`
  ring `Torus`) is provably never degenerate away from the domain edges
  documented above — verified directly (`torus_never_degenerates_for_any_
  finite_u_v` sweeps a 12x12 grid).
- 21 new unit tests: construction validation (positive/negative), exact
  analytic evaluation with independent geometric checks (point lies at the
  claimed radius/height/distance from the defining axis/center, normal is
  exactly the outward radial direction, `du`/`dv`/normal mutually
  orthogonal), the two genuine singularities (sphere pole, cone apex), and
  domain rejection (cone `v < 0`, sphere `|v| > pi/2`).

### `crates/cad-hir/src/typeck.rs`

- `CheckedType::Surface` — the single opaque nominal type for `Surface`,
  resolved from the bare name exactly like `CheckedType::Curve`/`Geometry`
  (kept distinct from both, for the identical reason `Curve` is kept
  distinct from `Geometry`).

### `crates/cad-hir/src/geometry_types.rs`

- `struct SurfaceEvaluation { point, du, dv, normal }` added to the
  always-seeded standard-type source, mirroring `CurveEvaluation` exactly.
  Updated the two tests asserting the always-seeded struct count (nine ->
  ten) and added a construction/field-read test for `SurfaceEvaluation`.

### `crates/cad-hir/src/builtins.rs`

- Six new `BuiltinFnId`s (`BuiltinCategory::Value`, no kernel/`GeometryGraph`
  involvement — mirrors every curve builtin): `plane_surface(origin: Point3,
  normal: Vector3<Float>) -> Surface`, `cylinder_surface(axis: Axis3,
  radius: Length) -> Surface`, `cone_surface(axis: Axis3, half_angle:
  Angle) -> Surface`, `sphere_surface(center: Point3, radius: Length) ->
  Surface`, `torus_surface(axis: Axis3, major_radius: Length,
  minor_radius: Length) -> Surface`, `evaluate_surface(surface: Surface, u:
  Float, v: Float) -> SurfaceEvaluation`. `ALL`/`catalogue` extended
  (31 -> 37 entries); the existing `catalogue_has_exactly_one_entry_per_
  builtin_fn_id` self-check covers the new entries with no changes needed.

### `crates/cad-runtime`

- `Value::Surface(Box<AnalyticSurface>)` (`value.rs`) — boxed for the
  identical reason `Value::Curve` is boxed (keeps `Value` small regardless
  of variant; `AnalyticSurface` is smaller than `AnalyticCurve` today but
  boxing now avoids re-deriving this once `AICAD-114` adds a `Vec`-carrying
  variant).
- `RuntimeError::InvalidSurfaceConstruction`/`SurfaceEvaluationFailed`
  (`error.rs`, codes `RUNTIME-E136`/`RUNTIME-E137`) — mirror
  `InvalidCurveConstruction`/`CurveEvaluationFailed` exactly.
- `Interpreter::dispatch_surface_builtin` (`interp.rs`) — the surface-family
  counterpart of `dispatch_curve_builtin`, factored into its own method for
  the identical stack-frame-size reason documented on that method. Hooked
  into `dispatch_builtin` via the same `matches!`-guard-then-early-return
  pattern the curve block already uses, before the `GeometryOp`-pushing
  Construction match (which gets an `unreachable!` arm for the six new
  surface `BuiltinFnId`s, exactly like curve's).
- 12 new end-to-end interpreter tests: each family's construction +
  `evaluate_surface` reproducing an independently-computed point/normal,
  the cone-apex and sphere-pole singularities surfacing as
  `RUNTIME-E137`, two construction-rejection tests (`RUNTIME-E136`), a
  `Surface` value flowing through an ordinary helper function, and a
  feature-trace-invisibility test mirroring `AICAD-112`'s identical curve
  test (`AICAD-107`'s `is_geometry_type_ref` gate is generic over any
  non-`Geometry` type name, so it already covers `Surface` with no change).

## Verification

```
cargo test -p cad-geometry-api --lib surface::        # 18 passed
cargo fmt --all -- --check                            # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings  # clean
cargo test -p cad-geometry-api -p cad-geometry-runtime -p cad-kernel-api \
  -p cad-occt-bridge -p cad-cli                        # all passed, incl. native OCCT tests
cargo test --workspace                                # all passed, 0 failed
```

## Limitations / follow-up

- Bezier/B-spline/NURBS surfaces (`AICAD-114`), the trimmed-surface model
  (`AICAD-115`), and offset/generalized-derivative operations across every
  family (`AICAD-116`) are explicitly out of this task's scope, per the
  fixed batch order.
- The torus is deliberately scoped to the non-self-intersecting "ring" case
  (`minor_radius < major_radius`) — a spindle or self-intersecting torus is
  rejected at construction, matching `AnalyticCurve::bspline`'s own
  "constructible-only-to-reject" precedent for an unsupported shape rather
  than a silently-wrong or panicking evaluation.
- No `make_face`/topology-construction path from a `Surface` exists yet —
  surfaces remain pure values until a later Stage-5 topology task
  (`AICAD-119`+) bridges them, exactly like `AICAD-109`'s curves.
- No active example added — consistent with `AICAD-108`-`111`'s own
  precedent of deferring example work to the batch's checkpoint (the next
  one is Checkpoint B, after `AICAD-117`/`118`).

## Next

Per the fixed batch order, `AICAD-114` (Bezier/B-spline/NURBS surfaces).
