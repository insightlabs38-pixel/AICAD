# AICAD-097: Create deterministic perturbation runner

## Status

Done. Second task of Batch S4-06.

## Objective

Per `docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md` §5's own "Topological
naming benchmark" methodology ("bind downstream features to semantic
refs; perturb upstream dimensions/features; rebuild; check intended
entity selection; check ambiguity is reported rather than silently
misresolved") and `project/TASKS.yaml`'s own acceptance criterion:
execute perturbations deterministically enough that failures can be
reproduced and inspected.

## Base / resulting commit

Base: this invocation's own `AICAD-096` commit (`ea01b69`, same session).
This task's commit: see `git log`.

## Design decisions

- **A pure, ground-truth-agnostic execution primitive, not a
  metrics/grading tool.** `crates/cad-cli/src/perturbation.rs`'s
  `run_case` reports the *real* `cad_query::ResolutionOutcome` (or a
  build-time failure), never a "correct"/"silent-wrong" verdict against
  an expected classification — that comparison is `AICAD-098`'s own,
  separate job. Keeping this module ground-truth-agnostic means it is
  reusable by a case whose expected classification is not yet decided
  (e.g. a future adversarial fixture `AICAD-099` discovers), not only by
  the six corpus cases `AICAD-096` already proved.
- **Generalizes, rather than duplicates, `AICAD-096`'s own six hand-
  written test functions.** `PerturbationCase`/`run_case` is exactly the
  "build baseline, build perturbed, resolve the same query against both"
  shape those six tests each wrote independently, factored into one
  reusable function. This module's own test suite reproduces one of
  those six results (`12_add_remove_hole`) and the `06_fillet_viability`
  kernel-failure case through the generic runner, proving the
  generalization is faithful rather than merely similar-looking.
- **Source text in, never a path, in the core API.** `PerturbationCase::
  new` takes `baseline_source`/`perturbed_source` as plain strings — the
  same "in-memory string, thin file-reading wrapper" split
  `crate::build::build_source`/`run_build` and `crate::refs_check`
  already established (`AICAD-095`'s own report cites the identical
  precedent). `PerturbationCase::from_paths` is that thin wrapper, reading
  two files relative to a caller-supplied `repo_root` (never
  `env!("CARGO_MANIFEST_DIR")` baked into the library itself — that stays
  a test-only convenience, matching every other test file in this crate).
