# AICAD-119: Implement general topology construction and exact validity evidence

## Status

Done. First task of batch S5-06.

## Objective

Complete the vertex->edge->wire->face->shell->solid construction pipeline
(`docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` §3): the missing base
case (vertex), a face on an explicit non-planar quadric surface, shell
assembly, and solid-from-shell, plus compound grouping — all as real
kernel-backed `GeometryOp`s, exposed to `.aicad` source for the first time
as ordinary `RuntimeBuiltin` calls, with validity kept a strictly separate
evidence question from construction success throughout (Stage-1 kernel
policy #14, `AGENTS.md`'s "Exact B-rep is canonical compiled geometry").

## Base / resulting commit

Base: `e313b50` (`origin/claude/aicad-stage5-dev`, `AICAD-118`, synced from
`branch/admiring-ramanujan-nhwypv` this session).

## Architecture: bounded scope, reusing existing machinery

`make_line_edge`/`make_circle_wire`/`make_arc_edge`/`make_wire_from_edges`/
`Shape::make_face` (planar) already existed since `AICAD-022`/`023` but had
**no source-language exposure at all** — `cad_hir::builtins` never named
them. This task's `make_edge`/`make_wire`/`make_face` builtins are their
first exposure; no new native/kernel work was needed for those three.

Genuinely new native/kernel capability (native/occt_bridge + cad-occt-bridge
+ `cad_geometry_api::ir::GeometryOp`/`SurfaceSpec`/`FaceOrientation`):

- `make_vertex` (`BRepBuilderAPI_MakeVertex`).
- `make_face_on_surface`, bounded to the 5 elementary quadric families
  `SurfaceSpec` covers (`Plane`/`Cylinder`/`Cone`/`Sphere`/`Torus` —
  `BRepBuilderAPI_MakeFace(gp_*, wire, Inside)`, one C++ template shared
  across all 5 gp_ types), with holes and one outer-wire orientation flag.
  Bezier/BSpline/Trimmed surfaces are `RuntimeError::UnsupportedTopology
  Construction` — no matching kernel op exists for them (a structural
  limit, mirroring `AnalyticSurface::offset`'s own precedent).
- `make_shell` (`BRep_Builder::MakeShell`/`Add` per face) — a **structural
  container only**, deliberately no sewing/gap-closing (`AICAD-120`'s job):
  faces that do not already share identical edges/vertices produce an
  open/non-manifold shell, not a silently repaired one.
- `make_solid` (`BRepBuilderAPI_MakeSolid`, with void/cavity shells) —
  verified empirically, not assumed: OCCT does **not** require its input
  shell closed; it reports done (and this bridge returns
  `AICAD_OCCT_OK`) even for an open shell, producing a structurally-real
  but invalid solid. This is Stage-1 kernel policy #14 in its starkest
  form.
- `compound` (`BRep_Builder::MakeCompound`/`Add`, any mix of kinds).
- `make_edge(curve: Curve)` bridges the pure-value `AnalyticCurve` world
  (`AICAD-109`-`111`, never touching the kernel) into real kernel
  edges/wires for the first time: `Circle` -> `CircleWire` (a closed wire,
  disclosed rather than pretended uniform — a full circle has no natural
  single start/end point for OCCT's edge model); `Arc` -> `ArcEdge` (its 3
  defining points obtained via `AnalyticCurve::evaluate`, reusing already-
  tested trigonometry); `Trimmed{base: Line, ..}` -> `LineEdge`. An
  untrimmed `Line`, `Ellipse`, `Bezier`, and `BSpline` are `Unsupported`
  (no matching op exists).
- `ValidationReport`/`aicad_validation_report_t` extended with
  `invalid_shell_count`/`invalid_solid_count` (previously only vertex/edge/
  wire/face) — the first shell/solid-producing construction ops needed
  this breakdown to exist.

`Geometry` remains the single flattened HIR/source type for every kernel
shape regardless of topological kind (matching the pre-existing `box`/
`union`/etc. precedent) — no new HIR type was introduced; `docs/plan/
05_...`'s per-kind `Vertex`/`Edge`/.../`Solid` type vocabulary remains
aspirational language design, not literal source to replicate (recorded
already by the prior architecture survey for this batch).

## What changed

- `native/occt_bridge/include/aicad_occt_bridge.h` /
  `src/aicad_occt_bridge.cpp`: 9 new `aicad_occt_make_*` functions (vertex,
  face-on-{plane,cylinder,cone,sphere,torus}, shell, solid, compound);
  `aicad_validation_report_t` gains 2 fields; `aicad_occt_shape_validate`
  counts shell/solid invalidity too.
- `crates/cad-occt-bridge/src/{ffi.rs,lib.rs}`: matching `unsafe extern`
  declarations; safe wrappers `OcctContext::{make_vertex,make_shell,
  make_compound}`, `Shape::{make_face_on_plane,_cylinder,_cone,_sphere,
  _torus,make_solid}`; `ValidationReport` extended; 10 new unit tests
  (including the "open shell -> solid succeeds structurally, `validate()`
  reports it invalid" case, empirically discovered while writing this
  task, not assumed up front).
- `crates/cad-geometry-api/src/ir.rs`: `SurfaceSpec`, `FaceOrientation`
  (named to avoid colliding with the pre-existing, unrelated
  `surface::Orientation` trim-winding type), `GeometryOp::{MakeVertex,
  MakeFaceOnSurface,MakeShell,MakeSolid,Compound}` with `push_op`
  structural validation; 10 new unit tests.
- `crates/cad-geometry-runtime/src/dispatch.rs`: dispatch + incremental-
  rebuild input-id arms for all 5 new ops; 5 new tests through the real
  `dispatch_graph` path (exact box-face-reassembly volume match; the
  open-shell/solid disclosed-invalid case at the dispatch layer).
- `crates/cad-hir/src/builtins.rs`: 8 new `BuiltinFnId`
  (`MakeVertex,MakeEdge,MakeWire,MakeFace,MakeFaceOnSurface,MakeShell,
  MakeSolid,Compound`), all `Construction` category; catalogue 48 -> 56.
- `crates/cad-runtime/src/{interp.rs,error.rs}`: dispatch arms;
  `curve_to_edge_op`/`surface_to_spec` conversion helpers;
  `RuntimeError::UnsupportedTopologyConstruction` (`RUNTIME-E143`); 12 new
  end-to-end interpreter tests (positive construction + every disclosed
  `Unsupported` family case).
- `examples/topology/topology_construction_basics.aicad` (new, registered
  in `active_examples.rs`/`README.md`): the full pipeline, teaching-style.
- `crates/cad-cli/tests/stage5_topology_construction.rs` (new): production-
  path proof via `ParametricBuildSession` — exact planar-face area,
  real-kernel-shape evidence for every Construction binding, and the
  central "construction succeeds, `is_valid` reports the honest negative"
  proof for both the open shell/solid case and a genuinely degenerate
  face-on-cylinder trim (found empirically, see Limitations).

## Verification

```
cargo fmt --all -- --check                                            # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings  # clean
cargo test -p cad-kernel-api -p cad-occt-bridge -p cad-geometry-runtime \
  -p cad-validation -p cad-cli                                        # all passed
cargo test --workspace                                                # 1686 passed, 0 failed
ctest (native/occt_bridge build, RelWithDebInfo)                       # 18/18 passed
```

Native smoke tests (throwaway, not committed) independently confirmed
before writing the Rust wrappers: unit-cube shell/solid volume = 1.0
exactly when built from a box's own real (shared) faces; `is_valid=0,
invalid_solid_count=1` for a solid built from one standalone face.

## Limitations / follow-up

- **No sewing/healing** — `make_shell` never merges coincident-but-
  distinct edges/vertices. Empirically confirmed two ways this task did
  *not* originally expect: (1) even when every face of a hand-built cube
  explicitly reused the *same* 12 shared edge handles (not merely
  coincident coordinates), the resulting shell still failed
  `BRepCheck_Analyzer`'s shell-level check (`invalid_shell_count: 1`,
  every individual vertex/edge/wire/face reported valid) — shared
  topology alone is not sufficient; only a real solid's own already-
  connected faces (reassembled via `GetFace`, proven in
  `dispatch.rs`'s own test) currently yields a valid multi-face shell.
  Root-causing the exact missing `BRepCheck` criterion (likely an
  orientation-consistency check `BRepCheck_Analyzer` applies at the shell
  level, not merely per-subshape) is left to `AICAD-120`, whose sewing
  pass needs to solve this class of problem anyway. (2) `make_face_on_
  surface` on a cylinder/cone/sphere/torus can construct successfully
  from a 3D wire that is geometrically ON the surface but produces a
  *degenerate* (zero-area) trimmed region — `BRepBuilderAPI_MakeFace
  (gp_*, wire, Inside)` does not itself compute/attach a correct 2D
  parametrization for every 3D-coincident wire; this needs further
  investigation (likely `BRepLib`/`ShapeFix` pcurve-repair machinery) that
  was out of this task's bounded scope. Both are disclosed in the new
  ACTIVE example's own doc comments and independently proven (not merely
  asserted) by `stage5_topology_construction.rs`.
- `make_face_on_surface` has no source-level control over
  `FaceOrientation::Reversed` yet (always builds forward) — a narrower
  scope than the underlying `GeometryOp`, not a kernel-layer gap.
- `cad-kernel-api::topology`'s `TopologyKind`/`ClassifiedShape`
  (`AICAD-108` prep) remain unused — deliberately `AICAD-121`'s job
  (inspection/classification), not this task's.
- No lineage/provenance evidence is emitted by any of these construction
  ops (`cad_geometry_api::operation_report::OperationReport`) — matches
  this task's own acceptance scope (construction + validity, not
  lineage); `AICAD-125` is where topology-changing-operation lineage for
  the reference system is due.

## Next dependency

`AICAD-120` (sewing/healing with bounded tolerance policy) depends
directly on this task's `make_shell`/`make_solid` and inherits the two
disclosed limitations above as its own starting problem statement.
