# AICAD-029 — Implement topology exploration

## Objective
Implement "topology exploration" per `project/TASKS.yaml` (AICAD-029) and
`docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` §3's exploration surface
(`topology_faces`, `topology_edges`, `topology_vertices`, `adjacent_faces`,
`face_edges`). This is the first task in Batch 1D — Inspection / validation
/ interchange, the batch after Batch 1C (hard geometry operations,
AICAD-025..028, complete and merged to `origin/main` via PR #3).

## Dependencies checked
AICAD-028 (shell/offset spike) — complete, merged to `origin/main`
(`903dceb`, PR #3). `aicad_occt_shape_edge_count`/`_get_edge`
(AICAD-027) and `aicad_occt_shape_face_count`/`_get_face` (AICAD-028)
already exist and were reused/extended here rather than duplicated.

## What was done

1. **`native/occt_bridge/include/aicad_occt_bridge.h`**: added
   `aicad_occt_shape_vertex_count`/`_get_vertex` (unique-vertex
   enumeration via `TopExp::MapShapes`, mirroring
   `shape_edge_count`/`_face_count` exactly), `aicad_occt_edge_vertices`
   (an edge's two endpoint vertices via `TopExp::Vertices`), and
   `aicad_occt_shape_edge_adjacent_face_count`/`_get` (edge-to-face
   adjacency via `TopExp::MapShapesAndAncestors`, indexed against
   `shape_edge_count`'s own enumeration order).
2. **`native/occt_bridge/src/aicad_occt_bridge.cpp`**:
   - `shape_vertex_count`/`get_vertex` follow
     `shape_edge_count`/`get_edge`'s exact pattern with `TopAbs_VERTEX`.
   - `edge_vertices` requires its argument to be exactly kind Edge
     (`LookupTyped`, matching `fillet`/`chamfer`'s own edge-typed-argument
     rejection) and calls `TopExp::Vertices(edge, v0, v1)` (the 2-output
     overload, no `CumOri` argument — the edge's own "first"/"last"
     orientation sense).
   - `edge_adjacent_face_count`/`_get` share a new private helper,
     `LookupIndexedEdge`, that rebuilds the same `TopTools_IndexedMapOfShape`
     `shape_edge_count`/`get_edge` build, so `edge_index`'s contract stays
     anchored to that function's own enumeration order. They then build a
     `TopTools_IndexedDataMapOfShapeListOfShape` via
     `TopExp::MapShapesAndAncestors(shape, TopAbs_EDGE, TopAbs_FACE, ...)`
     and query it by the specific edge shape found (via `Contains`/
     `FindFromKey`, i.e. shape-identity lookup, not by assuming index
     correspondence between the two independently-built maps).
   - All three additions follow the existing status-code/exception
     contract (`Standard_Failure` → `OPERATION_FAILED`, anything else →
     `INTERNAL`) and reuse `LookupAnyKind` (any shape kind accepted for
     `shape_handle`, matching `shape_edge_count`/`_face_count`'s own
     rationale) or `LookupTyped(..., TopAbs_EDGE, ...)` where a specific
     kind is required.
3. **`native/occt_bridge/CMakeLists.txt`**: no new OCCT module needed
   (`TopExp`, `TopTools_IndexedDataMapOfShapeListOfShape` are `TKernel`/
   `TKBRep`, already linked); registered `topology_test`.
4. **`native/occt_bridge/tests/topology_test.cpp`** (new): a box has
   exactly 8 unique vertices (plus the pre-existing 12 edges/6 faces,
   cross-checked here too); every 0..vertex_count index returns a real
   handle and out-of-range is rejected; every one of a box's 12 edges is
   adjacent to exactly 2 faces (manifold closed solid); the two adjacent
   faces returned are distinct real handles; out-of-range `edge_index`/
   `adjacent_index` are rejected; **`face_edges` (docs/plan/05 §3) is
   proven to already be covered** by applying the existing
   `shape_edge_count`/`_get_edge` to a Face handle (a square face reports
   exactly 4 boundary edges) rather than adding a new function for it; a
   bare face's own boundary edge is adjacent to exactly 1 face (itself,
   see "Key finding" below) in that face's own shape context;
   `edge_vertices` on an open line edge returns two vertices matching the
   input endpoints (verified via `shape_bounding_box` on each resulting
   single-vertex shape, since no point-coordinate query exists on a
   Vertex handle yet); `edge_vertices` on a closed full-circle edge
   returns two coincident vertices, not an error; `edge_vertices` rejects
   a non-Edge handle; a standalone (faceless) edge has 0 adjacent faces.
5. **`crates/cad-occt-bridge/src/ffi.rs`**: raw
   `aicad_occt_shape_vertex_count`/`_get_vertex`/`aicad_occt_edge_vertices`/
   `aicad_occt_shape_edge_adjacent_face_count`/`_get` declarations.
6. **`crates/cad-occt-bridge/src/lib.rs`**: `Shape::vertex_count(&self) ->
   KernelResult<usize>`, `Shape::get_vertex(&self, index: usize) ->
   KernelResult<Shape<'ctx>>`, `Shape::edge_vertices(&self) ->
   KernelResult<(Shape<'ctx>, Shape<'ctx>)>`,
   `Shape::edge_adjacent_face_count(&self, edge_index: usize) ->
   KernelResult<usize>`, `Shape::edge_adjacent_face(&self, edge_index:
   usize, adjacent_index: usize) -> KernelResult<Shape<'ctx>>`. 10 new
   Rust-level tests mirroring the native ones (vertex count, out-of-range
   rejection, per-edge adjacency count, adjacency out-of-range rejections
   ×2, `face_edges` coverage proof, open-edge endpoint match, closed-edge
   endpoint coincidence, non-Edge rejection, standalone-edge zero
   adjacency).

## Key finding: a bare Face's own boundary edge is adjacent to itself

The original test draft assumed a bare Face handle's own boundary edges
would report 0 adjacent faces within that face's own shape context (no
*other* face attached). Running the test showed 1, not 0.
`TopExp::MapShapesAndAncestors(shape, TopAbs_EDGE, TopAbs_FACE, map)`
treats the top-level shape passed in as its own ancestor when that
shape's own type matches the ancestor type being mapped — the same way a
bare Face handle already counts as its own single "face" for
`shape_face_count` purposes (a Face handle's `face_count()` is 1, not 0).
The test and this report were corrected to match the actual, verified
behavior rather than the original assumption — per AGENTS.md's evidence
rule, this is exactly the kind of claim that must be checked, not
presumed.

## Implementation decisions

- **No new function was added for `face_edges` or `topology_edges`/
  `topology_faces`** (docs/plan/05 §3) — AICAD-027/028's
  `shape_edge_count`/`_get_edge` and `shape_face_count`/`_get_face`
  already accept any shape kind via `LookupAnyKind`, so applying them to a
  Face handle (for `face_edges`) or any shape (for `topology_edges`/
  `topology_faces`) already implements those plan-doc entries.
  `topology_test.cpp` is the evidence this actually holds for `face_edges`,
  not just an assumption carried over from the header comment.
- **`edge_adjacent_face_count`/`_get` are indexed against `shape_handle`,
  not against a standalone edge handle** — an edge handle alone does not
  know which shape it was drawn from (the same handle could be shared, in
  principle, by two different lookups), so adjacency is queried as
  "the `adjacent_index`-th face of `shape_handle` adjacent to the edge at
  `edge_index` (per `shape_edge_count`'s own enumeration order over
  `shape_handle`)" — an (shape, index) pair, matching the existing
  `shape_get_edge`/`shape_get_face` indexing convention exactly, rather
  than inventing a new addressing scheme.
- **`edge_vertices` uses `TopExp::Vertices`' 2-output overload** (no
  `CumOri`), returning the edge's own "first"/"last" vertices in its own
  orientation sense — the simplest form sufficient for this task's own
  evidence (endpoint identity, not a full CumOri-aware topology walk).
- **No point-coordinate query was added for a Vertex handle.** Not
  required by AICAD-029's own scope (topology exploration, not geometric
  evaluation) — `topology_test.cpp`/the Rust tests instead confirm
  vertex identity indirectly via `shape_bounding_box` applied to the
  single-vertex shape, which recovers its coordinates (up to `Bnd_Box`'s
  own gap, see "Key finding" below on tolerances) without needing a new
  ABI function. A dedicated vertex-coordinate query is left for whichever
  later task's own evidence actually needs it (most likely folded into
  AICAD-030's measurement-query work, since `evaluate_curve`/
  `evaluate_surface`-style introspection is explicitly out of Stage-1's
  authorized scope per docs/plan/05 §8's own "New requirement" framing,
  which reads as later-stage work, not a Stage-1 gap).

