//! `AICAD-097`: a deterministic perturbation runner — the concrete
//! mechanism `docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md` §5's own
//! "Topological naming benchmark" methodology names ("bind downstream
//! features to semantic refs; perturb upstream dimensions/features;
//! rebuild; check intended entity selection; check ambiguity is reported
//! rather than silently misresolved") and `project/TASKS.yaml`'s own
//! acceptance criterion for this task ("execute perturbations
//! deterministically enough that failures can be reproduced and
//! inspected").
//!
//! # What this generalizes
//!
//! `AICAD-096` (`crates/cad-cli/tests/stage4_resolver_execution.rs`)
//! proved real resolver execution against six corpus cases with six
//! hand-written test functions, each independently building a baseline
//! and a perturbed variant and resolving the same query against both.
//! [`run_case`] is that same shape, generalized into one reusable,
//! pure function: given a case's own baseline/perturbed *source* (never
//! read from disk itself — mirrors `crate::build::build_source`'s own
//! "in-memory string, thin file-reading wrapper" split, `AICAD-095`'s
//! own report already cites the identical precedent for `refs_check`)
//! and one [`cad_query::Query`], it builds both variants and resolves the
//! same query against each, returning a [`PerturbationRun`] that records
//! the real outcome for each side.
//!
//! # Determinism
//!
//! [`run_case`] performs no randomness, no wall-clock/thread-count-
//! dependent behavior, and no I/O beyond what `ParametricBuildSession::
//! new`'s own already-deterministic build pipeline performs (itself
//! already relied upon for determinism by every corpus buildability test
//! this crate has). [`RunOutcome`] carries only plain, `PartialEq`-able
//! data (a candidate count, a broken/kernel-failure/unrelated-failure
//! tag, and — for the two failure tags — the exact diagnostic code or
//! message), so two runs of the same case are directly comparable for
//! bit-for-bit-equal reproduction, proven by this module's own
//! `running_the_same_case_twice_produces_identical_results` test.
//! [`PerturbationRun`] additionally echoes back the case's own
//! `id`/paths, so a failure is self-describing without cross-referencing
//! a separate case table.
//!
//! # What this does not decide
//!
//! [`RunOutcome`] reports the *real* [`cad_query::ResolutionOutcome`]
//! variant (or a build-time failure), never a "correct"/"silent-wrong"
//! verdict — that comparison against a case's own expected ground truth
//! is [`crate::metrics`]'s job (`AICAD-098`), kept as a separate,
//! later pass so this module stays a pure, ground-truth-agnostic
//! execution primitive, reusable by a future case whose expected
//! classification is not yet known/decided.

use std::path::Path;

use cad_occt_bridge::OcctContext;
use cad_query::{Query, ResolutionOutcome};

use crate::parametric_build::ParametricBuildSession;

/// One perturbation case: a stable `id`, the baseline/perturbed `.aicad`
/// *source text* (never a path — see this module's own doc comment for
/// why), and the single [`Query`] resolved against both.
#[derive(Debug, Clone)]
pub struct PerturbationCase {
    pub id: String,
    pub baseline_source: String,
    pub perturbed_source: String,
    pub query: Query,
}

impl PerturbationCase {
    pub fn new(
        id: impl Into<String>,
        baseline_source: impl Into<String>,
        perturbed_source: impl Into<String>,
        query: Query,
    ) -> PerturbationCase {
        PerturbationCase {
            id: id.into(),
            baseline_source: baseline_source.into(),
            perturbed_source: perturbed_source.into(),
            query,
        }
    }

    /// Reads `baseline_path`/`perturbed_path` (each resolved relative to
    /// `repo_root`) as this case's own source text — the thin,
    /// file-reading counterpart to [`PerturbationCase::new`], mirroring
    /// `crate::build::run_build`'s own identical split over
    /// `build_source`.
    pub fn from_paths(
        id: impl Into<String>,
        repo_root: &Path,
        baseline_path: &str,
        perturbed_path: &str,
        query: Query,
    ) -> std::io::Result<PerturbationCase> {
        let baseline_source = std::fs::read_to_string(repo_root.join(baseline_path))?;
        let perturbed_source = std::fs::read_to_string(repo_root.join(perturbed_path))?;
        Ok(PerturbationCase::new(
            id,
            baseline_source,
            perturbed_source,
            query,
        ))
    }
}

