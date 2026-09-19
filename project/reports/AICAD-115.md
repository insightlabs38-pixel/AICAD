# AICAD-115: Implement trimmed-surface semantic model and bounded construction

## Status

Done.

## Objective

Add a bounded (trimmed) surface: an underlying base surface plus an outer
trim loop and zero or more hole loops, with construction-time-validated
closure/planarity/orientation/domain evidence — never silent kernel repair.

## Base / resulting commit

Base: `3d2bbdd` (`origin/claude/aicad-stage5-dev`, `AICAD-114`).

## Architecture decision: a trim loop is exactly one already-closed curve, in parameter space

A trim loop (`TrimLoop`) reuses `crate::curve::AnalyticCurve` directly,
read with its point's `x`/`y` as `(u, v)` (`z` must be numerically zero) —
a conventional CAD-kernel "pcurve," but AICAD-owned data. **Composite
multi-segment loops** (e.g. a rounded-rectangle boundary built from several
curve pieces) **are explicitly out of scope** for this task: each loop must
already be one closed curve. This covers the common practical case (a
circular/elliptical hole or outer boundary — a full `Circle`/`Ellipse` is
always closed over one period) and any closed Bezier/B-spline loop, while
keeping the validation model tractable. This is a disclosed scope
limitation, not a corner cut — the same "constructible-only-to-reject"
pattern `AnalyticCurve::bspline`'s own `periodic` field already established
for an analogous "not yet, but a real future extension point" boundary.

## Architecture decision: one numerical core for closure, orientation, and point-membership

Every trim-loop question (is it closed? planar? what orientation? does it
contain a query point?) reduces to sampling the loop curve's own *exact*
analytic point+tangent (`AnalyticCurve::evaluate`) at a fixed grid and
running one of two classical closed-curve line integrals via composite
Simpson's rule (which uses the real tangent, not a finite difference, so it
converges far faster than a plain polygon/shoelace approximation at the
same sample count):

