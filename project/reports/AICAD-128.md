# AICAD-128: numerical, robustness, determinism, and resource adversarial campaign

## Status

Done. Second task of batch `S5-09`.

## Objective

Per `docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md` §5/§9/§10 and this
task's own acceptance list: exercise the `AICAD-127` corpus plus targeted
generated adversarial cases against near-degenerate, multi-scale,
periodic, tangent, self-intersecting, invalid-input, and resource-heavy
geometry; classify every failure; preserve any real bug as a minimized
regression; collect performance evidence where useful; never weaken a
tolerance to make a case pass.

## Base / resulting commit

Base: `37ca61c` (`origin/claude/aicad-stage5-dev`, `AICAD-127`).

## What changed

- `crates/cad-cli/tests/stage5_adversarial_campaign.rs` — new: 8
  adversarial cases against the real production path
  (`ParametricBuildSession`), targeting the categories this task's own
  acceptance list names. No corpus files added — every case is a
  generated `.aicad` source string (matching `stage5_curves_checkpoint.rs`'s
  own established inline-source convention), since none of these needed
  to be frozen as a reusable named fixture the way `AICAD-127`'s own
  positive-path corpus does.

## Cases and classification

| Case | Category (acceptance list) | Classification | Outcome |
| --- | --- | --- | --- |
| `near_degenerate_and_multi_scale_circular_faces_never_panic` | near-degenerate + multi-scale | numerical-robustness | **No defect found.** A 1nm-radius and a 1km-radius circular face each construct, report `is_valid=true`, and match the exact `pi*r^2` area to ~1e-16 relative error (machine-epsilon-level) — OCCT's own planar-face mass-property integration has no observed precision floor at either extreme scale tested. |
| `externally_tangent_spheres_report_explicit_degenerate_intersection` | tangent | explicit ambiguity/no-solution | **Discovered behavior, recorded precisely**: `intersect_surfaces` on two externally-tangent spheres does not return an empty `List` — it fails the *whole build* with `RuntimeError::GeometricQueryFailed` ("the input geometry is degenerate for this query"). Structured and explicit, never a panic/hang/silent-wrong curve, but a caller must handle it as a build error, not a zero-length list — worth knowing precisely, not merely "it didn't crash." |
| `periodic_circle_evaluation_wraps_consistently_past_one_period` | periodic | numerical-robustness | **No defect.** `evaluate_curve` on a `Circle` at `u`, `u+2*pi`, and `u-2*pi` all report the identical point to `1e-9` — confirms `AnalyticCurve`'s own "periodic curves accept any finite u" contract holds through the full `.aicad` production path, not just at the Rust-API level. |
| `sew_tolerance_boundary_behaves_as_a_real_threshold_not_silently` | numerical-robustness (sewing limit) | numerical-robustness | **No defect — real threshold confirmed.** Two adjacent squares sewn with a 0.001mm gap (inside a 0.01mm tolerance) merge to 7 edges/6 vertices (the shared edge coincides); the same construction with a 0.1mm gap (outside tolerance) stays at 8 edges/8 vertices (no merge). `sew`'s own tolerance parameter is proven to materially change behavior, not silently ignored. |
| `self_intersecting_curve_closest_point_reports_a_real_list_not_an_arbitrary_pick` | self-intersecting | explicit ambiguity | **No defect.** A figure-eight B-spline's closest point to the origin returns one real, inspectable `List` result (cardinality 1 for this specific geometry/probe — not every self-intersecting curve necessarily has multiple equidistant closest points to every probe). The required evidence (a real `List`, not an arbitrary scalar) holds. |
| `many_sided_regular_polygon_constructs_quickly_with_exact_area` | resource-heavy | resource-limit / performance | **No defect.** A 64-edge polygon constructs in ~30ms and matches the exact regular-polygon area formula to `1e-9`. No pathological scaling observed at this size. |
| `repeated_independent_builds_of_the_same_fixture_are_deterministic` | determinism (D5/`DL-12`) | reference/determinism | **No defect.** Two independent `OcctContext`s building `AICAD-127`'s own `02_twisted_variable_section_solid` fixture report bit-identical (`f64::to_bits` equal) areas and identical validity — Level 1/2 determinism (`DL-12`) holds for this fixture. |
| `malformed_bspline_knot_vector_is_rejected_explicitly_not_panicking` | invalid input | invalid input | **No defect.** A `knots`/`multiplicities` length mismatch is rejected at construction time with a clear diagnostic ("knots and multiplicities must have the same length"), never a panic or a silently-accepted malformed curve — `AnalyticCurve::bspline`'s own validation (`crates/cad-geometry-api/src/curve.rs`) is proven reachable and effective through the full `.aicad` path. |

