# AICAD-098: Implement benchmark metrics: correct / ambiguous-detected / broken-detected / silent-wrong / durability

## Status

Done. Third and final task of Batch S4-06.

## Objective

Per `docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md` §5's own "Metrics"
table for the topological-naming benchmark (`correct reference
resolution` / `ambiguous-but-detected` / `broken-but-detected` /
`silent wrong resolution` (must approach zero) / `reference durability
distribution`) and `project/TASKS.yaml`'s own acceptance criterion:
metrics must be deterministic/reproducible and every silent-wrong result
must be directly inspectable.

## Base / resulting commit

Base: this invocation's own `AICAD-097` commit (`7e77592`, prior
session). This task's commit: see `git log`.

## Continuation note

A prior invocation of this campaign produced this task's own
`crates/cad-cli/src/metrics.rs` before hitting a usage limit, and its
content was handed to this invocation verbatim (module doc comment,
types, `grade`/`aggregate` functions, and full test module already
written). This invocation verified the file was genuinely absent from
the working tree and from `git log` (neither committed nor staged
anywhere on `claude/aicad-stage4-dev`), confirmed the file's types line
up exactly against the real `crate::perturbation::{PerturbationRun,
RunOutcome}` and `cad_references::DurabilityLevel` (the latter already
derives `Ord`, required for its use as a `BTreeMap` key) with no
adjustment needed, wrote the file, wired it into `crates/cad-cli/src/
lib.rs`, ran `cargo fmt` (one mechanical reformat of a single
`assert_eq!` call), and then ran every required check fresh in this
session rather than trusting the prior session's own (unrecorded) run.

## Design decisions

(Carried over from the file as received; recorded here since this
invocation did not originate them but is responsible for verifying they
are sound.)

- **Grading needs ground truth `crate::perturbation` deliberately does
  not carry.** `crate::perturbation::run_case` reports only the real,
  raw `RunOutcome` — it has no notion of "correct." `BenchmarkCase` pairs
  one `PerturbationRun` with an author-asserted `ExpectedOutcome` and
  `DurabilityLevel`, mirroring `cad_references::recipe::
  ConstructionStrategy::semantic_query`'s own established policy that
  the caller must classify a query's own strength — never inferred from
  the query's clause content.
- **Only the perturbed side is graded.** Every frozen `AICAD-079A`
  case's own "Expected classification" describes the *perturbed*
  variant's outcome; `grade` reads `run.perturbed` only. A baseline that
  fails to resolve would be a separate `AICAD-096`/`097`-level bug
  report, not a grading question this module answers.
- **The fail-closed direction is never "silent wrong."** Per `AGENTS.md`'s
  semantic-reference outcome policy, `Grade::SilentWrong` is produced
  only when the real outcome is `Resolved` but ground truth expected
  `Ambiguous`/`Broken` — never the reverse. A resolver reporting
  `Ambiguous`/`Broken` where `Resolved` was expected grades `Mismatch`: a
  real shortfall, but the safe direction, kept structurally distinct from
  the catastrophic one.
- **`UnrelatedFailure` is never counted toward resolver success or
  failure**, matching `tests/semantic_refs/README.md`'s own six-class
  taxonomy and `crate::perturbation`'s own established
  `RunOutcome::UnrelatedFailure` boundary.
- **`silent_wrong_ids: Vec<String>`, not only a count.** This task's own
  acceptance criterion requires each silent-wrong result to be directly
  inspectable; `aggregate` records every silent-wrong case's own `id` in
  input order, alongside the plain-count fields the plan's §5 "Metrics"
  table names.
- **`by_durability: BTreeMap<DurabilityLevel, usize>`**, reusing
  `cad_query::health::ReferenceHealthReport`'s own already-established
  aggregation shape (`AICAD-095`) rather than inventing a second one.

## What was implemented

`crates/cad-cli/src/metrics.rs` (new module):

- **`ExpectedOutcome`** — `CorrectResolvedReference` /
  `ExplicitAmbiguity` / `ExplicitBrokenReference` / `KernelFailure`. Does
  not include `SilentWrong`/`UnrelatedFailure` — never a valid *expected*
  ground truth, only a possible *grade*.
- **`BenchmarkCase`** — `run: PerturbationRun`, `expected:
  ExpectedOutcome`, `durability: DurabilityLevel`.
- **`Grade`** — `Correct` / `AmbiguousDetected` / `BrokenDetected` /
  `KernelFailure` / `SilentWrong` / `Mismatch` / `UnrelatedFailure`.
- **`grade(&PerturbationRun, ExpectedOutcome) -> Grade`** — the single
  comparison function described above.
