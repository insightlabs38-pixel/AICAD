# AICAD-023 — Implement planar face from closed wire

## Objective
Implement construction of a planar face bounded by a closed wire, per
`project/TASKS.yaml` (AICAD-023) and
`docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` §3's `make_face` catalog
entry (Stage-1 scope: planar-only, matching this batch's other
constructive-geometry operations at raw-Rust/native fidelity).

## Dependencies checked
AICAD-022 (line/circle curves and edge/wire creation) — complete, commit
`6416756`. `make_face_from_wire` directly consumes the `Wire` handles
that task's `make_circle_wire`/`make_wire_from_edges` produce.

## What was done

1. **`native/occt_bridge/include/aicad_occt_bridge.h`**: added
   `aicad_occt_make_face_from_wire(context, wire_handle, out_handle)`,
   documented as requiring a single closed, planar wire; the face's plane
   is inferred from the wire's own geometry, not supplied separately
   (every Stage-1 profile is planar).
2. **`native/occt_bridge/src/aicad_occt_bridge.cpp`**: uses the
   `LookupTyped` helper (from AICAD-022) to require the input handle
   address a `TopAbs_WIRE` specifically, rejecting any other shape kind as
   `INVALID_ARGUMENT`; then `BRepBuilderAPI_MakeFace(wire, /*OnlyPlane=*/
   true)`, which fails (`IsDone() == false`, mapped to
   `OPERATION_FAILED`) for a non-planar wire.
   **Important finding, not a bug**: `BRepBuilderAPI_MakeFace`'s
   `IsDone()` reports only whether the builder algorithm ran to
   completion, not whether the resulting face is topologically valid — it
   happily "succeeds" for an **open** (unclosed) wire, producing a
   structurally complete but invalid face. This is exactly the Stage-1
   kernel policy #14 distinction ("validation and repair/healing are
   distinct semantic concepts") already built into this bridge:
   `make_face_from_wire`'s job is construction only, and the existing
   `shape_is_valid` query (from AICAD-016) is what a caller must use to
   catch this — not something this task silently folds into construction.
   Both native and Rust-level tests assert this explicitly (construction
   succeeds, `is_valid()` then correctly reports `false`), rather than
   treating the open-wire case as something requiring a code change.
3. **`native/occt_bridge/tests/face_test.cpp`** (new): face from a circle
   wire matches `pi*r^2` analytically; face from a 4-edge unit-square wire
   has area exactly `1.0`; wrong-handle-kind (an edge) rejected
   `INVALID_ARGUMENT`; an open (3-edge) wire's face construction succeeds
   but is reported invalid by `shape_is_valid` (the finding above); a
   non-planar closed wire (one vertex out of plane) is rejected
   `OPERATION_FAILED` by the `OnlyPlane=true` constructor; very small
   (1e-6 radius) and very large (1e8 radius) circle-wire faces both
   succeed.
4. **`crates/cad-occt-bridge/src/ffi.rs`** /
   **`crates/cad-occt-bridge/src/lib.rs`**: raw declaration plus
   `Shape::make_face(&self) -> KernelResult<Shape<'ctx>>` (operates on
   `self` as the wire, matching `transform`/future `extrude`/`revolve`'s
   "derived new `Shape` in the same context" pattern), with the same
   documented construction-vs-validity distinction in its doc comment. 4
   new Rust-level tests mirroring the native ones (circle/square area,
   open-wire invalid, wrong-kind rejection).

## Implementation decisions
- `BRepBuilderAPI_MakeFace`'s `OnlyPlane=true` constructor argument is
  used deliberately (rather than the default, which also attempts a
  general/non-planar surface fit) — Stage-1's profile-building pipeline
  (circle/rectangle wires → face → extrude/revolve) never needs a
  non-planar face, and requiring planarity here gives an earlier,
  more specific failure (`OPERATION_FAILED`) for a non-planar wire than
  letting a later, unrelated operation fail on an unexpectedly
  non-planar face.
- No proactive "is this wire closed?" check was added before calling
  `BRepBuilderAPI_MakeFace` (which would let this function itself reject
  an open wire with `INVALID_ARGUMENT` instead of the two-step
  construct-then-validate pattern). Kept the two-step pattern instead,
  because it is the more general, already-established pattern this
  bridge uses everywhere (construction reports `IsDone`, a separate query
  reports validity) — special-casing wire-closedness here would be an
  inconsistent one-off exception to that pattern for no real benefit
  (`shape_is_valid` already exists and callers already need to use it for
  every other geometric-correctness question).

## Files changed
- Edited: `native/occt_bridge/include/aicad_occt_bridge.h`,
  `native/occt_bridge/src/aicad_occt_bridge.cpp`,
  `native/occt_bridge/CMakeLists.txt`, `crates/cad-occt-bridge/src/ffi.rs`,
  `crates/cad-occt-bridge/src/lib.rs`.
- Added: `native/occt_bridge/tests/face_test.cpp`.

## Verification (exact commands/results)
```
$ cargo test -p cad-occt-bridge
running 21 tests ... test result: ok. 21 passed; 0 failed

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, no warnings)

$ rm -rf native/occt_bridge/build && cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3
-- Configuring done

$ cmake --build native/occt_bridge/build
[100%] Built target face_test

$ ctest --test-dir native/occt_bridge/build --output-on-failure
1/7 occt_probe ... Passed
2/7 abi_boundary_test ... Passed
3/7 lifecycle_test ... Passed
4/7 box_cylinder_test ... Passed
5/7 transform_test ... Passed
6/7 curve_edge_wire_test ... Passed
7/7 face_test ... Passed
100% tests passed, 0 tests failed out of 7

$ g++ -std=c++17 -Wall -Wextra -Wpedantic -c native/occt_bridge/src/aicad_occt_bridge.cpp \
    -I native/occt_bridge/include -isystem /usr/include/opencascade -o /tmp/w023.o
(no warnings)
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1,
CMake 3.28.3, OCCT 7.6.3 (`libocct-foundation-dev` 7.6.3+dfsg1-7.1build1).

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**.
- Task-specific: native `ctest` (7/7) and `cargo test -p cad-occt-bridge`
  (21/21) both pass, as shown above.

## Limitations / follow-up
- Faces with holes (`outer` + `holes: List<Wire>` in the eventual
  language-level `make_face` catalog entry) are out of scope here — this
  task's `make_face_from_wire` takes exactly one wire. A holed-face
  operation can be added later without breaking this signature (an
  overload/new function, per the capability-driven minimal-surface
  rule), and is not needed by any currently-authorized Stage-1 task.
- The open-wire-produces-an-invalid-face finding is recorded as
  documented, tested behavior here, not filed as a follow-up bug — it is
  correct behavior given `BRepBuilderAPI_MakeFace`'s actual contract, and
  this task's own tests are the permanent regression coverage for it.
