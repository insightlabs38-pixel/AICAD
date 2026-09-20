//! `AICAD-120`: proves `sew`/`heal` through the real production path
//! (`ParametricBuildSession`), mirroring `stage5_topology_construction.rs`'s
//! own `AICAD-119` precedent — independent exact checks against the real
//! kernel, not merely "it built."
//!
//! The full `HealReport`/`SewReport` structured evidence (including the
//! `kind_changed` disclosure — an unclosable Solid silently demoted to a
//! valid-looking Shell, `cad-occt-bridge`'s own `healing_an_unclosable_
//! solid_reports_valid_after_only_alongside_kind_changed` test) is a
//! kernel-adapter-layer (`cad_occt_bridge::{SewReport,HealReport}`)
//! concern: the source-language `sew`/`heal` builtins are `BuiltinCategory::
//! Construction`, matching every other `AICAD-119`/`120` construction
//! builtin, and return only `Geometry` -- the report itself is not (yet)
//! source-exposed, exactly like `AICAD-119`'s own `ValidationReport` full
//! breakdown isn't. This file therefore checks what IS reachable from
//! source: `is_valid`/`area`/`volume` on a `sew`/`heal` result.

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_runtime::Value;

const SOURCE: &str = "\
let a0: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 0mm, y = 0mm, z = 0mm), direction = Vector3(x = 1.0, y = 0.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let a1: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 10mm, y = 0mm, z = 0mm), direction = Vector3(x = 0.0, y = 1.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let a2: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 10mm, y = 10mm, z = 0mm), direction = Vector3(x = -1.0, y = 0.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let a3: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 0mm, y = 10mm, z = 0mm), direction = Vector3(x = 0.0, y = -1.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let square_a: Geometry = make_wire([a0, a1, a2, a3]);\n\
let face_a: Geometry = make_face(square_a);\n\
\n\
let b0: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 10mm, y = 0mm, z = 0mm), direction = Vector3(x = 1.0, y = 0.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let b1: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 20mm, y = 0mm, z = 0mm), direction = Vector3(x = 0.0, y = 1.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let b2: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 20mm, y = 10mm, z = 0mm), direction = Vector3(x = -1.0, y = 0.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let b3: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 10mm, y = 10mm, z = 0mm), direction = Vector3(x = 0.0, y = -1.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let square_b: Geometry = make_wire([b0, b1, b2, b3]);\n\
let face_b: Geometry = make_face(square_b);\n\
\n\
let sewed: Geometry = sew([face_a, face_b], 0.000001mm);\n\
let sewed_valid: Bool = is_valid(sewed);\n\
let sewed_area: Area = area(sewed);\n\
\n\
let solid: Geometry = box(dx = 1mm, dy = 1mm, dz = 1mm);\n\
let healed: Geometry = heal(solid, 0.000001mm);\n\
let healed_valid: Bool = is_valid(healed);\n\
let healed_volume: Volume = volume(healed);\n\
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
fn sewing_two_edge_adjacent_squares_merges_them_into_a_valid_two_square_area() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    assert!(bool_value(global(&session, "sewed_valid")));
    // Two 10mm x 10mm squares glued along one shared edge: 2e-4 m^2.
    let area = number_magnitude(global(&session, "sewed_area"));
    assert!((area - 2e-4).abs() < 1e-12, "area was {area}");
}

#[test]
fn healing_an_already_valid_box_preserves_its_exact_volume() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    assert!(bool_value(global(&session, "healed_valid")));
    // 1mm cube = 1e-9 m^3.
    let volume = number_magnitude(global(&session, "healed_volume"));
    assert!((volume - 1e-9).abs() < 1e-15, "volume was {volume}");
}

#[test]
fn sew_and_heal_dispatch_to_a_real_kernel_shape() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    for name in ["sewed", "healed"] {
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
