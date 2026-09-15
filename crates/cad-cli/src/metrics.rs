//! `AICAD-098`: benchmark metrics over a set of real
//! [`crate::perturbation::PerturbationRun`]s — the concrete "Metrics"
//! table `docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md` §5 names for the
//! topological-naming benchmark (`correct reference resolution` /
//! `ambiguous-but-detected` / `broken-but-detected` /
//! `silent wrong resolution` (must approach zero) / `reference durability
//! distribution`), reusing `tests/semantic_refs/README.md`'s own six-class
//! outcome taxonomy and `cad_query::health::ReferenceHealthReport`'s own
//! established `BTreeMap<DurabilityLevel, usize>` aggregation shape.
//!
//! # Grading needs ground truth `crate::perturbation` deliberately does not carry
//!
//! [`crate::perturbation::run_case`] reports the real, raw
//! [`crate::perturbation::RunOutcome`] — it has no notion of "correct."
//! Turning that into a grade needs a case's own **author-asserted**
//! expected outcome ([`ExpectedOutcome`]) and durability
//! ([`cad_references::DurabilityLevel`]) — never inferred from the
//! query's own clause content, exactly mirroring
//! `cad_references::recipe::ConstructionStrategy::semantic_query`'s own
//! already-established policy ("the caller must classify the query's own
//! strength... anything else is rejected"). [`BenchmarkCase`] pairs one
//! [`crate::perturbation::PerturbationRun`] with both.
//!
//! # Grading only the perturbed side
//!
//! Every frozen `AICAD-079A` case's own "Expected classification"
//! describes the *perturbed* variant's outcome (the whole point of the
//! benchmark methodology, `docs/plan/16...` §5: "perturb... rebuild...
//! check intended entity selection") — [`grade`] grades
//! `run.perturbed` only. A baseline build failing to resolve at all would
//! be a different, `AICAD-096`/`097`-level bug report, not a benchmark
//! grading question this module answers.
//!
//! # The fail-closed direction is never "silent wrong"
//!
//! Per `AGENTS.md`'s own semantic-reference outcome policy: correct
//! resolution and explicit ambiguity/breakage are both good; only a
//! resolver that silently *picks* an answer where ambiguity/breakage was
//! actually present is catastrophic. [`grade`] therefore only ever
//! produces [`Grade::SilentWrong`] when the real outcome was `Resolved`
//! but the case's own ground truth says it should have been `Ambiguous`
//! or `Broken` — never the reverse (a resolver reporting `Ambiguous`/
//! `Broken` where `Resolved` was expected is [`Grade::Mismatch`]: a real
//! shortfall worth investigating, but the *safe* direction, not the
//! catastrophic one).

use std::collections::BTreeMap;

use cad_references::DurabilityLevel;

use crate::perturbation::{PerturbationRun, RunOutcome};

/// A benchmark case's own author-asserted ground truth — mirrors
/// `project/benchmarks/stage4_semantic_reference/README.md`'s own
/// "Taxonomy: expected-result classification" table's first four
/// (target-outcome) columns exactly, and
/// `scripts/ci/semantic_ref_harness.py`'s own `CANONICAL` mapping
/// one layer up (`correct_resolved_reference`/`explicit_ambiguity`/
/// `explicit_broken_reference`/`kernel_failure`). `silent_wrong_
/// resolution`/`unrelated_build_failure` are deliberately not
/// constructible here — that README's own doc comment: "not a target
/// outcome for any case" — a grader needs names for them
/// ([`Grade::SilentWrong`]/[`Grade::UnrelatedFailure`]), but no case's own
/// ground truth ever declares one as *expected*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedOutcome {
    CorrectResolvedReference,
    ExplicitAmbiguity,
    ExplicitBrokenReference,
    KernelFailure,
}

/// One case's own real result, paired with the ground truth
/// [`grade`] needs to grade it — see this module's own doc comment.
#[derive(Debug, Clone)]
pub struct BenchmarkCase {
    pub run: PerturbationRun,
    pub expected: ExpectedOutcome,
    pub durability: DurabilityLevel,
}

/// The real outcome of comparing one case's actual perturbed-side
/// [`RunOutcome`] against its own [`ExpectedOutcome`] — never itself a
/// count; [`aggregate`] tallies these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    /// Expected `CorrectResolvedReference`, got `Resolved`.
    Correct,
    /// Expected `ExplicitAmbiguity`, got `Ambiguous`.
    AmbiguousDetected,
    /// Expected `ExplicitBrokenReference`, got `Broken`.
    BrokenDetected,
    /// Expected `KernelFailure`, got `KernelFailure`.
    KernelFailure,
    /// Expected `ExplicitAmbiguity`/`ExplicitBrokenReference`, but the
    /// resolver silently `Resolved` a candidate anyway — catastrophic,
    /// per this module's own doc comment.
    SilentWrong,
    /// The real outcome exists (not an infrastructure failure) but does
    /// not match the expected class and is not the `SilentWrong`
    /// direction above (e.g. expected `CorrectResolvedReference` but got
    /// `Ambiguous`/`Broken`/`KernelFailure`, or expected `KernelFailure`
    /// but the build actually succeeded) — a real shortfall to
    /// investigate, but fail-closed, never conflated with silent wrong.
    Mismatch,
    /// The run itself failed for a reason unrelated to resolution
    /// (`RunOutcome::UnrelatedFailure`) — never counted toward resolver
    /// success or failure, per `tests/semantic_refs/README.md`'s own
    /// taxonomy.
    UnrelatedFailure,
}

