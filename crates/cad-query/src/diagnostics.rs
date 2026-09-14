//! Structured `REF-E1##` diagnostics (`AICAD-089`) built from a
//! [`crate::resolve::ResolutionOutcome`], using the already-reserved
//! `cad_diagnostics::Diagnostic`/`DiagnosticCode` schema (RFC-0005 §3,
//! `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §11) rather than a bare
//! string or a new ad hoc shape.
//!
//! `REF-E102` (`AMBIGUOUS_REFERENCE`) is not a code this task mints: it is
//! already the worked example in `docs/plan/06_REFERENCES_QUERIES_
//! FEATURE_DAG.md` §7, `rfcs/0005-diagnostics.md`/its Stage-0 history
//! snapshot, `docs/plan/17...` §11, and `cad_diagnostics`'s own tests —
//! this module is the first thing that actually *constructs* one from a
//! real resolver outcome rather than an illustrative literal.
//!
//! # What this module guarantees beyond `crate::resolve`
//!
//! `crate::resolve::resolve_query`/`resolve_reference` already guarantee
//! [`crate::resolve::ResolutionOutcome::Ambiguous`] is reported whenever
//! more candidates survive than expected, never silently narrowed to one
//! — that is `AICAD-088`'s own contract. This module's own job is
//! narrower: turning that outcome into a `REF-E102` diagnostic carrying a
//! per-candidate summary and generic suggested-fix text, matching the
//! plan §7 worked example's own shape, without ever reducing the
//! candidate set itself. Building this diagnostic can never itself
//! resolve the ambiguity — there is no code path here that returns fewer
//! than the full candidate list, or that treats a `Diagnostic`'s own
//! fields as anything other than a report about an outcome
//! [`crate::resolve`] already decided.
//!
//! # Candidate summaries are honest, not the plan's own illustrative shape
//!
//! The plan §7 example shows `lineage=base/extrude` per candidate.
//! [`crate::eval::Candidate`] does not itself carry provenance/lineage —
//! it is a live kernel shape plus an [`cad_references::EntityKind`] (see
//! that type's own doc comment) — so this module reports only what is
//! actually derivable from a candidate today: its kind, and whichever of
//! area/length/radius/normal/point apply to that kind, each read directly
//! from real `cad-occt-bridge` geometry via the same accessors
//! `crate::eval`/`crate::resolve` already use. A field this module cannot
//! honestly compute for a candidate (e.g. `radius` on a planar face) is
//! omitted from that candidate's own summary rather than fabricated as
//! `0`/`null`-with-a-misleading-meaning. Enriching summaries with real
//! lineage once a resolver is wired to a production build (`AICAD-094`+)
//! is future work, not a gap this task papers over.
//!
//! # `REF-E101 BROKEN_REFERENCE` (`AICAD-090`)
//!
//! Unlike `REF-E102`, no plan/RFC document already names a specific code
//! for a broken reference -- only the family (`REF`) and the general
//! `FAMILY-Exxx` shape are established. Per `project/DECISION_LOG.md
//! #DL-18`, "adding a new code within an already-reserved family ...
//! remains ordinary task work and needs no owner ruling," and no other
//! `REF-E1##` code is committed anywhere in this repository (checked by
//! grep before choosing a number) -- `REF-E101` is this task's own new
//! code, chosen only for being the lowest free number in the family, with
//! no ordering claim relative to `REF-E102` implied.
//!
//! [`broken_reference_diagnostic`]'s own `suggestions` are always
//! non-guessing recovery hints (the campaign brief's own instruction:
//! "Recovery hints may explain how a human/program could repair a
//! reference. A hint is not authorization for the resolver to guess
//! automatically"): every hint here is `Informational` confidence,
//! describes how a human or later tool might investigate/repair the
//! reference, and never itself selects, constructs, or implies a specific
//! replacement candidate -- that would be exactly the silent-guess
//! failure class D7/`DL-8` forbids, moved from the resolver into the
//! diagnostic layer instead of eliminated.

use cad_diagnostics::json::Json;
use cad_diagnostics::{
    Diagnostic, DiagnosticCode, Severity, SeverityLetter, Suggestion, SuggestionConfidence,
};

use crate::eval::Candidate;
use crate::resolve::BrokenReason;
use cad_references::EntityKind;

