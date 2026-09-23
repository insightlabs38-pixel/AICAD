# AICAD-148 — Checkpoint B: solver-neutral pose/DOF/conflict/kinematic realization

## Result

`crates/cad-assembly-solver/tests/checkpoint_b.rs` proves `AICAD-145`
(DOF)/`146` (conflict)/`147` (kinematics) work together, deterministically
and solver-neutrally, across a hinge, a slider, and a nested hinge+slider
mechanism. 7 tests, all passing.

## Fixture and coverage

Three canonical mechanisms (`root--Revolute-->arm`, `root--Prismatic-->
carriage`, `root--Revolute-->mid--Prismatic-->end`), each checked for all
four required surfaces:

- **Pose**: `solve_assembly` with no coordinate pinned reports
  `Underconstrained` with exactly the joint's own DOF remaining (1, 1, 2).
- **DOF**: `analyze_dof` reports the same `remaining_dof`.
- **Conflict**: `diagnose` reports zero `conflicting` entries (internal
  consistency). A real finding surfaced here: `Revolute`'s own
  `axis_align` residual (and `Prismatic`'s `primary_diff`/
  `secondary_diff`/`perp_offset`) are rank-deficient by construction —
  aligning two unit vectors is a rank-2 constraint expressed as 3
  equations — so `AICAD-146` correctly reports a nonzero `redundant` list
  for every one of these fixtures. This is `grounding::lower`'s own
  pre-existing residual shape, not a defect; the tests assert
  `conflicting.is_empty()` (the actual pass criterion) rather than a
  wrong zero-redundancy expectation.
- **Motion**: `evaluate_kinematics` at several coordinate values each
  reports `Solved`.

Then the three solver-neutrality properties:

- **Adapter substitution**: an independently-implemented `OracleHingeAdapter`
  that reports the analytically correct answer directly (never running
  `BaselineSolver`'s iteration) produces the same `Solved` pose
  `BaselineSolver` does, within `1e-6`.
- **Relation-order perturbation**: the nested fixture's joints and
  coordinate assignments declared in reverse order produce an equivalent
  pose.
- **Initial-pose perturbation**: the hinge fixture re-authored at three
  different starting translations converges to the identical relative
  pose.
- **Repeatability**: the nested mechanism evaluated twice from the same
  input is bit-for-bit (`PartialEq`) identical.

## Verification

- `cargo test -p cad-assembly-solver` — 56/56 PASS (49 unit + 7
  `checkpoint_b`)
- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — PASS
- `cargo test --workspace` — PASS, no regressions

## Architecture verification

- No backend-specific semantics leaked into any classification: `Solved`/
  `Underconstrained`/`remaining_dof`/`redundant`/`conflicting` are all
  expressed in `mate:`/`joint:`/`coordinate:` residual tags and
  `OccurrencePath`, never a matrix row/column index or solver variable —
  confirmed directly by the adapter-substitution test producing identical
  results from two structurally different adapters.
- Observable pose is deterministic: neither relation declaration order
  nor the authored starting translation (both irrelevant inputs a
  solver-dependent implementation could leak through) changed the
  normalized result.
- Identity: `evaluate_kinematics`'s output poses carry exactly the input
  `OccurrencePath` set (already directly proven in `AICAD-147`'s own
  `kinematic_motion_preserves_every_occurrences_own_identity` test); this
  checkpoint's fixtures reuse that same invariant without re-asserting it.

## Limitations

- Three mechanisms, not a large/deeply nested corpus — that breadth is
  `AICAD-157`'s job.
- The adapter-substitution oracle is hand-derived for the specific hinge
  fixture's own known-simple closed form (root and arm both authored at
  the origin); it is a targeted proof the contract is respected, not a
  general property test over arbitrary mechanisms.

## Next

Batch `S6-06` (`AICAD-145`/`146`/`147`/`148`) is complete. `AICAD-149`
(immutable configuration overlays, batch `S6-07`) is the next executable
task.
