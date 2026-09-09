# AICAD-024 — Implement extrude and revolve

## Objective
Implement extrude and revolve (face → solid) per `project/TASKS.yaml`
(AICAD-024) and `docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s
`extrude`/`revolve` feature-catalog entries, at Stage-1's raw-Rust/native
fidelity. This is the last task in Batch 1B (constructive geometry,
AICAD-020..024) — it completes the pipeline
circle/rectangle wire → face → extrude/revolve → solid that AICAD-034's
Stage-1 bracket proof will build on.

## Dependencies checked
AICAD-023 (planar face from closed wire) — complete, commit `7ed6458`.
`extrude`/`revolve` both consume the `Face` handles `make_face_from_wire`
produces and the `LookupTyped`/`TryToDir`/`IsFinite3` helpers AICAD-022
introduced.

## What was done

1. **`native/occt_bridge/include/aicad_occt_bridge.h`**: added
   `aicad_occt_extrude(context, face_handle, direction[3], distance,
   out_handle)` (direction need not be unit-length; only its direction is
   used, magnitude is ignored) and `aicad_occt_revolve(context,
   face_handle, axis_origin[3], axis_direction[3], angle_radians,
   out_handle)` (0 < angle <= 2*pi).
2. **`native/occt_bridge/src/aicad_occt_bridge.cpp`**: both require the
   input handle address a `TopAbs_FACE` via `LookupTyped` (consistent
   with AICAD-023's pattern); `extrude` validates `distance` positive and
   finite and the direction normalizable, then
   `BRepPrimAPI_MakePrism(face, dir*distance)`; `revolve` validates the
   axis origin finite, direction normalizable, and angle in `(0, 2*pi]`
   (clamped to exactly `2*pi` within `1e-9` tolerance so floating-point
   overshoot of a caller-supplied "full turn" isn't spuriously rejected),
   then `BRepPrimAPI_MakeRevol(face, axis, angle, Copy=true)`. Both rely
   on OCCT's own Face→Solid generation rule for `BRepPrimAPI_MakePrism`/
   `MakeRevol` (the same rule `BRepPrimAPI_MakeBox` etc. use internally),
   not a separate explicit `BRepBuilderAPI_MakeSolid` step.
3. **`native/occt_bridge/tests/extrude_revolve_test.cpp`** (new): unit
   square extruded 5 units along +Z gives volume exactly `5.0`, matching
   bounding-box height; extrusion direction magnitude is proven ignored
   (a `(0,0,42)` direction with `distance=5.0` gives the same volume as
   `(0,0,1)`); zero/negative distance, zero-length direction, NaN
   direction, and wrong-handle-kind (a wire) all rejected
   `INVALID_ARGUMENT`; very small (1e-5) and very large (1e8) distances
   succeed. Revolve: an axis-offset rectangular profile (inner radius 1,
   outer radius 3, height 5, in the Y=0 half-plane containing the Z axis)
   revolved a full turn produces a tube whose volume matches the analytic
   `pi*(R^2-r^2)*h` formula; a half revolution gives exactly half that
   volume; zero/negative/`>2*pi` angle, zero-length axis direction, and
   wrong-handle-kind all rejected `INVALID_ARGUMENT`.
4. **`crates/cad-occt-bridge/src/ffi.rs`** /
   **`crates/cad-occt-bridge/src/lib.rs`**: raw declarations plus
   `Shape::extrude(&self, direction: Direction3, distance: f64)` and
   `Shape::revolve(&self, axis: Axis3, angle_radians: f64)`, both
   returning a new `Shape<'ctx>` in the same context. 5 new Rust-level
   tests: the full circle-wire → face → extrude pipeline (unit-square
   volume), the full axis-offset-rectangle → face → revolve pipeline
   (tube volume, using `Axis3`/`std::f64::consts::TAU` from
   `cad-kernel-api`), wrong-handle-kind rejection, and non-positive
   distance rejection.

## Implementation decisions
- Both operations require the input to already be a `Face` (via
  `LookupTyped`), not a bare `Wire` — matching AICAD-023's own contract
  (`make_face_from_wire` is the single required step between a
  profile wire and any solid-producing operation), and keeping each
  operation's precondition exactly one topological kind rather than
  accepting either a `Wire` or a `Face` and branching internally.
- `revolve`'s angle upper bound is enforced as `<= 2*pi` (with a small
  floating-point tolerance for exactly-2*pi inputs), not left unbounded —
  `docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s own `revolve` catalog entry
  defines `angle: Angle` with a `360deg` default, i.e. a bounded rotation
  concept; accepting an arbitrary angle (which OCCT would technically
  compute for) would silently accept a caller error (an angle in some
  other unit, or a mistaken multiple-turns value) rather than catching it
  at this boundary.

## Files changed
- Edited: `native/occt_bridge/include/aicad_occt_bridge.h`,
  `native/occt_bridge/src/aicad_occt_bridge.cpp`,
  `native/occt_bridge/CMakeLists.txt`, `crates/cad-occt-bridge/src/ffi.rs`,
  `crates/cad-occt-bridge/src/lib.rs`.
- Added: `native/occt_bridge/tests/extrude_revolve_test.cpp`.

## Verification (exact commands/results)
```
$ cargo test -p cad-occt-bridge
running 25 tests ... test result: ok. 25 passed; 0 failed

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, no warnings)

$ rm -rf native/occt_bridge/build && cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3
-- Configuring done

$ cmake --build native/occt_bridge/build
[100%] Built target extrude_revolve_test

$ ctest --test-dir native/occt_bridge/build --output-on-failure
1/8 occt_probe ... Passed
2/8 abi_boundary_test ... Passed
3/8 lifecycle_test ... Passed
4/8 box_cylinder_test ... Passed
5/8 transform_test ... Passed
6/8 curve_edge_wire_test ... Passed
7/8 face_test ... Passed
8/8 extrude_revolve_test ... Passed
100% tests passed, 0 tests failed out of 8

$ g++ -std=c++17 -Wall -Wextra -Wpedantic -c native/occt_bridge/src/aicad_occt_bridge.cpp \
    -I native/occt_bridge/include -isystem /usr/include/opencascade -o /tmp/w024.o
(no warnings)
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1,
CMake 3.28.3, OCCT 7.6.3 (`libocct-foundation-dev` 7.6.3+dfsg1-7.1build1)
— unchanged across all of Batch 1B (AICAD-020..024).

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**.
- Task-specific: native `ctest` (8/8) and `cargo test -p cad-occt-bridge`
  (25/25) both pass, as shown above.

## Limitations / follow-up
- `sweep`/`loft` (Batch 1C, AICAD-025) are separate, more general
  operations not implemented here — extrude/revolve cover only the
  straight-translation and single-axis-rotation cases.
- This completes Batch 1B. Per the active scheduled-task brief, the next
  step is the Batch 1B checkpoint
  (`project/gates/STAGE1-B_CONSTRUCTIVE_GEOMETRY.md`) before Batch 1C
  (AICAD-025..028, hard geometry operations) begins.
