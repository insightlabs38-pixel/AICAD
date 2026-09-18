//! `AICAD-104`: proves the complete `.aicad` source query vocabulary
//! (`AICAD-100A`'s original subset plus `AICAD-103`'s completion) through
//! the real production path — `ParametricBuildSession::new` (parse ->
//! `cad-hir` lower/typeck -> `crate::query_lowering::lower_hir_queries` ->
//! `register_query` -> initial build) and `ParametricBuildSession::
//! resolve_reference`/`resolve` (the real `cad_query::resolve_reference`/
//! `resolve_query` resolver) — never direct Rust construction of a
//! `Query`/`Candidate`/`ResolutionOutcome`.
//!
//! Covers every case category `AICAD-104`'s own acceptance requires:
//! positive, no-match, ambiguous, invalid-scope, wrong-cardinality,
//! nested-reference, and spatial-value — plus a dedicated proof that
//! `AICAD-103`'s own correction (`nearest_to`/`farthest_from` have real
//! semantics via the resolver's ranking rewrite) actually holds through
//! this exact production path, not merely at the lowering-unit-test
//! level `crate::query_lowering`'s own tests already cover.

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_query::{BrokenReason, ResolutionOutcome, ResolverContext};
use cad_references::{FeatureAnchor, QueryHandle};

fn describe(outcome: &ResolutionOutcome<'_>) -> String {
    match outcome {
        ResolutionOutcome::Resolved(candidates) => format!("Resolved({})", candidates.len()),
        ResolutionOutcome::Ambiguous(candidates) => format!("Ambiguous({})", candidates.len()),
        ResolutionOutcome::Broken(reason) => format!("Broken({reason:?})"),
    }
}

/// A box `10mm x 20mm x 5mm` (`base`, inside `part Wall`), spanning
/// `(0,0,0)` to `(10mm,20mm,5mm)` (`OcctContext::create_box`'s own
/// documented placement), plus a tiny nested `part Door { let hinge ...
/// }` two levels deep (`AICAD-101`) sharing the same scope. Face areas:
/// two `200mm^2` (the `10mm x 20mm` faces), two `100mm^2` (`20mm x 5mm`),
/// two `50mm^2` (`10mm x 5mm`) — every `area(...)` threshold below is
/// chosen against this exact, known geometry, never an assumption.
const SOURCE: &str = "\
part Wall {\n\
\tlet base: Geometry = box(10mm, 20mm, 5mm);\n\
\tpart Door {\n\
\t\tlet hinge: Geometry = box(1mm, 1mm, 1mm);\n\
\t}\n\
}\n\
query whole : Solid in Wall.base { unique(); }\n\
query none_such : Face in Wall.base { area(gt, 999999mm2); unique(); }\n\
query many_faces : Face in Wall.base { area(gt, 40mm2); unique(); }\n\
query bad_scope : Face in NoSuchFeature { planar(); }\n\
query too_few : Face in Wall.base { area(gt, 150mm2); expect_count(3); }\n\
query too_many : Face in Wall.base { area(gt, 150mm2); expect_count(1); }\n\
query hinge_ref : Solid in Wall.Door.hinge { unique(); }\n\
query near_hinge_ref : Solid in Wall.base { within(1m, Wall.Door.hinge); unique(); }\n\
query contains_pt : Solid in Wall.base { contains(1mm, 1mm, 1mm); unique(); }\n\
query nearest_pt : Solid in Wall.base { nearest_to(5mm, 10mm, 2.5mm); unique(); }\n\
query farthest_pt : Solid in Wall.base { farthest_from(5mm, 10mm, 2.5mm); unique(); }\n\
";

fn session(ctx: &OcctContext) -> ParametricBuildSession<'_> {
    ParametricBuildSession::new("test.aicad", SOURCE, ctx)
        .expect("the fixture should build cleanly through the real production path")
}

/// Resolves `name` via `ParametricBuildSession::source_references`
/// (every real source-declared `query { ... }`'s own qualified name and
/// the `AnyRef` it denotes) and `resolve_reference` -- the same lookup a
/// real caller resolving a named persistent query reference would use.
fn resolve_named<'ctx>(
    session: &ParametricBuildSession<'ctx>,
    name: &str,
) -> ResolutionOutcome<'ctx> {
    let (_, reference) = session
        .source_references()
        .iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no source-declared query named '{name}'"));
    session
        .resolve_reference(reference)
        .expect("resolution should not error")
        .outcome
}

#[test]
fn positive_case_resolves_to_exactly_one_candidate() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    match resolve_named(&session, "whole") {
        ResolutionOutcome::Resolved(candidates) => assert_eq!(candidates.len(), 1),
        other => panic!("expected Resolved(1), got {}", describe(&other)),
    }
}

#[test]
fn no_match_case_reports_broken_no_match_never_a_guess() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    match resolve_named(&session, "none_such") {
        ResolutionOutcome::Broken(BrokenReason::NoMatch) => {}
        other => panic!("expected Broken(NoMatch), got {}", describe(&other)),
    }
}

