# AICAD-028 — Implement shell/offset spike

## Objective
Implement `shell`/`offset` per `project/TASKS.yaml` (AICAD-028, titled
"shell/offset spike") and `docs/plan/01_SYSTEM_ARCHITECTURE.md`'s
`shell`/`offset` bridge-operation catalog entries. This is the fourth and
final task in Batch 1C. Per the active scheduled-task brief, these are
explicitly the operations this bridge is allowed to conclude have
"important kernel limitations" without unbounded effort to force universal
success — this report records what was verified to work and what was not
attempted, honestly.

## Dependencies checked
AICAD-027 (fillet and chamfer) — complete, commit `0115281`. `shell`
reuses the same raw/index-based selection pattern AICAD-027 established
for edges (`aicad_occt_shape_edge_count`/`get_edge`), applied here to
faces (`aicad_occt_shape_face_count`/`get_face`).

## What was done

1. **`native/occt_bridge/include/aicad_occt_bridge.h`**: added
   `aicad_occt_shape_face_count`/`_get_face` (unique-face enumeration via
   `TopExp::MapShapes`, mirroring AICAD-027's edge enumeration exactly),
   `aicad_occt_shell(context, shape_handle, faces_to_remove[], face_count,
   thickness, out_handle)`, and `aicad_occt_offset(context, shape_handle,
   distance, out_handle)`.
2. **`native/occt_bridge/src/aicad_occt_bridge.cpp`**:
   - `shape_face_count`/`get_face` follow `shape_edge_count`/`get_edge`'s
     exact pattern with `TopAbs_FACE` in place of `TopAbs_EDGE`.
   - `shell` builds a `TopTools_ListOfShape` from the caller's selected
     faces (each looked up via `LookupTyped(..., TopAbs_FACE, ...)`),
     then calls `BRepOffsetAPI_MakeThickSolid::MakeThickSolidByJoin(shape,
     closing_faces, thickness, 1e-6)` and checks `IsDone()`.
   - `offset` calls `BRepOffsetAPI_MakeOffsetShape::PerformByJoin(shape,
     distance, 1e-6)` (default join mode `GeomAbs_Arc`) and checks
     `IsDone()`.
   - Both target `shape_handle` via the existing kind-unrestricted
     `LookupAnyKind` helper (see "Implementation decisions").
3. **`native/occt_bridge/CMakeLists.txt`**: no new OCCT module needed —
   `BRepOffsetAPI_MakeOffsetShape`/`MakeThickSolid` live in the same
   `TKOffset` library AICAD-025 (sweep/loft) already links; registered
   `shell_offset_test`.
4. **`native/occt_bridge/tests/shell_offset_test.cpp`** (new): a box has
   exactly 6 unique faces; `get_face` rejects an out-of-range index;
   **shell** a box with its top face removed and wall thickness `t` (built
   inward) matches the analytic cavity-volume formula
   `dx·dy·dz - (dx-2t)(dy-2t)(dz-t)`; **offset** a box outward by distance
   `δ` matches the analytic Minkowski-sum-with-a-ball formula
   `dx·dy·dz + 2δ(dx·dy+dy·dz+dz·dx) + πδ²(dx+dy+dz) + (4/3)πδ³` (see
   "Key finding" below for why this formula applies); adversarial
   rejections (null/zero face list, zero thickness/distance, non-face
   handle); an oversized-wall-thickness probe (see "Adversarial cases and
   findings" below).
5. **`crates/cad-occt-bridge/src/ffi.rs`**: raw
   `aicad_occt_shape_face_count`/`_get_face`/`aicad_occt_shell`/`_offset`
   declarations.
6. **`crates/cad-occt-bridge/src/lib.rs`**: `Shape::face_count(&self) ->
   KernelResult<usize>`, `Shape::get_face(&self, index: usize) ->
   KernelResult<Shape<'ctx>>`, `Shape::shell(&self, faces_to_remove:
   &[&Shape<'ctx>], thickness: f64) -> KernelResult<Shape<'ctx>>`,
   `Shape::offset(&self, distance: f64) -> KernelResult<Shape<'ctx>>`. 6
   new Rust-level tests mirroring the native ones (face count,
   out-of-range rejection, shell volume, offset volume, non-face-handle
   rejection, zero-distance rejection).

## Key finding: offset and fillet-all-edges are the same geometric operation for a convex solid

This task's own evidence work produced a notable, verified cross-check:
offsetting a box outward by distance `δ` with OCCT's default `GeomAbs_Arc`
join mode is **the same Minkowski-sum-with-a-ball construction** as
AICAD-027's fillet-all-edges — both fill the gaps at edges/vertices with
pipes/spheres of radius `δ`/`r`, so the same closed-form volume formula
applies (with `δ` measured from the *original* faces for offset, vs.
`r` measured from an *inset* core for fillet, since fillet keeps the
original face planes in place while offset moves them outward). Both were
empirically verified against this same formula family independently
(different code paths, different tests), which is stronger cross-checking
evidence than either alone — an unplanned but genuine benefit of doing
both operations back-to-back within Batch 1C.

## Adversarial cases and findings

Per AGENTS.md's adversarial-case requirement ("shell/offset failure
modes" explicitly named), an oversized wall thickness (`10.0` on a
`1×1×1` box, where a geometrically valid inward shell requires
`|thickness| < min(dimension)/2` roughly) was probed rather than assumed
to fail a particular way. Finding: `BRepOffsetAPI_MakeThickSolid::IsDone()`
returns false and this bridge correctly reports
`AICAD_OCCT_ERR_OPERATION_FAILED` — cleanly rejected, not silently
clamped, matching the same "OCCT reported the requested geometric
operation could not be completed" contract used throughout this bridge.

## Implementation decisions

- **`shell`/`offset`'s target `shape_handle` is not restricted to
  `TopAbs_SOLID`** — the same reasoning as AICAD-026/027: OCCT's own
  `MakeThickSolidByJoin`/`PerformByJoin` accept a generic `TopoDS_Shape`
  (the latter's own doc comment explicitly lists "face, shell, solid, or
  compound of these"), and restricting to Solid would break chaining from
  a boolean result exactly as it would for fillet/chamfer.
- **`shell` requires `face_count >= 1`** (at least one face must be
  removed to open the shell) rather than also supporting OCCT's
  zero-closing-faces "fully enclosed hollow shell" case. This is the
  common "shell" use case (hollow with an opening) and the one this task's
  evidence targets; a fully-enclosed hollow shell is a legitimate but
  separate case not exercised here — see "Limitations" below.
- **`shell`'s tolerance and `offset`'s tolerance are both hardcoded to
  `1e-6`**, not exposed as a caller parameter — matching this bridge's
  existing pattern of not exposing OCCT's tuning parameters
  (`face_test`'s `BRepBuilderAPI_MakeFace`'s tolerance is likewise not
  caller-controlled) unless a task's own evidence requires varying it.
- **No caller-controlled join mode** (`GeomAbs_Arc` vs.
  `GeomAbs_Intersection`) is exposed for either operation — OCCT's default
  (`GeomAbs_Arc`) was used as-is, matching AICAD-025's "no caller-facing
  trihedron control for sweep" decision for the same reason: sufficient
  for this task's own analytic evidence, minimal-surface rule.

## Files changed
- Edited: `native/occt_bridge/include/aicad_occt_bridge.h`,
  `native/occt_bridge/src/aicad_occt_bridge.cpp`,
  `native/occt_bridge/CMakeLists.txt`, `crates/cad-occt-bridge/src/ffi.rs`,
  `crates/cad-occt-bridge/src/lib.rs`.
- Added: `native/occt_bridge/tests/shell_offset_test.cpp`.

## Verification (exact commands/results)
```
$ rm -rf native/occt_bridge/build && cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3

$ cmake --build native/occt_bridge/build -j$(nproc)
[100%] Built target shell_offset_test   (plus all pre-existing targets)

$ ctest --test-dir native/occt_bridge/build --output-on-failure
1/12 occt_probe .................. Passed
2/12 abi_boundary_test ........... Passed
3/12 lifecycle_test .............. Passed
4/12 box_cylinder_test ........... Passed
5/12 transform_test .............. Passed
6/12 curve_edge_wire_test ........ Passed
7/12 face_test .................... Passed
8/12 extrude_revolve_test ........ Passed
9/12 sweep_loft_test ............. Passed
10/12 boolean_test ................ Passed
11/12 fillet_chamfer_test ......... Passed
12/12 shell_offset_test ........... Passed
100% tests passed, 0 tests failed out of 12

$ ./native/occt_bridge/build/shell_offset_test
... 15 PASS lines, 1 INFO line (oversized-shell finding above), 0 FAIL ...
shell_offset_test: all checks PASSED

$ g++ -std=c++17 -Wall -Wextra -Wpedantic -c native/occt_bridge/src/aicad_occt_bridge.cpp \
    -I native/occt_bridge/include -isystem /usr/include/opencascade -o /tmp/w028.o
(no warnings)

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, no warnings)

$ cargo build --workspace --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.21s

$ cargo test -p cad-occt-bridge
running 53 tests ... test result: ok. 53 passed; 0 failed

$ cargo test --workspace
(every crate) test result: ok, 0 failed
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1
(`rust-toolchain.toml`), CMake 3.28.3, OCCT 7.6.3
(`libocct-modeling-algorithms-dev` 7.6.3+dfsg1-7.1build1, `TKOffset`
module, same as AICAD-025) — unchanged from Batch 1A/1B/AICAD-025..027.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — **PASS**.
- Task-specific: native `ctest` (12/12) and `cargo test -p
  cad-occt-bridge` (53/53) both pass, as shown above.

## Regressions added
None. All 11 pre-existing native test executables and all pre-existing
Rust tests continue to pass unchanged.

## Limitations (honest capability boundaries, per this task's explicit charter)
- **Only tested against a box (axis-aligned planar faces).** OCCT's own
  `BRepOffsetAPI_MakeOffsetShape` header documentation lists several
  failure modes this task did not probe: vertices where more than 3 edges
  converge, offset distances too large relative to local curvature
  (self-intersection), and C0-continuity BSpline surfaces. None of these
  were exercised here; the box case validated a strong (exact,
  closed-form) analytic result, but does not generalize to curved or
  more topologically complex inputs.
- **`shell` was only tested with exactly one closing face removed.**
  Removing multiple faces, or zero faces (a fully enclosed hollow shell,
  which OCCT supports via an empty `TopTools_ListOfShape`), was not
  exercised — this bridge's own `shell` currently requires
  `face_count >= 1`, so the zero-face case is not even reachable through
  this ABI yet.
- **No caller control over join mode, tolerance, or shell mode**
  (`BRepOffset_Skin` is OCCT's only implemented `Mode` value per its own
  header comment, so this is not a gap, but join mode and tolerance are
  genuine unexposed knobs).
- **No systematic failure-boundary mapping.** This task confirmed *one*
  oversized-thickness case fails cleanly, not the full boundary of
  "how large is too large" as a function of geometry. A systematic sweep
  belongs to Stage-1 hardening mode's bounded-fuzzing work
  (post-AICAD-037), consistent with AICAD-027's own equivalent limitation
  note for fillet.
- **Curved-geometry offset/shell (cylinders, filleted/chamfered results,
  sweep/loft results) was not tested.** Only planar-faced box geometry was
  used for this task's analytic evidence.

None of these limitations required forcing an artificial success or
weakening a test to hide a failure — every claim in this report and its
tests is backed by an actual passing (or, for the oversized case,
correctly-failing) run.
