# AICAD-112: Checkpoint A — curves, runtime/query foundations, abstraction provenance, and examples

## Status

Done. Checkpoint passed — no architecture conflict found. Batch `S5-03` complete.

## Objective

Re-verify, with direct evidence against the actual implementation (not
merely re-asserting prior claims), that `AICAD-109`-`111`'s curve surface
is real, kernel-neutral, and correctly integrated with the D21/D23/D24/D25
invariants Stage 5 depends on, before Stage-5 surface work continues into
surfaces (`S5-04`).

## Scope covered by this checkpoint

- `AICAD-107` (feature/provenance through abstraction) + `AICAD-109`-`111`
  (curves) — this batch's own full task set.
- Indirectly re-confirms `AICAD-105`/`106` (query/tolerance foundations),
  since curve work is the first real consumer of the `BuiltinCategory`
  extension mechanism `AICAD-105` established and the first real
  `ApproximationTolerance` consumer `AICAD-106` established.

## Vertical-slice production-path proof

New `crates/cad-cli/tests/stage5_curves_checkpoint.rs`, through
`ParametricBuildSession` (the same real production entry point
`stage5_kernel_backed_queries.rs` uses for kernel-backed queries — parse,
lower, type-check, interpret, no shortcut):

- `rational_bezier_arc_reproduces_the_exact_45_degree_unit_circle_point` —
  a rational quadratic Bezier built from real `.aicad` source
  (`bezier_curve`/`evaluate_curve`) reproduces the standard textbook
  45-degree unit-circle point *exactly* (`x = y = 1/sqrt(2)` to `1e-9`,
  `x^2 + y^2 == 1` to `1e-9`) — an independent, externally-checkable
  geometric identity, not merely "the code returned some point."
- `curve_construction_and_evaluation_never_touch_the_kernel` — for every
  binding in that same program, `ParametricBuildSession::
  shape_for_binding` returns `None`: no `OcctContext`/`Shape` call ever
  happened. Direct evidence for "no OCCT leakage," stronger than a design
  claim — curve construction/evaluation genuinely never reaches
  `cad-occt-bridge` at all, matching `cad_geometry_api::curve`'s own
  documented architecture.

## D21/D23/D24/D25 re-audit (`AGENTS.md` plan references `DL-23`/`DL-25`/
`DL-26`/`DL-27`)

**D21 (`DL-23`, closed catalogue).** `cad-hir::builtins`'s own
`catalogue_has_exactly_one_entry_per_builtin_fn_id`,
`every_catalogue_entry_has_a_category_consistent_with_its_return_type`,
`every_catalogue_name_is_unique`, and
`the_entire_builtin_catalogue_type_checks_against_an_otherwise_empty_program`
all still pass against the now-31-entry catalogue (13 new curve entries
across `AICAD-109`-`111`). The catalogue's own scaling mechanism worked
exactly as `DL-23` anticipated: a third `BuiltinCategory::Value` variant
(`AICAD-109`) was added without touching `Construction`/`Query`'s own
semantics, and every new entry is an ordinary closed catalogue row — no
open registration, no native callback, no compiler intrinsic.

**D23 (`DL-25`, tracked kernel-backed queries).** Re-confirmed as a
*boundary*, not weakened: `is_valid`/`volume`/`area` remain the only
kernel-backed (`BuiltinCategory::Query`) entries, still demand-
materialized through `KernelQueryExecutor`/query-budget accounting
(`AICAD-105`'s own tests, unchanged, still passing). Curve construction/
evaluation is deliberately **not** a D23 query — it is
`BuiltinCategory::Value` precisely because it needs no kernel result at
all (closed-form math). This checkpoint's own
`curve_construction_and_evaluation_never_touch_the_kernel` test is direct
evidence this boundary is real, not merely documented.

**D24 (`DL-26`, separated tolerance domains).** `AICAD-111`'s
`interpolate_curve` is the first real second consumer of a `DL-26` domain
beyond `AICAD-106`'s own original `classify_point` wiring: it takes
`cad_units::ApproximationTolerance` specifically (domain 3), with no
conversion path to/from `ConstructionTolerance` (domain 2) — enforced at
the type level (no `From`/`Into` impl exists between them, per `AICAD-106`'s
own design), so this is compile-time-verified, not merely convention.
`cad-units::tolerance`'s own distinctness tests (`TypeId`-level, positive/
negative magnitude rejection) are unchanged and still pass.

**D25 (`DL-27`, feature/provenance through abstraction).** New
`cad-runtime::interp` test,
`curve_construction_stays_invisible_to_the_feature_trace_alongside_real_geometry`:
a program mixing a real `Geometry`-producing call and `Curve`-typed
construction/evaluation, both reached through separate helper functions,
traces **only** the `Geometry` call (`interp.trace().len() == 1`) — curves
add no spurious `TraceEntry` and do not interfere with the real box's own
trace. This is the generic `is_geometry_type_ref` gate (`AICAD-107`)
applied without any curve-specific logic, confirmed by direct test rather
than by re-reading the gate's own source. Curve-typed *arguments* to an
existing traced builtin (e.g. an `Axis3`/`Point3` argument to `revolve`)
already flowed through the identical generic scalar-provenance path before
this task; nothing new was needed for `Curve` to do the same.
**Known forward-looking gap, not a defect**: no current builtin bridges a
`Curve` value into real topology (`make_edge`-shaped construction is later
Stage-5 scope, `S5-06`+), so there is no way yet to test "does a curve's
own provenance survive into topology" — that bridge, and its own D25
evidence, is that later task's job.

