# AICAD-076: Implement high-level extrude/revolve/hole/pocket

## Status

Done, with one real architecture finding escalated as `project/
OWNER_DECISIONS.md#D20` (not blocking this task — a safe, complete
workaround was found and shipped). Second and final task of Batch S3-06.

## Objective

Add `extrude`/`revolve`/`hole`/`pocket` to the Safe CAD `RuntimeBuiltin`
catalogue, consuming `AICAD-075A`'s spatial foundation. `AICAD-075A`'s own
"Implications for AICAD-076" section anticipated `revolve`/`hole` taking
a real `Axis3`-typed argument via `cad_runtime::spatial::
axis3_from_value`; implementing that revealed a genuine, repository-wide
compiler-architecture defect (below) that made a `Named` struct-typed
builtin parameter unsafe to add at all. This task ships a different,
safe, fully-typed design instead of guessing past that finding, and
records it for the owner and for `AICAD-077`.

## Base / resulting commit

- Base: `41283eb` (`AICAD-075A`, `origin/claude/aicad-stage3-dev`'s HEAD
  at the start of this invocation).
- This task's commit: see `git log` on `origin/claude/aicad-stage3-dev`
  (single commit, `AICAD-076`).

## The architecture finding (read before touching a struct-typed builtin again)

`crate::lower::Lowerer::seed_builtins` seeds *every* `BuiltinFnId` into
*every* compiled program's global scope unconditionally, and `cad_hir::
typeck::Checker::collect_signatures` eagerly resolves every seeded
function's own parameter/return types up front — for every function,
whether or not the program actually calls it. A `HirTypeRef::Named`
reference to a `cad_hir::geometry_types` struct (`Axis3`, `Frame3`,
`Point3`, `Plane`) that is not itself in scope (true for the overwhelming
majority of existing programs/tests, since composing `with_geometry_types`
has always been caller-optional) fails eagerly with `UNKNOWN_TYPE_NAME`
for *that program*, even when the program never references the offending
builtin. Confirmed empirically, not inferred: adding `axis: Axis3`/
`frame: Frame3` parameters to draft `revolve`/`hole`/`pocket` catalogue
entries broke **149** previously-passing `cad-hir` tests unrelated to
Stage-3 modeling — the exact count matched the exact number of `Named`
geometry-type references added (3, once `Vector3<Float>`'s own `Generic`
reference was ruled out as a cause — see below). Full workspace `cargo
test` was run with the broken draft in place specifically to measure the
true blast radius before deciding how to respond, per this campaign's own
evidence discipline.

A `HirTypeRef::Generic` reference to a user-defined generic struct
(`Vector3<Float>`) does not have this problem: an unresolvable generic
base (`Checker::resolve_generic_type_application`) returns `None` via its
own `?`-early-return, silently, with no diagnostic. This is exactly why
`extrude`'s `direction: Vector3<Float>` parameter is safe while a
hypothetical `axis: Axis3` parameter is not — the asymmetry between
`Named` and `Generic` type-reference resolution is the crux of the whole
finding.

**Full recorded in `project/OWNER_DECISIONS.md#D20`** (three live options,
none decided here: unconditionally bundle geometry types with builtin
seeding; permanently restrict builtins to primitive/`Generic` types only;
make signature resolution lazy per call site). This task did not pick one
— see "What this task shipped instead" below for what it did do, which is
compatible with any of the three options being chosen later.

## What was considered and rejected

- **`Vector3<Length>` standing in for a position instead of `Point3`**
  (also a safe `Generic` reference) — rejected: `AICAD-075A`'s own
  required semantic distinctions explicitly forbid collapsing `Point3`
  (position) into `Vector3` (displacement) merely because both would
  type-check; using it here to dodge an unrelated compiler defect would
  repeat exactly the mistake that task warned against.
- **Deferring `revolve`/`hole`/`pocket` entirely**, shipping only
  `extrude` (the one builtin with no struct-typed parameter at all) —
  rejected as unnecessarily conservative: a safe, fully-typed scalar-
  decomposed design exists and needed no guessing (see below), so
  deferring three of the four named deliverables would have been overly
  cautious given a real, complete alternative was available.