- **`BenchmarkMetrics`** — `correct`/`ambiguous_detected`/
  `broken_detected`/`kernel_failure`/`silent_wrong`/`mismatch`/
  `unrelated_failure: usize`, `silent_wrong_ids: Vec<String>`,
  `by_durability: BTreeMap<DurabilityLevel, usize>`, plus a `total()`
  helper summing the seven count fields.
- **`aggregate(&[BenchmarkCase]) -> BenchmarkMetrics`** — grades and
  tallies every case in input order.

`crates/cad-cli/src/lib.rs` — registers `pub mod metrics;`.

## Tests / verification

9 tests (`crates/cad-cli/src/metrics.rs`), all synthetic/unit-level over
hand-built `PerturbationRun`s (no kernel/build dependency, matching this
module's own pure-function nature):

- `a_correctly_resolved_case_grades_correct`
- `a_correctly_detected_ambiguity_grades_ambiguous_detected`
- `a_correctly_detected_broken_reference_grades_broken_detected`
- `a_real_kernel_failure_grades_kernel_failure`
- `resolving_where_ambiguity_was_expected_is_silent_wrong` — the
  catastrophic direction.
- `resolving_where_a_broken_reference_was_expected_is_silent_wrong` —
  the catastrophic direction again, against an expected broken
  reference.
- `over_reporting_ambiguity_is_a_mismatch_not_silent_wrong` — the safe
  (never-silent-wrong) direction: proves the resolver being more
  conservative than expected is `Mismatch`, not `SilentWrong`.
- `an_unrelated_failure_is_never_counted_as_resolver_success_or_failure`
- `aggregate_tallies_every_grade_and_records_silent_wrong_ids` — three
  mixed cases; asserts every count, the recorded `silent_wrong_ids`,
  `total()`, and the per-durability breakdown.
- `aggregate_is_deterministic` — running `aggregate` twice over identical
  input produces a bit-for-bit-equal `BenchmarkMetrics`.

Commands run (this invocation, fresh, whole workspace):

- `cargo fmt --all -- --check` -> one mechanical diff in `metrics.rs`
  (a multi-line `assert_eq!` wrap); fixed via `cargo fmt --all`,
  re-checked clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  -> zero warnings.
- `cargo test --workspace` -> 0 failed; 1,235 total passing tests (9 new
  in `metrics::tests`, confirmed by name in the `cad-cli` lib test
  binary's own output).
- `python3 scripts/ci/semantic_ref_harness.py validate` ->
  `{"cases": 10, "splits": {"held_out": 3, "public": 7}, "status": "ok"}`
  (frozen corpus unaffected).
- `python3 scripts/ci/semantic_ref_harness.py self-test` ->
  `{"status": "ok", "self_test": "silent-wrong gate exercised"}`.
- `python3 scripts/ci/stage4_task_audit.py --check` -> `Stage-4 task
  metadata audit OK`.

## Regressions

None — a new, additive module with no change to any existing public
API's behavior (`crate::perturbation`, `cad_query::resolve`, and
`cad_references::DurabilityLevel` are all unchanged by this task).

## Limitations

- **Not yet wired to the frozen corpus's own real fixtures or to a CLI
  subcommand.** This task's own file, as received and verified, proves
  `grade`/`aggregate` correct against hand-built `PerturbationRun`s only;
  no test here runs `crate::perturbation::run_case` against a real
  `.aicad` fixture and feeds the result through `aggregate`. Producing
  the actual Stage-4 benchmark run (real corpus cases -> `PerturbationRun`
  -> `BenchmarkCase` with each case's own author-asserted
  `ExpectedOutcome`/`DurabilityLevel` -> `aggregate` -> a reported
  `BenchmarkMetrics`) and any `cad`-facing surface for it remain open;
  `AICAD-099`'s own adversarial campaign is the natural next consumer,
  since it needs exactly this aggregation to classify what it finds.
- **`ExpectedOutcome`/`durability` are supplied by the caller, not
  derived from `project/benchmarks/stage4_semantic_reference/`'s own
  corpus metadata.** No loader from that corpus's case files into
  `BenchmarkCase` exists yet; this task's own acceptance criterion is the
  metrics/grading logic itself, not that loader.
- Inherits every unresolved boundary already recorded in `AICAD-096`'s
  and `AICAD-097`'s own reports (D31 `part`-scoped lineage evidence;
  `SpatialPredicate::Contains`/`NearestTo` still
  `EvalError::NotYetSpecified`) — `grade` reports whatever
  `crate::perturbation::run_case` actually returns given those
  boundaries; it does not work around either.

## Next dependency

`AICAD-099` (Run adversarial bug-hunt; minimize and preserve every
silent-wrong reproducer), `depends_on: AICAD-098` (satisfied). Per the
campaign's own fixed-batch-order instruction, `AICAD-099` is Batch S4-07
and begins in a future invocation, not this one — Batch S4-06
(`AICAD-096`, `AICAD-097`, `AICAD-098`) is now complete.
