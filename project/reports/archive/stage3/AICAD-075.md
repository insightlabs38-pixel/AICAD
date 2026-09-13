# AICAD-075: Lower solved closed sketch profiles to exact faces

## Status

Done. Third and final task of Batch S3-05.

## Objective

`AICAD-073`/`AICAD-074` gave Stage 3 a solver-independent sketch
constraint IR and a concrete `SketchSolver` (`RelaxationSolver`), but
nothing connected a solved sketch back to real geometry — the pipeline
stopped at a `SolveReport`'s flat `SketchVariable -> f64` map
(`SolvedValues`). This task closes that gap end to end: applying a
`SolveReport`'s values back onto a concrete `Sketch`, validating that the
target `Profile` is actually a closed loop, and lowering it into an exact
kernel face via the existing Geometry-IR/kernel-dispatch machinery
(`AICAD-059`/`AICAD-060`), proven against a real `OcctContext` with
closed-form area evidence (never a render-only check, per `AGENTS.md`'s
evidence rule).

## Base / resulting commit

- Base: `d82bf82` (`AICAD-074`, `origin/claude/aicad-stage3-dev`'s HEAD at
  the start of this invocation).
- This task's commit: see `git log` on `origin/claude/aicad-stage3-dev`
  (single commit, `AICAD-075`).

## Files changed

- **`native/occt_bridge/{include,src}/aicad_occt_bridge.{h,cpp}`** — new
  `aicad_occt_make_arc_edge(context, p_start, p_mid, p_end, out_handle)`,
  built on OCCT's `GC_MakeArcOfCircle(P1, P2, P3)` three-point
  constructor. See "Design decisions" #1 for why this capability was
  needed and why a three-point edge, not a center/radius/angle one.
- **`crates/cad-occt-bridge/src/{ffi,lib}.rs`** — `OcctContext::make_arc_edge`
  wrapping the new native call, mirroring `make_line_edge`'s exact
  pattern; 4 new tests (valid quarter-circle edge + bounding box,
  rejects coincident points, rejects collinear points, composes with
  `make_line_edge`/`make_wire_from_edges`/`make_face` into a valid
  semicircular face with the closed-form area).
- **`crates/cad-geometry-api/src/ir.rs`** — new `GeometryOp::ArcEdge { start,
  mid, end }` (pure `Point3`s, no dimensioned parameter, so `push_op`
  needs no new validation arm beyond an empty match arm, mirroring
  `LineEdge`); 1 new test.
- **`crates/cad-geometry-runtime/src/dispatch.rs`** — dispatches `ArcEdge`
  to `OcctContext::make_arc_edge`, mirroring every other single-kernel-call
  op; 1 new test (an arc edge plus a line edge composing into a valid
  semicircular face through the full `GeometryGraph -> dispatch_graph`
  path, not just the bridge layer).