## What this task shipped instead

Scalar-decomposition of position (matching `BuiltinFnId::Transform`'s own
existing Stage-2 precedent: `dx`/`dy`/`dz` scalars instead of a
`Vector3<Length>`, for the analogous "no safe way to use the richer type
yet" reason) plus `Vector3<Float>` (safe, `Generic`) for direction:

- **`extrude(target: Geometry, face: Int, direction: Vector3<Float>,
  distance: Length) -> Geometry`** — no position needed at all (extrudes
  from the face's own existing location).
- **`revolve(target: Geometry, face: Int, direction: Vector3<Float>,
  angle: Angle) -> Geometry`** — axis through the **world origin** only
  (matches `cylinder`'s own existing "+Z axis through origin, not
  user-relocatable" precedent, generalized to a caller-chosen direction).
- **`hole(target: Geometry, origin_x: Length, origin_y: Length,
  origin_z: Length, direction: Vector3<Float>, diameter: Length, depth:
  Length) -> Geometry`** — axis origin as three flat scalars.
- **`pocket(target: Geometry, origin_x: Length, origin_y: Length,
  origin_z: Length, width: Length, length: Length, depth: Length) ->
  Geometry`** — position as three flat scalars, world-axis-aligned (no
  orientation parameter yet, matching `box`'s own "no `frame` parameter
  yet" precedent).

All four are genuinely useful, fully typed (no bare floats standing in
for a `Length`), and required no ambiguous guess — this is `transform`'s
own already-accepted narrowing pattern applied consistently, not a new
invented convention.

## Files changed

- **`crates/cad-geometry-api/src/ir.rs`** — new `GeometryOp::GetFace {
  target: GeomId, face: FaceIndex }`: selects one face of `target` by raw
  kernel-enumeration-order index (mirroring `Fillet`/`Chamfer`'s own
  established `List<Int>` raw-index-selection precedent), producing it as
  its own new geometry value — the only source-visible way to obtain a
  profile for `extrude`/`revolve` before source-level sketch/profile
  construction exists (`cad_hir::sketch` has no grammar/lowering
  integration yet). Zero change to the already-tested `Extrude`/`Revolve`
  IR shapes themselves (both still take `profile: GeomId` unchanged) —
  `GetFace` is a new upstream node those existing shapes consume, not a
  redesign of them. 2 new tests (valid-target accepts and assigns the
  next sequential id; invalid-target operand rejected, mirroring
  `unknown_operand_is_rejected`'s own precedent for a different op).
- **`crates/cad-geometry-runtime/src/dispatch.rs`** — dispatches
  `GetFace` to `Shape::get_face`, mirroring every other single-kernel-call
  op. 2 new tests: `get_face_selects_a_real_valid_face_with_one_of_the_
  expected_areas` (a non-cubic box's selected face has one of the three
  possible face areas — kernel face-enumeration order is not part of this
  dispatcher's own contract, so the test does not assume which face index
  0 is) and `hole_builtin_end_to_end_bores_a_clean_through_hole` (real
  `.aicad` source through a real kernel, closed-form removed-volume
  check).
- **`crates/cad-kernel-api/src/geometry.rs`** — new `Frame3::from_z`
  (mirrors `Frame3::from_x`'s exact cyclic derivation, `x -> z -> y -> x`)
  — used to align a fixed-`+Z`-axis kernel primitive (`cylinder`) onto an
  arbitrary target axis via `Transform::from_frames`, needed by `hole`. 1
  new test (`frame_from_z_is_always_orthonormal_and_right_handed`,
  mirroring `frame_from_x`'s own test exactly).
- **`crates/cad-hir/src/builtins.rs`** — four new `BuiltinFnId` variants
  (`Extrude`, `Revolve`, `Hole`, `Pocket`) and catalogue entries (see
  signatures above); new `vector3_of` helper (mirrors `list_of` for
  `Vector3<T>`); a new "Why no `Axis3`/`Frame3`-typed parameter yet"
  module note recording the architecture finding in full, colocated with
  the affected code.
- **`crates/cad-runtime/src/interp.rs`** — `Interpreter::dispatch_builtin`
  restructured from "compute one `GeometryOp`, push it once" to "push one
  or more `GeometryOp` nodes via a shared `push_op` closure, return the
  last" — a behavior-preserving refactor for every pre-existing builtin
  (each now calls `push_op(...)` exactly where it used to build the
  shared `op` value; same nodes, same results) needed because `extrude`/
  `revolve` (via `GetFace`) and `hole`/`pocket` (via `Cylinder`/`Box` +
  `Transform`) each push more than one node. New `face_index`/
  `spatial_direction` argument-extraction closures (the latter wrapping
  `AICAD-075A`'s `crate::spatial::direction3_from_value`). New
  `compiled_with_geometry_types` test helper (mirrors `compiled_with_
  prelude` exactly). 5 new tests: one per builtin proving the exact
  `GeometryOp` sequence/field values each produces, plus
  `a_degenerate_direction_argument_is_reported_as_an_invalid_spatial_
  argument` (proving `RuntimeError::InvalidSpatialArgument`, `AICAD-075A`'s
  own addition, is now genuinely reachable through a real call site).
- **`crates/cad-runtime/src/error.rs`** — new `RuntimeError::
  InvalidSpatialArgument { name, span, reason: SpatialValueError }`
  (`RUNTIME-E128`): the diagnostic a genuinely invalid (not merely
  wrongly-shaped) `AICAD-075A`-spatial argument surfaces as, distinct
  from the generic `BuiltinArgumentShape` one — added by this task, not
  `AICAD-075A` (that task built `SpatialValueError` itself but did not
  yet need a `RuntimeError` variant for it, since nothing called
  `cad_runtime::spatial` from a real dispatch site until now).
- **`docs/API/safe-cad-api.md`** — documents all four new builtins, the
  scalar-decomposition rationale, and the narrowings relative to
  `docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s own signatures.
- **`project/OWNER_DECISIONS.md`** — new `D20` entry (see "The
  architecture finding" above).

## Design decisions

1. **`GetFace` is a standalone new `GeometryOp` node, not a change to
   `Extrude`/`Revolve`'s own operand shape.** Considered folding face
   selection directly into `Extrude`/`Revolve` (`profile: (GeomId,
   FaceIndex)` instead of `profile: GeomId`), mirroring `Fillet`/
   `Chamfer`'s own inline edge-selector resolution. Rejected: that would
   change two already-tested, Stage-2-era IR shapes for every existing
   and future caller, a larger and riskier change than adding one new,
   independent node type that those shapes consume unchanged.
2. **`dispatch_builtin` pushes multiple nodes via one shared closure,
   not a new second dispatch mechanism.** Still the one ordinary
   `RuntimeBuiltin` mechanism (`DL-15`) — `hole`/`pocket`/`extrude`/
   `revolve` are a "domain-meaningful name standing in for a short, fixed
   sequence of already-existing ops," exactly `BuiltinFnId::Plate`'s own
   precedent, just spanning more than one node this time instead of one.
3. **Scalar-decomposition over guessing an `Axis3`/`Frame3` signature or
   silently reworking the compiler.** See "What this task shipped
   instead" above and `project/OWNER_DECISIONS.md#D20`.
4. **`revolve`'s axis is through-origin-only, not a further scalar-
   decomposed arbitrary axis (6 params instead of 4).** Chosen over the
   more capable but more awkward alternative because it exactly mirrors
   `cylinder`'s own already-accepted "+Z through origin" convention
   (generalized from a fixed direction to a caller-chosen one) — a
   caller wanting an off-origin revolve axis composes `transform`
   before/after, using already-existing building blocks.
5. **`hole`'s axis origin is fully scalar-decomposed (not through-origin-
   only) because the feature is worthless otherwise** — a hole's entire
   value is being placed away from the origin, unlike revolve, where a
   through-origin axis is often already the natural design choice.

## Tests / verification

- `cargo fmt --all -- --check` → clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings, full workspace (29 crates) — including fixing a
  `clippy::doc_lazy_continuation` lint triggered by a doc-comment line
  beginning with `+ ` (interpreted as an unindented markdown list
  continuation), unrelated to this task's own logic.
- `cargo test --workspace` → 0 failures, 976 tests passed across every
  crate (verified twice: once against the broken draft catalogue to
  measure the 149-test blast radius precisely, once against the final
  shipped design to confirm zero regressions).
- `cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1` →
  3/3 (Stage-2 gate proof unaffected).

Per-crate test counts (this invocation's changes only; every other
crate's count is unchanged from `AICAD-075A`'s own last-verified totals):

| Crate | Before | After | Delta |
|---|---|---|---|
| `cad-geometry-api` | 20 | 22 | +2 (`GetFace` validation) |
| `cad-kernel-api` | 26 | 27 | +1 (`Frame3::from_z`) |
| `cad-hir` | 226 | 226 | +0 (catalogue/`ALL`-consistency tests already iterate every entry automatically; no new dedicated test needed) |
| `cad-runtime` | 121 | 126 | +5 (`extrude`/`revolve`/`hole`/`pocket` dispatch, invalid-spatial-argument) |
| `cad-geometry-runtime` | 20 unit + 4 integration | 22 unit + 4 integration | +2 unit (`GetFace` dispatch, `hole` end-to-end) |

## Limitations

- No source-level `Sketch`/`Profile` construction exists yet, so
  `extrude`/`revolve`'s only profile source is an existing solid's own
  face (`GetFace`) — a genuinely different, narrower capability than the
  plan's own `Profile|Sketch|FaceRef` signature. A future task wiring
  sketches into the language should give `extrude`/`revolve` a second,
  additional way to obtain a profile, not replace this one.
- `revolve` cannot revolve about an off-origin axis directly (compose
  `transform` before/after); `hole` has no `ThroughAll`/counterbore/
  countersink/thread; `pocket` is always axis-aligned with no
  orientation parameter and always a rectangle, never an arbitrary
  profile — see "What this task shipped instead" for the exact
  narrowings and their precedents.
- `project/OWNER_DECISIONS.md#D20` remains genuinely open. `AICAD-077`'s
  own planned `mirror(target, plane: Plane)` hits the identical wall (no
  clean scalar decomposition exists for a plane without splitting it
  into origin + normal fields spelled out separately, at minimum) — read
  that entry before designing `AICAD-077`'s own signature.
- `cad_runtime::spatial::axis3_from_value`/`frame3_from_value`/
  `plane3_from_value` (`AICAD-075A`) remain unconsumed by any concrete
  builtin after this task — only `direction3_from_value` gained a real
  caller (`spatial_direction`, used by `extrude`/`revolve`/`hole`). This
  corrects `AICAD-075A`'s own forward-looking "Implications for
  AICAD-076" expectation, which assumed `axis3_from_value` would be
  `AICAD-076`'s real caller; the finding above is why that did not
  happen as originally anticipated.

## Regressions

None. Every existing builtin's own dispatch behavior is unchanged (the
`dispatch_builtin` restructuring is proven behavior-preserving by every
pre-existing builtin test still passing unmodified); the full workspace
suite and the Stage-2 gate proof both pass with zero failures.

## Next dependency

Batch S3-06 (`AICAD-075A`, `AICAD-076`) is now complete. Per
`project/CURRENT_STAGE.md`'s fixed batch list, the next batch is S3-07
(`AICAD-077`, `AICAD-078`), with no checkpoint required between S3-06 and
S3-07 — `project/gates/STAGE3-C_MODELING.md` is prepared only after
Batches S3-06/S3-07/S3-08 all complete. `AICAD-077` ("Implement mirror
and linear/circular pattern basics") depends on `AICAD-076` (satisfied)
and should read `project/OWNER_DECISIONS.md#D20` before designing its own
`mirror`/`radial_pattern` signatures — its `mirror(target, plane: Plane)`
hits the identical struct-typed-parameter wall this task found, and its
`radial_pattern` axis can reuse this task's own `Vector3<Float>`-plus-
scalar-origin pattern exactly as `revolve`/`hole` do here (proven
interchangeable by `AICAD-075A`'s own geometry-backed test #2, which used
the same `Axis3` representation for both revolve and circular-pattern
preparation).
