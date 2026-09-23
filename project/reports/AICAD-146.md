# AICAD-146 — redundancy and conflict diagnostics

## Result

`cad-assembly-solver` now distinguishes redundant-but-consistent relations
from mutually conflicting ones structurally, at the assembly's own
authored pose, with no iterative solve required.

## Changes

- `linalg.rs`: `rank` now delegates to a new `rank_with_pivot_rows`, which
  additionally returns the original row indices selected as pivots during
  Gaussian elimination — `rank`'s own public behavior/tests are unchanged.
- New `conflict.rs` module: `diagnose(occurrences, mates, joints, profile)
  -> AssemblyDiagnosis` lowers via `grounding::lower` (the solver's own
  lowering) and, for every declared residual `rank_with_pivot_rows` did
  not select as a pivot, tests whether appending its own
  `[Jacobian row | current value]` to the pivot rows' own augmented matrix
  raises the rank. If not, the residual's value is exactly the
  combination its dependent Jacobian row already predicts — `redundant`.
  If it does, the residual's value contradicts that prediction —
  `conflicting`. `AssemblyDiagnosis` carries `remaining_dof` (from
  `grounding::structural_dof`), `redundant`, `conflicting`, and
  `unsupported` side by side, so ordinary underconstraint, redundancy, and
  conflict are three separate fields rather than one collapsed
  classification. Every flagged entry is a `mate:`/`joint:` residual tag,
  never a matrix row index or solver variable.
- `lib.rs`: exports `conflict`, `AssemblyDiagnosis`, `ConstraintDiagnosis`,
  `diagnose`.

This is a purely structural, non-iterative complement to
`baseline::SolveOutcome::Overconstrained` (which only appears after the
iterative backend stalls). Two contradictory `Distance` mates are now
flagged as conflicting directly at the authored pose, before any solver
runs.

## Verification

- `cargo test -p cad-assembly-solver` — 43/43 PASS (5 new `conflict::tests`,
  1 new `linalg::tests` for pivot-row tracking).
- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — PASS
- `cargo test --workspace` — PASS (all crates, 0 failures)

Fixtures covered: ordinary underconstraint alone (no redundant/conflicting
entries), a duplicated `Coincident` mate (redundant, minimal — only the
second mate's own 3 residuals flagged, not all 6), two contradictory
`Distance` mates (conflicting, detected structurally with zero iterations),
deterministic ordering across repeated runs on the same input, and an
unsupported `Tangent` mate carried through separately from both lists.

## Limitations

The augmented-rank test is evaluated at one point (the authored pose for
`diagnose`; `diagnose_at` is exposed for reuse at any other point, e.g.
after a solve, mirroring `structural_dof`/`structural_dof_at`). A
nonlinear relation pair that happens to agree at the authored pose but
disagrees elsewhere would not be flagged conflicting by this one-point
test; `baseline::SolveOutcome::Overconstrained`'s post-hoc iterative check
remains the backstop for that case. No minimal-conflict-set guarantee is
made or required (`AGENTS.md`: only required if `TASKS.yaml` says so, and
it does not here) — when several pivot-independent rows exist, each is
tested against the same fixed pivot set, not against every combination.

## Next

`AICAD-147` (baseline kinematic evaluation) is next in batch `S6-06`.
