# AICAD-144 — Baseline assembly numerical solver

## Result

Added `cad-assembly-solver::baseline::BaselineSolver`, the one bounded
deterministic Levenberg-Marquardt backend behind `AssemblySolverAdapter`,
plus `solve_assembly`, the AICAD-owned entry point that lowers an
occurrence tree's mates/joints (`grounding::lower`), runs a backend, and
classifies the numeric result into `SolveOutcome`'s structured evidence.

## Changes

- `BaselineSolver::solve`: damped Gauss-Newton (Levenberg-Marquardt) —
  finite-difference Jacobian, damped normal equations
  `(JᵀJ + λI) delta = -Jᵀr`, accept/reject by cost decrease. Distinguishes
  two failure shapes: damping exceeding `damping_max` with no improving
  step ever found reports `AdapterOutcome::NotConverged` *before* the
  iteration budget is spent (further search cannot help); exhausting the
  full `max_iterations` budget without converging or stalling reports
  `AdapterOutcome::ResourceLimitExceeded`. Both are bounded — the solver
  always terminates.
- `BaselineSolverProfile` (versioned, `v1()`): embeds `GroundingProfile`,
  adds `max_iterations`/damping/`conflict_multiplier` constants.
- `solve_assembly` classifies `AdapterOutcome` into `SolveOutcome`:
  `Converged` + `remaining_dof == 0` (Jacobian rank at the *solution*, via
  `grounding::structural_dof_at`) → `Solved`; `Converged` with remaining DOF
  → `Underconstrained`; `NotConverged`/`ResourceLimitExceeded` with any
  residual's final magnitude exceeding `convergence_tolerance ×
  conflict_multiplier` → `Overconstrained` (with the offending residual
  tags as evidence); otherwise → `NotConverged`/`ResourceLimitExceeded`
  passed through. A budget of `0` (nothing actually attempted) is excluded
  from conflict reclassification — an untried large initial residual is not
  evidence of an inconsistent relation set.
- Only `Solved`/`Underconstrained` carry a `poses` map; every failure
  variant carries none — a caller cannot mistake a failed/partial numeric
  attempt for authoritative geometry (this task's own "failure never
  leaves partial authoritative poses" acceptance criterion).
- `poses_of` reconstructs each occurrence's solved `Transform` from the
  same pivot-based delta parametrization `grounding`'s own (private)
  lowering used, verified in this module's own tests rather than assumed.

## Verification

- `cargo test -p cad-assembly-solver` — `baseline` 6/6 PASS: a `Coincident`
  mate converging position while correctly reporting 3 remaining
  (orientation) DOF; a `Distance` mate converging to exactly the target
  separation with 5 remaining DOF; two contradictory `Distance` mates on
  the same pair reported `Overconstrained` with real conflicting-residual
  tags; repeated solves of the identical problem bit-for-bit identical
  (`assert_eq!` on the full `SolveOutcome`); a `max_iterations: 0` profile
  exercised directly against the adapter reporting
  `AdapterOutcome::ResourceLimitExceeded`; swapping `BaselineSolver` for
  `EchoAdapter` on an already-trivially-solved (no relations) problem
  produces the identical `Solved` outcome.
- Full crate: 28/28 tests PASS. `cargo fmt --all -- --check` PASS.
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  PASS (0 warnings after fixing `type_complexity`,
  `needless_range_loop` — annotated, matrix-index loops — and one
  `cloned_ref_to_slice_refs`). `cargo build --workspace` and
  `cargo test --workspace` — every existing suite (103/103) still PASS;
  `cargo tree -p cad-assemblies` confirms no solver crate entered its
  dependency tree.

## Limitations

- The anchor-frame abstraction (occurrence origin + fixed local axes, see
  `AICAD-143`'s report) means the baseline solver operates at occurrence-pose
  granularity, not literal per-face B-rep geometry, and does not propagate a
  solved occurrence's motion to its own un-mated children (a disclosed
  scope bound; full mechanism motion belongs to `AICAD-147`'s kinematic
  evaluation, not this batch).
- `MateKind::Tangent` is `unsupported` at this granularity (see AICAD-143).
- Conflict/redundancy classification here is a residual-magnitude threshold
  only (no minimal-conflict-set search) — sufficient for this task's own
  acceptance criterion; `AICAD-146` owns deeper redundancy/conflict
  diagnostics.

## Next

Batch `S6-06` (`AICAD-145` semantic DOF analysis, `AICAD-146` redundancy/
conflict diagnostics, `AICAD-147` kinematic evaluation, `AICAD-148`
Checkpoint B).
