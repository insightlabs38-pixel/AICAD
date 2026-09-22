# AICAD-116: Implement surface evaluation, derivatives, normals, and offsets

## Status

Done. Final task of batch S5-04.

## Objective

Add bounded surface offsetting (`offset_surface`), and independently
re-verify — across every constructible surface family, not merely each
family's own hand-derived closed form — that `evaluate`'s analytic
derivatives/normals are correct. Evaluation/derivative/normal APIs
themselves were already delivered by `AICAD-113`-`115`; this task's own new
surface is offset plus the cross-family independent-evidence pass the
batch's acceptance line explicitly calls for.

## Base / resulting commit

Base: `2110288` (`origin/claude/aicad-stage5-dev`, `AICAD-115`).

## Architecture decision: exact, not approximated — no tolerance domain consumed

Every offset this task implements is closed-form: `Plane`/`Cylinder`/
`Sphere`/`Torus` reduce to one scalar adjustment (origin translation, or
radius/minor-radius arithmetic); `Cone` reduces to a derived apex-shift
identity (below). None needs `cad_units::ApproximationTolerance`,
`ConstructionTolerance`, or any other `project/DECISION_LOG.md#DL-26`
domain — there is no fitting/approximation step to bound. This
automatically satisfies the acceptance line "never borrows D5 equivalence
tolerance or silently widens modeling tolerance": there is no tolerance to
borrow or widen in the first place, mirroring `AnalyticCurve::offset`'s
(`AICAD-111`) own identical zero-tolerance precedent for `Line`/`Circle`/
`Arc`.

## The cone offset identity (derived and independently verified)

A cone's own outward normal at any surface point is `n = cos(half_angle) *
radial(u) - sin(half_angle) * axis_dir` — independent of the axial
parameter `v` (only `u`, the angle around the axis, matters). This constant
axial normal component is exactly what makes offsetting a cone by a fixed
distance `d` produce *another cone of the same half-angle*: substituting
`P + d*n` into the cone parametrization and solving shows it equals a
cone with `apex' = apex - (d / sin(half_angle)) * axis_dir`, `v' = v + d *
cos(half_angle)^2 / sin(half_angle)`, same `half_angle`. This was derived
algebraically (documented in `AnalyticSurface::offset`'s own doc comment)
and then checked independently in a unit test: evaluate the *original*
cone at an arbitrary `(u0, v0)`, compute `P0 + d*n0` directly from its own
returned point/normal, and confirm the *offset* cone's own evaluation at
`(u0, v0')` lands on that exact point (`1e-9` tolerance) — a genuine
cross-check against the constructor's own formula, not merely internal
self-consistency.

## What changed

### `crates/cad-geometry-api/src/surface.rs`

- `SurfaceOperationError` (`UnsupportedFamily`/`DegenerateResult`) and
  `AnalyticSurface::offset(&self, distance: Quantity) -> Result<...>` —
  exact for `Plane`/`Cylinder`/`Cone`/`Sphere`/`Torus`; `Bezier`/`BSpline`/
  `Trimmed` report `UnsupportedFamily` (exact offsetting of a freeform/
  trimmed surface is not, in general, expressible in the same family — the
  identical well-known limitation `AnalyticCurve::offset` already
  documents for a curve). `DegenerateResult` for a cylinder/sphere radius
  or torus minor-radius that would go non-positive, or a torus offset that
  would break the `minor_radius < major_radius` ring invariant.
- 7 new unit tests: plane origin-translation, cylinder/sphere radius
  adjustment (including a negative "shrink" distance), cylinder
  degenerate-radius rejection, torus minor-radius adjustment plus
  ring-invariant-breaking rejection, the cone cross-check above, and
  Bezier/trimmed unsupported-family rejection.
- **`analytic_derivatives_agree_with_a_central_finite_difference_
  everywhere`**: for every constructible family (`Plane`/`Cylinder`/
  `Cone`/`Sphere`/`Torus`/`Bezier`), a central finite difference of
  `evaluate`'s own point (step `1e-5`) is compared against the analytic
  `du`/`dv` `evaluate` returns — independent numerical evidence distinct
  from each family's own hand-derived closed-form check
  (`AICAD-113`/`114`'s own tests), satisfying this task's own "independent
  checks" acceptance line directly.

### `crates/cad-hir/src/builtins.rs`

- `offset_surface(surface: Surface, distance: Length) -> Surface`
  (`BuiltinCategory::Value`). `ALL`/`catalogue` extended (40 -> 41 entries).

### `crates/cad-runtime`

- `RuntimeError::SurfaceOperationFailed` (`error.rs`, `RUNTIME-E141`) —
  mirrors `CurveOperationFailed`.
- `dispatch_surface_builtin`'s new `OffsetSurface` arm (`interp.rs`).
- 3 new end-to-end interpreter tests: a widened cylinder's radius via
  `evaluate_surface`, a degenerate-offset rejection, and an
  unsupported-family (Bezier) rejection.

## Verification

```
cargo test -p cad-geometry-api --lib surface::   # 52 passed (unit); 140 passed (whole crate)
cargo fmt --all -- --check                       # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings  # clean
cargo test -p cad-geometry-api -p cad-geometry-runtime -p cad-kernel-api \
  -p cad-occt-bridge -p cad-query                # all passed (cad-query unaffected, 98/98)
cargo test --workspace                           # all passed, 0 failed
```

## Limitations / follow-up

- No active example added yet — deferred to Checkpoint B (`S5-05`, after
  `AICAD-117`/`118`), per `AICAD-113`-`115`'s own established precedent;
  the checkpoint's own acceptance requires at least one new representative
  example, which will need to draw on the full surfaces batch (`AICAD-
  113`-`116`) together.
- "Self-intersection... when detectable" (this task's own acceptance
  line) is not applicable to any of the five exact-offset families: an
  offset within the domain this module validates (radius/minor-radius
  staying positive, ring-torus invariant preserved) cannot self-intersect
  for these convex/developable analytic shapes. A genuinely
  self-intersecting offset case (e.g. an offset distance exceeding a
  freeform surface's own local curvature radius) does not arise since
  freeform families are `UnsupportedFamily` here.
- "Kernel failures" (this task's own acceptance line) do not apply: no
  offset in this module touches a kernel context, mirroring every other
  Stage-5 curve/surface value/evaluation operation to date.

## S5-04 batch complete

`AICAD-113`-`116` are all done. Batch `S5-04` (surfaces) is complete.
Per the fixed batch order, the next invocation begins `S5-05`
(`AICAD-117`-`118`: multi-solution geometric queries, then Checkpoint B).