## Tolerance finding: `Bnd_Box` enlarges even a single-point shape

`aicad_occt_shape_bounding_box` applied to a single Vertex shape does not
return an exactly-degenerate box (`min == max == the point`); it is
enlarged by the vertex's own confusion tolerance (`Precision::Confusion`,
empirically ~1e-7 per side, up to ~2e-7 between `min` and `max` on a
component). This was found by first writing (and failing) an assertion at
`1e-12` tolerance, then confirming the actual gap with a scratch program
before loosening the test tolerance to `1e-5` (comfortably above the
observed ~2e-7 gap while still a meaningful coordinate check) — not by
weakening the test to hide a bug, since the underlying `edge_vertices`
values were independently confirmed correct via the same scratch program
printed at full `%.12f` precision.

## Files changed
- Edited: `native/occt_bridge/include/aicad_occt_bridge.h`,
  `native/occt_bridge/src/aicad_occt_bridge.cpp`,
  `native/occt_bridge/CMakeLists.txt`, `crates/cad-occt-bridge/src/ffi.rs`,
  `crates/cad-occt-bridge/src/lib.rs`.
- Added: `native/occt_bridge/tests/topology_test.cpp`.

## Verification (exact commands/results)
```
$ cmake -S native/occt_bridge -B native/occt_bridge/build -DCMAKE_BUILD_TYPE=RelWithDebInfo
-- OCCT discovery: found OpenCASCADE 7.6.3

$ cmake --build native/occt_bridge/build -j$(nproc)
[100%] Built target topology_test   (plus all 12 pre-existing targets)

$ ctest --test-dir native/occt_bridge/build --output-on-failure
 1/13 occt_probe .................. Passed
 2/13 abi_boundary_test ........... Passed
 3/13 lifecycle_test .............. Passed
 4/13 box_cylinder_test ........... Passed
 5/13 transform_test .............. Passed
 6/13 curve_edge_wire_test ........ Passed
 7/13 face_test .................... Passed
 8/13 extrude_revolve_test ........ Passed
 9/13 sweep_loft_test ............. Passed
10/13 boolean_test ................ Passed
11/13 fillet_chamfer_test ......... Passed
12/13 shell_offset_test ........... Passed
13/13 topology_test ............... Passed
100% tests passed, 0 tests failed out of 13

$ ./native/occt_bridge/build/topology_test
... 23 PASS lines, 0 FAIL ...
topology_test: all checks PASSED

$ cargo fmt --all -- --check
(exit 0, after `cargo fmt --all` was run once to apply two formatting
fixes to newly-added Rust code)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, no warnings)

$ cargo build --workspace --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.09s

$ cargo test -p cad-occt-bridge
running 63 tests ... test result: ok. 63 passed; 0 failed

$ cargo test --workspace
(every crate) test result: ok, 0 failed
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1
(`rust-toolchain.toml`), CMake 3.28.3, OCCT 7.6.3
(`libocct-*-dev` 7.6.3+dfsg1-7.1build1) — unchanged from Batch 1A/1B/1C.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — **PASS**.
- Task-specific: native `ctest` (13/13) and `cargo test -p
  cad-occt-bridge` (63/63, 10 new) both pass, as shown above.

## Regressions added
None. All 12 pre-existing native test executables and all 53 pre-existing
Rust tests continue to pass unchanged.

## Limitations (honest capability boundaries)
- **No point-coordinate query on a Vertex handle yet** (see
  "Implementation decisions" above) — vertex identity is only checkable
  indirectly via `shape_bounding_box` in this task's own tests.
- **`edge_vertices` was only tested on a straight line edge and a full
  circle** — not on a BSpline/trimmed-curve edge, where "first"/"last"
  parametric-sense vertices could in principle interact with curve
  periodicity differently. Not required by this task's own evidence
  (no Stage-1 task yet constructs a BSpline edge).
- **Edge-to-face adjacency was only tested on a box (planar faces) and a
  standalone edge/face**, not on curved-face solids (a cylinder) or a
  non-manifold compound (an edge shared by 3+ faces, N>2). The
  implementation itself imposes no such restriction (the ABI doc comment
  explicitly allows N≠2), but no test exercises N>2 — left for Stage-1
  hardening mode's regression-expansion work, consistent with prior
  batches' equivalent limitation notes.
- **No `Query`/`.where(...)`-style filtering** (docs/plan/05 §6's
  preferred query/selection idiom) — this task, like AICAD-027/028's own
  raw index-based edge/face selection, implements only the raw/indexed
  access pattern the plan doc itself says is permitted "when an algorithm
  intentionally depends on the current transient topology enumeration."
  A query-based selection surface is later-stage (semantic-reference/
  query-language) work, not Stage-1 scope.

None of these limitations required forcing an artificial success or
weakening a test to hide a failure — every claim in this report and its
tests is backed by an actual passing run, and the one incorrect
assumption caught during testing (the "0 adjacent faces" case) was
corrected to match verified behavior rather than adjusted to match the
original assumption.