/// Grades `run.perturbed` against `expected` — see this module's own doc
/// comment for why only the perturbed side is graded.
pub fn grade(run: &PerturbationRun, expected: ExpectedOutcome) -> Grade {
    match (&run.perturbed, expected) {
        (RunOutcome::UnrelatedFailure { .. }, _) => Grade::UnrelatedFailure,
        (RunOutcome::Resolved { .. }, ExpectedOutcome::CorrectResolvedReference) => Grade::Correct,
        (RunOutcome::Ambiguous { .. }, ExpectedOutcome::ExplicitAmbiguity) => {
            Grade::AmbiguousDetected
        }
        (RunOutcome::Broken, ExpectedOutcome::ExplicitBrokenReference) => Grade::BrokenDetected,
        (RunOutcome::KernelFailure { .. }, ExpectedOutcome::KernelFailure) => Grade::KernelFailure,
        (
            RunOutcome::Resolved { .. },
            ExpectedOutcome::ExplicitAmbiguity | ExpectedOutcome::ExplicitBrokenReference,
        ) => Grade::SilentWrong,
        _ => Grade::Mismatch,
    }
}

/// Deterministic, reproducible aggregate counts over a set of
/// [`BenchmarkCase`]s — `docs/plan/16...` §5's own "Metrics" table, plus
/// [`ReferenceHealthReport`]'s own established durability-distribution
/// shape. Every silent-wrong case's own `id` is recorded directly
/// (`silent_wrong_ids`), per this task's own acceptance criterion ("each
/// silent-wrong result is directly inspectable") — never only a count.
///
/// [`ReferenceHealthReport`]: cad_query::health::ReferenceHealthReport
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BenchmarkMetrics {
    pub correct: usize,
    pub ambiguous_detected: usize,
    pub broken_detected: usize,
    pub kernel_failure: usize,
    pub silent_wrong: usize,
    pub mismatch: usize,
    pub unrelated_failure: usize,
    pub silent_wrong_ids: Vec<String>,
    pub by_durability: BTreeMap<DurabilityLevel, usize>,
}

impl BenchmarkMetrics {
    pub fn total(&self) -> usize {
        self.correct
            + self.ambiguous_detected
            + self.broken_detected
            + self.kernel_failure
            + self.silent_wrong
            + self.mismatch
            + self.unrelated_failure
    }
}

