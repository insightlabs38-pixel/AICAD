//! `AICAD-100A`: real production-evidence proofs for the resolver
//! strategies that previously had no evidence source in
//! [`ParametricBuildSession`] at all (`ExplicitExport`/`StructuralRole`/
//! `UserConfirmed`/`SemanticQuery`) — each now resolves from real session
//! state, never test-only injection.

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_query::{Query, ResolutionOutcome};
use cad_references::{
    AnyRef, ConstructionStrategy, DurabilityLevel, EntityKind, FeatureAnchor, QueryHandle, SolidRef,
};

const SOURCE: &str = "\
part Wall {\n\
\tlet body: Geometry = box(10mm, 10mm, 10mm);\n\
\tlet top_mounting_surface: Geometry = box(4mm, 4mm, 4mm);\n\
}\n\
let spacer: Geometry = box(5mm, 5mm, 5mm);\n\
";

fn session(ctx: &OcctContext) -> ParametricBuildSession<'_> {
    ParametricBuildSession::new("test.aicad", SOURCE, ctx)
        .expect("the fixture should build cleanly")
}

fn describe(outcome: &ResolutionOutcome<'_>) -> String {
    match outcome {
        ResolutionOutcome::Resolved(candidates) => format!("Resolved({})", candidates.len()),
        ResolutionOutcome::Ambiguous(candidates) => format!("Ambiguous({})", candidates.len()),
        ResolutionOutcome::Broken(reason) => format!("Broken({reason:?})"),
    }
}

/// `ExplicitExport { feature: "Wall", export_name: "body" }` resolves to
/// `Wall.body`'s own real current geometry — the same `<part>.<field>`
/// addressing `cad build --name Wall.body` already uses in production.
#[test]
fn explicit_export_resolves_a_real_part_nested_binding() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);

    let reference = AnyRef::Solid(SolidRef::from_strategy(
        ConstructionStrategy::ExplicitExport {
            feature: FeatureAnchor::named("Wall"),
            export_name: "body".to_string(),
        },
    ));
    let resolution = session
        .resolve_reference(&reference)
        .expect("resolution should not error");
    assert_eq!(resolution.durability, DurabilityLevel::Explicit);
    match resolution.outcome {
        ResolutionOutcome::Resolved(candidates) => {
            assert_eq!(candidates.len(), 1);
            assert!((candidates[0].shape().volume().unwrap() - 0.000001).abs() < 1e-12);
        }
        other => panic!("expected Resolved(1), got {}", describe(&other)),
    }
}

/// A top-level (non-`part`-nested) binding is addressed the same way, via
/// `FeatureAnchor::CurrentFeature`'s own documented "no meaning outside an
/// `expose {...}` body" restriction — `spacer` (declared at the top level,
/// no enclosing part) cannot be reached through `ExplicitExport` at all,
/// proving this evidence source never silently guesses a scope.
#[test]
fn explicit_export_does_not_apply_outside_a_named_scope() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);

    let reference = AnyRef::Solid(SolidRef::from_strategy(
        ConstructionStrategy::ExplicitExport {
            feature: FeatureAnchor::CurrentFeature,
            export_name: "spacer".to_string(),
        },
    ));
    let resolution = session
        .resolve_reference(&reference)
        .expect("resolution should not error");
    assert!(resolution.outcome.is_broken());
}

/// `StructuralRole("top_mounting_surface")` resolves to the real binding
/// literally named `top_mounting_surface` — the program's own declared
/// source name IS the real evidence, via the same collision-safe bare-name
/// lookup `D31`'s scoped-feature-name resolution already established.
#[test]
fn structural_role_resolves_a_binding_by_its_own_declared_name() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);

    let reference = AnyRef::Solid(SolidRef::from_strategy(
        ConstructionStrategy::StructuralRole("top_mounting_surface".to_string()),
    ));
    let resolution = session
        .resolve_reference(&reference)
        .expect("resolution should not error");
    assert_eq!(resolution.durability, DurabilityLevel::QueryStrong);
    match resolution.outcome {
        ResolutionOutcome::Resolved(candidates) => {
            assert!((candidates[0].shape().volume().unwrap() - 0.000000064).abs() < 1e-15)
        }
        other => panic!("expected Resolved(1), got {}", describe(&other)),
    }
}

