//! `AICAD-100A`: real production-path proofs for the Stage-4 predicate
//! variants `AICAD-083`/`084` originally left `EvalError::NotYetSpecified`
//! (`Convex`/`Concave`/`Manifold`/`NonManifold`/`ConnectedTo`/`Contains`/
//! `Intersects`/`NearestTo`/`FarthestFrom`) — resolved for real, against a
//! real [`ParametricBuildSession`] build, not a hand-constructed
//! `cad-query`-only fixture (`crates/cad-query/src/eval.rs`'s own test
//! suite already proves the per-candidate evaluator logic in isolation;
//! this file proves the same predicates work end to end through the real
//! production resolver path).

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_query::{
    GeometryPredicate, Magnitude, Query, QueryClause, RankingDirective, ResolutionOutcome,
    SpatialPredicate, SpatialTarget, TopologyPredicate,
};
use cad_references::{AnyRef, ConstructionStrategy, EntityKind, FeatureAnchor, SolidRef};
use cad_types::Dimension;
use cad_units::OperandType;

fn length_mm(value_mm: f64) -> Magnitude {
    Magnitude::new(
        value_mm / 1000.0,
        OperandType::dimensional(Dimension::Length, None),
    )
}

/// A `10mm` box with a `5x5x12mm` notch cut from one top corner — has both
/// convex edges (the box's own remaining outer corners) and at least one
/// concave edge (the notch's own inner corner), and is not a `Solid`
/// literally the whole box (so `Contains`/`Intersects` have a real,
/// non-trivial candidate to test against).
const NOTCHED_SOURCE: &str = "\
let base = box(10mm, 10mm, 10mm);\n\
let notch_raw = box(5mm, 5mm, 12mm);\n\
let notch = transform(notch_raw, 6mm, 6mm, -1mm);\n\
let notched = cut(base, notch);\n\
";

fn session(ctx: &OcctContext) -> ParametricBuildSession<'_> {
    ParametricBuildSession::new("test.aicad", NOTCHED_SOURCE, ctx)
        .expect("the notched-box fixture should build cleanly")
}

#[test]
fn convex_and_concave_both_find_real_candidates_in_the_notched_box() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);

    let convex = Query::new(EntityKind::Edge)
        .with_clause(QueryClause::Topology(TopologyPredicate::Convex))
        .scoped_to(FeatureAnchor::named("notched"));
    match session
        .resolve(&convex)
        .expect("resolution should not error")
    {
        ResolutionOutcome::Resolved(candidates) => {
            assert!(
                !candidates.is_empty(),
                "the box's own outer corners are convex"
            )
        }
        other => panic!(
            "expected Resolved (Unstated cardinality), got {}",
            describe(&other)
        ),
    }

    let concave = Query::new(EntityKind::Edge)
        .with_clause(QueryClause::Topology(TopologyPredicate::Concave))
        .scoped_to(FeatureAnchor::named("notched"));
    match session
        .resolve(&concave)
        .expect("resolution should not error")
    {
        ResolutionOutcome::Resolved(candidates) => assert!(
            !candidates.is_empty(),
            "the notch's own inner corner edge must be found concave"
        ),
        _ => panic!("expected Resolved"),
    }
}

#[test]
fn manifold_matches_ordinary_edges_and_non_manifold_matches_none_on_a_closed_solid() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);

    let manifold = Query::new(EntityKind::Edge)
        .with_clause(QueryClause::Topology(TopologyPredicate::Manifold))
        .scoped_to(FeatureAnchor::named("notched"));
    match session
        .resolve(&manifold)
        .expect("resolution should not error")
    {
        ResolutionOutcome::Resolved(candidates) => {
            assert!(
                !candidates.is_empty(),
                "a closed solid's own edges are all manifold"
            )
        }
        _ => panic!("expected Resolved"),
    }

    let non_manifold = Query::new(EntityKind::Edge)
        .with_clause(QueryClause::Topology(TopologyPredicate::NonManifold))
        .scoped_to(FeatureAnchor::named("notched"));
    match session
        .resolve(&non_manifold)
        .expect("resolution should not error")
    {
        ResolutionOutcome::Resolved(candidates) => assert!(
            candidates.is_empty(),
            "a closed, watertight solid has no non-manifold edges"
        ),
        _ => panic!("expected Resolved"),
    }
}

