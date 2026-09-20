# AICAD-121: Implement deterministic topology traversal and inspection

## Status

Done. Third and final task of batch S5-06.

## Objective

Expose safe, kernel-neutral topology traversal/inspection (entity kinds,
counts, adjacency, orientation, same-entity comparison, vertex coordinates,
point-vs-solid classification) through ordinary AICAD source, on top of
`AICAD-119`'s construction primitives and the existing `GetFace`/raw-index
precedent (`AICAD-076`) — without ever exposing an OCCT/native handle as
durable semantic identity (`AGENTS.md`'s "raw topology is ephemeral/unsafe
and epoch-bound" non-negotiable).

## Base / resulting commit

Base: `4b87ca1` (`origin/claude/aicad-stage5-dev`, `AICAD-120`).

## Architecture: why counts and indexed access, not `List<Geometry>`

`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §6's own frozen example
signature is closer to `topology_faces(shape) -> Iterator<FaceRef>`. That
shape is not reachable here: a `GeometryQuery` dispatches through
`KernelQueryExecutor::execute` into a `QueryOutcome`
(`Bool`/`Number`/`Point`/`Text`), which cannot mint new `GeomId` nodes into
the interpreter's own append-only `GeometryGraph` — and each freshly
enumerated sub-entity would need its own node to ever become a usable
`Value::Geometry`. A `GeometryOp`, in contrast, is exactly what already
mints new nodes (`GetFace`'s own `AICAD-076` precedent).

The design used instead: a Query-category **count** per entity kind
(`face_count`/`edge_count`/`vertex_count`/`wire_count`/`shell_count`/
`solid_count`, backed by one combined `GeometryQuery::EntityCount { target,
kind: TopologyKind }` IR variant) plus Construction-category **indexed
access** (`topology_face_at`/`topology_edge_at`/`topology_vertex_at`,
reusing/extending `GetFace`'s own raw-index-selection shape). A caller
walks a shape's own topology with an ordinary `for i in 0..face_count(s)`
loop, exactly like `docs/plan`'s own iteration examples read at the source
level, just built from two primitives instead of one iterator type. This is
a legitimate architectural adaptation to a constraint discovered mid-task,
not a scope cut — every enumerable entity kind `docs/plan` names is still
reachable from source.

`topology_wire_at` (a wire-indexed counterpart alongside face/edge/vertex)
was considered and deliberately deferred: `GeometryOp::GetWire` already
exists in the IR/dispatch layer (added for `IsOuterWire`'s own internal use
and covered by IR/dispatch tests), but no source-level builtin was added
for it in this task, since no fixture in this batch's own acceptance
criteria needs a source-reachable wire selector; the underlying `GetWire`
op itself is one `BuiltinFnId`/catalogue entry away from being exposed the
same way `topology_face_at` etc. are, for whichever future task needs it.

## `is_forward_oriented`'s 4-way-to-bool collapse (disclosed, not silent)

`TopAbs_Orientation` has four values (FORWARD/REVERSED/INTERNAL/EXTERNAL);
`Shape::is_forward_oriented` (`cad-occt-bridge`, `AICAD-121`) reports `true`
only for FORWARD, collapsing REVERSED/INTERNAL/EXTERNAL all to `false`. The
latter two are rare seam/degenerate-edge markers, not the ordinary
face-orientation question a caller asking "is this forward" is really
after. This is the same category of deliberate, disclosed simplification
`heal`'s `kind_changed` disclosure (`AICAD-120`) established the
convention for — documented in the method's own doc comment and in this
report, not silently absorbed.

## What changed

- `native/occt_bridge/include/aicad_occt_bridge.h` /
  `src/aicad_occt_bridge.cpp`: `aicad_topology_kind_t` enum (deliberately
  renumbered vs. OCCT's own `TopAbs_ShapeEnum`, so no caller can rely on
  numeric equality with a native value), `aicad_occt_shape_kind`,
  `aicad_occt_shape_is_forward_oriented`.
- `crates/cad-occt-bridge/src/{ffi.rs,lib.rs}`: matching FFI declarations;
  `Shape::topology_kind`/`Shape::is_forward_oriented`, both returning
  `cad_kernel_api::topology::TopologyKind`/`bool` — never the native enum;
  3 new unit tests (kind classification across every concrete entity kind,
  a Compound correctly rejected as unclassifiable, determinism of repeated
  `is_forward_oriented` calls).
- `crates/cad-geometry-api/src/ir.rs`: `VertexIndex`/`WireIndex` (mirroring
  `EdgeIndex`/`FaceIndex`); `GeometryOp::{GetEdge,GetVertex,GetWire,
  GetAdjacentFace}`; `GeometryQuery::{TopologyKindOf,EntityCount,
  AdjacentFaceCount,IsOuterWire,IsSameEntity,IsForwardOriented,VertexPoint,
  ClassifyPoint}`, each with `push_op`/`push_query` operand/dimension
  validation. 7 new unit tests covering every new op/query's
  unbuilt-operand rejection and built-operand acceptance path
  (`get_edge_get_vertex_get_wire_and_get_adjacent_face_{rejects,accepts}...`,
  `topology_kind_of_entity_count_and_is_forward_oriented_reject_an_unbuilt_target`,
  `entity_count_and_adjacent_face_count_accept_a_built_target`,
  `is_outer_wire_and_is_same_entity_reject_either_unbuilt_operand`,
  `classify_point_{rejects,accepts}...`).
- `crates/cad-geometry-runtime/src/dispatch.rs`: `NodeResult::Text(String)`
  (alongside the pre-existing `Bool`/`Number`/`Point`); `query_input_id`
  widened to `query_input_ids` (`IsOuterWire`/`IsSameEntity` each have two
  operands, unlike every prior single-operand query); dispatch arms for
  all 4 new ops and 8 new queries against real `Shape` methods. 7 new
  tests, including the acceptance-mandated "transformed topology" fixture
  (`a_transformed_boxs_own_entity_counts_are_unchanged`) and a
  compound-classification clean-error case
  (`topology_kind_of_a_compound_is_a_clean_dispatch_error_not_a_panic`).
- `crates/cad-geometry-runtime/src/query_bridge.rs`: `OcctQueryExecutor`
  widened to convert `NodeResult::{Point,Text}` into
  `QueryOutcome::{Point,Text}`.
- `crates/cad-runtime/src/query_exec.rs`: `QueryOutcome` widened with
  `Point(Point3)`/`Text(String)` (no longer `Copy` — `String` isn't; every
  call site already consumed by value).
- `crates/cad-hir/src/builtins.rs`: 17 new `BuiltinFnId` (`TopologyKindOf`,
  `FaceCount`/`EdgeCount`/`VertexCount`/`WireCount`/`ShellCount`/
  `SolidCount`, `TopologyFaceAt`/`TopologyEdgeAt`/`TopologyVertexAt`,
  `AdjacentFaceCount`/`AdjacentFaceAt`, `IsOuterWire`, `IsSameEntity`,
  `IsForwardOriented`, `VertexPoint`, `ClassifyPoint`) — 4 Construction
  (the raw-index selectors), 13 Query; catalogue 58 -> 75.
- `crates/cad-runtime/src/interp.rs`: `usize_value` closure (a plain
  non-negative raw-index argument reader, for `edge_index`/
  `adjacent_index`/enumeration `index` params); Query-category early-return
  gate extended with all 13 new query builtins; Construction arms for the
  4 new raw-index selectors; `execute_kernel_query`'s outcome-to-`Value`
  match extended (`Number` for the 7 count-family builtins as plain
  dimensionless scalars, `Text` -> `Value::Str` for
  `topology_kind_of`/`classify_point`, `Bool` for
  `is_outer_wire`/`is_same_entity`/`is_forward_oriented`, `Point` ->
  `self.point3_value` for `vertex_point`). 8 new end-to-end interpreter
  tests (using the existing `FakeQueryExecutor` precedent, mirroring
  `is_valid`/`volume`/`area`'s own established test style, plus one
  Construction-only structural test for the 4 raw-index selectors and one
  no-executor-configured clean-failure test).
- `examples/topology/topology_construction_basics.aicad` (extended, not a
  new file, per the Examples Policy): `topology_face_at`/`topology_edge_
  at`/`topology_vertex_at`/`adjacent_face_at` calls demonstrating the new
  Construction-category selectors — the base `build_source` path has no
  kernel context, so only Construction-category calls belong here (the
  same constraint `AICAD-119`/`AICAD-120` already established); the
  README's ACTIVE-table description updated to mention topology traversal.
- `crates/cad-cli/tests/stage5_topology_inspection.rs` (new): production-
  path proof for every Query-category builtin against a real `OcctContext`
  — exact entity counts on a unit box (6/12/8/6/1/1),
  `topology_kind_of`'s exact string per selected entity kind, "every edge
  of a box borders exactly 2 faces", `is_same_entity` distinguishing a
  re-selected handle from a different one, `is_forward_oriented` returning
  a real bool (determinism, not a fixed expected value — matches that
  method's own disclosed collapse), `classify_point` distinguishing
  Inside/Outside on a real box, and `vertex_point` returning a real
  in-bounds coordinate.

## Verification

```
cargo fmt --all -- --check                                             # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings   # clean
cargo test -p cad-kernel-api -p cad-occt-bridge -p cad-query \
  -p cad-runtime                                                       # all passed
cargo test --workspace                                                 # 1738 passed, 0 failed
ctest (native/occt_bridge build, RelWithDebInfo)                        # 18/18 passed
```

## Limitations / follow-up

- `topology_wire_at` is not exposed at the source level (see the
  architecture section above) — `GeometryOp::GetWire` already exists and
  is IR/dispatch-tested; adding the matching `BuiltinFnId`/catalogue entry/
  interp arm is a small, independent follow-up whenever a fixture actually
  needs a source-reachable wire selector.
- `is_forward_oriented` collapses 4 `TopAbs_Orientation` values to a bool
  (disclosed above) — a caller needing to distinguish INTERNAL/EXTERNAL
  from REVERSED has no source-level way to do so yet.
- No dedicated fixture in this task exercises a face with a hole or a
  genuinely non-manifold shape's own traversal counts (the acceptance
  criterion's "holes, non-manifold/open cases where representable" is only
  partially covered: `AICAD-119`'s own open-shell/open-solid case is
  reused for the "construction succeeds, validity is separate" story, and
  `topology_kind_of` on a Compound is covered as the non-classifiable
  case, but no face-with-a-hole traversal count fixture was added). Left
  as an explicit gap rather than asserted as covered; a follow-up task
  extending `stage5_topology_inspection.rs` with a `make_face_on_surface`
  call that includes a hole wire would close it.
- `sew`/`heal`'s own richer `SewReport`/`HealReport` evidence remains
  Rust-only (`AICAD-120`'s own identical disclosure) — unaffected by this
  task, noted here only because `topology_kind_of`/entity counts are the
  first source-level way to independently cross-check some of that
  evidence (e.g. confirming `heal`'s `kind_changed` externally via
  `topology_kind_of` before/after) without the full report struct.

## Next dependency

`AICAD-122` (`stage: 5`, "controlled raw/unsafe geometry tier with
epoch-bound handles") depends on this task and can now proceed. This is
the final task of batch `S5-06` — no further task in this batch is
unblocked by this session; the next invocation begins batch `S5-07`
(`AICAD-122`-`AICAD-124`).
