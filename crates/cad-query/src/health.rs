//! Reference health report (`AICAD-095`), per `docs/plan/
//! 06_REFERENCES_QUERIES_FEATURE_DAG.md` §12 ("New feature: reference
//! health report") and `docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md`
//! §2.3's own decision: "`cad refs check` reports explicit/lineage/query/
//! raw/broken references."
//!
//! # What this does and does not do
//!
//! [`check_reference_health`] is a **read-only aggregation** over an
//! already-supplied set of [`AnyRef`]s: for each, it calls the exact same
//! [`crate::resolve::resolve_reference_with_durability`] every other
//! Stage-4 consumer uses, and tallies the real outcome
//! ([`crate::resolve::ResolutionOutcome::Resolved`]/`Ambiguous`/`Broken`)
//! and the real static [`DurabilityLevel`] it reports — nothing here
//! re-implements resolution, widens a predicate, attempts a fingerprint
//! fallback, or otherwise "fixes" a broken/ambiguous reference to make the
//! report look better. This is exactly the campaign brief's own "SEMANTIC-
//! REFERENCE HEALTH" requirement: "expose reference health without
//! changing reference semantics... must not trigger hidden repair or
//! automatic fallback."
//!
//! # Where the reference set itself comes from
//!
//! Per `rfcs/0003-semantic-references.md` §7 and every predecessor
//! Stage-4 task's own identical precedent, `.aicad` source has no syntax
//! yet for a program to *declare* a persistent stable reference (`query {
//! ... }` blocks remain reserved/unimplemented). This module therefore
//! takes the reference set as a plain `&[AnyRef]` parameter rather than
//! trying to discover one from a real program — there is nothing in a
//! real `.aicad` program today for it to discover. `cad-cli`'s own `cad
//! refs check` subcommand (`crate::refs_check` in that crate) is
//! consequently a real, working command whose own reference set is
//! currently always empty for any real source file — see that crate's own
//! module doc comment for why that is an honest boundary, not a stub.

use std::collections::BTreeMap;

use cad_diagnostics::json::Json;
use cad_references::{AnyRef, DurabilityLevel};

use crate::resolve::{
    ResolutionOutcome, ResolveError, ResolverContext, resolve_reference_with_durability,
};

/// One [`check_reference_health`] run's own aggregate counts.
///
/// `by_durability` classifies *every* reference by its own static
/// durability (`AICAD-091`'s own established semantics: durability
/// describes how a reference was constructed, independent of whether it
/// currently resolves) — so a `Broken` reference still contributes to its
/// own durability bucket, alongside the separate `broken` outcome count.
/// [`DurabilityLevel::Raw`] can never appear here: no [`cad_references::
/// ConstructionStrategy`] produces it (see that enum's own doc comment),
/// so no real [`AnyRef`] can ever report it — a raw topology handle
/// (`cad_references::raw_handle::RawHandle`) is, by design, not an
/// `AnyRef` at all and so has no recipe for this report to classify (see
/// this module's own doc comment).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReferenceHealthReport {
    pub total: usize,
    pub resolved: usize,
    pub ambiguous: usize,
    pub broken: usize,
    pub by_durability: BTreeMap<DurabilityLevel, usize>,
}

impl ReferenceHealthReport {
    /// The count for one durability level (`0` if none of the checked
    /// references used it).
    pub fn durability_count(&self, level: DurabilityLevel) -> usize {
        self.by_durability.get(&level).copied().unwrap_or(0)
    }

