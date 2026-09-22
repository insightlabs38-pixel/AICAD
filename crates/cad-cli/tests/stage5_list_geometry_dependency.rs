//! `AICAD-131`: proves `List<Geometry>` builtin parameters (`compound`'s
//! own `shapes`, mirroring `make_wire`/`make_shell`/`make_solid`/`sew`'s
//! identical `list_of("Geometry")` shape) drive correct dirty-set /
//! incremental-invalidation through the real production path
//! (`ParametricBuildSession`), not merely the static `FeatureGraph`
//! introspection API `crates/cad-cli/tests/stage5_inspectability_fixture.rs`
//! already covers. Mirrors `stage3_parametric_incremental_rebuild.rs`'s own
//! "edit one param, rebuild, check exactly what recomputed vs. reused"
//! discipline.
//!
//! Before this task, `is_geometry_type`/`is_geometry_type_ref` (both
//! `cad_feature_graph::graph` and `cad_runtime::interp`) recognized only a
//! bare `Geometry`-typed parameter, so `grouped`'s own dependency on
//! `box_b` (reached through `compound`'s `List<Geometry>` parameter) was
//! invisible to `geometry_inputs` entirely -- editing `box_b_size` would
//! not have marked `grouped` dirty, and this test's own middle assertion
//! block would have failed (`grouped` absent from `dirty_feature_names`,
//! its own stale volume unchanged). See `project/OWNER_DECISIONS.md`'s
//! matching non-decision item and `project/reports/AICAD-131.md`.

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_runtime::value::{NumberValue, Value};
use cad_types::Dimension;
use cad_units::OperandType;

const SOURCE: &str = "\
param box_b_size: Length = 5mm;\n\
let box_a: Geometry = box(3mm, 3mm, 3mm);\n\
let box_b: Geometry = box(box_b_size, box_b_size, box_b_size);\n\
let box_c: Geometry = box(7mm, 7mm, 7mm);\n\
let grouped: Geometry = compound([box_a, box_b, box_c]);\n\
let unrelated: Geometry = box(9mm, 9mm, 9mm);\n\
";

fn length_value(metres: f64) -> Value {
    Value::Number(NumberValue {
        magnitude: metres,
        ty: OperandType::dimensional(Dimension::Length, None),
    })
}

fn cube_volume(edge_metres: f64) -> f64 {
    edge_metres * edge_metres * edge_metres
}

#[test]
fn editing_one_list_element_dirties_the_compound_and_reuses_its_siblings() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let mut session = ParametricBuildSession::new("test.aicad", SOURCE, &ctx)
        .expect("initial build should succeed");

    let box_a = session.binding_named("box_a").unwrap();
    let box_c = session.binding_named("box_c").unwrap();
    let grouped = session.binding_named("grouped").unwrap();
    let unrelated = session.binding_named("unrelated").unwrap();

    // Initial build: `grouped`'s own measured volume is the exact sum of
    // its three sub-solids' own volumes -- `compound` is a structural
    // grouping container only (`GeometryOp::Compound`'s own doc comment),
    // so `BRepGProp::VolumeProperties` integrates over every sub-solid.
    let initial_expected = cube_volume(0.003) + cube_volume(0.005) + cube_volume(0.007);
    let grouped_shape = session.shape_for_binding(grouped).unwrap();
    assert!((grouped_shape.volume().unwrap() - initial_expected).abs() < initial_expected * 1e-9);

    let box_a_handle_before = session.shape_for_binding(box_a).unwrap().handle();
    let box_c_handle_before = session.shape_for_binding(box_c).unwrap().handle();
    let unrelated_handle_before = session.shape_for_binding(unrelated).unwrap().handle();

    // A genuine no-op rebuild must reuse everything.
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

    // Edit `box_b_size` -- only `box_b` directly references it; `grouped`
    // reaches `box_b` only through `compound`'s own `List<Geometry>`
    // `shapes` parameter.
    session
        .set_param("box_b_size", length_value(0.010))
        .unwrap();
    let outcome = session.rebuild().expect("edit/rebuild should succeed");

    assert_eq!(
        outcome.dirty_feature_names,
        vec!["box_b".to_string(), "grouped".to_string()],
        "'box_b' (direct param reference) and 'grouped' (consumes box_b through compound's own \
         List<Geometry> parameter) must both be dirty; box_a/box_c/unrelated share no edge with \
         box_b_size at all"
    );
    assert_eq!(
        outcome.stats.recomputed.len(),
        2,
        "exactly box_b and grouped's own GeomId ranges should recompute, {:?}",
        outcome.stats.recomputed
    );
    assert_eq!(
        outcome.stats.reused.len(),
        3,
        "box_a, box_c, and unrelated should all be reused untouched, {:?}",
        outcome.stats.reused
    );

    // Correctness: `grouped`'s rebuilt volume reflects box_b's own new
    // 10mm edge length, plus the untouched box_a/box_c volumes.
    let updated_expected = cube_volume(0.003) + cube_volume(0.010) + cube_volume(0.007);
    let grouped_shape_after = session.shape_for_binding(grouped).unwrap();
    assert!(
        (grouped_shape_after.volume().unwrap() - updated_expected).abs() < updated_expected * 1e-9,
        "grouped's rebuilt volume did not reflect box_b_size's own edit"
    );

    // Strongest possible reuse evidence: box_a/box_c/unrelated's own
    // dispatched `Shape`s after this rebuild are the literal same kernel
    // handles as before -- not merely coincidentally-equal fresh rebuilds.
    let box_a_handle_after = session.shape_for_binding(box_a).unwrap().handle();
    let box_c_handle_after = session.shape_for_binding(box_c).unwrap().handle();
    let unrelated_handle_after = session.shape_for_binding(unrelated).unwrap().handle();
    assert_eq!(box_a_handle_before, box_a_handle_after);
    assert_eq!(box_c_handle_before, box_c_handle_after);
    assert_eq!(
        unrelated_handle_before, unrelated_handle_after,
        "'unrelated' shares no dependency with box_b_size or the compound at all"
    );
}
