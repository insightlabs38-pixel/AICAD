# AICAD-073: Create solver-independent sketch constraint IR/adapter

## Status

Done. First task of Batch S3-05.

## Objective

`docs/plan/04_HIGH_LEVEL_MODELING_API.md` §4's baseline sketch-constraint
catalogue (`coincident`/`horizontal`/`vertical`/`parallel`/
`perpendicular`/`tangent`/`concentric`/`equal`/`symmetric`/`distance`/
`angle`/`radius`/`diameter`/`fixed`/`midpoint`) has no representation
anywhere in the compiler yet. `project/DECISION_LOG.md#DL-20` (`D11`,
recorded immediately ahead of this task per the active campaign brief)
requires the AICAD constraint IR — not a solver backend — to be
authoritative for every observable constraint semantic, with a
solver-independence boundary that permits exactly one initial solver
implementation without ever letting that backend redefine dimensional
semantics, constraint-kind meaning, or success/failure classification.
This task gives that IR its shape and defines the solver-adapter
boundary; it does not implement a numeric solver (see "Design decisions"
#1 and `AICAD-074`, this batch's next task).

## Base / resulting commit

- Base: `f7ca37f` (`AICAD-072`, `origin/claude/aicad-stage3-dev`'s HEAD
  at the start of this invocation).
- This task's commit: see `git log` on `origin/claude/aicad-stage3-dev`
  (single commit, `AICAD-073`).

## Files changed

- `crates/cad-constraints/Cargo.toml` — added dependencies on `cad-ast`,
  `cad-diagnostics`, `cad-hir`, `cad-types`, `cad-units` (previously an
  empty placeholder crate; see "Design decisions" #2 for why `cad-hir`
  specifically).
- `crates/cad-constraints/src/sketch_constraint.rs` (new) — the entire
  IR/adapter:
  - `ConstraintId`: identity type, mirroring
    `cad_hir::sketch::SketchEntityId`'s exact shape (embeds the owning
    `SketchId`, minted only by `ConstraintSet::add`).
  - `PointRef`: a reference to one specific point on a sketch entity
    (`LineStart`/`LineEnd`/`CircleCenter`/`ArcCenter`/`ArcStart`/
    `ArcEnd`) — the operand shape `coincident`/`distance`/`midpoint`
    need; plus `resolve_point`, a structural (non-solving) lookup back
    to the point's current `Point2` coordinate.
  - `SketchVariable`: the typed/dimensioned free-scalar vocabulary
    `DL-20` requires (`PointX`/`PointY`/`Radius`/`StartAngle`/
    `EndAngle`), each with a `.dimension()`; plus `sketch_variables`, a
    structural enumeration of every free scalar a `Sketch`'s own
    entities contribute (4 per line, 3 per circle, 5 per arc).
  - `ConstraintKind`: all 15 baseline constraint kinds from `docs/plan/
    04_HIGH_LEVEL_MODELING_API.md` §4, each with strongly-typed operands
    (see "Design decisions" #3 for the specific entity-kind restrictions
    on `horizontal`/`vertical`/`parallel`/`perpendicular`/`angle`/
    `concentric`/`distance`/`tangent`).
  - `Constraint`/`ConstraintSet`: an explicit, append-only, ordered
    constraint list owned per-sketch, mirroring
    `cad_hir::sketch::Sketch`'s own identity/ownership discipline.
    `ConstraintSet::add` validates structure/dimension only (operand
    ownership, entity-kind correctness, quantity dimension/positivity)
    — never a numerical satisfiability judgment.
  - `ConstraintIrError` (+ `to_diagnostic`, reusing the already-registered
    `CONSTRAINT` diagnostic family, codes `CONSTRAINT-E001`..`E006` — the
    first module to use this family): structural/dimensional validation
    only, mirroring `cad_hir::sketch::SketchIrError`'s own restraint.
  - `SolveStatus` (`Solved`/`Underconstrained { remaining_dof }`/
    `Overconstrained { conflicting }`): the solve-status vocabulary
    `DL-20` requires "at minimum".
  - `SolvedValues`/`SolveReport`: a solver's variable-assignment output
    bundled with its `SolveStatus`.
  - `SketchSolver`: the solver-independence-boundary trait (`fn
    solve(&self, sketch: &Sketch, constraints: &ConstraintSet) ->
    SolveReport`). No concrete implementation ships with this task (see
    "Design decisions" #1); a `#[cfg(test)]`-only mock implementation
    proves the trait is usable and swappable.
  - 17 tests (identity scoping, every entity-kind/dimension/positivity
    validation path per constraint kind, `equal`/`tangent`'s
    comparable-category rules, `resolve_point` correctness for every
    entity kind, `sketch_variables`' per-kind DOF count, every error
    variant's diagnostic conversion, and the mock-solver interface
    proof).
- `crates/cad-constraints/src/lib.rs` — module doc comment (crate-level,
  replacing the AICAD-002 placeholder text) plus `pub mod
  sketch_constraint;` and a `pub use` of its public API.

## Design decisions

1. **This task ships the constraint IR and the `SketchSolver` trait, but
   no concrete solver implementation.** `DL-20` permits "exactly one
   initial solver implementation... behind this interface" — that
   implementation is `AICAD-074`'s own scope ("Implement common sketch
   constraints needed by baseline parts"), not this task's, which is
   scoped by its own title to "IR/adapter" only. Shipping a real
   implementation here would risk `AICAD-074` inheriting an ad hoc
   numeric approach never actually reviewed against `DL-20`'s own
   solver-independence boundary. The trait's usability is proven instead
   by a `#[cfg(test)]`-only mock (`AlwaysUnderconstrained`), demonstrating
   the interface accepts an arbitrary backend without needing to change
   `ConstraintKind`/`SolveStatus`/`Constraint` at all.
2. **`cad-constraints` depends on `cad-hir` directly, reusing
   `SketchId`/`SketchEntityId`/`Sketch`/`Quantity` rather than
   re-implementing them.** This differs from `cad-hir::sketch`'s own
   precedent of *not* reaching down into `cad-geometry-api`/
   `cad-kernel-api` (a genuine cross-layer violation that module's own
   doc comment declines) — `docs/plan/01_SYSTEM_ARCHITECTURE.md` §1's
   pipeline diagram places the constraint subsystem immediately *after*
   "CAD-HIR" ("Parameter + constraint graph"), so this is a same-
   direction, HIR-adjacent dependency, not a backward reach. It is also
   the most literal way to satisfy `DL-20`'s "mirroring `DL-19`'s own
   identity precedent... not a parallel identity scheme": there remains
   exactly one sketch-entity identity type in the whole workspace.
   `AICAD-072`'s own report left this exact question open as `AICAD-073`'s
   "own judgment call" — this is that call, made and documented.
3. **Several baseline constraint kinds are scoped narrower than
   `docs/plan/04_HIGH_LEVEL_MODELING_API.md` §4's table.**
   `horizontal`/`vertical`/`parallel`/`perpendicular`/`angle` accept only
   `Line` operands (the table also allows "axes", but no `Axis`
   sketch-entity kind exists yet — `AICAD-075A`, still ahead in the fixed
   batch order, owns the axis/frame foundation); `concentric` accepts
   only `Circle`/`Arc` (same reason); `distance` is point-to-point only
   (the table's fuller "entities" also covers point-to-line/entity-to-
   entity distance, not needed for any Stage-3 baseline part); `tangent`
   requires at least one `Circle`/`Arc` operand (line-to-line tangency is
   geometrically undefined, so that pairing is a structural
   `ConstraintIrError` rather than a silently-accepted-then-unsatisfiable
   constraint). Every restriction is additive-compatible: extending a
   kind to accept an axis operand once one exists is a new match arm, not
   a breaking change to `ConstraintKind`'s own shape.
4. **No hard/soft constraint strength, and no domains beyond 2D sketch
   constraints.** `docs/plan/08_CONSTRAINTS_REQUIREMENTS_TESTS.md` §5's
   hard/soft distinction and its other five constraint domains (3D
   geometric, assembly, algebraic parameter, engineering requirement,
   optimization) belong to that document's much broader unified
   constraint/requirement engine — pulling any of that forward here would
   be exactly the "do not pull forward... a full verification framework"
   scope expansion the campaign brief forbids. `AICAD-073`/`074`/`075`'s
   own titles all say "sketch constraints"/"sketch profiles" — this task
   stays there.
5. **Diagnostic family reuse, not a new family.** `CONSTRAINT` is already
   present in `cad_diagnostics::DIAGNOSTIC_FAMILIES` (RFC-0005's fixed
   taxonomy) but had never been used by any module before this task;
   `ConstraintIrError` is the first to claim it, with fresh codes
   `CONSTRAINT-E001`..`E006`.

## Verification

- `cargo build -p cad-constraints` → clean.
- `cargo test -p cad-constraints` → 17/17 new tests pass.
- `cargo fmt --all -- --check` → clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings, full workspace (30 crates checked — `cad-constraints`
  now compiles real code for the first time).
- `cargo test --workspace` → 0 failures across every crate.
  `cad-constraints`: 0 -> 17. Every other crate's test count is unchanged
  from Batch S3-04's own last-verified totals (`cad-hir` stays at 225;
  confirmed by full-suite output — no other crate's `test result:` line
  changed).