    /// Renders this report in the exact shape `docs/plan/
    /// 06_REFERENCES_QUERIES_FEATURE_DAG.md` §12's own worked example
    /// uses (`"342 semantic references"`, `"329 explicit/lineage"`, ...),
    /// combining `explicit`+`lineage` into one line (the plan's own
    /// grouping) and reporting `query_strong`/`query_geometric` under the
    /// plan's own "strong queries"/"geometric fallbacks" labels.
    pub fn to_human_string(&self) -> String {
        let explicit_or_lineage = self.durability_count(DurabilityLevel::Explicit)
            + self.durability_count(DurabilityLevel::Lineage);
        format!(
            "{} semantic references\n\
             {explicit_or_lineage} explicit/lineage\n\
             {} strong queries\n\
             {} geometric fallbacks\n\
             {} ambiguous\n\
             {} broken\n",
            self.total,
            self.durability_count(DurabilityLevel::QueryStrong),
            self.durability_count(DurabilityLevel::QueryGeometric),
            self.ambiguous,
            self.broken,
        )
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("total".to_string(), Json::Integer(self.total as i64)),
            ("resolved".to_string(), Json::Integer(self.resolved as i64)),
            (
                "ambiguous".to_string(),
                Json::Integer(self.ambiguous as i64),
            ),
            ("broken".to_string(), Json::Integer(self.broken as i64)),
            (
                "by_durability".to_string(),
                Json::object(
                    [
                        DurabilityLevel::Explicit,
                        DurabilityLevel::Lineage,
                        DurabilityLevel::QueryStrong,
                        DurabilityLevel::QueryGeometric,
                        DurabilityLevel::Raw,
                    ]
                    .into_iter()
                    .map(|level| {
                        (
                            level.as_str().to_string(),
                            Json::Integer(self.durability_count(level) as i64),
                        )
                    }),
                ),
            ),
        ])
    }
}