/// The real, raw outcome of resolving one [`PerturbationCase`]'s own
/// [`Query`] against one variant (baseline or perturbed) — a ground-
/// truth-agnostic recording of what actually happened, never a verdict.
/// Distinguishes a kernel-dispatch failure and an unrelated
/// build/resolution-infrastructure failure from an ordinary resolution
/// outcome, matching `tests/semantic_refs/README.md`'s own six-class
/// taxonomy (`RESOLVED_CORRECT`/`AMBIGUOUS`/`BROKEN` fold into
/// [`RunOutcome::Resolved`]/[`RunOutcome::Ambiguous`]/
/// [`RunOutcome::Broken`] here — "correct" vs. "silent wrong" is
/// [`crate::metrics`]'s own later distinction, not this type's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunOutcome {
    /// The query resolved to exactly `count` candidate(s) — `count` is
    /// always `1` for a `unique()` query that reached this variant
    /// (`cad_query::resolve`'s own fail-closed contract: a `unique()`
    /// query with more than one surviving candidate is
    /// [`RunOutcome::Ambiguous`], never a larger `Resolved`).
    Resolved { count: usize },
    /// More than the expected number of candidates survived —
    /// `count` candidates, never silently narrowed to one.
    Ambiguous { count: usize },
    /// Zero candidates survived, a construction strategy had no
    /// evidence source, or (`AICAD-092`/D7) the recipe was
    /// `GeometricFingerprint`-only.
    Broken,
    /// This variant's own `ParametricBuildSession::new` failed with a
    /// real `GEOM-*` kernel-dispatch diagnostic — a geometric-
    /// infeasibility outcome, never reached the resolver at all (matches
    /// `06_fillet_viability`'s own established `KERNEL_FAILURE`
    /// precedent, `AICAD-096`).
    KernelFailure { diagnostic_code: String },
    /// This variant's own build failed for a reason unrelated to
    /// resolution (a parse/type/non-`GEOM` diagnostic), or `resolve`
    /// itself returned a [`cad_query::ResolveError`] (a predicate this
    /// Stage-4 task does not yet define evaluation semantics for, or
    /// malformed predicate input) — never conflated with
    /// [`RunOutcome::Broken`], which is a real semantic outcome *about
    /// the reference*, not an execution failure.
    UnrelatedFailure { message: String },
}

/// One [`PerturbationCase`]'s own real, reproducible result: the case's
/// own `id` echoed back, plus the [`RunOutcome`] for each variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerturbationRun {
    pub id: String,
    pub baseline: RunOutcome,
    pub perturbed: RunOutcome,
}

fn run_variant(label: &str, source: &str, query: &Query) -> RunOutcome {
    let ctx = match OcctContext::new() {
        Ok(ctx) => ctx,
        Err(err) => {
            return RunOutcome::UnrelatedFailure {
                message: format!("kernel context creation failed: {err:?}"),
            };
        }
    };
    let session = match ParametricBuildSession::new(label, source, &ctx) {
        Ok(session) => session,
        Err(diagnostics) => {
            return match diagnostics
                .iter()
                .find(|d| d.code.as_string().starts_with("GEOM-"))
            {
                Some(kernel) => RunOutcome::KernelFailure {
                    diagnostic_code: kernel.code.as_string(),
                },
                None => RunOutcome::UnrelatedFailure {
                    message: format!("{diagnostics:?}"),
                },
            };
        }
    };
    match session.resolve(query) {
        Ok(ResolutionOutcome::Resolved(candidates)) => RunOutcome::Resolved {
            count: candidates.len(),
        },
        Ok(ResolutionOutcome::Ambiguous(candidates)) => RunOutcome::Ambiguous {
            count: candidates.len(),
        },
        Ok(ResolutionOutcome::Broken(_)) => RunOutcome::Broken,
        Err(_) => RunOutcome::UnrelatedFailure {
            message: "resolve returned a ResolveError".to_string(),
        },
    }
}

