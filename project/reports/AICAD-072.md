# AICAD-072: Create minimal sketch entity IR

## Status

Done. First and only task of Batch S3-04.

## Objective

`docs/plan/04_HIGH_LEVEL_MODELING_API.md` §3's `sketch`/`line`/`circle`/
`arc`/`rectangle`/`polygon`/`slot` feature-catalogue entries have no
representation anywhere in the compiler yet. `project/DECISION_LOG.md
#DL-19` (`D3`, recorded immediately ahead of this task per the active
campaign brief) requires an explicit, kernel-independent semantic
`Sketch` object with deterministic local entity identity and no hidden
global mutable registration. This task gives that object its minimal IR
shape.

## Base / resulting commit

- Base: `16b6070` (`AICAD-070`/`AICAD-071`, `origin/claude/
  aicad-stage3-dev`'s HEAD at the start of this invocation).
- This task's commit: see `git log` on `origin/claude/aicad-stage3-dev`
  (single commit, `AICAD-072`).

## Files changed

- `crates/cad-hir/src/sketch.rs` (new) — the entire IR:
  - `Point2`/`Vector2`/`Direction2`: pure 2D math primitives, mirroring
    `cad_kernel_api::geometry`'s `Point3`/`Vector3`/`Direction3` shape and
    invariants exactly, defined independently rather than imported (see
    "Design decisions" #1).
  - `Quantity`: a typed magnitude (`f64` + `cad_units::OperandType`),
    mirroring `cad_geometry_api::ir::Quantity`'s identical shape as an
    independent re-implementation (see "Design decisions" #1).
  - `RotationDirection`: `arc`'s sweep-orientation enum.
  - `SketchId`/`SketchEntityId`: identity types (see "Design decisions"
    #2).
  - `SketchPlane`: a fixed `WorldXy`/`WorldXz`/`WorldYz` enum (see
    "Design decisions" #3).
  - `SketchEntityKind`/`SketchEntity`: the three primitive entity shapes
    (`Line`/`Circle`/`Arc`) plus construction-flag/span metadata.
  - `Profile`: an explicit, ordered aggregation of one sketch's own
    entity ids (what `rectangle`/`polygon`/`slot` return).
  - `SketchIrError` (+ `to_diagnostic`, reusing the `GEOM` diagnostic
    family, codes `GEOM-E010`..`GEOM-E015`): structural/dimensional
    validation only, mirroring `cad_geometry_api::ir::GeometryIrError`'s
    own restraint (see "Design decisions" #4).
  - `Sketch`: owns a plane and an append-only entity list; `add_line`
    (infallible), `add_circle`/`add_arc` (dimension/positivity-checked),
    `make_profile` (non-empty + same-sketch-ownership checked), and the
    three composite constructors `add_rectangle`/`add_polygon`/
    `add_slot`, each expanding into `Line`/`Arc` primitives and returning
    a `Profile`.
  - 23 tests (identity scoping, every dimension/positivity/structural
    error path, rectangle rotation correctness, polygon open/closed
    wrapping, slot geometry correctness via its own derivation, every
    error variant's diagnostic conversion).
- `crates/cad-hir/src/lib.rs` — `pub mod sketch;` plus a `pub use` of its
  public types (`Quantity` re-exported as `SketchQuantity` to avoid a
  bare, easily-confused name at the crate root); module doc comment
  updated with a `sketch` bullet.

## Design decisions

1. **`Point2`/`Vector2`/`Direction2`/`Quantity` are independent, minimal
   re-implementations, not cross-layer dependencies.** `cad-hir` sits
   *above* `cad-geometry-api`/`cad-kernel-api` in `docs/plan/
   01_SYSTEM_ARCHITECTURE.md` §5's pipeline (Source AST -> HIR -> Feature
   IR -> Geometry IR -> Kernel call graph); depending on either crate
   from `cad-hir` would be a downward reach across layers this task does
   not need to cross, and no shared "pure 2D/3D math" crate exists to
   hold one canonical definition (adding one would itself be exactly the
   kind of new-crate/new-abstraction move `AGENTS.md` rules out for what
   one task needs). This mirrors `crate::ids`'s own module doc comment
   precedent for `BindingId` vs. `cad_compiler::binder::Symbol` — "a
   small, independent re-implementation... scoped to exactly what [this
   layer] needs" — and `cad_runtime::value::NumberValue`'s already-
   identical shape to `cad_geometry_api::ir::Quantity`. `Point2`'s
   coordinates are raw canonical-unit `f64` (not per-field `Quantity`s),
   matching `Point3`'s own established convention rather than inventing
   a new one.
2. **`SketchEntityId` embeds its owning `SketchId`; `SketchId` itself is
   a plain, publicly-constructible newtype for now.** `DL-19` requires
   identifying "both the owning sketch and the entity itself" — done
   structurally in the id's own shape rather than by caller convention.
   `SketchEntityId`s are minted only by their owning `Sketch`'s own
   `add_line`/`add_circle`/`add_arc` (private constructor, sequential
   per-sketch counter), mirroring `cad_geometry_api::ir::GeomId`'s "minted
   only by its owning graph" precedent exactly. `SketchId` cannot yet
   mirror `BindingId`'s "minted by one authoritative counter" precedent
   literally, because no task has yet built the "multiple sketches in one
   lowering context" registry that would need to own that counter (the
   `Lowerer`/`GeometryGraph` analogue for sketches does not exist before
   grammar/lowering wiring, which this task deliberately does not add —
   see #5 below). This is documented explicitly in the module doc comment
   rather than silently left implicit, and does not affect what `DL-19`
   actually requires this task to get right (the entity-local identity).
3. **`SketchPlane` is a fixed three-variant world-plane enum, not a
   general `Frame3`/`FaceRef`.** The plan doc's own `sketch.plane:
   Plane|FaceRef|Frame3` signature wants general framing eventually, but
   `AICAD-075A` (Batch S3-06, still ahead of this task in the fixed
   batch order) is explicitly charged with establishing "one coherent
   minimal axis/frame/rotation foundation for revolve, transform, mirror,
   and circular patterns" rather than each feature inventing its own.
   Building this sketch's own arbitrary-frame math now would risk
   exactly that outcome, and would likely need reworking once that
   foundation lands. `FaceRef`-based planes additionally require Stage-4
   semantic references (`AICAD-079A`/Stage 4, explicitly forbidden
   before then). Deferred, not silently dropped — documented in the
   module doc comment's "Scope cuts".
4. **No closed-profile / geometric-validity checking.** `Profile` is only
   an explicit, ordered id list; nothing here checks that the referenced
   entities actually close a loop, or any other numerical-tolerance
   geometric property. "Solved closed profile" is `AICAD-073`/
   `AICAD-074`/`AICAD-075`'s own constraint-solving territory (Batch
   S3-05, still ahead) — adding a tolerance-based closure check here
   ahead of that work would risk a second, competing tolerance policy,
   which `project/OWNER_DECISIONS.md#D19`'s D5 precedent treats as
   exactly the kind of numeric-tolerance decision that must not be made
   twice. `SketchIrError`'s variants are all static/structural/
   dimensional, mirroring `GeometryIrError`'s identical restraint (never
   a numerical geometric-validity judgment — that stays the kernel's/
   solver's job).
5. **No grammar/lowering integration; `rectangle`'s `corner_radius` is
   not implemented.** No `sketch { ... }` block syntax, builtin-function
   signature, or runtime `Value` variant exists yet for any of this
   module's types — matching `AICAD-070`'s own precedent of landing a
   data model before wiring it to source syntax (`geometry_types.rs`
   landed six struct *declarations* with no runtime construction path
   until a separate task closed that gap). `rectangle`'s `corner_radius`
   defaults to `0mm` in the plan doc and is not built here — rounding a
   profile's corners is fillet-shaped (an operation on already-built
   entities), not a distinct entity kind, and blocks no Batch S3-04/
   S3-05 dependency.
6. **Diagnostic family reuse, not a new family.** `cad_diagnostics::
   DIAGNOSTIC_FAMILIES` is a fixed RFC-0005 taxonomy (`PARSE`/`TYPE`/
   `UNIT`/`RUNTIME`/`BUDGET`/`GEOM`/`TOPO`/`REF`/`CONSTRAINT`/`ASM`/
   `TEST`/`REQ`/`IMPORT`/`EXPORT`/`DFM`/`SIM`/`PKG`/`SEC`); adding a new
   `SKETCH` family would be a public-schema change this task's
   `escalate_if` list does not authorize. `SketchIrError` reuses `GEOM`
   (sketch entities are a geometry-IR-adjacent domain) with fresh codes
   `GEOM-E010`..`GEOM-E015`, chosen past the highest code already used
   anywhere in the workspace (`cad_geometry_api`'s `GEOM-E001`..`E004`,
   `cad_feature_graph`/`cad_geometry_runtime`'s own already-collided
   `GEOM-E005`/`E006` — a pre-existing minor cross-crate numbering
   overlap this task did not introduce and leaves alone, being unrelated
   to `AICAD-072`'s own scope) to avoid adding a further collision.

## Test coverage

`crates/cad-hir/src/sketch.rs`, 23 new tests:

- Identity: sequential per-sketch entity ids; two sketches never share
  id space; `Sketch::get` rejects a foreign id.
- `circle`/`arc`: accepts valid input; rejects a zero/negative radius;
  rejects a wrong-dimension radius/angle parameter; records every field
  correctly including `construction`/`direction`.
- `make_profile`: rejects an empty list; rejects an entity from a
  different sketch.
- `rectangle`: produces exactly four `Line` entities of the requested
  size (checked via bounding box); rejects a non-positive size; a 90-
  degree rotation of a square leaves its bounding box unchanged (rigid-
  rotation correctness check).
- `polygon`: closed wraps back to the first point; open does not; rejects
  fewer than two points.
- `slot`: produces exactly two `Line` + two `Arc` entities with the
  expected offset/length/radius/sweep-angle/direction; rejects a
  degenerate (coincident-endpoint) centerline; rejects a non-positive
  width.
- Every `SketchIrError` variant converts to a well-formed `Diagnostic`
  with a valid `GEOM-Exxx` code.
- `Point2`/`Vector2`/`Direction2` basic arithmetic (subtraction,
  translation, normalization, perpendicular).
- `Quantity::of` dimension reporting.

## Exact verification commands/results

```
cargo build -p cad-hir
```
→ clean.

```
cargo test -p cad-hir sketch::
```
→ 23/23 new tests pass.

```
cargo fmt --all -- --check
```
→ one pass of `cargo fmt --all` needed to normalize this task's own new
code (three long-line/argument-list wraps); clean after.

```
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
→ zero warnings across the full workspace (29 crates checked).

```
cargo test --workspace
```
→ 0 failures across every crate. `cad-hir`: 225 (up from 202 before this
task — the 23 new `sketch` tests; every other crate's count unchanged
from `AICAD-071`'s own last-verified totals, confirmed by full-suite
output).

```
cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1
```
→ 3/3 (Stage-2 gate proof unaffected — no existing type, builtin, or
runtime behavior was touched by this task; it added a new, currently
unreferenced module only).

## Limitations / explicit non-goals

- No grammar, builtin-function signature, or runtime `Value` variant
  consumes this IR yet (see "Design decisions" #5).
- `SketchPlane` supports only the three world reference planes, not a
  general `Frame3`/`FaceRef` (see "Design decisions" #3).
- No closed-profile / geometric-validity / self-intersection checking
  (see "Design decisions" #4) — deliberately deferred to `AICAD-073`/
  `074`/`075`.
- `rectangle`'s `corner_radius` is not implemented (see "Design
  decisions" #5).
- `SketchId` has no authoritative minting registry yet (see "Design
  decisions" #2) — a plain, publicly-constructible newtype until a
  future task builds the "multiple sketches in one lowering context"
  container that would own that counter.
- No new third-party workspace dependency was added; no existing crate's
  `Cargo.toml` dependency list changed (this task's new code uses only
  `cad-hir`'s existing dependencies: `cad-ast`, `cad-diagnostics`,
  `cad-types`, `cad-units`).

## Regressions

None. This task only added a new, currently-unreferenced module
(`crates/cad-hir/src/sketch.rs`) and its `lib.rs` wiring — no existing
type, function, or test was changed. Full-workspace test count only grew
(202 -> 225 in `cad-hir`; every other crate unchanged); the Stage-2
end-to-end gate proof still passes 3/3.

## Next dependency

`AICAD-073` ("Create solver-independent sketch constraint IR/adapter"),
first task of Batch S3-05, per `project/DECISION_LOG.md#DL-20` (D11,
already resolved: constraint identity "mirroring `DL-19`'s own
sketch-entity-identity precedent", a solver adapter that owns numerical
algorithms only). `AICAD-073` will need to reference this task's
`SketchId`/`SketchEntityId` to express which entities a constraint
relates, and should read this module's own doc comment ("Scope cuts")
before deciding whether/how to thread `SketchPlane`/`Profile` into its
own IR.
