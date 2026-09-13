# AICAD-020 — Implement box and cylinder

## Objective
Implement `create_cylinder` in the native bridge/kernel-api/occt-bridge
stack (Stage-1 Batch 1B, first task), and extend `create_box`'s
adversarial coverage per the active scheduled-task brief's adversarial-case
list (zero/near-zero/very-small/very-large/mixed-scale dimensions,
infinity/NaN), per `project/TASKS.yaml` (AICAD-020) and
`docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s `box`/`cylinder` feature-catalog
entries (Stage-1 works at the raw Rust/native level per AICAD-034's
"directly through Rust/native API" framing, not the full typed language
surface those entries describe).

`create_box` itself already existed from AICAD-016 (needed there to test
the shape-handle table); this task is the first to give it a dedicated
adversarial test campaign, add `create_cylinder` alongside it, and add the
`shape_area`/`shape_bounding_box` query helpers both primitives' evidence
needs (AGENTS.md's evidence rule: bounding boxes/area, not just "OCCT
returned a shape").

## Dependencies checked
AICAD-019 (kernel context lifecycle) — complete; Batch 1A checkpoint
(`project/gates/STAGE1-A_KERNEL_BOUNDARY.md`) passed. Stage 0 approval is
recorded at `project/DECISION_LOG.md#DL-10`.

## What was done

1. **`native/occt_bridge/include/aicad_occt_bridge.h`**: added
   `aicad_occt_create_cylinder(context, radius, height, out_handle)`
   (capped cylinder, centered on the origin, axis along +Z — placement
   elsewhere is a separate future transform operation, not a second
   placement-aware constructor, per RFC-0002 §3's capability-driven
   minimal-surface rule) and two evidence query helpers,
   `aicad_occt_shape_area` and `aicad_occt_shape_bounding_box`.
2. **`native/occt_bridge/src/aicad_occt_bridge.cpp`**:
   - `aicad_occt_create_cylinder`: validates `radius > 0`, `height > 0`,
     and both finite (rejecting NaN via the `> 0` comparison and
     +infinity via an explicit `std::isfinite` check), then
     `BRepPrimAPI_MakeCylinder`.
   - **Bug found and fixed in `create_box` too**: its existing
     `!(dx > 0.0) || ...` check rejected NaN (every comparison with NaN is
     false) but *not* +infinity (`+infinity > 0.0` is true). Added the
     same explicit `std::isfinite` checks to `create_box` so both
     primitives reject infinity consistently — found by this task's own
     adversarial infinity test, not assumed.
   - `aicad_occt_shape_area`: `BRepGProp::SurfaceProperties`.
   - `aicad_occt_shape_bounding_box`: `Bnd_Box` + `BRepBndLib::Add`,
     rejecting a void box as `OPERATION_FAILED`.
3. **`native/occt_bridge/tests/box_cylinder_test.cpp`** (new): adversarial
   campaign for both primitives —
   - box: very small (1e-6), very large (1e9), mixed-scale (1000×1000×1e-6
     thin plate) dimensions, all still valid B-reps; +infinity and NaN
     rejection (infinity specifically added because of the bug above).
   - cylinder: analytic volume (`pi*r^2*h`) and surface area
     (`2*pi*r*h + 2*pi*r^2`, capped-cylinder formula) checked against
     `shape_volume`/`shape_area`; bounding box checked against
     `shape_bounding_box` (confirms the origin/+Z placement); zero/
     negative/NaN rejection; very small (1e-7)/very large (1e8)/
     mixed-scale (1e6 radius × 1e-6 height) dimensions.
4. **`native/occt_bridge/CMakeLists.txt`**: registered `box_cylinder_test`
   as a new CTest target/test.
5. **`crates/cad-occt-bridge/src/ffi.rs`**: raw `extern "C"` declarations
   for the three new native functions.
6. **`crates/cad-occt-bridge/src/lib.rs`**: `OcctContext::create_cylinder`
   (mirrors `create_box`'s pattern exactly), `Shape::area`,
   `Shape::bounding_box` (returning a new `BoundingBox { min: Point3, max:
   Point3 }` struct — the first use of `cad-kernel-api::Point3` in this
   crate), plus Rust-level tests for the cylinder happy path and
   dimension rejection.

## Implementation decisions (autonomous, per AGENTS.md's allowed scope)
- Cylinder placement is origin/+Z-only, deferring arbitrary placement to
  the (not-yet-implemented-at-this-task) transform operation rather than
  adding an axis parameter to `create_cylinder` itself — keeps each
  primitive constructor minimal and composes with the transform operation
  the next task in this batch adds, matching
  `docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s own `transform` feature
  design (`item`+`transform` → same-kind output) rather than baking
  placement into every primitive.
- `shape_area`/`shape_bounding_box` are introduced here (rather than
  deferred to AICAD-030, "bounds/length/area/volume/center-of-mass
  queries") because they are the only way to give cylinder placement and
  both primitives' adversarial-scale cases real analytic evidence now,
  matching this task's own evidence-rule obligation; AICAD-030 will still
  own the complete/final query surface (length, center-of-mass, etc. are
  not added here).

## Files changed
- Edited: `native/occt_bridge/include/aicad_occt_bridge.h`,
  `native/occt_bridge/src/aicad_occt_bridge.cpp`,
  `native/occt_bridge/CMakeLists.txt`, `crates/cad-occt-bridge/src/ffi.rs`,
  `crates/cad-occt-bridge/src/lib.rs`.
- Added: `native/occt_bridge/tests/box_cylinder_test.cpp`.

## Verification (exact commands/results)
```
$ rm -rf native/occt_bridge/build
$ cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3
... (module verification as in AICAD-015/016) ...
-- Configuring done

$ cmake --build native/occt_bridge/build
[100%] Built target box_cylinder_test

$ ctest --test-dir native/occt_bridge/build --output-on-failure
1/4 Test #1: occt_probe .......................   Passed
2/4 Test #2: abi_boundary_test ................   Passed
3/4 Test #3: lifecycle_test ...................   Passed
4/4 Test #4: box_cylinder_test ................   Passed
100% tests passed, 0 tests failed out of 4

$ cargo build -p cad-occt-bridge
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo test -p cad-occt-bridge
running 9 tests
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(exit 0, no warnings)
```

Environment (Stage-1 kernel policy #15): Ubuntu 24.04.4 LTS, x86_64,
GCC/G++ 13.3.0, Rust 1.98.1, CMake 3.28.3, OCCT 7.6.3
(`libocct-foundation-dev` 7.6.3+dfsg1-7.1build1) — same as AICAD-015..019.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**.
- Task-specific: native `ctest` (4/4) and `cargo test -p cad-occt-bridge`
  (9/9) both pass, as shown above.

## Limitations / follow-up
- `shape_area`/`shape_bounding_box` are evidence-tooling only at this
  point (not exposed as end-user geometry API), matching the header's own
  existing comment convention for `shape_is_valid`/`shape_volume`; the
  eventual end-user surface is `cad-geometry-api`'s job in a later stage.
- The infinity-rejection bug fixed in `create_box` here was never
  previously exercised by AICAD-016..019's own tests; this is exactly the
  kind of gap the adversarial-case campaign is meant to catch, and the fix
  is minimal (an added `isfinite` check, no behavior change for any
  previously-valid or previously-rejected input).