## Active example suite

Two new ACTIVE examples (`examples/README.md` maintenance policy — public
capability changes update affected examples in the same batch):

- `examples/curves/circle_curve_basics.aicad` — minimal teaching example:
  `circle_curve`/`evaluate_curve` at two parameters.
- `examples/curves/cable_routing_path.aicad` — realistic example: a
  90-degree bend represented as an exact rational-Bezier arc (the same
  construction real NURBS kernels use internally for a circular arc),
  straight lead-in/lead-out `line_curve` segments, and a
  `closest_point_on_curve` obstacle-clearance check — a genuine composed
  use of three different `AICAD-109`-`111` capabilities together.

Both registered in `crates/cad-cli/tests/active_examples.rs`'s `ACTIVE`
list; `every_active_example_builds_cleanly` (all 11 ACTIVE examples,
including these two) passes, as do the three reference-health fixtures
(unaffected, unchanged). No existing example was touched — nothing stale.

## Full verification

```
cargo fmt --all -- --check                                                                    # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings                          # clean
cargo test --workspace                                                                        # 83 test-result blocks, 0 failed
cargo test -p cad-cli --test active_examples                                                  # 3/3 (11 ACTIVE examples, 3 reference-health fixtures)
cargo test -p cad-cli --test stage5_curves_checkpoint                                          # 2/2
cargo test -p cad-runtime --lib -- curve_construction_stays_invisible                          # 1/1
cargo test -p cad-cli --test stage4_resolver_execution -- --test-threads=1                     # 11/11, unchanged
python3 scripts/ci/semantic_ref_harness.py validate                                            # ok
python3 scripts/ci/semantic_ref_harness.py self-test                                           # ok

# Native OCCT bridge (explicit checkpoint requirement)
cmake -S native/occt_bridge -B native/occt_bridge/build
cmake --build native/occt_bridge/build -j$(nproc)
ctest --test-dir native/occt_bridge/build --output-on-failure                                  # 18/18 passed
```

(83 test-result blocks includes the new `stage5_curves_checkpoint`
binary's own block, on top of the 82 already established through
`AICAD-111`.)

## Architecture note carried into this checkpoint for visibility

`AICAD-110` widened `cad_hir::typeck::CheckedType::List` from a plain
`HirType` (scalar/dimensional-only element) to `Box<CheckedType>` (any
element type), needed for `List<Point3>` control-point arguments —
judged an evidence-based generalization of an already-approved `D16`
mechanism, not requiring escalation, but flagged prominently in
`project/reports/AICAD-110.md`'s own "Architecture decision" section and
repeated here per this checkpoint's own "record... any architecture
conflict" requirement. Re-confirmed at this checkpoint: fully contained to
one file, zero behavior change for any pre-existing `List<Length>`/
`List<Int>` program, both positive (`List<Point3>` type-checks) and
negative (a struct/`Length` element mismatch is still rejected) cases have
direct tests, unchanged since `AICAD-110`. Not itself an architecture
conflict — recorded for independent owner review, per that report's own
framing, not as a blocker.

## Known limitations carried forward (not blocking this checkpoint)

- `AICAD-109`: Arc construction rejects `start_angle >= end_angle` (no
  wraparound-through-zero arcs).
- `AICAD-110`: periodic (closed/wrapping) B-splines are rejected, not
  implemented; `weights` is a mandatory `List<Float>` (empty = non-
  rational) rather than the plan's optional-parameter spelling — the
  catalogue has no optional-parameter mechanism.
- `AICAD-111`: `offset_curve` is exact only for Line/Circle/Arc (a
  well-known CAD limitation for the other families, not attempted
  approximately); `closest_point`'s numerical branch resolves a parameter
  only to roughly `sqrt(f64 epsilon)` near a flat minimum (an inherent
  numerical-method limit, not a defect); `interpolate_curve` is fixed at
  degree 3 with chord-length parametrization, no tangent/periodic option.
- No `make_edge`/topology-construction path from a `Curve` exists yet —
  curves remain pure values with no kernel/`GeometryGraph` participation
  until a later Stage-5 topology-construction task (`S5-06`+) bridges
  them, per each of `AICAD-109`-`111`'s own reports.

None of the above represents a substantive missing capability this
checkpoint's own acceptance criteria required; each is an explicit,
evidenced scope boundary recorded in its own originating task report.

## Stage-5 batch status

| Batch | Tasks | Status |
|---|---|---|
| S5-00 | AICAD-101..104, AICAD-104A | done |
| S5-01 | AICAD-105..106 | done |
| S5-02 | AICAD-107..108 | done |
| S5-03 | AICAD-109..112 | **done** |

Per the fixed batch order, the next invocation begins `S5-04`
(`AICAD-113..116`, surfaces).