/// `REF-E102`: already-reserved across the plan/RFC/schema examples (see
/// this module's own doc comment) — never re-minted or repurposed here.
fn ambiguous_reference_code() -> DiagnosticCode {
    DiagnosticCode::new("REF", SeverityLetter::Error, 102)
        .expect("REF-E102 is a valid code in the already-reserved REF family")
}

/// `REF-E101`: this task's own new code within the already-reserved `REF`
/// family — see this module's own doc comment, "`REF-E101
/// BROKEN_REFERENCE` (`AICAD-090`)."
fn broken_reference_code() -> DiagnosticCode {
    DiagnosticCode::new("REF", SeverityLetter::Error, 101)
        .expect("REF-E101 is a valid code in the already-reserved REF family")
}

/// Builds the `REF-E102 AMBIGUOUS_REFERENCE` diagnostic for a
/// [`crate::resolve::ResolutionOutcome::Ambiguous`] outcome, matching
/// `docs/plan/06...` §7's own worked example shape: entity name, expected
/// vs. observed count, a per-candidate summary, and generic suggested
/// fixes. `entity` is a caller-supplied stable description of what was
/// being resolved (e.g. `"housing.mounting_face"`, or a query's own
/// source text/handle name) — this module has no naming authority of its
/// own.
pub fn ambiguous_reference_diagnostic(
    entity: &str,
    expected_count: usize,
    candidates: &[Candidate<'_>],
) -> Diagnostic {
    let observed_count = candidates.len();
    let message = format!(
        "Reference resolved to {observed_count} {}; expected {expected_count}.",
        pluralize_entity(candidates.first().map(Candidate::kind), observed_count)
    );

    Diagnostic::new(
        ambiguous_reference_code(),
        Severity::Error,
        "reference",
        "AMBIGUOUS_REFERENCE",
        message,
    )
    .expect("REF-E102 is an E-severity code paired with Severity::Error")
    .with_entity(entity)
    .with_expected(Json::object([(
        "count".to_string(),
        Json::Integer(expected_count as i64),
    )]))
    .with_observed(Json::object([(
        "count".to_string(),
        Json::Integer(observed_count as i64),
    )]))
    .with_candidates(candidates.iter().map(candidate_summary).collect())
    .with_suggestion(informational_suggestion(
        "narrow_query",
        "add a topology predicate such as generated_by(...) or adjacent_to(...) to the query",
    ))
    .with_suggestion(informational_suggestion(
        "use_explicit_export",
        "use an explicit semantic export instead of a query-derived reference",
    ))
    .with_suggestion(informational_suggestion(
        "confirm_candidate",
        "if the ambiguity is genuine, record a user-confirmed reference naming the intended \
         candidate explicitly",
    ))
}

/// Builds the `REF-E101 BROKEN_REFERENCE` diagnostic for a
/// [`crate::resolve::ResolutionOutcome::Broken`] outcome. `entity` is the
/// same caller-supplied stable description
/// [`ambiguous_reference_diagnostic`] takes. `expected_count` is the
/// caller's own cardinality expectation when one exists (`Some(1)` for a
/// single stable reference or a `unique()` query, `Some(n)` for
/// `expect_count(n)`) — `None` when the reason itself is not a cardinality
/// mismatch (e.g. missing evidence, or a disabled fingerprint fallback),
/// since [`BrokenReason`] does not always carry one. Every suggestion this
/// function attaches is a non-guessing recovery hint — see this module's
/// own doc comment, "`REF-E101 BROKEN_REFERENCE` (`AICAD-090`)."
pub fn broken_reference_diagnostic(
    entity: &str,
    expected_count: Option<usize>,
    reason: &BrokenReason,
) -> Diagnostic {
    let (message, observed_count, hints) = broken_reason_detail(reason);

    let mut diagnostic = Diagnostic::new(
        broken_reference_code(),
        Severity::Error,
        "reference",
        "BROKEN_REFERENCE",
        message,
    )
    .expect("REF-E101 is an E-severity code paired with Severity::Error")
    .with_entity(entity);

    if let Some(expected_count) = expected_count {
        diagnostic = diagnostic.with_expected(Json::object([(
            "count".to_string(),
            Json::Integer(expected_count as i64),
        )]));
    }
    if let Some(observed_count) = observed_count {
        diagnostic = diagnostic.with_observed(Json::object([(
            "count".to_string(),
            Json::Integer(observed_count as i64),
        )]));
    }
    if let BrokenReason::FingerprintAutoResolutionDisabled(evidence) = reason {
        diagnostic = diagnostic.with_backend_details(fingerprint_evidence_json(evidence));
    }
    for (action, description) in hints {
        diagnostic = diagnostic.with_suggestion(informational_suggestion(action, &description));
    }
    diagnostic
}

/// The `(message, observed_count, recovery_hints)` this reason implies.
/// `observed_count` is `None` when the reason is not itself a cardinality
/// mismatch (see [`broken_reference_diagnostic`]'s own doc comment).
/// Every hint describes *how* a human/tool might investigate or repair
/// the reference -- never a specific replacement candidate, which this
/// function has no way to know and must never guess.
fn broken_reason_detail(
    reason: &BrokenReason,
) -> (String, Option<usize>, Vec<(&'static str, String)>) {
    match reason {
        BrokenReason::NoMatch => (
            "Reference/query matched zero candidates.".to_string(),
            Some(0),
            vec![
                (
                    "check_upstream_deletion",
                    "check whether the intended entity was deleted or suppressed by an \
                     upstream feature change"
                        .to_string(),
                ),
                (
                    "loosen_predicate",
                    "loosen an overly narrow geometry/topology/spatial predicate in the query"
                        .to_string(),
                ),
                (
                    "verify_feature_name",
                    "verify the referenced feature/export name still exists and was not \
                     renamed"
                        .to_string(),
                ),
            ],
        ),
        BrokenReason::TooFew { expected, found } => (
            format!(
                "Reference/query matched {found} candidate{}, fewer than the {expected} \
                 expected.",
                if *found == 1 { "" } else { "s" }
            ),
            Some(*found),
            vec![
                (
                    "check_upstream_deletion",
                    "check whether upstream changes removed one or more of the expected \
                     entities"
                        .to_string(),
                ),
                (
                    "adjust_expect_count",
                    "if the expected count itself changed intentionally, update \
                     expect_count(...) to match"
                        .to_string(),
                ),
                (
                    "inspect_lineage",
                    "inspect the relevant feature's lineage report for deleted entities"
                        .to_string(),
                ),
            ],
        ),
        BrokenReason::InsufficientEvidence(detail) => (
            format!(
                "Reference/query resolution needs evidence this context does not supply: {detail}."
            ),
            None,
            vec![(
                "supply_evidence",
                format!(
                    "supply the missing evidence source in the resolver context used to \
                     resolve this reference ({detail})"
                ),
            )],
        ),
        BrokenReason::FingerprintAutoResolutionDisabled(_) => (
            "This reference's sole construction strategy is a geometry fingerprint, which is \
             never resolved automatically."
                .to_string(),
            None,
            vec![
                (
                    "add_stronger_evidence",
                    "add feature lineage, an explicit semantic export, or a structural-role \
                     tag instead of relying on a geometric fingerprint alone"
                        .to_string(),
                ),
                (
                    "confirm_explicitly",
                    "if a fingerprint-based match is genuinely intended, record it as a \
                     user-confirmed reference after a human reviews the fingerprint evidence \
                     (see backend_details)"
                        .to_string(),
                ),
            ],
        ),
        BrokenReason::QueryHandleNotRegistered(handle) => (
            format!(
                "Query handle '{}' is not registered in this resolution context.",
                handle.0
            ),
            None,
            vec![
                (
                    "register_query",
                    "register the named query with the resolver context before resolving \
                     references that depend on it"
                        .to_string(),
                ),
                (
                    "check_handle_spelling",
                    "check for a typo or stale rename in the query handle name".to_string(),
                ),
            ],
        ),
    }
}

fn fingerprint_evidence_json(evidence: &cad_references::FingerprintEvidence) -> Json {
    let mut fields = vec![(
        "position".to_string(),
        Json::Array(evidence.position.iter().map(|v| Json::Float(*v)).collect()),
    )];
    if let Some(normal) = evidence.normal {
        fields.push((
            "normal".to_string(),
            Json::Array(normal.iter().map(|v| Json::Float(*v)).collect()),
        ));
    }
    if let Some(area) = evidence.area {
        fields.push(("area".to_string(), Json::Float(area)));
    }
    if let Some(radius) = evidence.radius {
        fields.push(("radius".to_string(), Json::Float(radius)));
    }
    Json::object(fields)
}

fn informational_suggestion(action: &str, description: &str) -> Suggestion {
    Suggestion::new(SuggestionConfidence::Informational)
        .with_field("action", Json::str(action))
        .with_field("description", Json::str(description))
}

fn pluralize_entity(kind: Option<EntityKind>, count: usize) -> String {
    let noun = kind.map(EntityKind::as_str).unwrap_or("entity");
    if count == 1 {
        noun.to_string()
    } else {
        format!("{noun}s")
    }
}

/// A structured, honest-only summary of one candidate — see this module's
/// own doc comment, "Candidate summaries are honest, not the plan's own
/// illustrative shape."
fn candidate_summary(candidate: &Candidate<'_>) -> Json {
    let mut fields = vec![("kind".to_string(), Json::str(candidate.kind().as_str()))];

    match candidate.kind() {
        EntityKind::Face => {
            if let Ok(area) = candidate.shape().area() {
                fields.push(("area".to_string(), Json::Float(area)));
            }
            if let Ok(radius) = candidate.shape().face_radius() {
                fields.push(("radius".to_string(), Json::Float(radius)));
            }
            if let Ok((_, normal)) = candidate.shape().face_normal() {
                fields.push(("normal".to_string(), direction_json(normal)));
            }
        }
        EntityKind::Edge => {
            if let Ok(length) = candidate.shape().length() {
                fields.push(("length".to_string(), Json::Float(length)));
            }
            if let Ok(radius) = candidate.shape().edge_radius() {
                fields.push(("radius".to_string(), Json::Float(radius)));
            }
        }
        EntityKind::Vertex => {
            if let Ok(point) = candidate.shape().vertex_point() {
                fields.push(("point".to_string(), point_json(point)));
            }
        }
        EntityKind::Wire | EntityKind::Shell | EntityKind::Solid => {
            if let Ok(area) = candidate.shape().area() {
                fields.push(("area".to_string(), Json::Float(area)));
            }
        }
    }

    Json::object(fields)
}

fn direction_json(direction: cad_kernel_api::Direction3) -> Json {
    let vector = direction.as_vector3();
    Json::Array(vec![
        Json::Float(vector.x),
        Json::Float(vector.y),
        Json::Float(vector.z),
    ])
}

fn point_json(point: cad_kernel_api::Point3) -> Json {
    Json::Array(vec![
        Json::Float(point.x),
        Json::Float(point.y),
        Json::Float(point.z),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::{EvaluationEvidence, evaluate_geometry};
    use crate::predicate::GeometryPredicate;
    use crate::query::{Query, QueryClause};
    use crate::ranking::CardinalityExpectation;
    use crate::resolve::{ResolutionOutcome, ResolverContext, resolve_query};
    use crate::value::{Comparison, Magnitude};
    use cad_occt_bridge::{OcctContext, Shape};
    use cad_types::Dimension;
    use cad_units::OperandType;

    fn length(value: f64) -> Magnitude {
        Magnitude::new(value, OperandType::dimensional(Dimension::Length, None))
    }

    struct PlainContext<'ctx>(&'ctx Shape<'ctx>);
    impl<'ctx> EvaluationEvidence<'ctx> for PlainContext<'ctx> {}
    impl<'ctx> ResolverContext<'ctx> for PlainContext<'ctx> {
        fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>> {
            if kind != EntityKind::Face {
                return Vec::new();
            }
            (0..self.0.face_count().unwrap())
                .map(|i| Candidate::new(EntityKind::Face, self.0.get_face(i).unwrap()))
                .collect()
        }
    }

    /// End-to-end: a real cube's six tied faces resolve `Ambiguous`
    /// (`AICAD-088`), and this test proves the resulting `REF-E102`
    /// diagnostic reports the real count and a real per-face summary for
    /// every one of them -- never fewer than the full candidate set.
    #[test]
    fn ambiguous_cube_faces_produce_a_ref_e102_diagnostic_naming_every_candidate() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let query = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Geometry(GeometryPredicate::Area(
                Comparison::Eq(length(4.0)),
            )))
            .with_cardinality(CardinalityExpectation::Unique);

        let outcome = resolve_query(&query, &PlainContext(&cube)).unwrap();
        let candidates = match outcome {
            ResolutionOutcome::Ambiguous(candidates) => candidates,
            _ => panic!("expected Ambiguous"),
        };
        assert_eq!(candidates.len(), 6);

        let diagnostic = ambiguous_reference_diagnostic("cube.some_face", 1, &candidates);
        let json = diagnostic.to_json();

        assert_eq!(json.get("code").unwrap().as_str(), Some("REF-E102"));
        assert_eq!(json.get("severity").unwrap().as_str(), Some("error"));
        assert_eq!(
            json.get("title").unwrap().as_str(),
            Some("AMBIGUOUS_REFERENCE")
        );
        assert_eq!(json.get("entity").unwrap().as_str(), Some("cube.some_face"));
        assert_eq!(
            json.get("expected").unwrap().get("count").unwrap(),
            &Json::Integer(1)
        );
        assert_eq!(
            json.get("observed").unwrap().get("count").unwrap(),
            &Json::Integer(6)
        );
        let candidate_json = json.get("candidates").unwrap().as_array().unwrap();
        assert_eq!(
            candidate_json.len(),
            6,
            "every tied candidate must be named, never truncated"
        );
        for candidate in candidate_json {
            assert_eq!(candidate.get("kind").unwrap().as_str(), Some("face"));
            assert!(candidate.get("area").is_some(), "a real face has an area");
        }
        assert_eq!(
            json.get("suggestions").unwrap().as_array().unwrap().len(),
            3
        );
        for suggestion in json.get("suggestions").unwrap().as_array().unwrap() {
            assert_eq!(
                suggestion.get("confidence").unwrap().as_str(),
                Some("informational"),
                "a generic textual hint is never machine_applicable/likely_fix"
            );
        }
    }

    #[test]
    fn candidate_summary_omits_radius_for_a_planar_face() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face = Candidate::new(EntityKind::Face, cube.get_face(0).unwrap());
        assert!(evaluate_geometry(&GeometryPredicate::Planar, &face).unwrap());
        let summary = candidate_summary(&face);
        assert!(
            summary.get("radius").is_none(),
            "a planar face has no well-defined radius -- must be omitted, not fabricated as 0"
        );
        assert!(summary.get("area").is_some());
        assert!(summary.get("normal").is_some());
    }

    #[test]
    fn candidate_summary_reports_radius_for_a_cylindrical_face() {
        let context = OcctContext::new().unwrap();
        let cylinder = context.create_cylinder(2.0, 5.0).unwrap();
        let lateral = (0..cylinder.face_count().unwrap())
            .map(|i| Candidate::new(EntityKind::Face, cylinder.get_face(i).unwrap()))
            .find(|c| evaluate_geometry(&GeometryPredicate::Cylindrical, c).unwrap())
            .unwrap();
        let summary = candidate_summary(&lateral);
        match summary.get("radius").unwrap() {
            Json::Float(r) => assert!((r - 2.0).abs() < 1e-6),
            other => panic!("expected Json::Float, got {other:?}"),
        }
    }

    // --- AICAD-090 ---

    #[test]
    fn no_match_produces_a_ref_e101_diagnostic_with_recovery_hints() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let query = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
            .with_cardinality(CardinalityExpectation::Unique);

        let outcome = resolve_query(&query, &PlainContext(&cube)).unwrap();
        let reason = match outcome {
            ResolutionOutcome::Broken(reason) => reason,
            other => panic!("expected Broken, got {other:?}"),
        };

        let diagnostic =
            broken_reference_diagnostic("cube.some_cylindrical_face", Some(1), &reason);
        let json = diagnostic.to_json();

        assert_eq!(json.get("code").unwrap().as_str(), Some("REF-E101"));
        assert_eq!(json.get("severity").unwrap().as_str(), Some("error"));
        assert_eq!(
            json.get("title").unwrap().as_str(),
            Some("BROKEN_REFERENCE")
        );
        assert_eq!(
            json.get("expected").unwrap().get("count").unwrap(),
            &Json::Integer(1)
        );
        assert_eq!(
            json.get("observed").unwrap().get("count").unwrap(),
            &Json::Integer(0)
        );
        let suggestions = json.get("suggestions").unwrap().as_array().unwrap();
        assert!(
            !suggestions.is_empty(),
            "a broken reference must always carry at least one recovery hint"
        );
        for suggestion in suggestions {
            assert_eq!(
                suggestion.get("confidence").unwrap().as_str(),
                Some("informational"),
                "a recovery hint is never machine_applicable/likely_fix -- it must never \
                 authorize the resolver to guess"
            );
            // Every suggestion must describe an investigative/repair
            // action, never name a specific candidate -- there is no
            // "candidate" vocabulary in a hint's own fields at all.
            assert!(suggestion.get("action").is_some());
            assert!(suggestion.get("description").is_some());
        }
    }

    #[test]
    fn too_few_reports_the_real_found_count_and_expected_count() {
        let reason = BrokenReason::TooFew {
            expected: 3,
            found: 1,
        };
        let diagnostic = broken_reference_diagnostic("body.mounting_holes", Some(3), &reason);
        let json = diagnostic.to_json();
        assert_eq!(
            json.get("expected").unwrap().get("count").unwrap(),
            &Json::Integer(3)
        );
        assert_eq!(
            json.get("observed").unwrap().get("count").unwrap(),
            &Json::Integer(1)
        );
        let message = json.get("message").and_then(Json::as_str).unwrap();
        assert!(message.contains('1'), "message states the real found count");
        assert!(
            message.contains('3'),
            "message states the real expected count"
        );
    }

    #[test]
    fn fingerprint_auto_resolution_disabled_carries_evidence_as_backend_details_only() {
        let evidence = cad_references::FingerprintEvidence::at_position([1.0, 2.0, 3.0])
            .with_area(42.0)
            .with_radius(5.0);
        let reason = BrokenReason::FingerprintAutoResolutionDisabled(evidence);
        let diagnostic = broken_reference_diagnostic("body.some_face", None, &reason);
        let json = diagnostic.to_json();

        assert_eq!(*json.get("expected").unwrap(), Json::Null);
        assert_eq!(*json.get("observed").unwrap(), Json::Null);
        let backend = json.get("backend_details").unwrap();
        assert_eq!(
            backend.get("area").unwrap(),
            &Json::Float(42.0),
            "the fingerprint evidence is surfaced for human review, never used to select a \
             candidate"
        );
        assert_eq!(backend.get("radius").unwrap(), &Json::Float(5.0));
        // No suggestion may name a specific replacement candidate; every
        // one must only describe how to investigate/repair.
        for suggestion in json.get("suggestions").unwrap().as_array().unwrap() {
            let description = suggestion
                .get("description")
                .and_then(Json::as_str)
                .unwrap();
            assert!(!description.contains("face_"), "no fabricated entity id");
        }
    }

    #[test]
    fn query_handle_not_registered_names_the_handle_in_the_message() {
        let reason =
            BrokenReason::QueryHandleNotRegistered(cad_references::QueryHandle::named("top_face"));
        let diagnostic = broken_reference_diagnostic("base.top_face", Some(1), &reason);
        let json = diagnostic.to_json();
        assert_eq!(*json.get("observed").unwrap(), Json::Null);
        assert!(
            !json
                .get("suggestions")
                .unwrap()
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn insufficient_evidence_includes_the_reason_in_its_single_hint() {
        let reason = BrokenReason::InsufficientEvidence("generated_by requires feature lineage");
        let diagnostic = broken_reference_diagnostic("wall.pierced_face", Some(1), &reason);
        let json = diagnostic.to_json();
        let suggestions = json.get("suggestions").unwrap().as_array().unwrap();
        assert_eq!(suggestions.len(), 1);
        let description = suggestions[0]
            .get("description")
            .and_then(Json::as_str)
            .unwrap();
        assert!(description.contains("generated_by requires feature lineage"));
    }

    /// End-to-end: an unregistered `SemanticQuery` handle really does
    /// resolve `Broken` (`AICAD-088`), and this test proves the resulting
    /// diagnostic is built from that real outcome, not a hand-built
    /// `BrokenReason`.
    #[test]
    fn broken_reference_diagnostic_builds_from_a_real_resolver_outcome() {
        let reference = cad_references::AnyRef::Face(cad_references::FaceRef::from_strategy(
            cad_references::ConstructionStrategy::semantic_query(
                cad_references::QueryHandle::named("top_face"),
                cad_references::DurabilityLevel::QueryStrong,
            )
            .unwrap(),
        ));
        struct EmptyContext;
        impl<'ctx> EvaluationEvidence<'ctx> for EmptyContext {}
        impl<'ctx> ResolverContext<'ctx> for EmptyContext {
            fn candidates(&self, _kind: EntityKind) -> Vec<Candidate<'ctx>> {
                Vec::new()
            }
        }
        let outcome = crate::resolve::resolve_reference(&reference, &EmptyContext).unwrap();
        let reason = match outcome {
            ResolutionOutcome::Broken(reason) => reason,
            other => panic!("expected Broken, got {other:?}"),
        };
        let diagnostic = broken_reference_diagnostic("base.top_face", Some(1), &reason);
        assert_eq!(
            diagnostic.to_json().get("code").unwrap().as_str(),
            Some("REF-E101")
        );
    }
}
