//! `AICAD-094`: end-to-end proof that semantic-reference resolution
//! ([`cad_query::resolve_query`]/[`cad_cli::ParametricBuildSession::
//! resolve_reference`]) observes the real, production incremental
//! regeneration path — `cad_cli::ParametricBuildSession::rebuild`, the
//! same orchestration `stage3_parametric_incremental_rebuild.rs` already
//! proves against `ParamModel`/`FeatureGraph` — rather than a
//! hand-constructed stand-in operation. Every assertion below is a real
//! kernel geometry property (radius/area/candidate count/`is_same`
//! identity), never a render-only check, per `AGENTS.md`'s evidence rule.

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_query::feature_lineage::ResultEntityOrigin;
use cad_query::{
    Candidate, Comparison, GeometryPredicate, Magnitude, Query, QueryClause, ResolutionOutcome,
};
use cad_references::{
    AnyRef, ConstructionStrategy, EntityKind, FaceRef, FeatureAnchor, LineageRole, RawHandle,
};
use cad_runtime::value::{NumberValue, Value};
use cad_types::Dimension;
use cad_units::OperandType;

/// `notched` is a named feature whose own final IR node is a real `Cut`
/// (lineage-capable) — a box with a through-hole bored by a cylinder whose
/// own `radius` is a top-level `param`, so editing it dirties `poker`
/// (which directly consumes `radius`) and `notched` (which consumes
/// `poker`) only. `spacer` is deliberately independent (an
/// untouched `Transform(Box)` feature, not lineage-capable, sharing no
/// dependency edge with `radius` at all) — the "unchanged branch is
/// reused, not rebuilt" half of this task's own reference-replay
/// requirement.
const SOURCE: &str = "\
param radius: Length = 3mm;\n\
let base = box(20mm, 20mm, 20mm);\n\
let poker = transform(cylinder(radius, 40mm), 10mm, 10mm, -10mm);\n\
let notched = cut(base, poker);\n\
let spacer = transform(box(10mm, 10mm, 10mm), 100mm, 0mm, 0mm);\n\
";

fn length(metres: f64) -> Magnitude {
    Magnitude::new(metres, OperandType::dimensional(Dimension::Length, None))
}

fn length_value(metres: f64) -> Value {
    Value::Number(NumberValue {
        magnitude: metres,
        ty: OperandType::dimensional(Dimension::Length, None),
    })
}

/// A `Query` selecting `spacer`'s own +Z top face by area + normal (the
/// same disambiguation combo `cad_query::resolve`'s own
/// `semantic_query_resolves_to_the_unique_matching_face` test uses) — a
/// `SemanticQuery`-shaped reference this task's own `ResolverContext::
/// candidates` implementation can answer with no lineage evidence at all.
fn spacer_top_face_query() -> Query {
    Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Area(
            Comparison::Eq(length(0.0001)),
        )))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Normal(
            cad_query::predicate::DirectionComparison {
                target: cad_query::Direction3::POSITIVE_Z,
                tolerance: None,
            },
        )))
        .with_cardinality(cad_query::CardinalityExpectation::Unique)
}

fn resolved_face<'ctx>(outcome: ResolutionOutcome<'ctx>) -> Candidate<'ctx> {
    match outcome {
        ResolutionOutcome::Resolved(mut candidates) => {
            assert_eq!(candidates.len(), 1);
            candidates.pop().unwrap()
        }
        ResolutionOutcome::Ambiguous(candidates) => {
            panic!(
                "expected Resolved, got Ambiguous({} candidates)",
                candidates.len()
            )
        }
        ResolutionOutcome::Broken(reason) => panic!("expected Resolved, got Broken({reason:?})"),
    }
}

/// The untouched-branch half: a `SemanticQuery`-shaped reference resolved
/// against `spacer` before and after a rebuild that dirties only
/// `notched` must keep resolving to the exact same live entity —
/// `Shape::is_same` identity, not merely equal geometry — proving
/// resolution really observes this round's own reused (never
/// recomputed) `Shape`, not a fresh copy.
#[test]
fn resolving_an_unrelated_reference_survives_a_rebuild_that_dirties_a_different_feature() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let mut session = ParametricBuildSession::new("test.aicad", SOURCE, &ctx)
        .expect("initial build should succeed");

    let query = spacer_top_face_query();
    let before = resolved_face(
        session
            .resolve(&query)
            .expect("resolution should not error"),
    );
    assert!((before.shape().area().unwrap() - 0.0001).abs() < 1e-12);

    session.set_param("radius", length_value(0.005)).unwrap();
    let outcome = session.rebuild().expect("edit/rebuild should succeed");
    assert_eq!(
        outcome.dirty_feature_names,
        vec!["poker".to_string(), "notched".to_string()]
    );

    let after = resolved_face(
        session
            .resolve(&query)
            .expect("resolution should not error"),
    );
    assert!(
        before.shape().is_same(after.shape()).unwrap(),
        "spacer was never touched by the edit, so re-resolving the same query after rebuild \
         must land on the literal same live entity, not merely an equal-area lookalike"
    );
}