- **`crates/cad-constraints/src/apply.rs`** (new) — `apply_solved_values(sketch,
  values) -> Result<Sketch, SketchIrError>`: the "apply-back" step
  `AICAD-074`'s own report left as this task's judgment call (see that
  report's "Limitations"/"Next dependency"). Builds and returns a **new**
  `Sketch` (D2 functional semantics — the input is never mutated), with
  every entity's point/radius/angle fields taken from `values` wherever a
  corresponding `SketchVariable` was assigned, and left as the original
  value otherwise (a partial/`Underconstrained` solve is not an error
  here — see module doc comment). 7 tests (full/partial substitution for
  each entity kind, an unchanged `SolvedValues::new()` reproducing the
  original sketch exactly byte-for-byte, entity-id stability across the
  rebuild, and a solved non-positive radius correctly rejected via
  `Sketch::add_circle`'s own existing validation rather than silently
  accepted).
- **`crates/cad-constraints/src/lib.rs`** — wires `pub mod apply;` and
  `pub use apply::apply_solved_values;`.
- **`crates/cad-geometry-runtime/Cargo.toml`** — `cad-hir`/`cad-constraints`
  promoted from dev-only (`cad-hir`) / absent (`cad-constraints`) to real
  dependencies. See "Design decisions" #2 for why this crate is the
  correct seam.
- **`crates/cad-geometry-runtime/src/sketch_lowering.rs`** (new) — the
  actual lowering:
  - `lower_profile_to_face(sketch, profile) -> Result<LoweredFace, SketchLoweringError>`,
    `LoweredFace { graph: GeometryGraph, face: GeomId }`.
  - Maps `SketchPlane` to 3D exactly per that type's own already-fixed
    doc-comment convention (`WorldXy`/`WorldXz`/`WorldYz` normals
    +Z/+Y/+X; sketch `(x, y)` map to the plane's first/second named
    axis).
  - `Line` -> `GeometryOp::LineEdge`; `Arc` -> the new
    `GeometryOp::ArcEdge` (endpoints from the arc's own stored
    center/radius/angles, a direction-aware interior "mid" point computed
    from `direction`'s actual sweep — see "Design decisions" #1); a lone
    `Circle` (a profile's only entity — a full circle is already closed
    by itself) -> `GeometryOp::CircleWire` directly, skipping the
    edge-chain/`WireFromEdges` path entirely.
  - Closed-loop validation: each entity's own 2D traversal endpoint must
    coincide with the next entity's start (wrapping around) within
    `SketchSolverProfile::v1().position_tolerance` — see "Design
    decisions" #3 for why this specific, already-established tolerance
    was reused rather than a new one invented.
  - `SketchLoweringError` (`ForeignProfile`/`UnknownEntity`/
    `CircleMustBeSoleProfileMember`/`DegenerateArc`/`NotClosed`), with a
    `to_diagnostic` mirroring every other Stage-3 IR error's pattern
    (`GEOM` family, codes `GEOM-E020`..`E024`, the next free block after
    `SketchIrError`'s own `E010`..`E015`).
  - 9 tests: a plain (already-closed, no solving) rectangle and slot
    (line+arc mix) lowering to a valid face with the exact closed-form
    area; a lone-circle profile; **a genuinely constraint-solved**
    rectangle (rough hand-placed lines -> `RelaxationSolver::solve` ->
    `apply_solved_values` -> `lower_profile_to_face`, proving the full
    pipeline this task exists to close) matching its exact target area;
    a gapped (not-actually-closed) profile rejected; a `Circle` mixed
    with another entity rejected; a zero-sweep `Arc` rejected as
    degenerate; a foreign-profile mismatch rejected; every error
    variant's diagnostic conversion.
- **`crates/cad-hir/src/sketch.rs`** — **bug fix**: `Sketch::add_slot`'s
  two cap arcs used `RotationDirection::CounterClockwise`, which (given
  this codebase's own established "increasing angle = counter-clockwise"
  convention, `Direction2::perp`'s own doc comment) sweeps the *inward*
  PI/2 semicircle instead of the outward one the doc comment right above
  it claims ("bulging away from the slot body"). Fixed to `Clockwise`
  (the arithmetic/angle values themselves were already correct — only
  the direction flag was wrong). See "A real bug found and fixed" below
  for the full discovery/evidence trail and the strengthened regression
  test that now actually checks bulge direction, not just radius/sweep
  magnitude.

## Design decisions

1. **A new native/kernel `ArcEdge` capability was added, using a
   three-point (`start`/`mid`/`end`) construction, not
   center/radius/angle.** `cad-geometry-api::ir::GeometryOp` had
   `LineEdge` and `CircleWire` (always a *full* circle) but nothing for a
   *partial* circle — needed because a sketch `Arc` entity is one of only
   three primitive entity kinds (`docs/plan/04_HIGH_LEVEL_MODELING_API.md`
   §3), and both `rectangle`/`slot` composite constructors already use
   `Arc` (slot's own semicircular caps). Grepping the native bridge
   confirmed no arc-edge capability existed anywhere (`native/occt_bridge`
   has no `Arc`-anything). This is squarely "add capabilities only as
   required" (`DECISION_LOG.md#DL-5`'s capability-driven minimal surface,
   already the precedent every one of `LineEdge`/`CircleWire`/
   `WireFromEdges`/`MakeFace` themselves followed) — not a kernel-boundary
   change, not a new trusted native/plugin boundary (that escalation
   trigger is about *plugin/extension* trust, not adding one more
   operation to the already-fully-trusted OCCT bridge this codebase
   already links against), and not a compiler intrinsic. A three-point
   edge (`GC_MakeArcOfCircle(P1, P2, P3)`) was chosen over a
   center/radius/start-angle/end-angle one because it needs no separate
   axis/sense parameter to disambiguate which of the two possible arcs
   between two endpoints is meant (mirrors `LineEdge`'s own "no
   handedness flag" minimalism) — the caller (`sketch_lowering`) already
   has to resolve a genuine interior point on the intended arc anyway
   (to disambiguate `RotationDirection`), so passing that same point
   through directly, rather than re-deriving an axis/sense from it
   natively, is the smaller native-side surface.
2. **`cad-geometry-runtime` is the correct crate for
   sketch-to-kernel-face lowering**, not a new crate and not
   `cad-constraints` itself. `docs/plan/22_REPOSITORY_WORK_PACKAGES.md`
   splits this exact seam across two work packages: WP-05 ("Geometry
   language API") owns "lowering to kernel bridge"; WP-08 ("Constraint
   system") owns "sketch solver adapter". `cad-geometry-runtime` already
   plays exactly this "seam crate" role for a different pair of layers —
   its own module doc comment describes `bridge.rs` as converting an
   evaluated `cad_runtime::value::NumberValue` (HIR/runtime-adjacent)
   into `cad_geometry_api::Quantity` (Geometry-IR vocabulary), and the
   crate already depended on `cad-runtime` (a higher, HIR-adjacent layer)
   for precisely that reason. Adding `cad-hir`/`cad-constraints` as real
   dependencies (previously `cad-hir` was dev-only, for tests; `cad-
   constraints` was absent) extends that same established precedent
   rather than introducing a new architectural boundary or a fourth crate
   for one task.
3. **Closed-loop validation reuses
   `SketchSolverProfile::v1().position_tolerance` (`1e-9` canonical
   length units) rather than inventing a new tolerance constant.**
   `cad_hir::sketch::Profile`'s own module doc comment explicitly warned
   against exactly this risk ("introducing a tolerance-based closure
   check here would risk a second, competing tolerance policy... which
   `AGENTS.md`'s D5 precedent... treats as exactly the kind of
   numeric-tolerance decision that must not be made twice"). A profile a
   `SketchSolver` reports `Solved` already closes within its own
   convergence tolerance by construction (that is what "solved" means);
   reusing that exact figure here — rather than, say, D19/D5's
   `linear_abs = 1e-4` (a *cross-kernel-comparison* tolerance for an
   unrelated purpose, at a different, much looser order of magnitude) —
   keeps exactly one tolerance authority for "is this loop closed,"
   consistent with the D19/D5 precedent's own spirit even though this
   isn't a D5 comparison at all.
4. **A lone `Circle` profile bypasses the edge-chain entirely** (`CircleWire`
   directly, no `WireFromEdges`), while a `Circle` combined with any other
   entity in the same profile is a structural error
   (`CircleMustBeSoleProfileMember`). A full circle has no "start"/"end" to
   chain from — `docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s own catalogue
   never shows a circle composed with another entity in one profile, and
   silently guessing a chaining rule for that case would be exactly the
   kind of unresolved-ambiguity guess `AGENTS.md` prohibits.
5. **This module takes an already-solved `Sketch`, not a `SolveReport`
   directly, and has no opinion on `SolveStatus`.** The caller (a future
   feature task, e.g. `AICAD-076`'s extrude) decides whether an
   `Underconstrained`/`Overconstrained`/`Unsupported` result should still
   attempt lowering; `lower_profile_to_face` only ever answers "does this
   concrete geometry close, and if so, what face does it bound" — keeping
   that policy decision out of this module matches `cad_hir::sketch`'s own
   established precedent of deferring numerical-validity policy to a
   later, dedicated layer.

## A real bug found and fixed: `add_slot`'s caps bulged inward

While writing this task's own geometry-backed slot test (`a_solved_slot_
with_arcs_lowers_to_a_valid_face_with_the_exact_area`), the computed face
area came back as `27.4336...`, not the expected stadium area
`10*4 + pi*2^2 = 52.5664...`. The difference (`52.5664 - 27.4336 =
25.1327... = 2 * pi * 2^2`) is exactly twice the area of one cap's own
half-disk — the signature of a cap bulging *into* the rectangle body
(subtracting its own half-disk from the total) instead of *away* from it
(adding it), not a self-intersection or an unrelated defect.

Root cause, confirmed by direct angle arithmetic: `add_slot`'s own doc
comment claims both caps "sweep exactly PI radians counter-clockwise,
bulging away from the slot body," but for the exact `start_angle`/
`end_angle` pair it computes (`end_cap`: `PI/2` -> `3*PI/2`, at a cap
centered on the slot's own `end` point), *increasing* angle — this
codebase's own established meaning of "counter-clockwise"
(`Direction2::perp`'s doc comment: "+90 degrees (counter-clockwise)")  —
sweeps through angle `PI` (the point *toward* the rectangle body, at
`center.x - radius`), not through angle `0` (the point *away* from it, at
`center.x + radius`). `Clockwise` (decreasing angle, through `0`) sweeps
the correct outward semicircle instead. This is a genuine, previously
undetected defect in already-merged `AICAD-072` code: that task's own
regression test checked radius, angular delta, and the `direction` label
itself, but never which side of the circle the arc actually traces — so
an internally-consistent-but-backward direction choice passed unnoticed.

Fix (`crates/cad-hir/src/sketch.rs`): both `add_arc` calls in `add_slot`
changed from `RotationDirection::CounterClockwise` to
`RotationDirection::Clockwise`; the angle *values* themselves needed no
change. The existing regression test
(`slot_produces_two_lines_and_two_semicircular_caps`) is strengthened to
compute each cap's own direction-aware midpoint and assert it lies
*outside* the rectangle span (`bulge_x > center.x` for the cap at `x=10`,
`bulge_x < center.x` for the one at `x=0`) — a check the original test
never made, and the exact property that would have caught this the first
time. `cargo test -p cad-hir sketch::` (23/23) and the new
`cargo test -p cad-geometry-runtime --lib sketch_lowering::` slot test
both pass after the fix; no other consumer of `add_slot` exists anywhere
in the workspace (`grep -rln add_slot` finds only `cad-hir` itself and
this task's own new `sketch_lowering.rs` test).

## Verification

```
$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, 29 crates)

$ cargo test -p cad-occt-bridge --lib
test result: ok. 88 passed (was 84; +4 arc-edge tests)

$ cargo test -p cad-geometry-api
test result: ok. 18 passed (was 17; +1 ArcEdge op test)

$ cargo test -p cad-geometry-runtime
test result: ok. 20 passed (was 10 at the start of this task -- 2 bridge +
  8 dispatch; +1 dispatch ArcEdge test, +9 sketch_lowering tests)

$ cargo test -p cad-constraints
test result: ok. 42 passed (was 35; +7 apply.rs tests)

$ cargo test -p cad-hir sketch::
test result: ok. 23 passed (unchanged count; slot test strengthened,
  underlying bug fixed)

$ cargo test --workspace
0 failures across every crate (full per-crate breakdown in this
  invocation's own terminal output; every crate's count either grew by
  exactly the numbers above or is unchanged from Batch S3-05's own
  AICAD-074 baseline -- cad-hir stays at 225).

$ cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1
test result: ok. 3 passed (Stage-2 gate proof unaffected).
```

Environment: Rust 1.98.1, edition 2024, unchanged from prior Stage-3
batches. No new third-party dependency was added anywhere (the native
addition uses OCCT's already-linked `GC_MakeArcOfCircle`, part of the
same OCCT distribution every other native bridge function already uses).

## Limitations

- `sketch_lowering` maps `SketchPlane`'s three fixed world planes only —
  no general `Frame3`-based sketch plane yet (explicitly `AICAD-075A`'s
  own charge, per `cad_hir::sketch`'s own "Scope cuts" and this task's
  own dependency on that not-yet-built foundation).
- Closed-loop validation is head-to-tail chain continuity only (each
  entity's end meets the next entity's start) — it does not detect
  self-intersection *within* a single closed loop (e.g. a figure-eight
  built from four lines that each individually connect end-to-end but
  cross each other partway). `Shape::make_face`/`Shape::is_valid` at the
  kernel level will still catch a self-intersecting result as invalid
  (this task's own dispatch/OCCT layer already does this for
  `WireFromEdges`/`MakeFace`, per `AICAD-060`'s own precedent), so no
  invalid geometry silently passes as valid — this module simply does
  not pre-empt that kernel-level check with its own duplicate one.
- No multi-loop profile (an outer boundary plus one or more inner holes)
  — `docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s own `Profile`/`sketch`
  vocabulary for this Stage-3 slice describes single closed loops; a
  multiple-wire face is `Shape::make_face`'s own possible future
  extension point, not something this task's `Profile` input shape
  supports requesting yet.
- `lower_profile_to_face` builds and returns its own fresh
  `GeometryGraph`, not a shared/merged one — there is no Feature-DAG-level
  graph-merge mechanism yet for a caller (e.g. a future extrude feature)
  to compose this profile's face into a larger multi-operation graph
  in-place; a caller dispatches this graph, then builds a *new* graph
  seeded from the resulting real kernel `Shape` for its own next
  operation (exactly the pattern `AICAD-060`'s own `dispatch_graph`
  already supports one node at a time). Left as-is deliberately: no
  Stage-3 task yet needs cross-graph composition, and inventing that
  mechanism now would be scope expansion beyond this task's own title.
- `SketchLoweringError::to_diagnostic` always reports position `(1, 1)`
  (no better span exists — this module operates on already-lowered
  `Sketch`/`Profile` values, not source text; the *original* source
  span for whichever `.aicad` construct produced the offending entity
  lives on `SketchEntity::span`/`Profile::span`, which a future
  source-integration task can thread through once sketches are wired to
  actual grammar/lowering, per `cad_hir::sketch`'s own still-open "no
  grammar/lowering integration" scope cut).
- No new third-party workspace dependency; the only new cross-crate
  dependency edges are `cad-geometry-runtime -> cad-hir` (promoted from
  dev-only) and `cad-geometry-runtime -> cad-constraints` (new), both
  along the same seam-crate precedent described in "Design decisions" #2.

## Regressions

One found and fixed (see "A real bug found and fixed" above) — a
pre-existing defect in `AICAD-072`'s `Sketch::add_slot`, not a regression
introduced by this task. No other existing behavior changed: every other
crate's test count is unchanged from `AICAD-074`'s own baseline, and the
Stage-2 end-to-end gate proof still passes 3/3.

## Next dependency

Batch S3-05 (`AICAD-073`/`AICAD-074`/`AICAD-075`) is now complete. Per
`project/CURRENT_STAGE.md`'s fixed batch list, the next artifact is the
`project/gates/STAGE3-B_SKETCH_CONSTRAINTS.md` checkpoint (prepared
alongside this report — see that file), gating Batch S3-06 (`AICAD-075A`,
then `AICAD-076`).

**Batch-boundary note for the next invocation:** this invocation's own
top-level campaign brief text said "complete AICAD-075 normally... finish
the batch that these two tasks [`AICAD-075`, `AICAD-075A`] are in,"
appearing to treat `AICAD-075`/`AICAD-075A` as one batch. `project/
CURRENT_STAGE.md`'s own authoritative fixed batch list (and
`project/TASKS.yaml`) instead places `AICAD-075` in Batch S3-05 (with
`AICAD-073`/`AICAD-074`) and `AICAD-075A` in the *separate* Batch S3-06
(with `AICAD-076`), with the `STAGE3-B_SKETCH_CONSTRAINTS.md` checkpoint
required in between — and the campaign brief's own general rules
elsewhere state "Do not: combine batches" and "each invocation works on
exactly ONE fixed batch." This report treats `project/CURRENT_STAGE.md`/
`project/TASKS.yaml` as authoritative on the batch boundary itself (per
the campaign brief's own "the repository... project state files... are
authoritative") and stops at the end of S3-05 plus its required gate,
rather than continuing into `AICAD-075A` in this same invocation. The
next invocation should begin Batch S3-06 (`AICAD-075A` first, per its own
`depends_on: [AICAD-075]`), reading `docs/plan/
04_HIGH_LEVEL_MODELING_API.md`, `docs/plan/
05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`, `docs/plan/
06_REFERENCES_QUERIES_FEATURE_DAG.md`, and this report's own "Limitations"
(the `SketchPlane` fixed-enum boundary `AICAD-075A` is charged with
generalizing) first. If the owner intended the two tasks to share one
invocation regardless, that is a one-line confirmation for
`project/OWNER_DECISIONS.md`/the next session's own instructions, not
something this task should resolve by guessing.