/// Grades and tallies every case in `cases`, in order — deterministic
/// (no case is graded against any other; order only affects
/// `silent_wrong_ids`'s own reported sequence, which mirrors input order
/// exactly).
pub fn aggregate(cases: &[BenchmarkCase]) -> BenchmarkMetrics {
    let mut metrics = BenchmarkMetrics::default();
    for case in cases {
        match grade(&case.run, case.expected) {
            Grade::Correct => metrics.correct += 1,
            Grade::AmbiguousDetected => metrics.ambiguous_detected += 1,
            Grade::BrokenDetected => metrics.broken_detected += 1,
            Grade::KernelFailure => metrics.kernel_failure += 1,
            Grade::SilentWrong => {
                metrics.silent_wrong += 1;
                metrics.silent_wrong_ids.push(case.run.id.clone());
            }
            Grade::Mismatch => metrics.mismatch += 1,
            Grade::UnrelatedFailure => metrics.unrelated_failure += 1,
        }
        *metrics.by_durability.entry(case.durability).or_insert(0) += 1;
    }
    metrics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perturbation::RunOutcome;

    fn run(id: &str, baseline: RunOutcome, perturbed: RunOutcome) -> PerturbationRun {
        PerturbationRun {
            id: id.to_string(),
            baseline,
            perturbed,
        }
    }

    #[test]
    fn a_correctly_resolved_case_grades_correct() {
        let r = run(
            "11_extrusion_resize",
            RunOutcome::Resolved { count: 1 },
            RunOutcome::Resolved { count: 1 },
        );
        assert_eq!(
            grade(&r, ExpectedOutcome::CorrectResolvedReference),
            Grade::Correct
        );
    }

    #[test]
    fn a_correctly_detected_ambiguity_grades_ambiguous_detected() {
        let r = run(
            "01_topology_split_merge",
            RunOutcome::Resolved { count: 1 },
            RunOutcome::Ambiguous { count: 2 },
        );
        assert_eq!(
            grade(&r, ExpectedOutcome::ExplicitAmbiguity),
            Grade::AmbiguousDetected
        );
    }

    #[test]
    fn a_correctly_detected_broken_reference_grades_broken_detected() {
        let r = run(
            "12_add_remove_hole",
            RunOutcome::Resolved { count: 1 },
            RunOutcome::Broken,
        );
        assert_eq!(
            grade(&r, ExpectedOutcome::ExplicitBrokenReference),
            Grade::BrokenDetected
        );
    }

    #[test]
    fn a_real_kernel_failure_grades_kernel_failure() {
        let r = run(
            "06_fillet_viability",
            RunOutcome::Resolved { count: 1 },
            RunOutcome::KernelFailure {
                diagnostic_code: "GEOM-E005".to_string(),
            },
        );
        assert_eq!(
            grade(&r, ExpectedOutcome::KernelFailure),
            Grade::KernelFailure
        );
    }

    /// The catastrophic direction: ground truth says the reference should
    /// have been ambiguous, but the resolver silently picked one
    /// candidate anyway.
    #[test]
    fn resolving_where_ambiguity_was_expected_is_silent_wrong() {
        let r = run(
            "synthetic_silent_wrong",
            RunOutcome::Resolved { count: 1 },
            RunOutcome::Resolved { count: 1 },
        );
        assert_eq!(
            grade(&r, ExpectedOutcome::ExplicitAmbiguity),
            Grade::SilentWrong
        );
    }

    /// The catastrophic direction again, this time against an expected
    /// broken reference.
    #[test]
    fn resolving_where_a_broken_reference_was_expected_is_silent_wrong() {
        let r = run(
            "synthetic_silent_wrong_broken",
            RunOutcome::Resolved { count: 1 },
            RunOutcome::Resolved { count: 1 },
        );
        assert_eq!(
            grade(&r, ExpectedOutcome::ExplicitBrokenReference),
            Grade::SilentWrong
        );
    }

    /// The safe (never silent-wrong) direction: the resolver was more
    /// conservative than ground truth expected.
    #[test]
    fn over_reporting_ambiguity_is_a_mismatch_not_silent_wrong() {
        let r = run(
            "over_cautious",
            RunOutcome::Resolved { count: 1 },
            RunOutcome::Ambiguous { count: 2 },
        );
        assert_eq!(
            grade(&r, ExpectedOutcome::CorrectResolvedReference),
            Grade::Mismatch
        );
    }

    #[test]
    fn an_unrelated_failure_is_never_counted_as_resolver_success_or_failure() {
        let r = run(
            "infra_broke",
            RunOutcome::UnrelatedFailure {
                message: "kernel context creation failed".to_string(),
            },
            RunOutcome::UnrelatedFailure {
                message: "kernel context creation failed".to_string(),
            },
        );
        assert_eq!(
            grade(&r, ExpectedOutcome::CorrectResolvedReference),
            Grade::UnrelatedFailure
        );
    }

    #[test]
    fn aggregate_tallies_every_grade_and_records_silent_wrong_ids() {
        let cases = vec![
            BenchmarkCase {
                run: run(
                    "correct",
                    RunOutcome::Resolved { count: 1 },
                    RunOutcome::Resolved { count: 1 },
                ),
                expected: ExpectedOutcome::CorrectResolvedReference,
                durability: DurabilityLevel::QueryGeometric,
            },
            BenchmarkCase {
                run: run(
                    "broken",
                    RunOutcome::Resolved { count: 1 },
                    RunOutcome::Broken,
                ),
                expected: ExpectedOutcome::ExplicitBrokenReference,
                durability: DurabilityLevel::QueryGeometric,
            },
            BenchmarkCase {
                run: run(
                    "silent_wrong",
                    RunOutcome::Resolved { count: 1 },
                    RunOutcome::Resolved { count: 1 },
                ),
                expected: ExpectedOutcome::ExplicitAmbiguity,
                durability: DurabilityLevel::Lineage,
            },
        ];
        let metrics = aggregate(&cases);
        assert_eq!(metrics.correct, 1);
        assert_eq!(metrics.broken_detected, 1);
        assert_eq!(metrics.silent_wrong, 1);
        assert_eq!(metrics.silent_wrong_ids, vec!["silent_wrong".to_string()]);
        assert_eq!(metrics.total(), 3);
        assert_eq!(
            metrics.by_durability.get(&DurabilityLevel::QueryGeometric),
            Some(&2)
        );
        assert_eq!(
            metrics.by_durability.get(&DurabilityLevel::Lineage),
            Some(&1)
        );
    }

    /// Running `aggregate` twice over the identical input produces a
    /// bit-for-bit identical `BenchmarkMetrics` — this module's own
    /// determinism/reproducibility contract, matching
    /// `crate::perturbation`'s own established discipline.
    #[test]
    fn aggregate_is_deterministic() {
        let cases = vec![BenchmarkCase {
            run: run(
                "case",
                RunOutcome::Resolved { count: 1 },
                RunOutcome::Ambiguous { count: 2 },
            ),
            expected: ExpectedOutcome::ExplicitAmbiguity,
            durability: DurabilityLevel::QueryStrong,
        }];
        assert_eq!(aggregate(&cases), aggregate(&cases));
    }
}
