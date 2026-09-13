# AICAD-074: Implement common sketch constraints needed by baseline parts

## Status

Done. Second task of Batch S3-05.

## Objective

`AICAD-073` gave the Stage-3 sketch constraint IR its shape and a
`SketchSolver` trait as the solver-independence boundary (`DL-20`), but
shipped no concrete solver — the IR could validate a constraint's
structure/dimensions but nothing could actually determine whether a
`ConstraintSet` was satisfiable, let alone compute the geometry that
satisfies it. `DL-20` permits "exactly one initial solver
implementation... behind this interface"; this task provides that
implementation, covering every one of `docs/plan/
04_HIGH_LEVEL_MODELING_API.md` §4's 15 baseline constraint kinds, so a
baseline part's sketch (rough hand-placed geometry plus constraints) can
actually be solved into exact geometry.

## Base / resulting commit

- Base: `0fb633b` (`AICAD-073`, `origin/claude/aicad-stage3-dev`'s HEAD
  at the start of this invocation).
- This task's commit: see `git log` on `origin/claude/aicad-stage3-dev`
  (single commit, `AICAD-074`).

## Files changed

- `crates/cad-constraints/src/sketch_solver.rs` (new) — the entire
  solver:
  - `SketchSolverProfile` (+ `CURRENT_VERSION`, `v1()`): a versioned
    convergence-tolerance profile (`position_tolerance`/
    `angle_tolerance = 1e-9`, `max_iterations = 512`), mirroring
    `cad_validation::profile::ComparisonProfile`'s own versioned-constant
    pattern but intentionally a separate type (a different concern —
    solver-convergence tolerance, not cross-kernel geometry-comparison
    tolerance). See "Design decisions" #1 for how `max_iterations` was
    measured, not guessed.
  - `RelaxationSolver`: a Gauss-Seidel-style direct-projection relaxation
    `SketchSolver` implementing a fixed correction rule per
    `ConstraintKind` (e.g. `horizontal` averages a line's two endpoint
    `y`s; `distance` moves both points apart/together along their current
    connecting direction; `tangent` moves a circle's center to the
    correct offset from a line, or two circles' centers to
    external-tangency distance) applied once per constraint per
    iteration, for `max_iterations` iterations.
  - Classification (`DL-20`'s solve-status vocabulary plus one
    task-added extension): `Unsupported` (checked first — a constraint no
    correction rule could ever act on, e.g. both operands are immovable
    points) -> `Underconstrained { remaining_dof }` (a structural,
    entity-kind-aware equation-count table against
    `sketch_variables(sketch).len()`, checked regardless of whether
    relaxation happened to converge to *some* solution — `DL-20`
    forbids letting a converged-but-arbitrary branch stand in for a
    genuinely well-defined result) -> `Solved`/`Overconstrained` (only
    once the naive equation count is non-positive; every constraint's
    final residual determines which).
  - `SolveStatus::Unsupported { constraints: Vec<ConstraintId> }` (new
    variant, added to `sketch_constraint.rs`'s existing enum): reports a
    backend capability gap honestly, distinct from `Overconstrained`
    (which would wrongly imply the constraints are in tension) and
    `Solved` (which would wrongly imply the constraint was enforced).
  - 17 tests: one closed-form test per constraint kind (or kind-pairing,
    for `equal`/`tangent`/`symmetric`'s two supported operand
    categories), a full closed-rectangle integration test (four
    roughly-placed lines, `fixed`/`coincident`/`horizontal`/`vertical`/
    `distance` closing the loop, converging to the exact corners
    `(0,0)`/`(10,0)`/`(10,5)`/`(0,5)`), a dedicated `Underconstrained`
    test (a lone unconstrained line), a dedicated `Overconstrained` test
    (`fixed` and a conflicting `distance` on the same line), and two
    `Unsupported` tests (a mismatched-kind `symmetric` pairing; a
    `coincident` between two arc endpoints, both immovable by this
    solver).
- `crates/cad-constraints/src/sketch_constraint.rs`:
  - `arc_point` changed from private to `pub(crate)` so `sketch_solver`
    reuses the exact same arc-endpoint trigonometry as this module's own
    `resolve_point`, rather than a second re-implementation.
  - `SolveStatus` gained the `Unsupported` variant described above (an
    additive change — every existing variant/match arm elsewhere is
    unaffected; the one place matching `SolveStatus` in this same file's
    own tests already has a catch-all arm).
- `crates/cad-constraints/src/lib.rs` — module doc comment updated
  (`sketch_solver` bullet) plus `pub mod sketch_solver;` and a `pub use`
  of `RelaxationSolver`/`SketchSolverProfile`.

## Design decisions

1. **`max_iterations` was measured, not guessed.** A first pass shipped
   `max_iterations = 64`, based on the (incorrect) assumption that a
   chain of relative constraints halves its residual every iteration.
   The closed-rectangle integration test failed with that value
   (`Overconstrained`, most constraints' residuals still ~`3e-6` /
   `1e-9`-normalized far above `1.0`). A standalone measurement (this
   task's own convergence trace, reproduced in the module doc comment)
   showed the actual rate is closer to `0.7`x per iteration for a
   four-side closed loop connected purely by relative constraints:
   `~3e-6` at 64 iterations, `~1e-8` at 100, `~4e-12` at 150. `512` was
   chosen as at least an order of magnitude of headroom beyond the worst
   case this task's own test suite exercises, while remaining
   computationally trivial (`O(constraint count)` per iteration).
2. **`SolveStatus` gained a fourth, task-added `Unsupported` variant**
   rather than reusing `Overconstrained` or silently reporting `Solved`
   for a constraint this backend cannot act on. `DL-20`'s own text
   permits extending the vocabulary beyond "at minimum" three values, and
   explicitly forbids letting an arbitrary/unenforced outcome masquerade
   as a well-defined one — folding "I cannot move this" into
   "these constraints conflict" would misdescribe the actual problem to
   whatever consumes this status (a future diagnostic-rendering task),
   and folding it into `Solved` would be actively wrong (an unenforced
   constraint is not a satisfied one).
3. **A fixed, documented convention resolves every multi-solution
   constraint kind, rather than an ad hoc choice.** `tangent` between two
   circles/arcs always targets *external* tangency (center distance =
   sum of radii); `angle(a, b, value)` always means "`b`'s direction is
   `a`'s direction rotated by `value`" (mirroring `arc.direction`'s own
   already-established default-CCW convention elsewhere in this
   codebase); `symmetric` between two lines reflects `start`<->`start`/
   `end`<->`end` specifically. `DL-20` forbids a backend letting an
   *arbitrary* branch silently become semantics — a single, fixed,
   documented convention consistently applied is different from that,
   and mirrors precedent this codebase already established for an
   analogous default-direction choice.
4. **`coincident`/`distance`/`midpoint` cannot move an arc's `start`/
   `end` point directly** — only `line.start`/`line.end`/
   `circle.center`/`arc.center` are ever assigned to by a correction
   rule. Constraining an arc's endpoint would require deciding which of
   its center/radius/start-angle/end-angle to adjust instead, an
   ambiguity `docs/plan/04_HIGH_LEVEL_MODELING_API.md` §4's own baseline
   catalogue does not ask this task to resolve (it has no arc-endpoint-
   specific constraint). A constraint that can only be satisfied by
   moving an arc endpoint this way reports `Unsupported`, per design
   decision #2, rather than a silent no-op.
5. **Direct-projection Gauss-Seidel relaxation, not a general nonlinear
   solver.** No symbolic Jacobian, no Newton iteration, no line search —
   every correction rule is a closed-form geometric projection (average
   two points, rotate a direction, scale a length). This is sufficient
   for every baseline-part pattern this task's own test suite exercises
   (including the closed-rectangle loop, the hardest case tested) and
   avoids a much larger numerical-analysis surface (linearization,
   convergence radius, singular-Jacobian handling) `AGENTS.md`'s "no
   larger... system than the task needs" non-negotiable and this task's
   own title ("...needed by baseline parts") both counsel against.

## Verification

- `cargo build -p cad-constraints` → clean.
- `cargo test -p cad-constraints` → 35/35 (18 from `AICAD-073` unchanged
  + 17 new).
- `cargo fmt --all -- --check` → clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings, full workspace.
- `cargo test --workspace` → 0 failures across every crate.
  `cad-constraints`: 18 -> 35. Every other crate's test count is
  unchanged from `AICAD-073`'s own last-verified totals (`cad-hir` stays
  at 225).
