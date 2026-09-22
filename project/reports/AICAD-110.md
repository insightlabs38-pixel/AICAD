# AICAD-110: Implement Bezier, B-spline, and NURBS curve families

## Status

Done.

## Objective

Extend `cad_geometry_api::curve::AnalyticCurve` (`AICAD-108`/`109`) with
Bezier and B-spline/NURBS curve families, with explicit degree/knot/
multiplicity/control-point/weight/parameter-domain semantics, deterministic
validation, and exact evaluation — wired through the real `.aicad` source
path exactly like `AICAD-109`'s analytic families.

## Base / resulting commit

Base: `d7403bd` (this branch, `AICAD-109`).

## Architecture decision: widened `CheckedType::List<T>` (typeck.rs)

Bezier/B-spline control points fundamentally need a `List<Point3>`
argument (`docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`'s own
`bezier_curve`/`bspline_curve` signatures use exactly this shape). The
type checker's `CheckedType::List(HirType)` was restricted to
scalar/dimensional (`Value`) elements only — a documented `AICAD-056`/D16
"Stage-2 collection foundation" scope limit, not a permanent one (D16's
own decision text: "the minimum coherent... model," "does not need to
implement the entire future collection library" — explicitly not framed
as final). Widened `CheckedType::List` to `Box<CheckedType>` (any element
type), removing the `UNSUPPORTED_COLLECTION_ELEMENT_TYPE` restriction for
`List` specifically (kept unchanged for `Range`, which has no Stage-5
struct-element need). Contained entirely to `crates/cad-hir/src/typeck.rs`
— confirmed by grep that `CheckedType::List`/`Range` are referenced
nowhere else in the workspace. Every existing `List<Length>`/`List<Int>`
test/program is unaffected (same `types_compatible` outcome, generalized
from `value_types_compatible` to full recursive `types_compatible`); two
new tests prove the positive (`List<Point3>` type-checks) and negative
(mixing a struct and a `Length` element is still rejected) cases. This is
a purely internal type-checker representation change — no new grammar, no
change to `[e1, e2, ...]` list-literal syntax.

Judged as evidence-based generalization within an already-approved
mechanism (D16 authorized `List<T>`; only its element-*kind* coverage
narrows/widens), not a new architecture decision requiring escalation — it
does not touch functional/value semantics, unit semantics, D5/D19
comparison, tolerance domains, kernel boundaries, the safe/raw distinction,
or reference resolution. Recorded here prominently per `AGENTS.md`'s
"record the precise blocker/decision" convention in case the owner wants
to review it independently.

## What changed

### `crates/cad-hir/src/typeck.rs`

`CheckedType::List(HirType)` -> `CheckedType::List(Box<CheckedType>)` (see
above). Updated `types_compatible`, `describe`, `resolve_type_ref`'s
`List` arm, `check_list_literal`, and `check_iterable_element_type`
accordingly; `check_range_expr`/`Range` unchanged. 3 existing tests
updated for the new literal shape; 2 new tests added.

### `crates/cad-geometry-api/src/curve.rs`

- `AnalyticCurve::Bezier { control_points: Vec<Point3>, weights:
  Option<Vec<f64>> }` and `::BSpline { degree, control_points, knots,
  multiplicities, weights, periodic }` — new variants (this enum is no
  longer `Copy`, only the largest variant's `Vec` fields forced that; every
  pre-existing call site already worked unmodified with `Clone`).
- Validated constructors `AnalyticCurve::bezier`/`bspline`, each
  documented with its exact rejection list (too few control points,
  mismatched/invalid weights, mismatched knot/multiplicity arrays,
  non-finite/non-increasing knots, an invalid multiplicity — 1..=degree
  interior, 1..=degree+1 at an end knot — an expanded-knot/control-point
  count mismatch, and `periodic: true`).
- One shared exact evaluation core (`nurbs_evaluate`, `de_boor`,
  `derivative_control_points`, `find_span`, homogeneous-coordinate
  helpers): standard de Boor's algorithm on homogeneous control points
  (correct for both rational and non-rational curves), combined with the
  analytic B-spline hodograph derivative formula via the quotient rule for
  the rational case — an *exact* closed-form tangent, not a finite-
  difference approximation. A Bezier curve is evaluated as the
  mathematically equivalent clamped B-spline with no interior knots
  (`clamped_bezier_knots`) through this same core, not a separate
  implementation. Every helper defends against out-of-bounds indexing via
  one invariant check (`knot_vector.len() == control_points.len() + degree
  + 1`) before any indexing happens, matching this module's "defensive,
  not authoritative" convention for a directly struct-literal-constructed
  invalid curve.
