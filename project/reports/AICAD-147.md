# AICAD-147 — baseline kinematic evaluation

## Result

`cad-assembly-solver` can now drive an assembly to an explicit typed
joint-coordinate assignment (a hinge angle, a slider distance, ...) and
report the resulting pose, reusing the existing lowering/solver/
classification machinery rather than a separate closed-form geometry path.

## Changes

- `baseline.rs`: `solve_assembly`'s classification logic is factored out
  into a new `pub fn classify(lowering, occurrences, outcome, profile) ->
  SolveOutcome`, so a caller with its own (modified) `Lowering` can reuse
  the same conflict/DOF classification without duplicating it.
  `solve_assembly` itself is behaviorally unchanged.
- New `kinematics.rs` module: `evaluate(occurrences, mates, joints,
  coordinates, adapter, profile) -> Result<SolveOutcome, KinematicsError>`.
  For each `(JointId, JointCoordinate)` pair: validates the joint exists
  and the coordinate respects its own declared family/limits
  (`Joint::validate_coordinate`) *before* lowering; then lowers normally
  (`grounding::lower`) and appends one extra pinning residual per
  coordinate scalar (a `(cos, sin)` pair for a rotation about the shared
  primary axis, a signed offset along it for a translation) to the same
  problem, and solves/classifies it through the unchanged adapter/
  `classify` path. A joint's own structural residuals plus its pinning
  residuals together fully determine its free DOF, so a fully-coordinated
  mechanism reports `SolveOutcome::Solved`.
- The free occurrence's own initial guess is seeded from the requested
  coordinate (rather than left at all-zero) before solving. This is a
  real fix, not cosmetic: a large rotation (found empirically at exactly
  180 degrees) is a genuine Gauss-Newton saddle point from the identity
  guess (`d(cos)/d(angle)` vanishes at `angle = 0` for any target), which
  reproducibly stalled at iteration 0. Seeding from the coordinate itself
  avoids the degeneracy.
- `lib.rs`: exports `kinematics`, `KinematicsError`, `evaluate_kinematics`,
  and `baseline::classify`.

Identity is untouched by design: `evaluate` returns the same `SolveOutcome`
shape `solve_assembly` does (`poses: Vec<(OccurrencePath, Transform)>`),
and every `OccurrencePath` in the output is one already present in the
input `occurrences` — motion only ever changes the paired `Transform`.

## Verification

- `cargo test -p cad-assembly-solver` — 49/49 PASS (7 new
  `kinematics::tests`).
- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — PASS
- `cargo test --workspace` — PASS (all crates, 0 failures)

Fixtures covered: a hinge sweep (0/30/90/180 degrees, verifying the arm's
own secondary axis rotates exactly as predicted, including the 180-degree
saddle-point case above), a slider sweep (0/5/-3 length units), a nested
hinge-then-slider mechanism solved with both coordinates pinned
simultaneously in one shared solve, a coordinate rejected before any
solve for violating the joint's own declared limits, an unknown-joint
coordinate rejected, and a direct check that `evaluate`'s output poses
carry exactly the same `OccurrencePath` set (same values, same order) the
input occurrences did.

## Limitations

The pinning-residual seed for a joint's own child is computed from its
parent's *authored* (unmoved) frame, not the parent's own solved pose in
a longer chain — correct for every fixture here (each downstream joint's
axis happens to be invariant to the upstream joint's own rotation) but a
disclosed heuristic, not a proof that seeding is always well-conditioned
for an arbitrary deep chain; the shared solve still finds the correct
final pose regardless (seeding only affects convergence robustness, never
correctness), and `AICAD-148`'s Checkpoint B corpus is the place to widen
coverage if a harder chain is found to need it. As required by the task,
no time integration/dynamics exists — `evaluate` takes one coordinate
assignment and returns one pose, never a trajectory.

## Next

`AICAD-148` (Checkpoint B) is next and final in batch `S6-06`.