- **A distinct `KernelFailure`/`UnrelatedFailure` pair, mirroring `tests/
  semantic_refs/README.md`'s own six-class taxonomy exactly** (`RESOLVED_
  CORRECT`/`AMBIGUOUS`/`BROKEN` fold into `RunOutcome::Resolved`/
  `Ambiguous`/`Broken`, since "correct" vs. "silent wrong" needs ground
  truth this module does not have): a `ParametricBuildSession::new`
  failure carrying a real `GEOM-*` diagnostic is `KernelFailure` (a
  geometric-infeasibility outcome that never reached the resolver, per
  `06_fillet_viability`'s own established precedent); any other build
  failure, or a `cad_query::ResolveError` from `resolve` itself, is
  `UnrelatedFailure` — deliberately never folded into `Broken`, which
  `crate::eval`'s/`crate::resolve`'s own module doc comments already
  reserve for a real semantic outcome *about the reference*, not an
  execution failure.
- **Determinism proven, not merely asserted.** `RunOutcome`/
  `PerturbationRun` derive `PartialEq`/`Eq` over plain data only (counts,
  tags, diagnostic codes/messages — no live kernel handle, no iteration-
  order-sensitive collection), and `running_the_same_case_twice_produces_
  identical_results` runs the identical case twice (two fresh
  `OcctContext`s, two fresh builds) and asserts the two `PerturbationRun`s
  are equal, proving reproducibility empirically rather than only by
  code-reading.
- **No CLI subcommand added.** This task's own acceptance criterion is
  about the runner mechanism, not a `cad`-facing surface; `AICAD-098`
  (benchmark metrics) is the more natural point to decide whether a
  user-facing report belongs here, matching `AICAD-094`/`095`'s own
  precedent of landing a mechanism one task before its first real
  caller/reporting surface.

## What was implemented

`crates/cad-cli/src/perturbation.rs` (new module):

- **`PerturbationCase`** — `id`/`baseline_source`/`perturbed_source`/
  `query`; `new` (in-memory) and `from_paths` (file-reading wrapper).
- **`RunOutcome`** — `Resolved { count }` / `Ambiguous { count }` /
  `Broken` / `KernelFailure { diagnostic_code }` /
  `UnrelatedFailure { message }`.
- **`PerturbationRun`** — `id`/`baseline: RunOutcome`/
  `perturbed: RunOutcome`.
- **`run_case(&PerturbationCase) -> PerturbationRun`** — the runner
  itself; builds each variant independently via a fresh `OcctContext`/
  `ParametricBuildSession`, classifies a build failure into
  `KernelFailure`/`UnrelatedFailure`, and otherwise resolves `query` and
  maps the real `ResolutionOutcome` (or `ResolveError`) into the
  remaining `RunOutcome` variants.

`crates/cad-cli/src/lib.rs` — registers and re-exports the new module;
module doc comment updated.

## Tests / verification

5 new tests (`crates/cad-cli/src/perturbation.rs`):

- `reproduces_the_known_add_remove_hole_result` — the exact `AICAD-096`
  `12_add_remove_hole` result (`Resolved(1)` then `Broken`), reproduced
  through the generic runner.
- `running_the_same_case_twice_produces_identical_results` — two full
  independent runs of the same case, asserted equal.
- `a_parse_failure_is_an_unrelated_failure_not_a_semantic_outcome` —
  malformed source on both sides classifies `UnrelatedFailure`, never
  `Broken`/`KernelFailure`.
- `a_real_kernel_failure_is_classified_kernel_failure_not_broken` — a
  hand-built fillet-viability case (`fillet_radius` `8mm` -> `12mm`,
  the exact real `GEOM-E005` threshold `06_fillet_viability`'s own
  `case.md` already measured) reproduced through the generic runner.
- `from_paths_reads_real_files_off_disk` — the file-reading wrapper
  against the real `12_add_remove_hole` fixture files.

Commands run:

- `cargo fmt --all -- --check` -> clean (whole workspace).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  -> zero warnings (whole workspace).
- `cargo test -p cad-cli --lib perturbation` -> 5/5 passed.
- `cargo test --workspace` -> 0 failed, 1225 total passing tests (1219
  `AICAD-096` baseline + 6 new: the 5 above plus one pre-existing count
  correction from `cargo test`'s own per-binary reporting).

## Regressions

None — a new, additive module with no changes to any existing public
API's behavior (`ParametricBuildSession`/`cad_query::resolve` themselves
are unchanged by this task).

## Limitations

- **Not wired to a CLI subcommand or to the frozen corpus's own
  `results.json`/`scripts/ci/semantic_ref_harness.py grade` contract
  yet.** `AICAD-098` (benchmark metrics) is the natural next task to
  decide that reporting surface, per `AGENTS.md`'s "implement the
  smallest change that satisfies the task" — this task's own acceptance
  criterion is the runner mechanism and its determinism, not a report.
- **Inherits `AICAD-096`'s own two boundaries** (D31 `part`-scoped
  lineage evidence; `SpatialPredicate::Contains`/`NearestTo` still
  `EvalError::NotYetSpecified`) — `run_case` reports whatever
  `cad_query::resolve` actually returns given those boundaries; it does
  not work around either.

## Next dependency

`AICAD-098` (Implement benchmark metrics: correct / ambiguous-detected /
broken-detected / silent-wrong / durability), `depends_on: AICAD-097`
(satisfied).
