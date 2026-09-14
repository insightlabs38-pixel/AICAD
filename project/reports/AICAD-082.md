# AICAD-082: Implement geometry predicates: planar/cylindrical/radius/area/length/normal/axis

## Status

Done. First task of Batch S4-01.

## Objective

Give `crate::predicate::GeometryPredicate` (`AICAD-081`, representation
only) real evaluation semantics against a live `cad-occt-bridge` build —
"does `planar`/`cylindrical`/.../`radius == ...`/`normal ~= ...` hold for
this specific face/edge candidate?" No query executor, ranking, or
resolver — that remains `AICAD-088`+.

## Base / resulting commit

- Base: `fbff442` (this session's `claude/aicad-stage4-dev`, created this
  invocation from `origin/branch/intelligent-ramanujan-tegtaz` — see
  `project/SESSION_HANDOFF.md`'s "Working-branch note" resolution below).
- This task's commit: see `git log` (`AICAD-082` commit).

## What was implemented

Geometry predicates need face/edge analytic classification
(surface/curve family, radius, axis, normal) that did not exist anywhere
in the kernel-neutral stack before this task — `cad-kernel-api` only had
opaque handles and pure math value types, and `cad-occt-bridge` exposed
only aggregate area/length/volume, never per-face/edge family or
geometric parameters. This task adds that missing layer, then a
`cad-query` evaluator on top of it:

- **`native/occt_bridge`** (`aicad_occt_bridge.h`/`.cpp`) — 7 new ABI
  functions: `aicad_occt_shape_surface_type`/`_face_radius`/`_face_axis`/
  `_face_normal` (via `BRepAdaptor_Surface`), `_curve_type`/`_edge_radius`/
  `_edge_axis` (via `BRepAdaptor_Curve`). Two new AICAD-owned, ABI-stable
  enums (`aicad_surface_kind_t`, `aicad_curve_kind_t`) — never OCCT's own
  `GeomAbs_*` values re-exported, matching this header's own top-of-file
  "kernel-neutral at the ABI" contract. `face_radius`/`face_axis` are
  defined only for the surface kinds that have a single well-defined
  value (Cylinder/Sphere/Torus for radius; Cylinder/Cone/Torus for axis);
  `edge_radius`/`edge_axis` only for Circle (an Ellipse has two radii,
  not one). Every other combination fails with
  `AICAD_OCCT_ERR_INVALID_ARGUMENT` rather than guessing which value to
  report.
- **`cad-occt-bridge`** (`src/lib.rs`) — safe wrappers
  (`Shape::surface_type`/`face_radius`/`face_axis`/`face_normal`/
  `curve_type`/`edge_radius`/`edge_axis`) plus a kernel-neutral
  `SurfaceKind`/`CurveKind` enum pair (never leaking an OCCT type above
  the adapter, per RFC-0002 §3). 19 new unit tests (box faces/edges
  classify as Plane/Line; a capped cylinder has exactly one Cylinder face
  and exactly 2 Circle rim edges; radius/axis match the constructed
  cylinder's own dimensions; the wrong-entity-kind/wrong-surface-kind
  rejections each return `InvalidArgument`).
- **`cad-occt-bridge`** also gained `Shape::is_same` (`AICAD-083`
  preparation — see that task's report for why it belongs to this same
  native round) via `TopoDS_Shape::IsSame`.
- **`cad-query`** (new `src/eval.rs`) — `evaluate_geometry(predicate,
  candidate) -> EvalResult<bool>`, a `Candidate<'ctx>` wrapping an
  `EntityKind` + live `cad_occt_bridge::Shape<'ctx>`, and `EvalError`
  (`Kernel`/`NoEvidence`/`NotYetSpecified`/`InvalidInput`). Surface-family
  predicates (`Planar`/.../`Bspline`) apply only to Face candidates
  (`false`, not an error, for any other entity kind — the same way an
  ordinary filter predicate never matches an incomparable value).
  `Radius`/`Axis` dispatch by candidate kind (Face or Edge) and treat the
  bridge's own `InvalidArgument` (wrong surface/curve kind) as `false`.
  `Area`/`Length` reuse the existing generic `Shape::area`/`length`.
  `Normal` applies only to Face candidates (a face's own outward normal
  at its parametric-domain midpoint).

## Design decisions

1. **No invented numeric-tolerance policy for `==`/`~=`.**
   `project/DECISION_LOG.md#DL-26` forbids a new numerical domain
   silently borrowing D5/D19 or inventing its own default "close enough"
   threshold. `Comparison::Eq`/`DirectionComparison` therefore never pick
   an arbitrary geometric tolerance: `Eq` uses a tiny *relative*
   floating-point-representation-noise allowance (`eval::approx_eq`,
   `1e-9` relative) — the same kind of allowance ordinary floating-point
   code applies before comparing two independently computed reals, never
   a widened "counts as equal" radius — and a `DirectionComparison` with
   no explicit `tolerance` requires the same numerical-exactness
   allowance rather than a chosen default; when the query author *does*
   give a `tolerance`, that exact author-supplied value is used, never
   one this module chose. See `eval.rs`'s own module doc comment
   ("No invented numeric-tolerance policy") for the full reasoning,
   including why `Shape::classify_point`'s own tolerance parameter (an
   OCCT algorithm input, not a matching policy — this bridge already
   hardcodes `1e-6` for the same reason in `aicad_occt_shell`/`_offset`)
   is a different kind of number DL-26 does not cover.
