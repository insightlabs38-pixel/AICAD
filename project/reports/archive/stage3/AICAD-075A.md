# AICAD-075A: Establish general axis/frame/rotation semantics

## Status

Done. First task of Batch S3-06.

## Objective

`AICAD-070` declared passive, language-facing `Axis3`/`Frame3` struct
shapes but explicitly deferred giving them real semantics ("a coherent
axis/frame/rotation semantics for revolve/transform/mirror/circular
patterns is `AICAD-075A`'s own explicitly assigned task" — that module's
own doc comment, and `docs/API/safe-cad-api.md`'s identical note). This
task closes that gap: it audits the existing spatial-math layer, confirms
it is already the one coherent kernel-neutral model every later Stage-3
modeling operation must share, adds the one missing representation
(`Plane3`, for mirror), and builds the validated `Value::Struct ->
cad_kernel_api` conversion boundary a future `revolve`/`mirror`/
`radial_pattern` `RuntimeBuiltin` will use — proven end to end against a
real kernel with representative revolve/circular-pattern/mirror-plane/
general-transform preparation, per this task's own "geometry-backed
tests... not full `AICAD-076`/`AICAD-077` feature implementation" scope
limit.

## Base / resulting commit

- Base: `72328ef` (`origin/claude/aicad-stage3-dev`'s HEAD at the start of
  this invocation — the `STAGE3-B_SKETCH_CONSTRAINTS.md` Batch S3-05
  checkpoint commit).
- This task's commit: see `git log` on `origin/claude/aicad-stage3-dev`
  (single commit, `AICAD-075A`).

## What was audited first (per this task's own required first step)

- **`crates/cad-kernel-api/src/geometry.rs`** (`AICAD-021`, Stage 1):
  already declares `Point3`/`Vector3`/`Direction3`/`Axis3`/`Frame3`/
  `Transform` with every invariant this task's own "required semantic
  distinctions" list asks for — `Direction3` constructible only via
  `Vector3::normalize` (rejects non-finite/`< 1e-12` vectors), `Frame3`
  either derived-and-therefore-always-valid (`from_x`) or explicitly
  validated for orthonormality/right-handedness within `1e-6`
  (`Frame3::new`), and `Transform` built from `Transform::rotation`
  (right-hand-rule Rodrigues formula about an arbitrary `Axis3`),
  `Transform::compose` ("self first, then other" — documented and
  tested), and `Transform::from_frames`. Already reused directly, not
  duplicated, one layer up: `cad_geometry_api::ir::GeometryOp::Revolve`
  takes `axis: cad_kernel_api::Axis3` and `GeometryOp::Transform` takes
  `transform: cad_kernel_api::Transform` verbatim, and
  `cad_occt_bridge::Shape::revolve`/`transform` take the identical types
  one layer below — confirmed by direct inspection, not assumed.
  **Finding:** this crate is already `AICAD-075A`'s required "one
  documented, kernel-neutral spatial model" — no duplicate math layer was
  needed, only documentation of the fixed conventions (added to this
  module's own doc comment) and the one missing representation
  (`Plane3`, below).
- **`crates/cad-hir/src/geometry_types.rs`** (`AICAD-070`, Stage 3): the
  *language-facing* `Axis3`/`Frame3` struct declarations are ordinary,
  unvalidated struct shapes (`direction`/`x_axis`/`y_axis`/`z_axis` are
  plain `Vector3<Float>`, not a validated direction type — the struct
  system enforces no numeric invariant at construction). **Finding:**
  this is exactly the passive-data gap this task's own title describes —
  the fix is not a language-level type-system change (out of scope, and
  not requested) but a conversion/validation boundary at the point a
  constructed value is actually consumed, mirroring how `cad_kernel_api`
  itself validates at construction.
- **`GeometryOp::Revolve`** (`cad_geometry_api::ir`): exists, fully wired
  through `cad_geometry_runtime::dispatch` to `Shape::revolve`, but had
  **zero test coverage anywhere in the repository** before this task
  (confirmed by grepping the whole workspace) — no Stage-2/3 task had
  ever exercised it end to end. Not this task's defect to fix by adding
  a `revolve` builtin (that is `AICAD-076`'s own scope), but this task's
  own new geometry-backed test (below) is the first evidence this
  pathway actually produces correct geometry.
- No existing `Plane`/mirror representation exists anywhere
  (`cad_kernel_api`, `cad_geometry_api::ir`, `cad_occt_bridge` all
  grepped clean) — `AICAD-077`'s mirror feature starts from nothing at
  the kernel-adapter level, confirming `Plane3` (this task's own addition)
  is genuinely new, not a duplicate of something already present.
- `cad_hir::sketch::SketchPlane` (`AICAD-072`) is a fixed three-variant
  world-plane enum whose own doc comment already defers generalizing to
  a `Frame3`-based plane to this task. This task's own acceptance
  criteria do not require that generalization (only "one coherent
  axis/frame/rotation representation... no feature invents its own
  coordinate convention" for revolve/transform/mirror/circular-pattern);
  `SketchPlane` is left unchanged — see "Limitations" below.

## Architectural finding: mirror cannot be a `Transform`

`cad_kernel_api::Transform` is, by construction and by its own existing
test (`every_produced_transform_is_rigid_within_native_tolerance`, which
checks every produced transform's linear part has determinant `+1`), a
**proper** rigid motion only — rotation composed with translation, never
a reflection. A mirror operation is an **improper** isometry (determinant
`-1`). This means `AICAD-077`'s mirror feature cannot be expressed as a
`cad_kernel_api::Transform` value without weakening that already-tested
invariant — it needs its own `Plane3`-based kernel-adapter entry point
(a new native/bridge function taking a plane, not a transform). This is
recorded explicitly in `cad_kernel_api::geometry`'s own module doc
comment (new "Rigidity (no reflection)" bullet) so `AICAD-077` does not
rediscover this the hard way or, worse, silently weaken `Transform`'s
rigidity contract to force mirror through it.

## What was added

- **`crates/cad-kernel-api/src/geometry.rs`**:
  - New `Plane3 { origin: Point3, normal: Direction3 }` — a
    geometrically-defined plane, deliberately distinct from `Frame3`
    (many frames determine the same plane; a plane's own semantics never
    depend on an arbitrarily chosen in-plane basis). `Plane3::from_frame`
    derives the plane a `Frame3` determines (origin + `z` axis);
    `Plane3::signed_distance` is the one supporting primitive a future
    mirror operation needs (point-plane signed distance), not the mirror
    operation itself.
  - Extended module doc comment: an explicit "The Stage-3 spatial
    foundation (`AICAD-075A`)" section restating the fixed
    handedness/positive-rotation, identity, composition-order, rigidity
    (including the mirror finding above), frame-validity,
    direction-validity, and semantic-vs-representation-equivalence
    conventions, so no later task re-derives or contradicts them.
  - `lib.rs`: `Plane3` added to the crate's public re-export list.
  - 3 new tests: `plane_from_frame_uses_frame_origin_and_z_as_normal`,
    `different_frames_sharing_origin_and_normal_produce_equal_planes`
    (semantic equivalence, not representation identity — two distinct
    valid right-handed frames sharing an origin/normal produce `==`
    planes), `signed_distance_is_zero_on_the_plane_and_signed_off_it`.
- **`crates/cad-hir/src/geometry_types.rs`**: new `Plane { origin: Point3,
  normal: Vector3<Float> }` struct declaration, mirroring `Axis3`'s own
  `origin`/direction-as-`Vector3<Float>` shape exactly — the mirror-plane
  representation sufficient for `AICAD-077` to consume. Module doc
  comment updated to record this task's resolution of the "coherent
  semantics" deferral. Existing tests updated for the new 7-declaration
  count (was 6); 1 new test (`Plane` construction/type-check).
- **`crates/cad-runtime/src/spatial.rs`** (new module — the actual
  conversion boundary): `SpatialValueError` (`MalformedValue` —
  defensive-only, mirrors `RuntimeError::BuiltinArgumentShape`'s "trusts,
  but verifies" precedent; `DegenerateDirection`; `NonOrthonormalFrame` —
  both genuine run-time failures type-checking cannot catch) and six pure
  conversion functions: `point3_from_value`, `vector3_float_from_value`,
  `direction3_from_value` (the one point invalid input is rejected
  explicitly, never silently repaired or panicked), `axis3_from_value`,
  `frame3_from_value` (validates orthonormality/handedness via
  `Frame3::new` itself), `plane3_from_value`. Mirrors
  `cad_runtime::interp::dispatch_builtin`'s own existing precedent of
  building a `cad_kernel_api::Transform` inline from evaluated argument
  values (the `BuiltinFnId::Transform` arm already does exactly this for
  translation) — factored out here because `AICAD-076`/`AICAD-077` will
  each need every one of these conversions, not just the translation-only
  case Stage 2 needed. 11 new tests, each using a **genuine
  evaluator-produced** `Value::Struct` (real `.aicad` source compiled and
  run through a real `Interpreter`, never a hand-built struct value —
  `BindingId::new` is `pub(crate)`-only in `cad-hir`, so no other
  construction path exists, matching every other struct-value test in
  this crate's own `interp.rs` test module): valid/degenerate direction,
  valid axis/degenerate-direction axis, valid/left-handed/non-orthogonal
  frame, valid/degenerate plane, and defensive malformed-value rejection.
  `cad-runtime/src/lib.rs` registers the new module and re-exports
  `SpatialValueError`.
- **`crates/cad-geometry-runtime/tests/spatial_axis_frame_foundation.rs`**
  (new integration test file, 4 tests) — the geometry-backed proof this
  task's own testing section requires, each starting from a genuine
  evaluated `.aicad` `Axis3`/`Frame3`/`Plane` value converted through
  `cad_runtime::spatial`, then driving real kernel geometry:
  1. `revolve_about_an_evaluated_axis3_value_matches_the_closed_form_cylinder_volume`
     — a rectangle profile with one edge on the converted axis, revolved
     `2*pi`, matches `pi * width^2 * height` to `1e-6` relative — the
     first-ever test of `GeometryOp::Revolve`'s dispatch path (see
     "Finding" above).
  2. `circular_pattern_positions_built_from_an_evaluated_axis3_value_land_at_the_expected_angles`
     — the *same* converted `Axis3` drives 4 evenly-spaced
     `Transform::rotation` copies of a real kernel box, each landing its
     kernel-computed center of mass at the exact closed-form
     `(r*cos(angle), r*sin(angle), z)` position, proving one `Axis3`
     representation serves both revolve and circular-pattern preparation
     without either inventing its own convention (this task's own
     integration requirement).
  3. `mirror_plane_from_an_evaluated_plane_value_reports_the_correct_signed_distance_to_a_real_shapes_center_of_mass`
     — a converted `Plane3`'s `signed_distance` to a real kernel box's
     own kernel-computed center of mass matches the closed form exactly,
     tying `Plane3`'s math to real geometry rather than only
     `cad-kernel-api`'s own pure-math unit tests.
  4. `frame_based_transform_from_an_evaluated_frame3_value_moves_a_real_kernel_box_to_the_expected_place`
     — a converted `Frame3` (a 90-degree rotation via
     `Transform::from_frames`) moves a real kernel box to the exact
     closed-form bounding box, proving `Frame3`'s rotation semantics are
     now real and usable, not just passively declared.
- **`docs/API/safe-cad-api.md`**: documents the new `Plane` geometry
  type, records this task's resolution (new "Axis/frame/rotation
  semantics (`AICAD-075A`)" section), and corrects the `transform`
  entry's now-stale "no `Vector3`/`Axis3`... surface syntax" parenthetical
  (that syntax has existed since `AICAD-070`; what remained missing was
  the semantics this task supplies, and the actual builtin wiring
  `AICAD-076`/`AICAD-077` still owns).

## Design decisions

1. **Reuse, do not duplicate.** No new pure-math layer was created —
   `cad_kernel_api::geometry` already satisfied every acceptance
   criterion for "one coherent kernel-neutral spatial model." This
   task's own work is (a) the one missing representation (`Plane3`), (b)
   explicit documentation of the already-fixed conventions so they
   cannot silently drift, and (c) the `Value -> cad_kernel_api`
   conversion boundary that was genuinely missing.
2. **Conversion functions are pure and span-free, matching
   `cad_geometry_runtime::bridge::number_value_to_quantity`'s own
   precedent exactly.** `cad_runtime::spatial` knows nothing about
   `Span`/diagnostics — a future `dispatch_builtin` call site
   (`AICAD-076`/`AICAD-077`) maps `SpatialValueError` into its own
   span-carrying `RuntimeError`, exactly like the existing `quantity`/
   `geometry` closures in `dispatch_builtin` already do for
   `RuntimeError::BuiltinArgumentShape`.
3. **`SpatialValueError::MalformedValue` is defensive-only.** A
   type-checked `.aicad` program's `Axis3`/`Frame3`/`Plane` argument is
   guaranteed the right shape by `cad_hir::typeck`; this variant exists
   for the same reason `RuntimeError::BuiltinArgumentShape` does
   ("trusts, but verifies"), not because it is expected to be reachable.
   `DegenerateDirection`/`NonOrthonormalFrame` are the genuine run-time
   failures this task's "reject invalid/degenerate spatial values
   explicitly" requirement is actually about — they depend on evaluated
   numeric values, not shape, so no type check can catch them.
4. **No `RuntimeBuiltin` was added.** `BuiltinFnId::Transform`'s own doc
   comment already recorded "no AICAD source-level vector/axis/rotation
   type exists yet to name a rotation unambiguously... Stage-3's own
   axis/frame/rotation foundation is `AICAD-075A`'s task, not this
   one's" — this task supplies that foundation; wiring a new builtin
   (`revolve`, `rotate`, `mirror`, `radial_pattern`) that actually
   consumes it through `cad_runtime::spatial` is `AICAD-076`/
   `AICAD-077`'s own scope, per this task's own "do not duplicate full
   AICAD-076/AICAD-077 feature implementation" limit.
5. **`Plane3` is not a `Frame3`.** Considered representing a mirror
   plane as a `Frame3` (reusing an existing type, zero new code) instead
   of adding `Plane3`. Rejected: a `Frame3` forces an arbitrary in-plane
   `x`/`y` choice a plane's own semantics (which side is "outside",
   mirroring) never depend on, and this task's own required semantic
   distinctions explicitly list `Plane` as its own concept, separate
   from `Frame3`. `Plane3::from_frame` gives a caller that already has a
   `Frame3` (e.g. a sketch plane, once `SketchPlane` is generalized) a
   direct, deterministic way to get the plane it determines.
6. **`SketchPlane` is not generalized in this task.** `cad_hir::sketch`'s
   own doc comment defers a general `Frame3`-based sketch plane to this
   task, but this task's own acceptance criteria (axis/frame/rotation
   for revolve/transform/mirror/circular-pattern) do not require it, and
   generalizing `SketchPlane` would touch `AICAD-075`'s already-proven
   sketch-lowering pipeline for no benefit this task's own consumers
   need — left as an explicit limitation (below) for whichever future
   task actually needs a non-world sketch plane.

## Tests / verification

- `cargo fmt --all -- --check` → clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings, full workspace (29 crates).
- `cargo test --workspace` → 0 failures across every crate.
- `cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1` →
  3/3 (Stage-2 gate proof unaffected).

Per-crate test counts (this invocation's changes only; every other
crate's count is unchanged from Batch S3-05's own last-verified totals):

| Crate | Before | After | Delta |
|---|---|---|---|
| `cad-kernel-api` | 23 | 26 | +3 (`Plane3`) |
| `cad-hir` | 225 | 226 | +1 (`Plane` construction; the 6->7 declaration-count test was updated, not added) |
| `cad-runtime` | 110 | 121 | +11 (new `spatial.rs`) |
| `cad-geometry-runtime` | 20 unit + 0 integration | 20 unit + 4 integration | +4 (new `tests/spatial_axis_frame_foundation.rs`) |

## Limitations

- `cad_hir::sketch::SketchPlane` remains a fixed three-world-plane enum,
  not a general `Frame3`-based plane — see "Design decisions" #6. Any
  future task wanting a sketch on an arbitrary plane needs to generalize
  it, reusing `Frame3`/`Plane3` from this task rather than inventing a
  new representation.
- No `RuntimeBuiltin` consumes `Axis3`/`Frame3`/`Plane` yet — `box`/
  `cylinder`/`transform`/`plate` are all unchanged from Stage 2/`AICAD-071`.
  `AICAD-076`/`AICAD-077` add the first consumers.
- Mirror's own kernel-adapter operation (a `Plane3`-based native/bridge
  function, since it cannot be a `Transform` — see "Architectural
  finding" above) does not exist yet; this task establishes the
  representation and the one supporting primitive (`signed_distance`),
  not the operation. `AICAD-077` will need to add a new native/bridge
  capability (mirroring `AICAD-075`'s own `ArcEdge`-capability precedent
  for "add exactly the missing kernel primitive a task needs").
- `Plane3::signed_distance` is the only plane-specific operation added;
  a full point-reflection helper (`point - 2 * signed_distance * normal`)
  was deliberately not added, since no task yet consumes it and adding
  it now would be speculating about `AICAD-077`'s own exact API shape.
- `cad_runtime::spatial`'s functions are not yet called from anywhere
  outside their own tests (confirmed intentional, not dead code — they
  are `pub`, and `cargo clippy`'s `dead_code` lint does not flag unused
  `pub` library items). `AICAD-076`/`AICAD-077` are the first real
  callers.

## Implications for AICAD-076/AICAD-077

- `AICAD-076` (revolve/extrude/hole/pocket): add a `BuiltinFnId::Revolve`
  (or similar) whose `dispatch_builtin` arm calls
  `cad_runtime::spatial::axis3_from_value` on the evaluated `Axis3`
  argument, then builds `GeometryOp::Revolve { profile, axis, angle }`
  exactly as this task's own geometry-backed test does directly.
- `AICAD-077` (mirror/circular pattern): reuse the same
  `axis3_from_value` for `radial_pattern`'s axis (proven interchangeable
  with revolve's own axis representation by this task's own test #2);
  add `plane3_from_value` for `mirror`'s plane, plus the new
  `Plane3`-based native/bridge mirror operation itself (see
  "Limitations").
- Any later general non-translation `transform`/`rotate` builtin should
  use `frame3_from_value`/`axis3_from_value` plus
  `Transform::rotation`/`Transform::compose`/`Transform::from_frames`
  directly — no new rotation convention should be introduced.

## Implications Stage 5/6 should preserve

- The right-hand-rule/Rodrigues rotation convention, `Transform`'s
  proper-rigid-only (no reflection) invariant, `Frame3`'s
  orthonormal-and-right-handed-within-`1e-6` invariant, and
  `Direction3`'s `< 1e-12`-degenerate-rejection threshold are now
  explicitly documented in `cad_kernel_api::geometry`'s own module doc
  comment as the fixed Stage-3 foundation. Later assembly/component
  frames (Stage 6) and advanced geometry (Stage 5) should extend this
  model (e.g. more `Frame3`/`Axis3`/`Plane3` values, more operations
  consuming them) rather than introducing a second, incompatible
  convention.
- Mirror's own future kernel operation must not be forced through
  `Transform` (determinant `+1` only) — any assembly-level "mirrored
  component" concept in Stage 6 needs the same `Plane3`-based approach,
  not a `Transform`-based one.

## Next dependency

Batch S3-06 continues with `AICAD-076` ("Implement high-level extrude/
revolve/hole/pocket"), which depends on this task (`depends_on:
[AICAD-075A]` in `project/TASKS.yaml`) and is now unblocked. Per
`project/CURRENT_STAGE.md`'s fixed batch list, Batch S3-06 has no
checkpoint of its own between `AICAD-075A` and `AICAD-076` — both
complete before the next checkpoint (`project/gates/
STAGE3-C_MODELING.md`, after Batches S3-06/S3-07/S3-08).