- New `QueryFailure`-consistent domain: `u` outside `[knot_vector[degree],
  knot_vector[n]]` is `OutOfDomain`; `periodic: true` reaching `evaluate`
  (only via direct construction) is `Unsupported`; a degenerate/invalid
  shape is `Degenerate`.
- 20 new unit tests: every validation-rejection branch, a linear Bezier
  reducing to a line, a quadratic Bezier matching the textbook formula,
  Bezier endpoint interpolation, a **rational quadratic Bezier reproducing
  an exact 90-degree unit-circle arc** (the standard NURBS-textbook
  example — independent reference check, not merely internal self-
  consistency: `x^2+y^2==1` and the known 45-degree point both verified to
  `1e-9`), a degree-1 B-spline passing through every control point at its
  own knot, out-of-domain rejection, and a cubic B-spline's analytic
  tangent cross-checked against an independent central-finite-difference
  numerical derivative (agrees to `1e-4`, a genuinely different code path
  from the one being tested).

### `crates/cad-hir/src/builtins.rs`

New `BuiltinFnId::BezierCurve`/`BSplineCurve` (`BuiltinCategory::Value`,
`AICAD-109`'s category):
`bezier_curve(control_points: List<Point3>, weights: List<Float>) ->
Curve`, `bspline_curve(degree: Int, control_points: List<Point3>, knots:
List<Float>, multiplicities: List<Int>, weights: List<Float>, periodic:
Bool) -> Curve`. `weights: List<Float>` (not `List<Float>?`) — the closed
`RuntimeBuiltin` catalogue (`BuiltinFnSpec`) has no optional-parameter
mechanism (no existing entry has ever needed one); an **empty** list means
"non-rational," reusing a mechanism the catalogue already fully supports
rather than inventing one. Documented explicitly on `BuiltinFnId::
BezierCurve`'s own doc comment as a deliberate narrowing from
`docs/plan`'s literal `?`-suffixed spelling.

### `crates/cad-runtime`

`Interpreter::dispatch_curve_builtin` gained `point_list`/`float_list`/
`optional_weights`/`usize_list`/`usize_value`/`bool_value` argument-
extraction closures (each `List<T>` element converted through the same
`crate::spatial`/`Value::Number` conversion any scalar argument already
uses) and two new match arms. No new `RuntimeError` variants needed —
`InvalidCurveConstruction`/`BuiltinArgumentShape` already cover every new
failure mode. 5 new source-level tests (`compiled(...)` +
`Interpreter::call_by_name`) proving the real `.aicad` -> HIR -> runtime
path: the quadratic Bezier and rational-arc exact values reproduced
through real source syntax (`[Point3(...), ...]` list literals), a
degree-1 B-spline control-point interpolation, and two construction-
rejection cases.

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

New tests: `cad-hir::typeck` (+2), `cad-geometry-api::curve` (+20),
`cad-runtime::interp` (+5).

## Limitations / follow-up

- `periodic: true` (closed/wrapping B-splines) is rejected, not
  implemented — genuinely different control-point/knot relationship and
  evaluation from the clamped/open case, a documented scope limitation
  (`CurveConstructionError::UnsupportedPeriodic`'s own doc comment).
- `weights: List<Float>?` (docs/plan's own optional-parameter spelling) is
  represented as a mandatory `List<Float>` where empty means non-rational
  — the catalogue has no optional-parameter mechanism to spell the literal
  `?` shape; this is the same pattern every other optional-in-the-plan
  parameter in this catalogue already uses (see `plate`'s own precedent).
- `trim_curve`/`offset_curve`/`interpolate_curve` (`docs/plan`'s remaining
  curve-operation entries) are explicitly `AICAD-111`'s own scope
  ("curve operations"), not this task's.
- `CheckedType::List` widening (above) is a real, if contained and
  low-risk, type-checker architecture change — flagged prominently for
  independent owner review even though it was not treated as escalation-
  blocking.

## Next dependency

`AICAD-111` (Batch S5-03) depends on `AICAD-109`, `AICAD-110`, and
`AICAD-106`.
