# AICAD-027 — Implement fillet and chamfer

## Objective
Implement `fillet`/`chamfer` per `project/TASKS.yaml` (AICAD-027) and
`docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s `fillet`/`chamfer` catalog
entries (which round/chamfer "selected edges"), at Stage-1 raw-kernel
fidelity. This is the third task in Batch 1C. Unlike every prior
operation, this one requires a genuine architectural decision — selecting
*which* edges to round/chamfer — that this report resolves by implementing
an already-approved design rather than inventing a new one.

## Dependencies checked
AICAD-026 (boolean union/cut/intersect) — complete, commit `7b235ef`.

## The edge-selection question (why this needed care)

`docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s `fillet`/`chamfer` entries take
an `edges: Query<EdgeRef>|List<EdgeRef>` parameter — `EdgeRef` is a
persistent *semantic* reference, part of the Stage-4 semantic-reference/
topological-naming system (`OWNER_DECISIONS.md` D7/D8,
`project/CURRENT_STAGE.md`'s Stage-4 gate) that does not exist yet. Stage 1
cannot implement that high-level contract directly.

`docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` §5-6 already specifies,
as approved plan content (not an open `OWNER_DECISIONS.md` item), exactly
the fallback for this situation: raw, index-based topology access
(`raw_edge(f, 2)`), explicitly documented as ephemeral, epoch-bound, and
never a durable reference — "Indices are permitted only when an algorithm
intentionally depends on the current transient topology enumeration." This
task implements exactly that already-approved pattern rather than
selecting among unresolved alternatives, so it did not trigger the
`escalate_if: an unresolved architecture alternative must be selected`
condition.

## What was done

1. **`native/occt_bridge/include/aicad_occt_bridge.h`**: added
   `aicad_occt_shape_edge_count(context, handle, out_count)`,
   `aicad_occt_shape_get_edge(context, handle, index, out_edge_handle)`
   (0-based, ephemeral, epoch-bound — matches `docs/plan/05`'s own raw-
   access contract), `aicad_occt_fillet(context, shape_handle, edges[],
   edge_count, radius, out_handle)`, and `aicad_occt_chamfer(context,
   shape_handle, edges[], edge_count, distance, out_handle)` (symmetric
   chamfer only — no two-distance or distance-angle variant yet).
2. **`native/occt_bridge/src/aicad_occt_bridge.cpp`**:
   - `edge_count`/`get_edge` use `TopExp::MapShapes(shape, TopAbs_EDGE,
     map)` — **verified empirically, not assumed**, that a plain
     `TopExp_Explorer(shape, TopAbs_EDGE)` traversal revisits each edge
     once per adjacent face (24 visits for a box's 12 actual edges; a
     throwaway probe confirmed this before writing the real
     implementation), so `TopExp::MapShapes`'s de-duplicated
     `TopTools_IndexedMapOfShape` is the correct primitive for a "unique
     edge count"/"the Nth unique edge" contract.
   - `fillet`/`chamfer` build `BRepFilletAPI_MakeFillet`/`MakeChamfer`
     from the target shape, `.Add(radius/distance, edge)` for each
     selected edge (each looked up via the existing `LookupTyped(...,
     TopAbs_EDGE, ...)`, so a non-edge handle is rejected exactly like
     every other wrong-kind argument in this bridge), then
     `.Build()`/`.IsDone()`.
   - The shared `LookupBooleanOperand` helper from AICAD-026 was renamed
     to `LookupAnyKind` (an internal-only rename, no behavior change) —
     it is now used by fillet/chamfer's target-shape lookup too, not only
     booleans.
3. **`native/occt_bridge/CMakeLists.txt`**: added `TKFillet` (provides
   `BRepFilletAPI_MakeFillet`/`MakeChamfer`) as a new required-module
   check; registered `fillet_chamfer_test`.
4. **`native/occt_bridge/tests/fillet_chamfer_test.cpp`** (new): a box has
   exactly 12 unique edges (not 24); `get_edge` rejects an out-of-range
   index; **fillet ALL 12 edges** of a box with radius `r` matches the
   analytic "rounded box" (Minkowski-sum-with-a-ball) volume formula
   `Lx·Ly·Lz + 2r(Lx·Ly+Ly·Lz+Lz·Lx) + πr²(Lx+Ly+Lz) + (4/3)πr³` (Lx/Ly/Lz
   = box dimensions inset by `r`); **chamfer exactly ONE geometrically
   identified edge** (found by its bounding box, not an assumed index)
   matches `box_volume - (d²/2)·edge_length` (a single symmetric-chamfered
   edge, with neither neighboring edge also modified, removes a clean
   triangular prism the full length of that edge); `edge_count` works on
   a Compound (a disjoint two-box union has 12+12=24 edges), proving
   fillet/chamfer's target-kind-unrestricted contract; adversarial
   rejections (null/zero edge list, non-positive radius/distance,
   non-edge handle); an oversized-fillet-radius probe (see "Adversarial
   cases and findings" below).
5. **`crates/cad-occt-bridge/src/ffi.rs`**: raw
   `aicad_occt_shape_edge_count`/`_get_edge`/`aicad_occt_fillet`/
   `_chamfer` declarations.
6. **`crates/cad-occt-bridge/src/lib.rs`**: `Shape::edge_count(&self) ->
   KernelResult<usize>`, `Shape::get_edge(&self, index: usize) ->
   KernelResult<Shape<'ctx>>`, `Shape::fillet(&self, edges: &[&Shape<'ctx>],
   radius: f64) -> KernelResult<Shape<'ctx>>`, `Shape::chamfer(&self,
   edges: &[&Shape<'ctx>], distance: f64) -> KernelResult<Shape<'ctx>>`. 7
   new Rust-level tests mirroring the native ones one-to-one (edge count,
   out-of-range rejection, fillet-all rounded-box volume, single-edge
   chamfer volume, non-edge-handle rejection for both operations,
   non-positive-radius rejection, Compound acceptance).

## Adversarial cases and findings

Per AGENTS.md's adversarial-case requirement ("impossible fillets"
explicitly named), an oversized fillet radius (`10.0` on a `1×1×1` box,
where a geometrically valid fillet requires roughly `radius <
min(dimension)/2`) was probed rather than assumed to fail a particular
way. Finding: `BRepFilletAPI_MakeFillet::IsDone()` returns false and this
bridge correctly reports `AICAD_OCCT_ERR_OPERATION_FAILED` — the request
is cleanly rejected as a normal operation failure, not silently clamped to
a smaller radius and not a crash/hang. This matches the existing
`OPERATION_FAILED` semantics used throughout this bridge for "OCCT
reported the requested geometric operation could not be completed."

## Implementation decisions

- **Edge selection is raw and index-based** (`get_edge(shape, index)`),
  not a persistent reference — see "The edge-selection question" above.
  This is the one and only mechanism this task adds for identifying
  "which edge"; there is no query/predicate layer (`docs/plan/05`'s own
  §6 "prefer queries" is explicitly a *higher-fidelity, later* target this
  raw form does not need to anticipate).
- **`fillet`/`chamfer`'s target `shape_handle` is not restricted to
  `TopAbs_SOLID`** — the same reasoning as AICAD-026's boolean operand
  decision: `BRepFilletAPI_MakeFillet`/`MakeChamfer` both accept a generic
  `TopoDS_Shape`, and a boolean result (a Compound) must remain
  fillet-able without first being re-wrapped into a Solid, which OCCT's
  boolean algorithms never actually produce (AICAD-026's own empirical
  finding). Verified here too: filleting/chamfering a Compound is
  explicitly tested and works.
- **Individual edge arguments to `fillet`/`chamfer` ARE restricted to
  `TopAbs_EDGE`** via the existing `LookupTyped` helper — unlike the
  *target* shape, each *edge selector* has exactly one topological kind
  that makes sense, matching this bridge's established
  "restrict only when the underlying OCCT operation itself requires a
  specific kind" rule (extrude/revolve/sweep's own precondition pattern).
- **Chamfer exposes only the single-distance symmetric form**
  (`BRepFilletAPI_MakeChamfer::Add(Standard_Real, TopoDS_Edge)`), not the
  two-distance, distance-angle, or per-face variants OCCT also supports.
  This is the minimal supported form per RFC-0002 §3; a caller-selectable
  asymmetric chamfer is a natural, non-breaking future extension of the
  same ABI function once a task's plan reference specifically needs it.
- **`LookupBooleanOperand` was renamed to `LookupAnyKind`** (a private,
  internal-only identifier) because it is now shared by fillet/chamfer's
  target-shape lookup, not only booleans — an implementation-only rename
  preserving public semantics, within AGENTS.md's "autonomously allowed"
  scope.

## Files changed
- Edited: `native/occt_bridge/include/aicad_occt_bridge.h`,
  `native/occt_bridge/src/aicad_occt_bridge.cpp`,
  `native/occt_bridge/CMakeLists.txt`, `crates/cad-occt-bridge/src/ffi.rs`,
  `crates/cad-occt-bridge/src/lib.rs`.
- Added: `native/occt_bridge/tests/fillet_chamfer_test.cpp`.

## Verification (exact commands/results)
```
$ rm -rf native/occt_bridge/build && cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3   (TKFillet discovery check passes)

$ cmake --build native/occt_bridge/build -j$(nproc)
[100%] Built target fillet_chamfer_test   (plus all pre-existing targets)

$ ctest --test-dir native/occt_bridge/build --output-on-failure
1/11 occt_probe ................. Passed
2/11 abi_boundary_test .......... Passed
3/11 lifecycle_test ............. Passed
4/11 box_cylinder_test .......... Passed
5/11 transform_test ............. Passed
6/11 curve_edge_wire_test ....... Passed
7/11 face_test .................. Passed
8/11 extrude_revolve_test ....... Passed
9/11 sweep_loft_test ............ Passed
10/11 boolean_test ............... Passed
11/11 fillet_chamfer_test ........ Passed
100% tests passed, 0 tests failed out of 11

$ ./native/occt_bridge/build/fillet_chamfer_test
... 22 PASS lines, 1 INFO line (oversized-fillet finding above), 0 FAIL ...
fillet_chamfer_test: all checks PASSED

$ g++ -std=c++17 -Wall -Wextra -Wpedantic -c native/occt_bridge/src/aicad_occt_bridge.cpp \
    -I native/occt_bridge/include -isystem /usr/include/opencascade -o /tmp/w027.o
(no warnings)

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, no warnings)

$ cargo build --workspace --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.23s

$ cargo test -p cad-occt-bridge
running 47 tests ... test result: ok. 47 passed; 0 failed

$ cargo test --workspace
(every crate) test result: ok, 0 failed
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1
(`rust-toolchain.toml`), CMake 3.28.3, OCCT 7.6.3
(`libocct-modeling-algorithms-dev` 7.6.3+dfsg1-7.1build1, providing
`libTKFillet.so.7.6.3`) — unchanged from Batch 1A/1B/AICAD-025/026.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — **PASS**.
- Task-specific: native `ctest` (11/11) and `cargo test -p
  cad-occt-bridge` (47/47) both pass, as shown above.

## Regressions added
None. All 10 pre-existing native test executables and all pre-existing
Rust tests continue to pass unchanged.

## Limitations / follow-up
- No query/predicate edge-selection layer exists (`edge.convex`,
  `edge.length > 10mm`, etc., per `docs/plan/05` §6) — only raw
  index-based selection. This is explicitly the plan's own lower-fidelity
  fallback, not a gap this task needed to close; a query layer belongs
  with Stage 3/4's semantic-reference system.
- Chamfer is symmetric-distance-only; no two-distance, distance-angle, or
  per-face chamfer variant is exposed.
- Variable-radius fillet (`Fn<EdgeRef,Length>` in the high-level catalog)
  is out of scope — only constant radius/distance per call.
- This task's "impossible fillet" adversarial probe used one box/radius
  combination; a systematic sweep of the fillet-failure boundary (e.g.
  exactly which radius-to-dimension ratio starts failing, or filleting
  edges that meet at especially sharp dihedral angles) was not performed
  here and would fit Stage-1 hardening mode's bounded-fuzzing work
  (post-AICAD-037) better than this task's scope.
- Fillet/chamfer were only tested against a box's axis-aligned edges;
  behavior on curved edges (e.g. a cylinder's circular edge) or edges
  from extrude/revolve/sweep/loft results was not exercised here.
