//! `AICAD-119`: proves the general topology construction surface (vertex/
//! edge/wire/face/face-on-surface/shell/solid/compound) through the real
//! production path (`ParametricBuildSession`), mirroring
//! `stage5_queries_checkpoint.rs`'s own `AICAD-117`/`118` precedent —
//! independent exact checks against the real kernel, not merely "it
//! built," plus direct evidence for the batch's central acceptance
//! criterion: construction success is never treated as proof of a valid
//! exact B-rep. `examples/topology/topology_construction_basics.aicad` is
//! this same construction surface's registered ACTIVE example (which
//! cannot itself call `is_valid`/`area` — the plain `build_source` harness
//! `crates/cad-cli/tests/active_examples.rs` uses has no live kernel
//! query executor configured, unlike this test's real
//! `ParametricBuildSession`).

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_runtime::Value;

const SOURCE: &str = "\
let bottom: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 0mm, y = 0mm, z = 0mm), direction = Vector3(x = 1.0, y = 0.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let right: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 10mm, y = 0mm, z = 0mm), direction = Vector3(x = 0.0, y = 1.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let top: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 10mm, y = 10mm, z = 0mm), direction = Vector3(x = -1.0, y = 0.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let left: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 0mm, y = 10mm, z = 0mm), direction = Vector3(x = 0.0, y = -1.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let square: Geometry = make_wire([bottom, right, top, left]);\n\
let square_face: Geometry = make_face(square);\n\
let square_face_valid: Bool = is_valid(square_face);\n\
let square_face_area: Area = area(square_face);\n\
\n\
let bore_circle: Geometry = make_edge(circle_curve(\n\
    center = Point3(x = 0mm, y = 0mm, z = 0mm),\n\
    normal = Vector3(x = 0.0, y = 0.0, z = 1.0),\n\
    radius = 5mm,\n\
));\n\
let cylinder_face: Geometry = make_face_on_surface(\n\
    cylinder_surface(\n\
        axis = Axis3(origin = Point3(x = 0mm, y = 0mm, z = 0mm), direction = Vector3(x = 0.0, y = 0.0, z = 1.0)),\n\
        radius = 5mm,\n\
    ),\n\
    bore_circle,\n\
    [],\n\
);\n\
let cylinder_face_valid: Bool = is_valid(cylinder_face);\n\
\n\
let open_shell: Geometry = make_shell([square_face]);\n\
let open_solid: Geometry = make_solid(open_shell, []);\n\
let open_solid_valid: Bool = is_valid(open_solid);\n\
\n\
let corner: Geometry = make_vertex(Point3(x = 0mm, y = 0mm, z = 0mm));\n\
let bundle: Geometry = compound([corner, square]);\n\
let bundle_valid: Bool = is_valid(bundle);\n\
";

fn session(ctx: &OcctContext) -> ParametricBuildSession<'_> {
    ParametricBuildSession::new("test.aicad", SOURCE, ctx)
        .expect("the fixture should build cleanly through the real production path")
}

fn bool_value(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        other => panic!("expected Value::Bool, got {other:?}"),
    }
}

fn number_magnitude(value: &Value) -> f64 {
    match value {
        Value::Number(n) => n.magnitude,
        other => panic!("expected Value::Number, got {other:?}"),
    }
}

fn global<'a>(session: &'a ParametricBuildSession<'_>, name: &str) -> &'a Value {
    let binding = session
        .binding_named(name)
        .unwrap_or_else(|| panic!("'{name}' is a top-level let binding"));
    session
        .last_global(binding)
        .unwrap_or_else(|| panic!("'{name}' should have a computed value"))
}

#[test]
fn a_square_wire_bounds_a_planar_face_with_the_exact_expected_area() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    assert!(bool_value(global(&session, "square_face_valid")));
    // 10mm x 10mm square = 1e-4 m^2 in canonical units.
    let area = number_magnitude(global(&session, "square_face_area"));
    assert!((area - 1e-4).abs() < 1e-12, "area was {area}");
}

#[test]
fn a_shell_and_solid_built_from_a_single_standalone_face_are_structurally_real_but_invalid() {
    // The batch's own central acceptance criterion: `make_shell`/
    // `make_solid` never silently sew/repair a disconnected input into a
    // safe closed solid by accident. Construction itself (the `Geometry`
    // binding existing at all, proven below by
    // `construction_never_fails_even_where_validity_evidence_is_negative`)
    // succeeds; `is_valid` is the required separate evidence, and it must
    // report false here.
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    assert!(!bool_value(global(&session, "open_solid_valid")));
}

#[test]
fn a_compound_of_a_vertex_and_a_wire_is_valid() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    assert!(bool_value(global(&session, "bundle_valid")));
}

#[test]
fn construction_never_fails_even_where_validity_evidence_is_negative() {
    // Every one of these bindings computed a value at all (no runtime
    // error) -- including `cylinder_face`, whose own `is_valid` is allowed
    // to (and empirically does) come back false, since its bounding wire
    // has no matching parametrization on the cylinder (see the ACTIVE
    // example's own doc comment for why): construction success and
    // topological validity are always distinct questions in this batch,
    // never conflated by treating a failed `is_valid` as a construction
    // error.
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    for name in [
        "square_face",
        "cylinder_face",
        "open_shell",
        "open_solid",
        "corner",
        "bundle",
    ] {
        let value = global(&session, name);
        assert!(
            matches!(value, Value::Geometry(_)),
            "'{name}': expected Value::Geometry, got {value:?}"
        );
    }
    // `cylinder_face`'s own degenerate-trim evidence, disclosed rather
    // than hidden: it constructs, but is not a usable face.
    assert!(!bool_value(global(&session, "cylinder_face_valid")));
}

#[test]
fn every_construction_binding_dispatched_a_real_kernel_shape() {
    // The converse of `stage5_queries_checkpoint.rs`'s own "never touches
    // the kernel" proof for pure-value query builtins: every one of these
    // bindings is `BuiltinCategory::Construction`, so each must have a
    // real kernel `Shape` behind it, proving actual OCCT dispatch
    // happened for the vertex/edge/wire/face/shell/solid/compound
    // pipeline, not a value computed some other way.
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    for name in [
        "bottom",
        "square",
        "square_face",
        "cylinder_face",
        "open_shell",
        "open_solid",
        "corner",
        "bundle",
    ] {
        let binding = session
            .binding_named(name)
            .unwrap_or_else(|| panic!("'{name}' is a top-level let binding"));
        assert!(
            session.shape_for_binding(binding).is_some(),
            "'{name}' is a Construction-category Geometry value -- it must have a real kernel \
             Shape behind it"
        );
    }
}
