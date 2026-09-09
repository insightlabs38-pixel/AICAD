# AICAD-030 — Implement bounds/length/area/volume/center-of-mass queries

## Objective
Implement "bounds/length/area/volume/center-of-mass queries" per
`project/TASKS.yaml` (AICAD-030). The second task in Batch 1D. Volume
(`aicad_occt_shape_volume`), area (`aicad_occt_shape_area`), and bounds
(`aicad_occt_shape_bounding_box`) already exist (added incidentally as
"query helpers used to prove [earlier] operations produced a real, valid
B-rep" during Batch 1A/1B) — this task adds the two still missing:
length and center of mass.

## Dependencies checked
AICAD-029 (topology exploration) — complete, commit `dd823dd`. This
task's adversarial tests reuse AICAD-029's `shape_get_vertex` to obtain a
bare Vertex handle (no edge/face/solid content), which did not exist
before AICAD-029.

## What was done

1. **`native/occt_bridge/include/aicad_occt_bridge.h`**: added
   `aicad_occt_shape_length(context, handle, *out_length)` and
   `aicad_occt_shape_center_of_mass(context, handle, out_center[3])`.
2. **`native/occt_bridge/src/aicad_occt_bridge.cpp`**:
   - `shape_length` calls `BRepGProp::LinearProperties(*shape, props,
     Standard_True)` (`SkipShared=true`) and returns `props.Mass()`. See
     "Key finding" below for why `SkipShared` is required.
   - `shape_center_of_mass` explicitly dispatches on the shape's own
     highest-dimensional content — `BRepGProp::VolumeProperties` if it
     contains any `TopAbs_SOLID` (via `TopExp::MapShapes` presence
     check), else `BRepGProp::SurfaceProperties` if it contains any
     `TopAbs_FACE`, else `BRepGProp::LinearProperties(...,
     Standard_True)` if it contains any `TopAbs_EDGE`, else fails with
     `AICAD_OCCT_ERR_OPERATION_FAILED`. See "Implementation decisions"
     below for why this dispatch is explicit rather than delegated to
     `VolumeProperties`' own no-solid behavior.
   - Both reuse the existing `context->shapes.Lookup` pattern (no kind
     restriction — matches `shape_volume`/`shape_area`/`shape_bounding_box`'s
     own precedent of accepting any shape kind) and the standard
     `Standard_Failure` → `OPERATION_FAILED` / anything else → `INTERNAL`
     exception mapping.
3. **`native/occt_bridge/CMakeLists.txt`**: no new OCCT module needed
   (`BRepGProp` is already linked, used by `shape_volume`/`shape_area`
   since AICAD-016/023); registered `measurement_test`.
4. **`native/occt_bridge/tests/measurement_test.cpp`** (new): a 3-4-5
   line edge has length exactly 5.0; a box's total edge length matches
   `4*(dx+dy+dz)` (the unique-edge total, not double-counted); a
   `create_box`'s volume-weighted center of mass is its geometric center
   `(dx/2, dy/2, dz/2)`; a planar square face's area-weighted center of
   mass is its geometric center; a standalone line edge's
   length-weighted center of mass is its midpoint; `center_of_mass` on a
   bare Vertex (via AICAD-029's `get_vertex`) fails cleanly with
   `OPERATION_FAILED` rather than returning a meaningless `(0,0,0)`;
   `length` on a bare Vertex is exactly `0.0`, not an error (a length
   measurement over zero edges is legitimately zero, unlike
   center-of-mass, which has no meaningful zero-content answer).
5. **`crates/cad-occt-bridge/src/ffi.rs`**: raw
   `aicad_occt_shape_length`/`aicad_occt_shape_center_of_mass`
   declarations.
6. **`crates/cad-occt-bridge/src/lib.rs`**: `Shape::length(&self) ->
   KernelResult<f64>`, `Shape::center_of_mass(&self) ->
   KernelResult<Point3>`. 6 new Rust-level tests mirroring the native
   ones.

## Key finding: `BRepGProp::LinearProperties` double-counts shared edges by default

The first length implementation (without `SkipShared`) reported a box's
total edge length as `72.0` instead of the expected `36.0 =
4*(2+3+4)`, exactly double. `BRepGProp::LinearProperties`'s default
`SkipShared=false` counts an edge once per adjacent face occurrence in
the shape's own structure — the same double-counting
`aicad_occt_shape_edge_count`'s own doc comment already identified for a
raw `TopExp_Explorer` traversal (24 vs. 12 for a box, AICAD-027). Passing
`SkipShared=true` (found via OCCT's own `BRepGProp::LinearProperties`
signature, not guessed) fixes this and matches
`aicad_occt_shape_edge_count`'s established "unique edges" semantics.
This was caught by the test itself (an assertion against the analytic
`4*(dx+dy+dz)` formula, not a "does it run" check), then root-caused and
fixed, not weakened.

## Implementation decisions

- **`center_of_mass` explicitly dispatches on shape content
  (Solid > Face > Edge) rather than always calling `VolumeProperties`.**
  Empirically, `BRepGProp::VolumeProperties` on a shape with no Solid
  returns a zero-mass result with an origin-centred (or otherwise
  meaningless) `CentreOfMass()`, not a failure — silently returning
  `(0,0,0)` for e.g. a bare face's center of mass would be a wrong answer
  presented as a valid one, not an honest capability boundary. The
  explicit dispatch (checking `TopExp::MapShapes(..., TopAbs_SOLID,
  ...).Extent() > 0`, etc.) makes the "what am I actually measuring"
  choice visible and testable, matching AGENTS.md's evidence rule.
- **No caller-selectable measurement kind.** A caller cannot force
  "always area-weighted, even if a solid is present" — the dispatch is
  automatic based on content. This is the minimal-surface choice
  sufficient for this task's own evidence (a solid's mass properties
  come from its volume in ordinary engineering use); a caller-selectable
  override is not exposed unless a later task's own evidence needs it.
- **`shape_length` always uses `SkipShared=true`** (unique-edge length),
  not exposed as a caller option — this is the one semantically
  consistent choice matching `shape_edge_count`'s own established
  "unique edges" contract; an "edge-occurrence-weighted" length (the
  `SkipShared=false` behavior) was not identified as meaningful for any
  Stage-1 consumer.

## Files changed
- Edited: `native/occt_bridge/include/aicad_occt_bridge.h`,
  `native/occt_bridge/src/aicad_occt_bridge.cpp`,
  `native/occt_bridge/CMakeLists.txt`, `crates/cad-occt-bridge/src/ffi.rs`,
  `crates/cad-occt-bridge/src/lib.rs`.
- Added: `native/occt_bridge/tests/measurement_test.cpp`.

## Verification (exact commands/results)
```
$ cmake --build native/occt_bridge/build -j$(nproc)
[100%] Built target measurement_test   (plus all 13 pre-existing targets)

$ ctest --test-dir native/occt_bridge/build --output-on-failure
 1/14 occt_probe .................. Passed
 ...
13/14 topology_test ............... Passed
14/14 measurement_test ............ Passed
100% tests passed, 0 tests failed out of 14

$ ./native/occt_bridge/build/measurement_test
... 8 PASS lines, 0 FAIL ...
measurement_test: all checks PASSED

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, no warnings)

$ cargo build --workspace --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.81s

$ cargo test -p cad-occt-bridge
running 69 tests ... test result: ok. 69 passed; 0 failed

$ cargo test --workspace
(every crate) test result: ok, 0 failed
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1
(`rust-toolchain.toml`), CMake 3.28.3, OCCT 7.6.3
(`libocct-*-dev` 7.6.3+dfsg1-7.1build1) — unchanged from Batch 1A/1B/1C/
AICAD-029.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — **PASS**.
- Task-specific: native `ctest` (14/14) and `cargo test -p
  cad-occt-bridge` (69/69, 6 new) both pass, as shown above.

## Regressions added
None. All 13 pre-existing native test executables and all 63
pre-existing Rust tests continue to pass unchanged.

## Limitations (honest capability boundaries)
- **`center_of_mass`'s dispatch was only tested for each of the three
  branches individually** (a solid box, a bare planar face, a standalone
  edge) — not for a shape that legitimately mixes dimensionalities in a
  way where the "highest present" rule might be debatable (e.g. a
  Compound containing both a Solid and a disconnected free Edge). The
  implemented rule (Solid wins if present at all) is the physically
  sensible default but was not adversarially probed against such a mixed
  compound.
- **No moment-of-inertia or other second-order mass property** is
  exposed — `GProp_GProps` computes these too, but nothing in Stage-1's
  authorized scope needs them; only `Mass()`/`CentreOfMass()` are
  surfaced, matching the capability-driven minimal-surface rule.
- **`shape_length`'s `SkipShared=true` choice was only verified against a
  box** (a closed manifold solid where every edge is shared by exactly 2
  faces). A shape with a mix of shared and free edges (e.g. a solid with
  an attached free wire) was not tested — `SkipShared=true`'s exact
  behavior in that mixed case was not independently confirmed here,
  though OCCT's own documented semantics ("skip edges shared between two
  or more faces") imply free edges are unaffected either way.

None of these limitations required forcing an artificial success or
weakening a test to hide a failure — every claim in this report and its
tests is backed by an actual passing run, and the one bug caught during
testing (the double-counted length) was root-caused and fixed via OCCT's
own documented API, not worked around.