- `cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1` →
  3/3 (Stage-2 gate proof unaffected).

## Limitations

- `RelaxationSolver` is one valid `SketchSolver` implementation, not a
  claim of optimality/robustness for arbitrary constraint graphs outside
  this task's own tested patterns — a future task may add a different
  backend behind the same trait without changing `ConstraintKind`/
  `SolveStatus` semantics (the whole point of `DL-20`'s boundary).
- The naive, entity-kind-aware equation-count table used for
  `Underconstrained` classification is a heuristic, not a rigorous
  structural-rigidity analysis (e.g. bipartite matching over the
  variable/constraint graph) — it can under-report a genuine localized
  conflict when unrelated degrees of freedom elsewhere in the same
  sketch remain free (a real limitation of global-count-based DOF
  accounting, not specific to this implementation). `DL-20` explicitly
  does not require a provably minimal conflict set, so this is an
  accepted, documented gap rather than a silently swept-under judgment
  call.
- No general nonlinear solver (see "Design decisions" #5) — a
  constraint combination requiring true simultaneous nonlinear
  resolution beyond this task's direct-projection rules (none arose in
  any baseline-part pattern tested) is not guaranteed to converge to
  `Solved` even when a solution exists.
- No `apply_solved_values`-style helper that writes a `SolveReport`'s
  `SolvedValues` back onto a mutable `Sketch` — tests read specific
  `SketchVariable`s directly from `SolvedValues` instead. Left to
  `AICAD-075` ("Lower solved closed sketch profiles to exact faces"),
  which is the first consumer that actually needs a fully-updated
  `Sketch` to lower into geometry, rather than building it here ahead of
  a concrete need.
- No new third-party workspace dependency was added; no existing crate
  outside `cad-constraints` itself was changed.

## Regressions

None. This task added a new module (`sketch_solver.rs`) and one
additive `SolveStatus` variant to the module `AICAD-073` introduced — no
existing type, function, or test in any other crate was changed, and no
existing `sketch_constraint.rs` test needed updating (the one place that
matches `SolveStatus` in that file's own tests already has a catch-all
arm). Full-workspace test count only grew (`cad-constraints`: 18 -> 35;
every other crate unchanged); the Stage-2 end-to-end gate proof still
passes 3/3.

## Next dependency

`AICAD-075` ("Lower solved closed sketch profiles to exact faces"), third
and final task of Batch S3-05. Depends on `AICAD-074` (satisfied).
`AICAD-075` will need a way to get a `Sketch` reflecting a `SolveReport`'s
`SolvedValues` (see "Limitations" above) before it can lower a solved
`Profile`'s entities into kernel-level wires/faces — its own judgment
call is whether to add that small apply-back step itself or request it
as a preceding gap-fill, and whether `Profile`'s existing lack of
closed-loop/self-intersection validation (`AICAD-072`'s own "Scope cuts")
needs to be addressed as part of "closed sketch profile" lowering or
deferred again.