/// Runs `case`: builds `case.baseline_source`/`case.perturbed_source`
/// independently (never a live single-session edit — matches the whole
/// `AICAD-079A` corpus's own established "separate build per variant,
/// re-measure, never assume" convention) and resolves `case.query`
/// against each, returning the real outcome for both — see this module's
/// own doc comment for the determinism/reproducibility contract.
pub fn run_case(case: &PerturbationCase) -> PerturbationRun {
    let baseline = run_variant(
        &format!("{}/baseline", case.id),
        &case.baseline_source,
        &case.query,
    );
    let perturbed = run_variant(
        &format!("{}/perturbed", case.id),
        &case.perturbed_source,
        &case.query,
    );
    PerturbationRun {
        id: case.id.clone(),
        baseline,
        perturbed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_query::{
        CardinalityExpectation, Comparison, GeometryPredicate, Magnitude, QueryClause,
    };
    use cad_references::EntityKind;
    use cad_types::Dimension;
    use cad_units::OperandType;

    fn length_mm(value_mm: f64) -> Magnitude {
        Magnitude::new(
            value_mm / 1000.0,
            OperandType::dimensional(Dimension::Length, None),
        )
    }

    /// The exact `12_add_remove_hole` case (`AICAD-096`), rebuilt as a
    /// [`PerturbationCase`] to prove this module reproduces that already-
    /// established resolver-execution result through the generic runner,
    /// not just through a hand-written test function.
    fn add_remove_hole_case() -> PerturbationCase {
        let baseline_source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("project/benchmarks/stage4_semantic_reference/resolver_execution/12_add_remove_hole/baseline.aicad"),
        )
        .unwrap();
        let perturbed_source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("project/benchmarks/stage4_semantic_reference/resolver_execution/12_add_remove_hole/perturbed.aicad"),
        )
        .unwrap();
        let query = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
            .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
                Comparison::Eq(length_mm(2.5)),
            )))
            .with_cardinality(CardinalityExpectation::Unique);
        PerturbationCase::new(
            "12_add_remove_hole",
            baseline_source,
            perturbed_source,
            query,
        )
    }

    #[test]
    fn reproduces_the_known_add_remove_hole_result() {
        let run = run_case(&add_remove_hole_case());
        assert_eq!(run.id, "12_add_remove_hole");
        assert_eq!(run.baseline, RunOutcome::Resolved { count: 1 });
        assert_eq!(run.perturbed, RunOutcome::Broken);
    }

    #[test]
    fn running_the_same_case_twice_produces_identical_results() {
        let case = add_remove_hole_case();
        let first = run_case(&case);
        let second = run_case(&case);
        assert_eq!(first, second);
    }

    /// A case whose baseline source fails to parse at all — never a
    /// `KernelFailure` (no `GEOM-*` diagnostic is even reachable), and
    /// never a semantic `Broken` (no resolver ever ran).
    #[test]
    fn a_parse_failure_is_an_unrelated_failure_not_a_semantic_outcome() {
        let query = Query::new(EntityKind::Face).with_cardinality(CardinalityExpectation::Unique);
        let case = PerturbationCase::new(
            "malformed",
            "this is not valid aicad source ((( ",
            "this is not valid aicad source ((( ",
            query,
        );
        let run = run_case(&case);
        assert!(matches!(run.baseline, RunOutcome::UnrelatedFailure { .. }));
        assert!(matches!(run.perturbed, RunOutcome::UnrelatedFailure { .. }));
    }

    /// The real `06_fillet_viability` kernel-dispatch failure
    /// (`AICAD-096`), reproduced through the generic runner: the
    /// perturbed variant never reaches `resolve` at all.
    #[test]
    fn a_real_kernel_failure_is_classified_kernel_failure_not_broken() {
        let baseline_source = "\
param box_x: Length = 20mm;\n\
param box_y: Length = 20mm;\n\
param box_z: Length = 10mm;\n\
param fillet_radius: Length = 8mm;\n\
\n\
part Block {\n\
    let body: Geometry = fillet(box(box_x, box_y, box_z), [1], fillet_radius);\n\
}\n\
";
        let perturbed_source = baseline_source.replace("8mm", "12mm");
        let query = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
            .with_cardinality(CardinalityExpectation::Unique);
        let case = PerturbationCase::new(
            "06_fillet_viability",
            baseline_source,
            perturbed_source,
            query,
        );
        let run = run_case(&case);
        assert_eq!(run.baseline, RunOutcome::Resolved { count: 1 });
        assert!(matches!(
            run.perturbed,
            RunOutcome::KernelFailure { ref diagnostic_code } if diagnostic_code == "GEOM-E005"
        ));
    }

    #[test]
    fn from_paths_reads_real_files_off_disk() {
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let query = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
            .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
                Comparison::Eq(length_mm(2.5)),
            )))
            .with_cardinality(CardinalityExpectation::Unique);
        let case = PerturbationCase::from_paths(
            "12_add_remove_hole",
            &repo_root,
            "project/benchmarks/stage4_semantic_reference/resolver_execution/12_add_remove_hole/baseline.aicad",
            "project/benchmarks/stage4_semantic_reference/resolver_execution/12_add_remove_hole/perturbed.aicad",
            query,
        )
        .expect("both fixture files should exist");
        let run = run_case(&case);
        assert_eq!(run.baseline, RunOutcome::Resolved { count: 1 });
        assert_eq!(run.perturbed, RunOutcome::Broken);
    }
}
