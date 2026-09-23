# AICAD-145 — semantic degrees-of-freedom analysis

## Result

`cad-assembly-solver` now reports assembly DOF in semantic terms (per
occurrence, attributed to the mate/joint tags touching it) instead of a bare
solver-facing rank number, and fixes a real DOF-answer gap `AICAD-143`'s own
`structural_dof` left open: an occurrence no relation ever names has 6 real
DOF as an unconstrained rigid body, not 0.

## Changes

- `grounding.rs`: `lower` now delegates to a new private
  `lower_with_free_variable_selection`, parametrized by which occurrences
  become free variables. Added `pub fn lower_for_dof_analysis`, using the
  `AllNonGround` policy (every non-ground occurrence is free, not just
  relation-touched ones) — `lower`'s own narrowing is a solver
  variable-count optimization, not a DOF claim, and DOF analysis must not
  confuse the two. `lower`'s own behavior/tests are unchanged.
- New `dof.rs` module: `analyze(occurrences, mates, joints, profile) ->
  AssemblyDof`. Computes DOF over `lower_for_dof_analysis`'s lowering via
  the existing `structural_dof`, then attributes each free occurrence's
  `OccurrenceDof.relations` to the `mate:`/`joint:` tags naming it.
  `AssemblyDof.rank_tolerance` carries the exact `GroundingProfile` value
  the reported rank was computed under, so the threshold is visible policy
  evidence rather than an implied fact about the assembly.
- `lib.rs`: exports `dof`, `AssemblyDof`, `OccurrenceDof`,
  `analyze_dof`, `lower_for_dof_analysis`.

## Verification

- `cargo test -p cad-assembly-solver` — 37/37 PASS (7 new `dof::tests`,
  1 new `grounding::tests` distinguishing the two lowering policies).
- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — PASS
- `cargo test --workspace` — PASS (all crates, 0 failures)

Fixtures covered: free rigid body (6 DOF), hinge/revolute (1 DOF),
slider/prismatic (1 DOF), fixed joint (0 DOF), a nested nested two-joint
chain (2 DOF, correctly additive), an unsupported `Tangent` mate (excluded
from the count, reported separately), and a threshold-boundary test
showing a genuine `1e-7` Jacobian coupling classifies as rank 1 under a
strict tolerance and rank 0 under a loose one — the same numeric evidence,
policy-dependent classification, per `AICAD-145`'s own acceptance
criterion.

## Limitations

DOF is reported as one aggregate rank/remaining-DOF pair over the whole
system plus a per-occurrence relation list, not a full per-direction
nullspace decomposition naming exactly *which* rotational/translational
axis remains free at each occurrence — sufficient for the required
fixtures (all non-branching chains where per-joint DOF is independently
additive) but a coupled/branching mechanism's remaining freedom is only
reported as a total, not decomposed per occurrence. No escalation: this
matches the task's own acceptance wording ("report remaining semantic
DOF... identify relevant semantic subjects/relations"), not a per-axis
breakdown requirement.

## Next

`AICAD-146` (redundancy/conflict diagnostics) is next in batch `S6-06`.