#[test]
fn structural_role_reports_broken_for_an_unnamed_role() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);

    let reference = AnyRef::Solid(SolidRef::from_strategy(
        ConstructionStrategy::StructuralRole("no_such_role".to_string()),
    ));
    let resolution = session
        .resolve_reference(&reference)
        .expect("resolution should not error");
    assert!(resolution.outcome.is_broken());
}

/// `UserConfirmed`'s own `confirmation_note` resolves the same way — real
/// evidence (a real declared binding name), never a fabricated workflow.
#[test]
fn user_confirmed_resolves_by_the_confirmed_bindings_own_name() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);

    let reference = AnyRef::Solid(SolidRef::from_strategy(
        ConstructionStrategy::UserConfirmed {
            confirmation_note: "spacer".to_string(),
        },
    ));
    let resolution = session
        .resolve_reference(&reference)
        .expect("resolution should not error");
    assert_eq!(resolution.durability, DurabilityLevel::Explicit);
    assert!(resolution.outcome.is_resolved());
}

/// `SemanticQuery` resolves through this session's own real registered-
/// query registry (`ParametricBuildSession::register_query`) — a real
/// production API, not a test-only bypass: a future `.aicad` `query {
/// ... }` declaration lowers into exactly this same call.
#[test]
fn semantic_query_resolves_through_the_real_registered_query_registry() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let mut session = session(&ctx);

    let handle = QueryHandle::named("wall_body");
    let query = Query::new(EntityKind::Solid)
        .with_cardinality(cad_query::CardinalityExpectation::Unique)
        .scoped_to(FeatureAnchor::named("Wall.body"));
    session.register_query(handle.clone(), query);

    let reference = AnyRef::Solid(SolidRef::from_strategy(
        ConstructionStrategy::semantic_query(handle, DurabilityLevel::QueryStrong).unwrap(),
    ));
    let resolution = session
        .resolve_reference(&reference)
        .expect("resolution should not error");
    assert!(resolution.outcome.is_resolved());
}

#[test]
fn semantic_query_reports_broken_for_an_unregistered_handle_in_a_real_session() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);

    let reference = AnyRef::Solid(SolidRef::from_strategy(
        ConstructionStrategy::semantic_query(
            QueryHandle::named("never_registered"),
            DurabilityLevel::QueryStrong,
        )
        .unwrap(),
    ));
    let resolution = session
        .resolve_reference(&reference)
        .expect("resolution should not error");
    assert!(resolution.outcome.is_broken());
}

/// `ConstructionStrategy::Ancestry`'s own real derived-scope path (the
/// `resolve_reference` `Ancestry` arm, `AICAD-100A`): an ancestor with no
/// derivable feature scope (anything other than `FeatureLineage`-strategy,
/// e.g. `ExplicitExport` here) must fail closed rather than falling back
/// to the whole-session universe — real, not a guess, and distinct from
/// the `Broken(InsufficientEvidence)` a `FeatureLineage`-anchored ancestor
/// with real captured lineage would instead resolve through
/// (`crates/cad-query/src/resolve.rs`'s own `descended_from` tests already
/// cover that positive path in depth).
#[test]
fn ancestry_with_a_non_derivable_scope_fails_closed_in_a_real_session() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);

    let reference = AnyRef::Solid(SolidRef::from_strategy(ConstructionStrategy::Ancestry(
        Box::new(AnyRef::Solid(SolidRef::from_strategy(
            ConstructionStrategy::ExplicitExport {
                feature: FeatureAnchor::named("Wall"),
                export_name: "body".to_string(),
            },
        ))),
    )));
    let resolution = session
        .resolve_reference(&reference)
        .expect("resolution should not error");
    assert!(resolution.outcome.is_broken());
}