/// The dirty-branch half: a `FeatureLineage`-shaped reference
/// (`generated_by(notched)`) resolved before and after an edit that
/// actually changes `notched`'s own geometry must reflect the *real*
/// regenerated evidence, not stale/cached evidence from the first build —
/// the crux of "reference replay must observe actual regenerated
/// semantic/lineage state."
#[test]
fn resolving_a_generated_by_reference_reflects_the_real_regenerated_hole_radius() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let mut session = ParametricBuildSession::new("test.aicad", SOURCE, &ctx)
        .expect("initial build should succeed");

    let generated_by_notched = AnyRef::Face(FaceRef::from_strategy(
        ConstructionStrategy::FeatureLineage {
            feature: FeatureAnchor::named("notched"),
            role: LineageRole::Generated,
        },
    ));

    let resolution = session
        .resolve_reference(&generated_by_notched)
        .expect("resolution should not error");
    assert_eq!(
        resolution.durability,
        cad_references::DurabilityLevel::Lineage
    );
    let before = resolved_face(resolution.outcome);
    let radius_before = before
        .shape()
        .face_radius()
        .expect("the new wall face is cylindrical");
    assert!(
        (radius_before - 0.003).abs() < 1e-9,
        "radius_before = {radius_before}"
    );

    // Edit `radius` -- a real geometry change to the exact feature this
    // reference is anchored to.
    session.set_param("radius", length_value(0.005)).unwrap();
    let outcome = session.rebuild().expect("edit/rebuild should succeed");
    assert_eq!(
        outcome.dirty_feature_names,
        vec!["poker".to_string(), "notched".to_string()]
    );

    let after = resolved_face(
        session
            .resolve_reference(&generated_by_notched)
            .expect("resolution should not error")
            .outcome,
    );
    let radius_after = after.shape().face_radius().expect("still cylindrical");
    assert!(
        (radius_after - 0.005).abs() < 1e-9,
        "re-resolving generated_by(notched) after the edit must reflect the real new 5mm \
         radius, not the stale 3mm evidence from the first build (radius_after = {radius_after})"
    );
    assert!(
        !before.shape().is_same(after.shape()).unwrap(),
        "notched was recomputed (a real new OCCT shape), so the re-resolved candidate must not \
         be the same live entity as the pre-edit one"
    );
}

/// Independent confirmation via the lower-level `EvaluationEvidence`
/// surface directly (`generated_by`/`modified_by`), not only through the
/// resolver: the classified lineage report itself names the new wall face
/// `New`, matching `cad_query::feature_lineage`'s own established
/// classification semantics against this session's real captured
/// evidence.
#[test]
fn the_new_cylindrical_wall_face_is_classified_new_by_real_captured_lineage() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = ParametricBuildSession::new("test.aicad", SOURCE, &ctx)
        .expect("initial build should succeed");

    let notched = session.binding_named("notched").unwrap();
    let notched_shape = session.shape_for_binding(notched).unwrap();
    let new_faces: Vec<_> = (0..notched_shape.face_count().unwrap())
        .map(|i| notched_shape.get_face(i).unwrap())
        .filter(|face| face.surface_type().unwrap() == cad_occt_bridge::SurfaceKind::Cylinder)
        .collect();
    assert_eq!(new_faces.len(), 1, "exactly one new cylindrical wall face");

    let anchor = FeatureAnchor::named("notched");
    let candidate = Candidate::new(
        EntityKind::Face,
        notched_shape
            .get_face(
                (0..notched_shape.face_count().unwrap())
                    .find(|&i| {
                        notched_shape.get_face(i).unwrap().surface_type().unwrap()
                            == cad_occt_bridge::SurfaceKind::Cylinder
                    })
                    .unwrap(),
            )
            .unwrap(),
    );
    assert_eq!(
        cad_query::EvaluationEvidence::generated_by(&session, &candidate, &anchor),
        Some(true)
    );
    assert_eq!(
        cad_query::EvaluationEvidence::modified_by(&session, &candidate, &anchor),
        Some(false)
    );
    let _ = ResultEntityOrigin::New; // documents which classification this proves, above
}