- `cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1` →
  3/3 (Stage-2 gate proof unaffected).

## Limitations

- No numeric constraint solving (see "Design decisions" #1) — deliberately
  deferred to `AICAD-074`.
- `horizontal`/`vertical`/`parallel`/`perpendicular`/`angle`/`concentric`
  do not yet accept an axis operand; `distance` is point-to-point only;
  `tangent` rejects a line/line pairing (see "Design decisions" #3).
- No hard/soft constraint strength, no 3D-geometric/assembly/algebraic-
  parameter/engineering-requirement/optimization constraint domains (see
  "Design decisions" #4) — out of Stage-3 sketch-constraint scope
  entirely.
- `ConstraintSet`/`SketchVariable` do not yet integrate with any grammar/
  lowering/runtime `Value` — matching `AICAD-072`'s own "declare the IR
  before wiring it to source syntax" precedent; no `.aicad` source
  program can construct a constraint yet.
- No new third-party workspace dependency was added; `cad-constraints`
  now depends on four existing in-workspace crates it previously had zero
  dependencies on (`cad-ast`, `cad-diagnostics`, `cad-hir`, `cad-types`,
  `cad-units`), all already present elsewhere in the dependency graph.

## Regressions

None. This task only populated a previously-empty placeholder crate
(`cad-constraints`) and its `Cargo.toml` — no existing type, function, or
test in any other crate was changed. Full-workspace test count only grew
(cad-constraints: 0 -> 17; every other crate unchanged); the Stage-2
end-to-end gate proof still passes 3/3.

## Next dependency

`AICAD-074` ("Implement common sketch constraints needed by baseline
parts"), second task of Batch S3-05. Depends on `AICAD-073` (satisfied).
`AICAD-074` is expected to provide the first concrete `SketchSolver`
implementation `DL-20` permits, scoped to whichever constraint kinds a
baseline part (e.g. a bolt-pattern plate) actually needs — it should read
this module's own doc comment ("Scope cuts") before deciding whether any
of the narrower-than-plan-table restrictions (see "Design decisions" #3)
need to be lifted for that scope, and must not redefine
`ConstraintKind`/`SolveStatus`/`SketchVariable`'s own semantics while
doing so (`DL-20`'s solver-independence boundary).
