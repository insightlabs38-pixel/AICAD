# AICAD-021 — Implement points/vectors/axes/frames and rigid transforms

## Objective
Implement points/vectors/axes/frames and rigid transforms per
`project/TASKS.yaml` (AICAD-021) and
`docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` §3
(`point3`/`vector3`/`direction`/`axis`/`frame`/`transform_from_frames`),
at Stage-1's raw-Rust/native fidelity: typed engineering quantities
(`Length`, etc., via `cad-units`) are a later stage's concern, per
AICAD-034's "directly through Rust/native API" framing for the whole
Stage-1 proof.

`docs/plan/01_SYSTEM_ARCHITECTURE.md` §4's bridge operation list has no
separate "transform" primitive constructor — placement crosses the bridge
via one general operation
(`docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s `transform` feature: "Apply
rigid/affine transform... Returns: same as input"), which this task adds
as `aicad_occt_transform_shape`. Points/vectors/axes/frames are pure math
with no reason to cross the bridge on their own — they live entirely in
`cad-kernel-api` as new backend-independent value types.

## Dependencies checked
AICAD-020 (box/cylinder) — complete, commit `896d8f3`. `cad-kernel-api`
and `cad-occt-bridge` are otherwise unchanged since AICAD-018/019.

## What was done

1. **`crates/cad-kernel-api/src/geometry.rs`** (new module,
   `#![forbid(unsafe_code)]` inherited crate-wide): `Point3`, `Vector3`
   (with `Add`/`Sub`/`Neg`/`Mul<f64>` operator impls rather than
   trait-name-colliding inherent methods, `dot`, `cross`, `length`,
   `normalize -> Option<Direction3>`), `Direction3` (a newtype that can
   **only** be constructed unit-length, via `Vector3::normalize`),
   `Axis3` (origin + direction), `Frame3` (origin + orthonormal
   right-handed x/y/z — either derived from a single `x` direction via a
   stable Gram-Schmidt construction that always succeeds, or validated
   from an explicit triple and rejected with `KernelError::InvalidArgument`
   if not orthonormal/right-handed), and `Transform` (rigid
   rotation+translation: `identity`, `translation`, `rotation` about an
   arbitrary `Axis3` via Rodrigues' formula, `from_frames` (rigid
   frame-to-frame remapping), `compose`, `apply_point`/`apply_vector`/
   `apply_direction`, and `to_row_major_3x4` for the FFI boundary).
   23 unit tests, including a `from_frames` round-trip, four repeated
   90-degree rotations returning to start, and a test that every
   `Transform` this module can produce independently passes the exact
   same rigidity check (column norms/orthogonality/determinant) the
   native bridge itself re-validates.
2. **`native/occt_bridge/include/aicad_occt_bridge.h`**: added
   `aicad_occt_transform_shape(context, handle, matrix[12], out_handle)`,
   documenting the row-major 3x4 matrix layout explicitly (linear part +
   translation column) and that it always returns a NEW handle (DL-2
   functional/value semantics — never mutates the input in place).
3. **`native/occt_bridge/src/aicad_occt_bridge.cpp`**: `Norm3` helper
   (shared going forward); `aicad_occt_transform_shape` validates every
   matrix entry finite, then **independently re-validates rigidity**
   (each 3x3-block column has unit norm, columns are mutually orthogonal,
   determinant is +1 i.e. a proper rotation, not a reflection) before
   ever constructing a `gp_Trsf` — a defense-in-depth check against a bug
   in the Rust math layer silently corrupting kernel geometry (AGENTS.md
   evidence rule), not merely trusting the caller. Applies via
   `BRepBuilderAPI_Transform` with `Copy=true`.
4. **`native/occt_bridge/tests/transform_test.cpp`** (new): identity is a
   no-op on volume/bounding-box and still produces a new handle; pure
   translation shifts the bounding box by exactly the given vector;
   90-degree Z rotation swaps a box's X/Y extents and preserves volume;
   four repeated 90-degree rotations return to the original extents;
   non-rigid matrices (non-uniform scale, reflection, shear) are all
   rejected `INVALID_ARGUMENT`; NaN/infinity anywhere in the matrix is
   rejected; a stale handle is rejected.
