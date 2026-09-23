# AICAD-142 — Numerical assembly-solver adapter contract

## Result

Added a new crate, `cad-assembly-solver`, that defines the `D28`/`DL-30`
solver-independence boundary one level stricter than `DL-20`'s own sketch-
solver precedent: `AssemblySolverAdapter` never sees a `Mate`/`Joint`/
`MateKind`/`OccurrenceTopologyRef` at all, only a semantic-neutral
`AssemblyProblem` (a flat variable vector plus opaque scalar `Residual`
equations) and an `AdapterOutcome` over that same numeric vocabulary.
`cad-assemblies` does not depend on this new crate — `cargo tree -p
cad-assemblies` still shows no solver crate anywhere in its tree, so
Checkpoint A's invariant is unaffected.

## Changes

- New crate `crates/cad-assembly-solver` (added to the workspace), depending
  on `cad-assemblies`/`cad-kernel-api`/`cad-types`/`cad-units`/
  `cad-diagnostics`, one direction only.
- `adapter.rs`: `AssemblyProblem` (shape-validated at construction:
  `initial_guess` length/finiteness), `Residual` (opaque `tag` + boxed
  evaluator, `Debug` implemented by hand since it holds an `Rc<dyn Fn>`),
  `AdapterSolution`, `AdapterOutcome` (`Converged`/`NotConverged`/
  `InvalidInput`/`ResourceLimitExceeded`), and the `AssemblySolverAdapter`
  trait.
- Three reference/mock adapters proving the contract without any real
  numerical algorithm: `EchoAdapter` (reports the initial guess as-is),
  `ZeroStartAdapter` (starts every variable at `0.0` instead), and
  `RejectingAdapter` (always `InvalidInput`). `EchoAdapter`/`ZeroStartAdapter`
  agreeing on an already-zero-residual problem is the "alternate-adapter
  mapping fixture" required check — two independently implemented backends
  reaching the same semantic conclusion.

## Verification

- `cargo test -p cad-assembly-solver` — 7/7 `adapter` tests PASS (mismatched-
  length/non-finite construction rejection, both mock adapters' converged/
  not-converged paths, `RejectingAdapter`, adapter-substitution agreement, a
  non-finite-residual invalid-input case).
- `cargo fmt --all -- --check` — PASS.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  PASS.
- `cargo tree -p cad-assemblies` — no `cad-assembly-solver` entry.

## Limitations

None material to this task's own scope. `crate::grounding`'s lowering
(`AICAD-143`) and `crate::baseline`'s real backend (`AICAD-144`) are
separate, later commits in this same batch.

## Next

`AICAD-143` (deterministic grounding/pose policy and the Mate/Joint →
`AssemblyProblem` lowering).
