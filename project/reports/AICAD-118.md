# AICAD-118: Checkpoint B — surfaces and multi-solution geometry-query gate

## Status

Done. Batch `S5-05` complete.

## Objective

Prove the Stage-5 surfaces (`AICAD-113`-`116`) and multi-solution
geometric-query (`AICAD-117`) surface through the real production path
(`ParametricBuildSession`), with a dedicated ambiguity/cardinality campaign
and at least one new representative example, before topology work
(`AICAD-119`+) proceeds.

## Base / resulting commit

Base: this branch's own `AICAD-117` commit.

## Production-path verification

New `crates/cad-cli/tests/stage5_queries_checkpoint.rs` (mirrors
`stage5_curves_checkpoint.rs`'s own `AICAD-112` precedent): a real
`.aicad` program, parsed/lowered/type-checked/interpreted through
`ParametricBuildSession`, that constructs a plane and a sphere, intersects
them (`intersect_surfaces`), evaluates the resulting circle
(`evaluate_curve`), and separately projects a point onto the sphere
(`project_point_to_surface`) — draws on curves, surfaces, and queries
together, per this checkpoint's own acceptance line.

- `plane_through_a_spheres_center_gives_back_the_exact_sphere_radius`:
  independent check — a plane through a sphere's own center always cuts a
  great circle of exactly the sphere's own radius (5 m in, 5 m out).
- `projecting_a_point_outside_the_sphere_finds_the_exact_clearance`:
  independent check — probe at x = 13 m against a radius-5 m sphere
  centered at the origin gives clearance = 13 - 5 = 8 m exactly.
- `surface_surface_intersection_and_projection_never_touch_the_kernel`:
  every binding this program produces (`Surface`/`Curve`/`List`/`Length`
  values) has `shape_for_binding(...) == None` — direct evidence that
  `AICAD-117`'s own module doc comment claim ("no kernel call, no
  `GeometryGraph` node") holds through the real production path, not only
  at the Rust unit-test level.

## Ambiguity/cardinality campaign

`AICAD-117`'s own 41 `cad-geometry-api::query` unit tests plus 20 new
`cad-geometry-api::surface::project_point` tests already are this
campaign — every query family's own report covers: a unique solution
(crossing lines/planes, a line through a sphere), zero solutions (parallel
lines/planes, a distant sphere), multiple solutions (two overlapping
circles, a line through a sphere at two points), tangency (a tangent
plane/sphere), coincidence/overlap (identical lines/circles/planes/
spheres), and an out-of-scope family combination (`Cylinder`-`Cylinder`,
`Trimmed` on either side) — each with its own dedicated, independently-
computed expected value, never merely "it returned something." No test
found a case where a multi-solution query silently collapsed onto one
kernel-selected representative — every genuinely ambiguous/coincident
configuration this batch supports is a `QueryFailure::Degenerate` or an
explicit multi-element `Solutions` list, never a single arbitrary pick.

## Examples

Two new ACTIVE examples (registered in `crates/cad-cli/tests/
active_examples.rs`, `examples/README.md`):

- `examples/surfaces/surface_query_basics.aicad` (teaching): a plane
  through a sphere's own center (`intersect_surfaces`), and a point
  projected onto the sphere (`project_point_to_surface`).
- `examples/surfaces/pipe_clearance_check.aicad` (realistic): a cylindrical
  pipe surface, a parallel support-rail curve's real minimum clearance
  distance to the pipe's own curved surface (`distance_curve_surface` — not
  merely axis-to-axis distance), and a drainage line's exact crossing point
  through the pipe's own end-cap plane (`intersect_curve_surface`).

Both build cleanly through `every_active_example_builds_cleanly`.

## Verification

```
cargo fmt --all -- --check                                                                # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings                      # clean
cargo test --workspace                                                                    # 0 failed, every crate
cmake -S native/occt_bridge -B native/occt_bridge/build
cmake --build native/occt_bridge/build -j$(nproc)
ctest --test-dir native/occt_bridge/build --output-on-failure                              # 18/18 passed
cargo test -p cad-cli --test stage4_resolver_execution -- --test-threads=1                 # 11/11, unchanged
cargo test -p cad-cli --test active_examples                                                # 3/3, includes 2 new examples
cargo test -p cad-cli --test stage5_queries_checkpoint                                      # 3/3, new this task
python3 scripts/ci/semantic_ref_harness.py validate                                         # ok
python3 scripts/ci/semantic_ref_harness.py self-test                                        # ok
```

`cad-query`'s own suite (98 tests) is unaffected; Stage-4 resolver
production-path regressions (`case01`-`case12`) are unchanged.

## No architecture boundary bypassed

- Every `AICAD-117` builtin is an ordinary `BuiltinCategory::Value`
  `RuntimeBuiltin` — same catalogue/dispatch mechanism as every prior
  Stage-5 curve/surface builtin, no new binding kind (`D18`/`DL-15`
  unweakened).
- `Curve`/`Surface` remain the same two single opaque nominal types; no new
  kernel-specific type reaches source/HIR (`D6` unweakened).
- No query in this batch pushes a `GeometryGraph`/`GeometryQuery` node or
  touches `OcctContext` — proven directly by the new checkpoint test above,
  not merely asserted.
- `QueryFailure`/`QueryOutcome` (`AICAD-108`) were instantiated, not
  extended or weakened — no new failure variant was needed.
- Tolerance stays in `ConstructionTolerance` (`DL-26` domain 2) throughout;
  no cross-domain reuse, no invented global epsilon.

## Known limitations carried into Stage-5 topology work

See `AICAD-117.md`'s own "Limitations / follow-up" for the full list
(surface/surface intersection's structural scope limit to `Plane`-`Plane`/
`Plane`-`Sphere`/`Sphere`-`Sphere`; `Line`-vs-`Cone` non-intersecting
distance; every query touching `Trimmed`). None of these block `AICAD-119`
(general topology construction), which consumes curves/surfaces as
construction inputs, not as a caller of these specific intersection/
distance queries.

## S5-05 batch complete

`AICAD-117`-`118` are done. Batch `S5-05` (geometric queries + Checkpoint B)
is complete. Per the fixed batch order, the next invocation begins `S5-06`
(`AICAD-119`-`121`: topology construction/healing/inspection).
