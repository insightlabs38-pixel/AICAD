//! `AICAD-101`: nested `part`-in-`part` semantic-reference resolution.
//!
//! Proves, through the real production `ParametricBuildSession`/
//! `resolve_reference` path (never direct Rust construction of a
//! `Value::Part` or a hand-built candidate), that:
//!
//! - a feature declared two levels deep inside nested `part` bodies is
//!   discovered and addressable by its full dotted scope path, exactly
//!   like a single-level `part`-nested feature already was (`D31`,
//!   `AICAD-100A`);
//! - a bare leaf name repeated in two different part scopes fails closed
//!   (`Broken`), never silently resolving to an arbitrary one of the two,
//!   while each fully-qualified path still resolves unambiguously.

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_query::ResolutionOutcome;
use cad_references::{AnyRef, ConstructionStrategy, FeatureAnchor, SolidRef};

fn describe(outcome: &ResolutionOutcome<'_>) -> String {
    match outcome {
        ResolutionOutcome::Resolved(candidates) => format!("Resolved({})", candidates.len()),
        ResolutionOutcome::Ambiguous(candidates) => format!("Ambiguous({})", candidates.len()),
        ResolutionOutcome::Broken(reason) => format!("Broken({reason:?})"),
    }
}

const TWO_LEVEL_SOURCE: &str = "\
part Wall {\n\
\tpart Door {\n\
\t\tlet hinge: Geometry = box(2mm, 2mm, 2mm);\n\
\t}\n\
\tlet sill: Geometry = box(10mm, 1mm, 1mm);\n\
}\n\
";

#[test]
fn a_feature_nested_two_levels_deep_resolves_by_its_full_dotted_scope_path() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = ParametricBuildSession::new("test.aicad", TWO_LEVEL_SOURCE, &ctx)
        .expect("the fixture should build cleanly");

    let reference = AnyRef::Solid(SolidRef::from_strategy(
        ConstructionStrategy::ExplicitExport {
            feature: FeatureAnchor::named("Wall.Door"),
            export_name: "hinge".to_string(),
        },
    ));
    let resolution = session
        .resolve_reference(&reference)
        .expect("resolution should not error");
    match resolution.outcome {
        ResolutionOutcome::Resolved(candidates) => {
            assert_eq!(candidates.len(), 1);
            assert!((candidates[0].shape().volume().unwrap() - 0.000000008).abs() < 1e-15);
        }
        other => panic!("expected Resolved(1), got {}", describe(&other)),
    }
}

#[test]
fn a_sibling_leaf_at_the_enclosing_scope_still_resolves_unambiguously() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = ParametricBuildSession::new("test.aicad", TWO_LEVEL_SOURCE, &ctx)
        .expect("the fixture should build cleanly");

    // `sill` lives directly in `Wall`, a sibling of the nested `Door` part
    // -- proves nesting a part alongside ordinary features does not
    // disturb the enclosing scope's own bindings.
    let reference = AnyRef::Solid(SolidRef::from_strategy(
        ConstructionStrategy::ExplicitExport {
            feature: FeatureAnchor::named("Wall"),
            export_name: "sill".to_string(),
        },
    ));
    let resolution = session
        .resolve_reference(&reference)
        .expect("resolution should not error");
    match resolution.outcome {
        ResolutionOutcome::Resolved(candidates) => assert_eq!(candidates.len(), 1),
        other => panic!("expected Resolved(1), got {}", describe(&other)),
    }
}

const COLLIDING_LEAF_SOURCE: &str = "\
part Wall {\n\
\tpart Left {\n\
\t\tlet body: Geometry = box(1mm, 1mm, 1mm);\n\
\t}\n\
\tpart Right {\n\
\t\tlet body: Geometry = box(2mm, 2mm, 2mm);\n\
\t}\n\
}\n\
";

#[test]
fn a_bare_leaf_name_repeated_in_two_part_scopes_fails_closed_never_picks_one() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = ParametricBuildSession::new("test.aicad", COLLIDING_LEAF_SOURCE, &ctx)
        .expect("the fixture should build cleanly");

    let reference = AnyRef::Solid(SolidRef::from_strategy(
        ConstructionStrategy::StructuralRole("body".to_string()),
    ));
    let resolution = session
        .resolve_reference(&reference)
        .expect("resolution should not error");
    match resolution.outcome {
        ResolutionOutcome::Broken(_) => {}
        other => panic!(
            "a bare name colliding across two different part scopes must fail closed, never \
             pick an arbitrary candidate -- got {}",
            describe(&other)
        ),
    }

    for (scope, expected_volume) in [
        ("Wall.Left", 0.000000001_f64),
        ("Wall.Right", 0.000000008_f64),
    ] {
        let reference = AnyRef::Solid(SolidRef::from_strategy(
            ConstructionStrategy::ExplicitExport {
                feature: FeatureAnchor::named(scope),
                export_name: "body".to_string(),
            },
        ));
        let resolution = session
            .resolve_reference(&reference)
            .expect("resolution should not error");
        match resolution.outcome {
            ResolutionOutcome::Resolved(candidates) => {
                assert_eq!(candidates.len(), 1);
                assert!((candidates[0].shape().volume().unwrap() - expected_volume).abs() < 1e-15);
            }
            other => panic!(
                "expected Resolved(1) for '{scope}', got {}",
                describe(&other)
            ),
        }
    }
}