## Real findings

No crash, hang, UB, stale dereference, arbitrary silent selection, or
silent-wrong result was found in any of the eight cases — every adversarial
input produced either a correct positive result (with independently
verified numerics) or an explicit, structured failure. The one genuinely
noteworthy discovery (tangent-sphere intersection failing the whole build
rather than returning an empty list) is not a defect — `RuntimeError::
GeometricQueryFailed` is exactly `AICAD-117`'s own documented explicit-
failure contract for a geometric query with no representable answer — but
it is recorded precisely above since "no exception, always an empty List"
would have been a reasonable but wrong assumption for downstream code to
make.

No tolerance was widened, no ambiguity was resolved arbitrarily, and no
test/gate was weakened to make any case pass — every assertion reflects
the actually-observed kernel behavior, re-derived by running the case
(not assumed in advance; see this file's own git history for the two
initial assertions this task itself had to correct once real behavior was
observed: the tangent-sphere case's expected outcome, and the sew-boundary
case's differentiating signal, both fixed to match reality rather than
forcing a different result).

## Performance/resource evidence

64-edge polygon construction: ~30ms (informal, single measurement, not a
formal benchmark harness) — recorded only because it materially informs
this task's own "resource-heavy" acceptance item; no new benchmark target
is invented beyond what this task requires.

## No architecture boundary bypassed

No new `RuntimeBuiltin`, type, or kernel-adapter surface was added. No
`AICAD-127` fixture was edited. `AICAD-127`'s own `02_twisted_variable_
section_solid/case.aicad` is reused read-only by the determinism case.

## Verification

```
cargo fmt --all -- --check                                              # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings    # clean
cargo test -p cad-cli --test stage5_adversarial_campaign                # 8/8 passed
cargo test --workspace                                                  # 1826 passed, 0 failed
```

(1818 after `AICAD-127` + 8 new adversarial-campaign tests.) The
determinism case above *is* the required "rerun deterministic/repeat-build
checks" evidence — it builds the same fixture twice independently within
the test itself.

## Limitations (honest, not hidden)

1. This campaign found no reproducible defect to minimize into a
   permanent regression — a genuine, disclosed outcome (not every
   adversarial campaign must find a bug), not a shortcut: each case was
   run against the real kernel and its actual observed behavior recorded,
   with two of the eight cases' own initial assumptions corrected once
   real behavior diverged from what seemed reasonable in advance.
2. `AICAD-127`'s own discovered capability gap (Bezier/B-spline curves and
   surfaces cannot become real kernel topology) means this campaign's
   topology-construction-level adversarial cases (sew boundary, tangent
   query, near-degenerate circle, many-sided polygon) necessarily use the
   analytic (`Circle`/`Arc`/`Line`; `Sphere`) family, not freeform
   geometry — the freeform layer's own adversarial surface is exercised
   only at the value level (self-intersecting curve, periodic evaluation).
3. Eight cases is a floor, not exhaustive coverage of every category this
   task's acceptance list names (multi-scale and near-degenerate were
   combined into one case rather than kept separate); `AICAD-129`/`130`
   may add more if the final gate's own re-audit finds a gap.

## Next dependency

`AICAD-129` (core-vs-library boundary, learnability/inspectability,
maintained-examples audit) depends on this task and `AICAD-127`
(satisfied).