- **Signed area** (Green's theorem, `(1/2) oint (u dv - v du)`) — its sign
  gives orientation (positive = counter-clockwise = outer boundary,
  negative = clockwise = hole), and its magnitude the closed-form-checkable
  enclosed area (proven against `pi*r^2` for a circular loop to `1e-6`
  relative accuracy in a unit test).
- **Winding number** (`(1/(2*pi)) oint d(theta)` around a query point) —
  the standard robust point-in-region test for a loop with curved, not
  merely polygonal, edges; `|winding| > 0.5` distinguishes inside from
  outside without needing an exact `0`/`1` floating-point result.

One shared `sample_uv_curve`/`simpson_integrate` pair backs both, so the
orientation this module computes at construction and the point-membership
test `AnalyticSurface::evaluate` uses at every query are provably
consistent with each other (same curve, same samples, same quadrature).

## What changed

### `crates/cad-geometry-api/src/surface.rs`

- `AnalyticSurface::Trimmed { base: Box<AnalyticSurface>, outer: TrimLoop,
  holes: Vec<TrimLoop> }` — fields private (unlike every other variant):
  only [`AnalyticSurface::trim`] constructs one, since `outer`/`holes`
  carry construction-time-validated orientation a struct-literal caller
  could otherwise silently invalidate.
- `Orientation` (`CounterClockwise`/`Clockwise`), `TrimError`
  (`UnsupportedCurveFamily`/`NotClosed`/`NonPlanarLoop`/`DegenerateLoop`),
  `TrimLoop` (validated by `TrimLoop::new(curve, ConstructionTolerance)` —
  closure, planarity, and area/orientation, in that order), and
  `SurfaceTrimError` (`OuterMustBeCounterClockwise`/`HoleMustBeClockwise`/
  `LoopOutsideBaseDomain`) for `AnalyticSurface::trim`'s own combination
  checks.
- `AnalyticSurface::trim(base, outer, holes)` — rejects a wrongly-oriented
  outer/hole, or any loop with a sampled `(u, v)` point where
  `base.evaluate` fails (checked directly against `base`, not merely
  `base`'s own nominal parameter bounds).
- `AnalyticSurface::evaluate`'s new `Trimmed` arm: `QueryFailure::
  OutOfDomain` if `(u, v)` is outside `outer` or inside any hole,
  otherwise delegates to `base.evaluate(u, v)`.
- `AnalyticSurface::anchor`'s new `Trimmed` arm delegates to `base.anchor()`.
- 12 new unit tests: orientation from a plain circle vs. its own
  `normal`-reversed counterpart, rejection of an unclosed `Arc`, an
  unsupported `Line`, and a non-planar (tilted-normal) circle, the
  closed-form-area cross-check, a plane trimmed by an outer circle plus an
  off-center hole evaluated at inside/outside/hole/near-boundary points,
  wrong-hole-orientation rejection, a loop landing outside a cone's own
  `v >= 0` domain, and anchor delegation.

### `crates/cad-hir/src/builtins.rs`

- `trim_surface(base: Surface, outer: Curve, holes: List<Curve>,
  tolerance: Length) -> Surface` (`BuiltinCategory::Value`). `ALL`/
  `catalogue` extended (39 -> 40 entries).

### `crates/cad-runtime`

- `RuntimeError::InvalidTrimLoop`/`SurfaceTrimFailed`/
  `InvalidToleranceMagnitude` (`error.rs`, codes `RUNTIME-E138`/`E139`/
  `E140`) — the last wraps `cad_units::ToleranceError` directly (mirrors
  `DimensionalArithmetic`'s own "reuse, don't re-derive" pattern) for a
  non-finite/non-positive `tolerance` argument, which `TrimError` has no
  variant for (using `TrimError::DegenerateLoop` as a placeholder was
  considered and rejected as a mislabel during this task's own review).
- `dispatch_surface_builtin`'s new `TrimSurface` arm (`interp.rs`):
  extracts `outer`/`holes` as `AnalyticCurve`s, builds each `TrimLoop`
  (surfacing `InvalidTrimLoop`/`InvalidToleranceMagnitude`), then calls
  `AnalyticSurface::trim` (surfacing `SurfaceTrimFailed`).
- 4 new end-to-end interpreter tests, including a hole excluding interior
  points while accepting near-boundary ones.

## Regression found and fixed: a test-helper name collision

`geom_id_path_resolves_the_producing_call_for_a_value_built_through_a_helper`
started failing once `trim_surface`'s first parameter was named `base`:
`cad_hir::lower::Lowerer::seed_builtins` seeds every catalogue parameter as
its own global-scope binding, and the test helper `binding_named` searched
`lowered.bindings` by name only — so it silently returned the *builtin
parameter's* `base` binding (never assigned a value) instead of the test
program's own top-level `let base = ...;`. Root-caused and fixed at the
helper itself (excludes `BindingKind::Param`), not by renaming around the
collision — the same latent fragility would otherwise resurface for any
future catalogue parameter name matching an existing test's own binding
name.

## Verification

```
cargo test -p cad-geometry-api --lib surface::   # 44 passed (unit); 133 passed (whole crate)
cargo fmt --all -- --check                       # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings  # clean
cargo test -p cad-geometry-api -p cad-geometry-runtime -p cad-kernel-api \
  -p cad-occt-bridge                             # all passed, incl. native OCCT tests
cargo test --workspace                           # all passed, 0 failed
```

## Limitations / follow-up

- Composite multi-segment trim loops are not supported (see above) —
  future work, extensible without breaking this API (`TrimLoop` already
  hides its own representation).
- `TrimLoop::contains` re-samples its curve on every call (no caching) —
  a disclosed performance limitation; point-in-region membership is not
  yet a hot path anywhere in Stage 5.
- Self-intersecting loops are not detected — only closure/planarity/
  orientation/base-domain are validated. A genuinely self-intersecting
  loop's winding-number behavior is well-defined but not specifically
  tested; deferred to the `AICAD-127`-`129` adversarial campaign.
- `ConstructionTolerance`'s canonical (`Length`) magnitude is reused as a
  dimensionless `(u, v)`-space epsilon (disclosed in `TrimLoop::new`'s own
  doc comment) since `u`/`v` carry mixed dimensions per surface family and
  Stage 5 has no separate parameter-space tolerance domain.
- No active example added yet — deferred to Checkpoint B, per `AICAD-113`/
  `114`'s own precedent.

## Next

Per the fixed batch order, `AICAD-116` (surface evaluation, derivatives,
normals, and offsets) — the final task in batch `S5-04`.
