//! `AICAD-104A`: end-to-end proof that a `part`-scoped `param` (discovered
//! by `AICAD-101`'s own limitation sweep as invisible to `cad_runtime::
//! params::ParamModel`) now participates in the *real* production
//! `ParamModel -> FeatureGraph -> ParametricBuildSession` edit/rebuild
//! path exactly like a top-level one — deterministic scoped identity, no
//! collision with a same-named param in a different scope, correct
//! dependency-driven dirty propagation, and genuine incremental reuse of
//! everything the edit does not affect. Mirrors `stage3_parametric_
//! incremental_rebuild.rs`'s own proof shape, scoped inside a `part`.

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_runtime::value::{NumberValue, Value};
use cad_types::Dimension;
use cad_units::OperandType;

const SOURCE: &str = "\
part Wall {\n\
\tparam width: Length = 40mm;\n\
\tparam height: Length = width / 2.0;\n\
\tlet dependent = box(width, height, 5mm);\n\
\tlet independent = transform(box(20mm, 20mm, 5mm), 1000mm, 0mm, 0mm);\n\
\tlet combined = union(dependent, independent);\n\
}\n\
";

fn length_value(metres: f64) -> Value {
    Value::Number(NumberValue {
        magnitude: metres,
        ty: OperandType::dimensional(Dimension::Length, None),
    })
}

#[test]
fn initial_build_produces_correct_geometry_for_a_part_scoped_param_chain() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = ParametricBuildSession::new("test.aicad", SOURCE, &ctx)
        .expect("initial build should succeed");

    let dependent = session.binding_named("Wall.dependent").unwrap();
    let combined = session.binding_named("Wall.combined").unwrap();

    let dependent_shape = session.shape_for_binding(dependent).unwrap();
    let expected_dependent = 0.04 * 0.02 * 0.005;
    assert!(
        (dependent_shape.volume().unwrap() - expected_dependent).abs() < expected_dependent * 1e-9
    );

    let combined_shape = session.shape_for_binding(combined).unwrap();
    let expected_independent = 0.02 * 0.02 * 0.005;
    let expected_combined = expected_dependent + expected_independent;
    assert!(
        (combined_shape.volume().unwrap() - expected_combined).abs() < expected_combined * 1e-9
    );
}

/// The full remediation proof: edit `Wall.width` (a part-scoped param;
/// `Wall.height`, also part-scoped, is derived from it), rebuild, and
/// check both correctness AND genuine incrementality through the real
/// production path -- never a test-only model.
#[test]
fn editing_a_part_scoped_param_rebuilds_only_the_affected_branch_and_reuses_the_rest() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let mut session = ParametricBuildSession::new("test.aicad", SOURCE, &ctx)
        .expect("initial build should succeed");

    let dependent = session.binding_named("Wall.dependent").unwrap();
    let independent = session.binding_named("Wall.independent").unwrap();
    let combined = session.binding_named("Wall.combined").unwrap();

    let independent_handle_before = session.shape_for_binding(independent).unwrap().handle();

    // A genuine no-op rebuild (no param edited) reuses everything.
    let noop = session.rebuild().expect("no-op rebuild should succeed");
    assert!(
        noop.dirty_feature_names.is_empty(),
        "{:?}",
        noop.dirty_feature_names
    );
    assert!(
        noop.stats.recomputed.is_empty(),
        "{:?}",
        noop.stats.recomputed
    );
    assert_eq!(
        noop.stats.reused.len(),
        4,
        "dependent, independent's own box + transform, combined"
    );

    // Edit the part-scoped `width` by its fully qualified dotted name
    // (`AICAD-104A`'s own `set_param`/`ParamModel::find_by_name`
    // extension) -- `height` (also part-scoped) is derived from it, both
    // feed `dependent`, and `combined` consumes `dependent`.
    session.set_param("Wall.width", length_value(0.08)).unwrap();
    let outcome = session.rebuild().expect("edit/rebuild should succeed");

    assert_eq!(
        outcome.dirty_feature_names,
        vec!["Wall.dependent".to_string(), "Wall.combined".to_string()],
        "dependent (direct param reference) and combined (consumes dependent) must both be \
         dirty; independent shares no edge with width/height at all"
    );

    let dependent_shape = session.shape_for_binding(dependent).unwrap();
    let combined_shape = session.shape_for_binding(combined).unwrap();
    assert_eq!(outcome.stats.reused.len(), 2, "{:?}", outcome.stats.reused);
    assert_eq!(
        outcome.stats.recomputed.len(),
        2,
        "{:?}",
        outcome.stats.recomputed
    );

    let expected_dependent = 0.08 * 0.04 * 0.005;
    assert!(
        (dependent_shape.volume().unwrap() - expected_dependent).abs() < expected_dependent * 1e-9,
        "dependent's volume did not reflect width's edit and height's own re-derivation"
    );

    let expected_independent = 0.02 * 0.02 * 0.005;
    let expected_combined = expected_dependent + expected_independent;
    assert!(
        (combined_shape.volume().unwrap() - expected_combined).abs() < expected_combined * 1e-9,
        "combined's rebuilt volume must include independent's own untouched (reused) volume"
    );

    // Strongest possible reuse evidence: the literal same kernel handle,
    // not merely a coincidentally-equal freshly-built one.
    let independent_handle_after = session.shape_for_binding(independent).unwrap().handle();
    assert_eq!(
        independent_handle_before, independent_handle_after,
        "independent's own kernel shape must be the literal same handle across the rebuild"
    );
}

/// A same-named param in a different, unrelated part scope never collides
/// with `Wall.width` -- distinct `ParamId`s, distinct qualified names,
/// distinct dirty-propagation behavior.
#[test]
fn identical_param_names_in_two_different_parts_never_collide_in_the_production_path() {
    let source = "\
part Left {\n\
\tparam width: Length = 1mm;\n\
\tlet body = box(width, 1mm, 1mm);\n\
}\n\
part Right {\n\
\tparam width: Length = 2mm;\n\
\tlet body = box(width, 1mm, 1mm);\n\
}\n\
";
    let ctx = OcctContext::new().expect("context creation should succeed");
    let mut session = ParametricBuildSession::new("test.aicad", source, &ctx)
        .expect("initial build should succeed");

    let left_body = session.binding_named("Left.body").unwrap();
    let right_body = session.binding_named("Right.body").unwrap();

    session
        .set_param("Left.width", length_value(0.005))
        .unwrap();
    let outcome = session.rebuild().expect("edit/rebuild should succeed");

    assert_eq!(
        outcome.dirty_feature_names,
        vec!["Left.body".to_string()],
        "editing Left.width must never mark Right.body dirty, despite the identical leaf name"
    );

    let left_shape = session.shape_for_binding(left_body).unwrap();
    assert!((left_shape.volume().unwrap() - (0.005 * 0.001 * 0.001)).abs() < 1e-15);
    let right_shape = session.shape_for_binding(right_body).unwrap();
    assert!((right_shape.volume().unwrap() - (0.002 * 0.001 * 0.001)).abs() < 1e-15);
}
