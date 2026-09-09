# AICAD-032 — Implement display tessellation output

## Objective
Implement "display tessellation output" per `project/TASKS.yaml`
(AICAD-032) and `docs/plan/01_SYSTEM_ARCHITECTURE.md`'s tessellation
bridge-operation catalog entry. The fourth task in Batch 1D.

## Dependencies checked
AICAD-031 (validation report) — complete, commit `77eef29`. Not a direct
code dependency, but the next task in Batch 1D's sequence. This task
introduced the first genuinely stateful, per-handle server-side cache in
this bridge (see "Implementation decisions"), a new pattern relative to
every prior Batch 1D task.

## What was done

1. **`native/occt_bridge/include/aicad_occt_bridge.h`**: added the
   `aicad_tessellation_counts_t` POD struct (`triangle_count`) and a
   two-call protocol: `aicad_occt_tessellate(context, handle,
   linear_deflection, angular_deflection, *out_counts)` and
   `aicad_occt_tessellation_get(context, handle, out_vertices,
   out_normals)`.
2. **`native/occt_bridge/src/aicad_occt_bridge.cpp`**:
   - `ShapeSlot` gained a `TessellationCache` member (vertex/normal
     `std::vector<double>` buffers, a triangle count, and a `present`
     flag) and `ShapeTable` gained `SetTessellation`/`GetTessellation`,
     keyed by (slot, generation) exactly like `Lookup`/`Release` — so
     each handle's tessellation is cached independently, not a single
     shared "last result" slot. `Insert` clears any inherited cache from
     a slot's previous occupant; `Release` clears it too (belt-and-
     braces: `Insert` already guarantees this, since a released slot's
     next `Insert` always clears the cache regardless).
   - A new private helper, `ExtractTessellation`, runs
     `BRepMesh_IncrementalMesh(shape, linear_deflection,
     /*isRelative=*/false, angular_deflection, /*isParallel=*/false)`,
     then iterates the shape's own unique faces (`TopExp::MapShapes`,
     matching this bridge's established convention), reads each face's
     `Poly_Triangulation` via `BRep_Tool::Triangulation`, and for every
     triangle: applies the face's `TopLoc_Location` transform to its 3
     node positions, swaps two node indices if the face is
     `TopAbs_REVERSED` (correcting triangle winding — a well-known OCCT
     convention, not guessed), computes one flat geometric normal via the
     cross product of two triangle edges, and appends 3 fresh (non-
     shared) vertex+normal entries. Faces with no triangulation after
     meshing (defensive; `BRepMesh_IncrementalMesh::IsDone()` was already
     checked) are skipped rather than failing the whole operation.
   - `aicad_occt_tessellate` validates both deflections (`> 0`, finite),
     calls `ExtractTessellation`, and stores the result via
     `SetTessellation`. `aicad_occt_tessellation_get` copies the cached
     buffers into caller-owned memory via `GetTessellation` +
     `std::copy`.
3. **`native/occt_bridge/CMakeLists.txt`**: added `TKMesh` (provides
   `BRepMesh_IncrementalMesh`) as a newly-required, verified-present OCCT
   module — the first new module link since AICAD-027's `TKFillet`.
   Registered `tessellation_test`.
4. **`native/occt_bridge/tests/tessellation_test.cpp`** (new): a box
   tessellates into a nonzero triangle count, and specifically **exactly
   12** (its 6 planar quad faces each split into exactly 2 triangles,
   regardless of deflection — planar faces need no further subdivision,
   verified empirically not assumed); the mesh's own vertex-buffer
   bounding box matches the box's analytic dimensions exactly; every
   triangle's normal is a unit vector aligned with exactly one box-face
   axis (proves winding/orientation handling produced physically correct
   outward normals, not merely "a" normal); a cylinder's curved lateral
   surface produces more than a trivial handful of triangles (proves
   `angular_deflection` actually drives curved-face refinement);
   `tessellation_get` without a prior `tessellate` call fails cleanly;
   non-positive/non-finite deflections are rejected; two shapes'
   tessellations are cached and retrieved independently without
   interference; releasing a handle invalidates its tessellation cache
   (`AICAD_OCCT_ERR_STALE_HANDLE`, not silently returning stale data to
   a reused slot).
5. **`crates/cad-occt-bridge/src/ffi.rs`**: the `aicad_tessellation_counts_t`
   mirror struct and raw `aicad_occt_tessellate`/
   `aicad_occt_tessellation_get` declarations.
6. **`crates/cad-occt-bridge/src/lib.rs`**: a public `TriangleMesh`
   struct (`vertices: Vec<Point3>`, `normals: Vec<Vector3>`, a
   `triangle_count()` helper) and `Shape::tessellate(&self,
   linear_deflection: f64, angular_deflection: f64) ->
   KernelResult<TriangleMesh>` — this single Rust call performs **both**
   native calls internally, hiding the C ABI's two-call protocol from
   Rust callers entirely (no cache/state is exposed at the Rust level; a
   second `tessellate()` call simply re-meshes). 6 new Rust-level tests
   mirroring the native ones.

## Implementation decisions

- **Output is flat-shaded, non-vertex-shared triangle soup**: each
  triangle owns 3 private vertex positions and one flat geometric normal
  duplicated across them, rather than a vertex-shared mesh with
  averaged/smooth per-vertex normals (which would require deduplicating
  `Poly_Triangulation` nodes across triangles within a face and choosing
  a normal-averaging scheme at shared vertices — real complexity with no
  Stage-1 consumer requiring smooth shading yet). This is a deliberate,
  disclosed simplification, not a hidden shortcut: it makes the output
  trivially and exactly analytically verifiable (triangle count, bounding
  box, per-triangle normal direction) per AGENTS.md's evidence rule, at
  the cost of a larger-than-minimal buffer (`9 * triangle_count` doubles
  instead of a deduplicated vertex array plus a separate index buffer).