5. **`crates/cad-occt-bridge/src/ffi.rs`** /
   **`crates/cad-occt-bridge/src/lib.rs`**: raw declaration plus
   `Shape::transform(&self, t: &Transform) -> KernelResult<Shape<'ctx>>`
   (constructs the row-major matrix via `Transform::to_row_major_3x4`,
   returns a new `Shape` in the same context/lifetime). 3 Rust-level
   tests: identity no-op + new-handle, translation bounding-box shift,
   and non-mutation of the source shape.

## Implementation decisions
- `Transform` is deliberately **rigid-only** (no scale/shear/reflection),
  matching the task title ("rigid transforms") and the native bridge's
  own contract — a future `scale`/`mirror` feature (both present in
  `docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s catalog) is a distinct,
  separate operation, not folded into this one, avoiding an implicit
  widening of what "transform" means here.
- `Frame3` cannot be constructed non-orthonormal/left-handed at all:
  `from_x` derives a valid triple by construction, `new` validates and
  rejects. This mirrors the native bridge's own defense-in-depth pattern
  (never trust a value crossing a boundary to already satisfy an
  invariant) at the pure-Rust layer too.
- Rust-level tests intentionally do **not** attempt to exercise
  transform's stale/foreign/invalid-handle rejection paths: a `Shape`'s
  borrow-checker-enforced lifetime makes calling `.transform()` on an
  already-released shape a compile error in this crate's safe API, which
  is documented in `transform_does_not_mutate_the_source_shape`'s comment
  rather than silently omitted. Those paths are exhaustively covered
  natively instead.

## Files changed
- Added: `crates/cad-kernel-api/src/geometry.rs`.
- Edited: `crates/cad-kernel-api/src/lib.rs` (module wiring + re-exports),
  `native/occt_bridge/include/aicad_occt_bridge.h`,
  `native/occt_bridge/src/aicad_occt_bridge.cpp`,
  `native/occt_bridge/CMakeLists.txt`, `crates/cad-occt-bridge/src/ffi.rs`,
  `crates/cad-occt-bridge/src/lib.rs`.
- Added: `native/occt_bridge/tests/transform_test.cpp`.

## Verification (exact commands/results)
```
$ cargo test -p cad-kernel-api -p cad-occt-bridge
running 23 tests (cad-kernel-api) ... test result: ok. 23 passed
running 12 tests (cad-occt-bridge) ... test result: ok. 12 passed

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, no warnings)

$ rm -rf native/occt_bridge/build && cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3
-- Configuring done

$ cmake --build native/occt_bridge/build
[100%] Built target transform_test

$ ctest --test-dir native/occt_bridge/build --output-on-failure
1/5 occt_probe ... Passed
2/5 abi_boundary_test ... Passed
3/5 lifecycle_test ... Passed
4/5 box_cylinder_test ... Passed
5/5 transform_test ... Passed
100% tests passed, 0 tests failed out of 5

$ g++ -std=c++17 -Wall -Wextra -Wpedantic -c native/occt_bridge/src/aicad_occt_bridge.cpp \
    -I native/occt_bridge/include -isystem /usr/include/opencascade -o /tmp/w021.o
(no warnings)
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1,
CMake 3.28.3, OCCT 7.6.3 (`libocct-foundation-dev` 7.6.3+dfsg1-7.1build1).

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**.
- Task-specific: native `ctest` (5/5) and `cargo test -p cad-kernel-api -p
  cad-occt-bridge` (35/35) both pass, as shown above.

## Limitations / follow-up
- `Frame3`/`Axis3`/`Transform` are raw-`f64` value types; they do not yet
  carry `cad-units` typed quantities (`Length`, `Angle`) — that
  integration belongs to the language/type-system stages, not Stage 1's
  raw kernel layer.
- `Transform::rotation` composes about an arbitrary axis point (not just
  the world origin), needed for placing features away from the origin in
  later batches/AICAD-034's bracket demo; this was verified directly
  (`rotation_about_an_offset_axis_keeps_the_axis_fixed`).
