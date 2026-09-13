# AICAD-022 — Implement line/circle curves and edge/wire creation

## Objective
Implement line/circle curve construction and edge/wire creation per
`project/TASKS.yaml` (AICAD-022) and
`docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` §3's
`line_curve`/`circle_curve`/`make_edge`/`make_wire` catalog entries, at
Stage-1's raw-Rust/native fidelity.

## Dependencies checked
AICAD-021 (transform) — complete, commit `f1c5c2d`.

## What was done

1. **`native/occt_bridge/include/aicad_occt_bridge.h`**: added
   `#include <stddef.h>` (needed for `size_t`), and three new operations
   under an explicit "minimal curve/edge/wire construction" comment
   explaining the scope decision below:
   - `aicad_occt_make_line_edge(context, p0[3], p1[3], out_handle)` —
     straight edge between two points.
   - `aicad_occt_make_circle_wire(context, center[3], normal[3], radius,
     out_handle)` — closed circular wire directly (not just an edge: a
     full circle is inherently closed, and every consumer needs a Wire).
   - `aicad_occt_make_wire_from_edges(context, edges[], edge_count,
     out_handle)` — joins an ordered edge list into one wire; the array
     is a caller-owned pointer + length (no STL container crosses the
     boundary, per Stage-1 kernel policy #8).
2. **`native/occt_bridge/src/aicad_occt_bridge.cpp`**: implementations
   plus four new shared helpers (`IsFinite3`, `ToPnt`, `TryToDir`,
   `LookupTyped`) in the existing anonymous-namespace helper block.
   `LookupTyped` looks up a handle and additionally requires its
   `ShapeType()` match an expected `TopAbs_ShapeEnum` (e.g. `TopAbs_EDGE`)
   — a caller-contract violation (wrong handle kind) is rejected as
   `INVALID_ARGUMENT` by direct check, not by letting an OCCT `TopoDS::`
   cast exception propagate into the generic `Standard_Failure` catch and
   get mapped to a less specific `OPERATION_FAILED`.
   `make_line_edge` rejects coincident/near-coincident endpoints
   (`< 1e-12`) before ever calling OCCT. `make_circle_wire` rejects a
   non-positive/non-finite radius and a zero-length normal.
   `make_wire_from_edges` rejects an empty list, and surfaces
   `BRepBuilderAPI_MakeWire`'s own per-edge `IsDone()` failure (e.g. two
   disconnected edges) as `OPERATION_FAILED`.
3. **`native/occt_bridge/tests/curve_edge_wire_test.cpp`** (new): line
   edge happy path + bounding-box check; coincident/near-coincident/NaN
   endpoint rejection; very large (1e9)/very small-but-above-tolerance
   (1e-5) spans; circle wire happy path + centered bounding-box check;
   zero/negative radius and zero-length normal rejection; very small
   (1e-7)/very large (1e8) radii; non-unit-length normal acceptance
   (direction only matters); a 4-edge unit square wire built via
   `make_wire_from_edges`, checked valid with the expected bounding box;
   empty-list rejection; two disconnected edges rejected
   `OPERATION_FAILED`; a wrong-kind handle (a solid) rejected
   `INVALID_ARGUMENT` by `LookupTyped`.
4. **`crates/cad-occt-bridge/src/ffi.rs`** /
   **`crates/cad-occt-bridge/src/lib.rs`**: raw declarations plus
   `OcctContext::make_line_edge(p0: Point3, p1: Point3)`,
   `OcctContext::make_circle_wire(center: Point3, normal: Direction3,
   radius: f64)`, and `OcctContext::make_wire_from_edges<'ctx>(edges:
   &[&Shape<'ctx>]) -> KernelResult<Shape<'ctx>>` (the `'ctx` lifetime
   parameter ties every edge and the returned wire to the exact same
   context borrow, so the type system itself rejects mixing edges from a
   different `OcctContext` value at compile time, on top of the native
   `FOREIGN_CONTEXT` runtime check). 5 new Rust-level tests.

## Implementation decisions
- **No standalone `Curve` handle kind yet.** A raw `Geom_Curve` (line or
  circle parametrization independent of any topological edge) is not
  exposed as its own kernel handle — curves are constructed directly into
  an `Edge`/`Wire`, per RFC-0002 §3's capability-driven minimal-surface
  rule: no current Stage-1 task needs curve evaluation independent of an
  edge (that is `evaluate_curve` in
  `docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` §3, not scheduled for
  Stage 1). A dedicated `Curve` handle can be added later without
  breaking this surface, documented explicitly in the header.
- `make_circle_wire` returns a `Wire` directly rather than requiring a
  separate edge-then-wire step (unlike `make_line_edge`, which stays an
  `Edge` so it can be combined with others via `make_wire_from_edges`) —
  a full circle is inherently closed and every Stage-1 consumer
  (`aicad_occt_make_face_from_wire`, the next task) needs a `Wire`, so
  requiring an intermediate wire-building call for every circle would add
  ceremony with no benefit.

## Files changed
- Edited: `native/occt_bridge/include/aicad_occt_bridge.h`,
  `native/occt_bridge/src/aicad_occt_bridge.cpp`,
  `native/occt_bridge/CMakeLists.txt`, `crates/cad-occt-bridge/src/ffi.rs`,
  `crates/cad-occt-bridge/src/lib.rs`.
- Added: `native/occt_bridge/tests/curve_edge_wire_test.cpp`.

## Verification (exact commands/results)
```
$ cargo test -p cad-occt-bridge
running 17 tests ... test result: ok. 17 passed; 0 failed

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, no warnings)

$ rm -rf native/occt_bridge/build && cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3
-- Configuring done

$ cmake --build native/occt_bridge/build
[100%] Built target curve_edge_wire_test

$ ctest --test-dir native/occt_bridge/build --output-on-failure
1/6 occt_probe ... Passed
2/6 abi_boundary_test ... Passed
3/6 lifecycle_test ... Passed
4/6 box_cylinder_test ... Passed
5/6 transform_test ... Passed
6/6 curve_edge_wire_test ... Passed
100% tests passed, 0 tests failed out of 6

$ g++ -std=c++17 -Wall -Wextra -Wpedantic -c native/occt_bridge/src/aicad_occt_bridge.cpp \
    -I native/occt_bridge/include -isystem /usr/include/opencascade -o /tmp/w022.o
(no warnings)
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1,
CMake 3.28.3, OCCT 7.6.3 (`libocct-foundation-dev` 7.6.3+dfsg1-7.1build1).

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**.
- Task-specific: native `ctest` (6/6) and `cargo test -p cad-occt-bridge`
  (17/17) both pass, as shown above.

## Limitations / follow-up
- `make_line_edge`'s coincident-point rejection threshold (`1e-12`) is
  well below OCCT's own default confusion tolerance
  (`Precision::Confusion()`, `1e-7`); a span at or just below `1e-7` is
  rejected by OCCT itself as `OPERATION_FAILED` rather than by this
  bridge's own pre-check as `INVALID_ARGUMENT` — both are correct
  rejections, just from different layers, and the test campaign uses a
  `1e-5` span for its "very small but valid" case to stay unambiguously on
  the accepted side of OCCT's own tolerance rather than landing exactly on
  that boundary.
