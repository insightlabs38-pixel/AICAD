# AICAD-143 — Deterministic grounding, initial pose, and lowering

## Result

Added `cad-assembly-solver::grounding`: an explicit, deterministic
grounding/free-variable/initial-pose policy, plus the AICAD-owned lowering
from a resolved occurrence tree's `Mate`/`Joint` relations into
`adapter::AssemblyProblem`. `DL-30`'s explicit-grounding and deterministic-
representative-pose requirements are frozen here, before any iterative
solver exists to define them by accident.

## Changes

- **Grounding policy**: the assembly's own top-level (depth-1) occurrence is
  always the canonical ground — never a free variable, regardless of how
  many relations name it. Stage-6 source has no explicit-ground declaration
  syntax yet; this is the other half of `DL-30`'s "explicit if supplied,
  otherwise deterministic canonical" rule.
- **Free-variable policy**: only occurrences actually named by a mate
  subject or joint parent/child (other than the ground) become free
  6-scalar rigid-pose unknowns (`BTreeSet<OccurrencePath>`, so the free set
  is independent of `mates`/`joints` input order). An occurrence no
  relation touches keeps its authored pose exactly.
- **Pose parametrization**: each free occurrence's 6 scalars are a
  translation + axis-angle rotation delta, pivoted at that occurrence's own
  authored world origin, composed onto its authored `WorldPose`. The
  all-zero vector reproduces the authored pose exactly.
- **Representative-pose policy**: documented (not just incidental) — a
  damped Gauss-Newton/Levenberg-Marquardt step (`AICAD-144`) never assigns a
  nonzero correction to a direction with no Jacobian support, so an
  underconstrained free direction deterministically keeps its authored
  value rather than an arbitrary backend-dependent one.
- **Anchor geometry** (disclosed scope bound): a mate/joint subject's
  geometric meaning is its own occurrence's frame (origin, `+Z` primary
  axis, `+X` secondary axis) — not literal per-face kernel geometry. No
  kernel/OCCT dependency exists anywhere in this crate. Consequence:
  `MateKind::Tangent` has no realization at this granularity and is
  reported in a lowering's own `unsupported` list rather than silently
  mis-solved (mirrors `cad_constraints::sketch_solver`'s own
  `SolveStatus::Unsupported` precedent).
- `mate_residuals`/`joint_residuals` build each approved `MateKind`/
  `JointKind`'s equations from four geometric primitives (`point_diff`,
  `axis_align`, `perp_offset`/`along_offset`, `primary_diff`/
  `secondary_diff`); a joint family's declared `dof()` matches exactly the
  nullspace its own equations leave (verified directly for `Revolute`/
  `Lock` in tests).
- `linalg.rs`: shared dense-matrix utilities (`jacobian` via central finite
  differences, `rank` via pivoted row-echelon reduction, `solve_linear_
  system`, `gram`/`transpose_mul_vec`) used by both this module's own
  `structural_dof`/`structural_dof_at` and `crate::baseline`'s iteration.
- New `ASM-E008` diagnostic (`LoweringError`) for a mate/joint subject
  naming an occurrence absent from the resolved tree — fail-closed,
  mirroring `AssemblyBrokenReason::OccurrenceNotFound`'s own precedent.

## Verification

- `cargo test -p cad-assembly-solver` — `grounding` 9/9 PASS: root exclusion,
  untouched-occurrence exclusion, all-zero-reproduces-authored-pose,
  relation-order-independence of the free-variable set, `Tangent`
  unsupported reporting, unknown-occurrence fail-closed (+ `ASM-E008`), a
  `Lock` mate removing all 6 DOF, a `Revolute` joint leaving exactly 1 DOF,
  an untouched occurrence having 0 free DOF. `linalg` 6/6 PASS (Jacobian
  exactness on a linear map, full/rank-deficient/zero-matrix rank, linear
  solve + singular rejection).
- `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets
  --all-features -- -D warnings` — PASS.

## Limitations

`rank_tolerance`/`finite_difference_step` (`GroundingProfile::v1`) are fixed
absolute constants tuned for typical mm-to-m-scale assemblies, not
scale-relative — a disclosed limitation for assemblies at extreme physical
scale, matching `cad_validation::profile::ComparisonProfile`'s own
fixed-absolute-tolerance precedent, not a bug.

## Next

`AICAD-144` (the baseline solver behind this lowering).