/// Checks every reference in `references` against `ctx` and aggregates
/// the result — see this module's own doc comment for exactly what this
/// does and does not do. Propagates the first [`ResolveError`]
/// (a hard execution failure, distinct from a semantic `Broken` outcome —
/// see `crate::resolve`'s own module doc comment) rather than silently
/// skipping the reference that caused it, matching every other resolver
/// consumer's own "a `ResolveError` aborts, it is never swallowed"
/// convention.
pub fn check_reference_health<'ctx>(
    references: &[AnyRef],
    ctx: &dyn ResolverContext<'ctx>,
) -> Result<ReferenceHealthReport, ResolveError> {
    let mut report = ReferenceHealthReport {
        total: references.len(),
        ..ReferenceHealthReport::default()
    };

    for reference in references {
        let resolution = resolve_reference_with_durability(reference, ctx)?;
        *report
            .by_durability
            .entry(resolution.durability)
            .or_insert(0) += 1;
        match resolution.outcome {
            ResolutionOutcome::Resolved(_) => report.resolved += 1,
            ResolutionOutcome::Ambiguous(_) => report.ambiguous += 1,
            ResolutionOutcome::Broken(_) => report.broken += 1,
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::{Candidate, EvaluationEvidence};
    use crate::feature_lineage::ResultEntityOrigin;
    use cad_occt_bridge::{OcctContext, Shape};
    use cad_references::{ConstructionStrategy, EntityKind, FaceRef, FeatureAnchor, LineageRole};

    /// `Shape` is not `Clone` (it uniquely owns a kernel-side resource),
    /// so this context re-derives a fresh face-0 `Shape` handle from
    /// `cube` on every `candidates` call, exactly like the real
    /// `cad_cli::ParametricBuildSession`/`reference_replay::candidates_of_
    /// kind` implementation does against a live build.
    struct FixedContext<'ctx> {
        cube: Shape<'ctx>,
    }

    impl<'ctx> EvaluationEvidence<'ctx> for FixedContext<'ctx> {
        fn generated_by(
            &self,
            _candidate: &Candidate<'ctx>,
            anchor: &FeatureAnchor,
        ) -> Option<bool> {
            if *anchor == FeatureAnchor::named("generates") {
                Some(true)
            } else {
                Some(false)
            }
        }
    }

    impl<'ctx> ResolverContext<'ctx> for FixedContext<'ctx> {
        fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>> {
            if kind != EntityKind::Face {
                return Vec::new();
            }
            vec![Candidate::new(
                EntityKind::Face,
                self.cube.get_face(0).unwrap(),
            )]
        }

        /// A persistent `FeatureLineage` reference always requests an
        /// explicit scope (`AICAD-100A`) — this fixture's own single
        /// feature is `"generates"`, so answering only that name (exactly
        /// the same single candidate `candidates` above already returns)
        /// keeps this test's own real end-to-end `Resolved` outcome real.
        fn candidates_in_scope(
            &self,
            kind: EntityKind,
            scope: &FeatureAnchor,
        ) -> Option<Vec<Candidate<'ctx>>> {
            if *scope != FeatureAnchor::named("generates") {
                return None;
            }
            Some(self.candidates(kind))
        }
    }

    /// A real, end-to-end mix: one `Explicit`-durability reference that
    /// resolves (`ExplicitExport`, via `ResolverContext::resolve_export`'s
    /// default -- `Broken(InsufficientEvidence)`, proving the report
    /// counts a `Broken` reference under its own real durability, not
    /// silently as `Resolved`), one `Lineage`-durability reference that
    /// resolves against a real evaluated `generated_by` predicate, and one
    /// `GeometricFingerprint` reference (always `Broken`, per D7/`DL-8`,
    /// classified `QueryGeometric`).
    #[test]
    fn aggregates_real_resolutions_across_mixed_durability_and_outcome() {
        let ctx = OcctContext::new().unwrap();
        let cube = ctx.create_box(2.0, 2.0, 2.0).unwrap();
        let resolver = FixedContext { cube };

        let explicit_export = AnyRef::Face(FaceRef::from_strategy(
            ConstructionStrategy::ExplicitExport {
                feature: FeatureAnchor::named("base"),
                export_name: "top".to_string(),
            },
        ));
        let lineage = AnyRef::Face(FaceRef::from_strategy(
            ConstructionStrategy::FeatureLineage {
                feature: FeatureAnchor::named("generates"),
                role: LineageRole::Generated,
            },
        ));
        let fingerprint = AnyRef::Face(FaceRef::from_strategy(
            ConstructionStrategy::GeometricFingerprint(
                cad_references::FingerprintEvidence::at_position([0.0, 0.0, 0.0]),
            ),
        ));

        let report =
            check_reference_health(&[explicit_export, lineage, fingerprint], &resolver).unwrap();

        assert_eq!(report.total, 3);
        assert_eq!(report.resolved, 1, "only the lineage reference resolves");
        assert_eq!(report.ambiguous, 0);
        assert_eq!(report.broken, 2);
        assert_eq!(report.durability_count(DurabilityLevel::Explicit), 1);
        assert_eq!(report.durability_count(DurabilityLevel::Lineage), 1);
        assert_eq!(report.durability_count(DurabilityLevel::QueryGeometric), 1);
        assert_eq!(report.durability_count(DurabilityLevel::QueryStrong), 0);
        assert_eq!(report.durability_count(DurabilityLevel::Raw), 0);
        let _ = ResultEntityOrigin::New; // documents the lineage evidence's own shape
    }

    #[test]
    fn an_empty_reference_set_is_an_honest_all_zero_report() {
        struct EmptyContext;
        impl<'ctx> EvaluationEvidence<'ctx> for EmptyContext {}
        impl<'ctx> ResolverContext<'ctx> for EmptyContext {
            fn candidates(&self, _kind: EntityKind) -> Vec<Candidate<'ctx>> {
                Vec::new()
            }
        }

        let report = check_reference_health(&[], &EmptyContext).unwrap();
        assert_eq!(report, ReferenceHealthReport::default());
        assert_eq!(
            report.to_human_string(),
            "0 semantic references\n\
             0 explicit/lineage\n\
             0 strong queries\n\
             0 geometric fallbacks\n\
             0 ambiguous\n\
             0 broken\n"
        );
    }

    #[test]
    fn json_report_names_every_durability_level_including_zero_counts() {
        let report = ReferenceHealthReport::default();
        let json = report.to_json();
        let by_durability = json.get("by_durability").unwrap();
        for level in [
            "explicit",
            "lineage",
            "query_strong",
            "query_geometric",
            "raw",
        ] {
            assert_eq!(
                by_durability.get(level).unwrap(),
                &Json::Integer(0),
                "missing or non-zero {level}"
            );
        }
    }
}