2. **`torus` radius is the major (tube-path) radius only.** The header
   documents this explicitly; a torus's minor radius is not exposed by
   this task (no predicate needs it yet — a future task can add a second
   accessor without breaking this one).
3. **A wrong-entity-kind or wrong-surface/curve-kind combination
   evaluates to `false`, not an error**, at the `cad-query` evaluator
   layer (the underlying bridge call itself does return
   `InvalidArgument`, caught and translated here) — matching ordinary
   query-filter semantics (`SELECT ... WHERE surface_type = 'cylinder'`
   against a vertex row is empty, not a type error).

## Tests / verification

- `cargo fmt --all -- --check` → clean (whole workspace).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings (whole workspace).
- `cargo test -p cad-occt-bridge --lib` → 120/120 passed (92 pre-existing
  + 28 new: 19 classification + 3 `is_same` + 6 `AICAD-083`/`084`
  native-support tests landed in this same round, see those tasks'
  reports).
- `cargo test -p cad-query` → 32/32 passed, including 8 `AICAD-082`
  geometry-predicate tests (planar/cylindrical matching; radius match +
  non-match on a planar face; area match; normal match against exactly
  one box face; axis match against a cylinder's lateral face).
- `cargo test --workspace` → 72/72 test binaries green, 1,131 total
  passing tests, 0 failed, no regressions.
- `python3 scripts/ci/semantic_ref_harness.py validate`/`self-test` →
  both `"status": "ok"` (frozen `AICAD-079A` corpus untouched).
- `python3 scripts/ci/stage4_task_audit.py --check` → `Stage-4 task
  metadata audit OK`.

## Limitations

- `curvature` remains unimplemented (`AICAD-081`'s own deferral; no task
  has specified its comparison semantics yet).
- `face_normal`'s reported point is the face's own parametric-domain
  midpoint, not an area centroid — documented on the native header and
  `Shape::face_normal` directly; a caller needing the true centroid
  should combine this with the already-existing `Shape::center_of_mass`
  restricted to that one face.
- No production caller wires real query authoring syntax to
  `evaluate_geometry` yet (`query { ... }` blocks remain reserved,
  unimplemented `.aicad` syntax) — this task is evaluator plumbing only,
  consumed directly (not through source syntax) until a resolver exists.

## Regressions

None.

## Next dependency

`AICAD-083` (topology predicates), `depends_on: AICAD-082` — implemented
in this same invocation, see `project/reports/AICAD-083.md`.