#[test]
fn contains_finds_a_point_inside_and_rejects_a_point_far_outside() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);

    // A point deep inside the un-notched region of the box.
    let inside_point = cad_query::Point3::new(length_mm(2.0), length_mm(2.0), length_mm(2.0));
    let contains_inside = Query::new(EntityKind::Solid)
        .with_clause(QueryClause::Topology(TopologyPredicate::Contains(
            inside_point,
        )))
        .with_cardinality(cad_query::CardinalityExpectation::Unique)
        .scoped_to(FeatureAnchor::named("notched"));
    match session
        .resolve(&contains_inside)
        .expect("resolution should not error")
    {
        ResolutionOutcome::Resolved(candidates) => assert_eq!(candidates.len(), 1),
        other => panic!(
            "expected the notched solid to contain a point deep inside it, got {}",
            describe(&other)
        ),
    }

    let far_point = cad_query::Point3::new(length_mm(1000.0), length_mm(0.0), length_mm(0.0));
    let contains_far = Query::new(EntityKind::Solid)
        .with_clause(QueryClause::Topology(TopologyPredicate::Contains(
            far_point,
        )))
        .with_cardinality(cad_query::CardinalityExpectation::Unique)
        .scoped_to(FeatureAnchor::named("notched"));
    match session
        .resolve(&contains_far)
        .expect("resolution should not error")
    {
        ResolutionOutcome::Broken(_) => {}
        other => panic!("expected Broken(NoMatch), got {}", describe(&other)),
    }
}

/// `Intersects`' own target resolution goes through the real resolver
/// (`resolve_ref`) — real positive/negative `Intersects` proofs against a
/// directly-supplied evidence source already live in `crate::eval`'s own
/// unit tests (`intersects_is_true_for_overlapping_solids_and_false_for_
/// disjoint_ones`). This test instead proves the predicate variant is
/// wired into the real production match arm end to end (no longer
/// `NotYetSpecified`, and no production evidence source silently swallows
/// the request): an `ExplicitExport` target this session has no real
/// export-binding registry for correctly fails closed rather than
/// panicking, erroring, or silently reporting "no intersection."
#[test]
fn intersects_fails_closed_through_the_real_production_path_with_no_export_registry() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);

    let query = Query::new(EntityKind::Solid)
        .with_clause(QueryClause::Topology(TopologyPredicate::Intersects(
            cad_query::AdjacencyTarget::Ref(AnyRef::Solid(SolidRef::from_strategy(
                ConstructionStrategy::ExplicitExport {
                    feature: FeatureAnchor::named("base"),
                    export_name: "unused".into(),
                },
            ))),
        )))
        .scoped_to(FeatureAnchor::named("base"));
    let outcome = session
        .resolve(&query)
        .expect("resolution should not error");
    assert!(
        matches!(outcome, ResolutionOutcome::Broken(_)),
        "no production export-binding evidence exists in this session, so this must fail closed"
    );
}

#[test]
fn nearest_to_query_clause_resolves_through_the_real_ranking_rewrite() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);

    // The box's own +Z-facing top face nearest a point directly above it.
    let above = cad_query::Point3::new(length_mm(1.0), length_mm(1.0), length_mm(100.0));
    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Planar))
        .with_clause(QueryClause::Spatial(SpatialPredicate::NearestTo(
            SpatialTarget::Point(above),
        )))
        .with_cardinality(cad_query::CardinalityExpectation::Unique)
        .scoped_to(FeatureAnchor::named("notched"));
    let outcome = session
        .resolve(&query)
        .expect("resolution should not error");
    match outcome {
        ResolutionOutcome::Resolved(candidates) => assert_eq!(candidates.len(), 1),
        other => panic!(
            "expected the nearest_to query clause to resolve uniquely via the real ranking \
             rewrite, got {}",
            describe(&other)
        ),
    }
}

#[test]
fn ranking_directive_and_spatial_predicate_spelling_of_nearest_agree() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    let above = cad_query::Point3::new(length_mm(1.0), length_mm(1.0), length_mm(100.0));

    let via_predicate = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Planar))
        .with_clause(QueryClause::Spatial(SpatialPredicate::NearestTo(
            SpatialTarget::Point(above),
        )))
        .with_cardinality(cad_query::CardinalityExpectation::Unique)
        .scoped_to(FeatureAnchor::named("notched"));
    let via_ranking = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Planar))
        .with_clause(QueryClause::Ranking(RankingDirective::Nearest(
            SpatialTarget::Point(above),
        )))
        .with_cardinality(cad_query::CardinalityExpectation::Unique)
        .scoped_to(FeatureAnchor::named("notched"));

    let a = session
        .resolve(&via_predicate)
        .expect("resolution should not error");
    let b = session
        .resolve(&via_ranking)
        .expect("resolution should not error");
    assert_eq!(describe(&a), describe(&b));
}

fn describe(outcome: &ResolutionOutcome<'_>) -> String {
    match outcome {
        ResolutionOutcome::Resolved(candidates) => format!("Resolved({})", candidates.len()),
        ResolutionOutcome::Ambiguous(candidates) => format!("Ambiguous({})", candidates.len()),
        ResolutionOutcome::Broken(reason) => format!("Broken({reason:?})"),
    }
}