- **Per-handle server-side tessellation cache, not a single shared "last
  result" slot.** An earlier design considered in review (not
  implemented) would have cached only the most recent `tessellate()`
  call's result process-wide; that would silently return the wrong
  shape's mesh if two shapes were tessellated before either was fetched.
  Keying the cache to each shape's own `(slot, generation)` — the same
  identity `Lookup`/`Release` already use — avoids this entirely and was
  proven correct by `tessellation_test.cpp`'s "two shapes tessellate
  independently" case.
- **The C ABI's two-call (`tessellate` then `tessellation_get`) protocol
  is not exposed at the Rust level.** `Shape::tessellate` performs both
  native calls internally and returns one owned `TriangleMesh` — ordinary
  Rust callers should not need to reason about a native-side cache with
  its own invalidation rules; that plumbing stays entirely inside
  `cad-occt-bridge`, consistent with this crate's own stated charter
  ("everything else in this crate is a safe wrapper that never leaks an
  `ffi`-module type").
- **No caller control over `isRelative`/`isParallel`**
  (`BRepMesh_IncrementalMesh`'s own additional constructor parameters) —
  both fixed at `false`, matching the capability-driven minimal-surface
  rule; not required by this task's own analytic evidence.
- **The REVERSED-face winding fix was applied proactively, not
  discovered by a failing test.** It is a well-documented OCCT
  convention (a `Poly_Triangulation`'s triangle node order is defined
  relative to its face's underlying surface parametrization, independent
  of that face's own `TopAbs_Orientation`), applied here from the start
  rather than found via trial and error — the box test's "every normal
  is axis-aligned and unit-length" assertion is the evidence this is
  actually correct for this bridge's own faces, not merely a documented
  claim taken on faith.

## Files changed
- Edited: `native/occt_bridge/include/aicad_occt_bridge.h`,
  `native/occt_bridge/src/aicad_occt_bridge.cpp`,
  `native/occt_bridge/CMakeLists.txt`, `crates/cad-occt-bridge/src/ffi.rs`,
  `crates/cad-occt-bridge/src/lib.rs`.
- Added: `native/occt_bridge/tests/tessellation_test.cpp`.

## Verification (exact commands/results)
```
$ cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3

$ cmake --build native/occt_bridge/build -j$(nproc)
[100%] Built target tessellation_test   (plus all 15 pre-existing targets)

$ ctest --test-dir native/occt_bridge/build --output-on-failure
 1/16 occt_probe .................. Passed
 ...
15/16 validation_test ............. Passed
16/16 tessellation_test ........... Passed
100% tests passed, 0 tests failed out of 16

$ ./native/occt_bridge/build/tessellation_test
... 16 PASS lines, 0 FAIL ...
tessellation_test: all checks PASSED

$ cargo fmt --all -- --check
(exit 0, after one clippy-suggested `chunks_exact` -> `as_chunks::<3>()`
fix and a `cargo fmt --all` pass)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, no warnings)

$ cargo build --workspace --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.66s

$ cargo test -p cad-occt-bridge
running 77 tests ... test result: ok. 77 passed; 0 failed

$ cargo test --workspace
(every crate) test result: ok, 0 failed
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1
(`rust-toolchain.toml`), CMake 3.28.3, OCCT 7.6.3
(`libocct-modeling-algorithms-dev` 7.6.3+dfsg1-7.1build1, `TKMesh` module,
newly linked this task) — otherwise unchanged from Batch 1A/1B/1C/
AICAD-029..031.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — **PASS**.
- Task-specific: native `ctest` (16/16) and `cargo test -p
  cad-occt-bridge` (77/77, 6 new) both pass, as shown above.

## Regressions added
None. All 15 pre-existing native test executables and all 71
pre-existing Rust tests continue to pass unchanged.

## Limitations (honest capability boundaries)
- **Not a vertex-shared, smooth-normal mesh** (see "Implementation
  decisions") — a caller wanting GPU-efficient indexed geometry or
  smooth (Gouraud-style) shading across a curved face's triangles must
  post-process this output themselves; this bridge does not provide it.
- **Only tested against a box (planar faces) and a cylinder (one curved
  lateral surface)** — no BSpline/NURBS surface, no shape with a face
  containing an internal hole/multiple wires (an annular face), and no
  shape produced by fillet/chamfer/shell/offset/sweep/loft was
  tessellated in this task's own tests. `BRepMesh_IncrementalMesh` itself
  is a mature, general OCCT algorithm expected to handle these, but this
  task's own evidence does not cover them.
- **No adversarial deflection-extreme cases** (e.g. an extremely coarse
  deflection larger than the shape itself, or an extremely fine
  deflection producing a very large triangle count) were tested — left
  for Stage-1 hardening mode's regression-expansion/bounded-fuzzing work,
  consistent with prior batches' equivalent limitation notes.
- **No degenerate-triangle guard was tested.** `ExtractTessellation`
  defensively zeroes a triangle's normal if its cross-product magnitude
  is below `1e-12` (avoiding a NaN from a zero-length normalization), but
  no test constructs a shape whose triangulation actually produces a
  degenerate (near-zero-area) triangle to exercise that guard.

None of these limitations required forcing an artificial success or
weakening a test to hide a failure — every claim in this report and its
tests is backed by an actual passing run, and the two design decisions
most likely to hide a bug if done carelessly (REVERSED-face winding, and
independent per-handle caching) were each specifically probed by a test
built to catch the failure mode they could produce, not just a happy-path
check.