#[test]
fn ambiguous_case_reports_every_candidate_never_picks_one_arbitrarily() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    match resolve_named(&session, "many_faces") {
        ResolutionOutcome::Ambiguous(candidates) => assert_eq!(candidates.len(), 6),
        other => panic!("expected Ambiguous(6), got {}", describe(&other)),
    }
}

#[test]
fn invalid_scope_case_reports_broken_scope_not_found() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    match resolve_named(&session, "bad_scope") {
        ResolutionOutcome::Broken(BrokenReason::ScopeNotFound(anchor)) => {
            assert_eq!(anchor, FeatureAnchor::named("NoSuchFeature"));
        }
        other => panic!(
            "expected Broken(ScopeNotFound(_)), got {}",
            describe(&other)
        ),
    }
}

/// `expect_count`'s own cardinality is only honored by `resolve`
/// (`cad_query::resolve_query`, which uses the query's own stated
/// cardinality) -- `resolve_reference` always forces `Unique` for a
/// single stable reference regardless of a `SemanticQuery`-backed
/// query's own cardinality (`ParametricBuildSession::resolve_reference`'s
/// own doc comment), so these two cases must go through `resolve`.
#[test]
fn wrong_cardinality_too_few_reports_broken_too_few() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    let query = session
        .lookup_query(&QueryHandle::named("too_few"))
        .expect("registered by ParametricBuildSession::new");
    match session.resolve(query).expect("resolution should not error") {
        ResolutionOutcome::Broken(BrokenReason::TooFew { expected, found }) => {
            assert_eq!(expected, 3);
            assert_eq!(found, 2);
        }
        other => panic!(
            "expected Broken(TooFew {{ expected: 3, found: 2 }}), got {}",
            describe(&other)
        ),
    }
}

#[test]
fn wrong_cardinality_too_many_reports_ambiguous_never_an_arbitrary_pick() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    let query = session
        .lookup_query(&QueryHandle::named("too_many"))
        .expect("registered by ParametricBuildSession::new");
    match session.resolve(query).expect("resolution should not error") {
        ResolutionOutcome::Ambiguous(candidates) => assert_eq!(candidates.len(), 2),
        other => panic!("expected Ambiguous(2), got {}", describe(&other)),
    }
}

#[test]
fn nested_reference_scope_two_levels_deep_resolves_correctly() {
    // `Wall.Door.hinge` -- a query scoped two `part` levels deep
    // (`AICAD-101`'s unbounded nesting reaching all the way through to
    // Stage-5 query resolution).
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    match resolve_named(&session, "hinge_ref") {
        ResolutionOutcome::Resolved(candidates) => assert_eq!(candidates.len(), 1),
        other => panic!("expected Resolved(1), got {}", describe(&other)),
    }
}

#[test]
fn nested_reference_as_a_clause_argument_resolves_correctly() {
    // `within(1m, Wall.Door.hinge)` -- a dotted nested reference used as
    // a clause *argument* (not the query's own scope), proving the
    // reference resolves through `ConstructionStrategy::StructuralRole`
    // rather than reporting `Broken(InsufficientEvidence)`.
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    match resolve_named(&session, "near_hinge_ref") {
        ResolutionOutcome::Resolved(candidates) => assert_eq!(candidates.len(), 1),
        other => panic!("expected Resolved(1), got {}", describe(&other)),
    }
}

#[test]
fn spatial_value_contains_a_literal_point3_resolves_correctly() {
    // `contains(1mm, 1mm, 1mm)` -- a literal `Point3` argument
    // (`AICAD-102`'s spatial-value construction, `AICAD-103`'s clause
    // lowering) against `base`'s own solid, which spans `(0,0,0)` to
    // `(10mm,20mm,5mm)` and so genuinely contains this point.
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    match resolve_named(&session, "contains_pt") {
        ResolutionOutcome::Resolved(candidates) => assert_eq!(candidates.len(), 1),
        other => panic!("expected Resolved(1), got {}", describe(&other)),
    }
}

#[test]
fn spatial_value_nearest_to_and_farthest_from_resolve_through_the_ranking_rewrite() {
    // Direct proof of the `AICAD-103` correction: `nearest_to`/
    // `farthest_from` lower to `SpatialPredicate::NearestTo`/
    // `FarthestFrom`, which only have real semantics via `cad_query::
    // resolve::filter_and_rank`'s own ranking rewrite -- this exercises
    // that exact production path, not `cad_query::eval::evaluate_spatial`
    // directly (which would report `NotYetSpecified`).
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    match resolve_named(&session, "nearest_pt") {
        ResolutionOutcome::Resolved(candidates) => assert_eq!(candidates.len(), 1),
        other => panic!("expected Resolved(1), got {}", describe(&other)),
    }
    match resolve_named(&session, "farthest_pt") {
        ResolutionOutcome::Resolved(candidates) => assert_eq!(candidates.len(), 1),
        other => panic!("expected Resolved(1), got {}", describe(&other)),
    }
}