/// `AICAD-100A`: Edge lineage, not only Face — the same real
/// `notched = cut(base, poker)` operation's own new circular rim edges
/// (where the new cylindrical wall face meets `base`'s own top/bottom
/// faces) are classified `New` by real captured evidence, exactly
/// mirroring `the_new_cylindrical_wall_face_is_classified_new_by_real_
/// captured_lineage`'s own Face-kind proof one entity kind over.
#[test]
fn the_new_hole_rim_edges_are_classified_new_by_real_captured_edge_lineage() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = ParametricBuildSession::new("test.aicad", SOURCE, &ctx)
        .expect("initial build should succeed");

    let notched = session.binding_named("notched").unwrap();
    let notched_shape = session.shape_for_binding(notched).unwrap();
    let circular_edges: Vec<_> = (0..notched_shape.edge_count().unwrap())
        .map(|i| notched_shape.get_edge(i).unwrap())
        .filter(|edge| edge.curve_type().unwrap() == cad_occt_bridge::CurveKind::Circle)
        .collect();
    assert_eq!(
        circular_edges.len(),
        2,
        "a straight-through round hole has exactly two circular rim edges (top and bottom)"
    );

    let anchor = FeatureAnchor::named("notched");
    for edge in &circular_edges {
        let candidate = Candidate::new(EntityKind::Edge, edge.duplicate().unwrap());
        assert_eq!(
            cad_query::EvaluationEvidence::generated_by(&session, &candidate, &anchor),
            Some(true),
            "each rim edge must be classified generated_by(notched)"
        );
        assert_eq!(
            cad_query::EvaluationEvidence::modified_by(&session, &candidate, &anchor),
            Some(false)
        );
    }

    // A pre-existing, untouched box edge is neither generated nor
    // modified by `notched` -- proves this evidence source does not
    // over-match every edge in the result shape.
    let untouched_edge = (0..notched_shape.edge_count().unwrap())
        .map(|i| notched_shape.get_edge(i).unwrap())
        .find(|edge| edge.curve_type().unwrap() != cad_occt_bridge::CurveKind::Circle)
        .expect("a straight-line box edge exists");
    let untouched_candidate = Candidate::new(EntityKind::Edge, untouched_edge);
    assert_eq!(
        cad_query::EvaluationEvidence::generated_by(&session, &untouched_candidate, &anchor),
        Some(false)
    );
}

/// The same Edge lineage through the real resolver path (`Query`/
/// `TopologyPredicate::GeneratedBy`), scoped to `notched` (`AICAD-099A`).
#[test]
fn generated_by_notched_resolves_ambiguous_across_both_real_rim_edges() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = ParametricBuildSession::new("test.aicad", SOURCE, &ctx)
        .expect("initial build should succeed");

    let query = cad_query::Query::new(EntityKind::Edge)
        .with_clause(cad_query::QueryClause::Topology(
            cad_query::TopologyPredicate::GeneratedBy(FeatureAnchor::named("notched")),
        ))
        // Narrows to the two genuinely circular rim edges, excluding a
        // real OCCT implementation detail (a straight vertical seam edge
        // some kernel versions add where a full cylindrical face is
        // internally split into two half-cylinder patches) -- `edge_
        // radius` is only defined for `CurveKind::Circle`
        // (`Shape::edge_radius`'s own doc comment), so a non-circular
        // edge simply never matches this clause, not an arbitrary
        // narrowing.
        .with_clause(cad_query::QueryClause::Geometry(
            cad_query::GeometryPredicate::Radius(cad_query::Comparison::Gt(length(0.0))),
        ))
        .with_cardinality(cad_query::CardinalityExpectation::Unique)
        .scoped_to(FeatureAnchor::named("notched"));
    let outcome = session
        .resolve(&query)
        .expect("resolution should not error");
    match outcome {
        cad_query::ResolutionOutcome::Ambiguous(candidates) => assert_eq!(
            candidates.len(),
            2,
            "both real rim edges are genuinely generated_by(notched); a resolver that narrowed \
             this to one would be silently wrong"
        ),
        other => {
            let description = match &other {
                cad_query::ResolutionOutcome::Resolved(c) => format!("Resolved({})", c.len()),
                cad_query::ResolutionOutcome::Ambiguous(c) => format!("Ambiguous({})", c.len()),
                cad_query::ResolutionOutcome::Broken(r) => format!("Broken({r:?})"),
            };
            panic!("expected Ambiguous(2), got a different outcome ({description})")
        }
    }
}

/// `AICAD-093`'s raw-handle epoch, wired to a real session
/// (`AICAD-094`): a `RawHandle` minted around a real live `Candidate` from
/// before a rebuild round is explicitly rejected after it, even though
/// nothing about that `Candidate`'s own Rust value changed or was
/// dropped -- it is simply epoch-stale, exactly `AICAD-093`'s own
/// `raw_handle` module doc comment's "explicitly invalid once its owning
/// context/epoch is invalid" contract, now proven against a real
/// `ParametricBuildSession` round instead of a hand-rolled counter.
#[test]
fn a_raw_handle_minted_before_a_rebuild_is_rejected_after_it() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let mut session = ParametricBuildSession::new("test.aicad", SOURCE, &ctx)
        .expect("initial build should succeed");

    let spacer = session.binding_named("spacer").unwrap();
    let spacer_shape = session.shape_for_binding(spacer).unwrap();
    let face = spacer_shape.get_face(0).unwrap();
    let handle: RawHandle<_> = session.epoch_counter().mint(face);
    assert!(handle.get(session.epoch_counter()).is_ok());

    // A no-op rebuild (no param edited) still advances the epoch, per
    // `ParametricBuildSession::rebuild`'s own doc comment: staleness
    // must not depend on whether anything actually changed.
    session.rebuild().expect("no-op rebuild should succeed");

    assert!(
        handle.get(session.epoch_counter()).is_err(),
        "a handle minted before any rebuild round must be rejected after it"
    );
}
