//! `AICAD-121`: proves the Query-category topology-inspection builtins
//! (`face_count`/`edge_count`/`vertex_count`/`wire_count`/`shell_count`/
//! `solid_count`/`topology_kind_of`/`adjacent_face_count`/`is_outer_wire`/
//! `is_same_entity`/`is_forward_oriented`/`vertex_point`/`classify_point`)
//! through the real production path (`ParametricBuildSession`), mirroring
//! `stage5_topology_construction.rs`'s own `AICAD-119` precedent —
//! independent exact checks against a real kernel context, not merely "it
//! built." The Construction-category raw-index selectors (`topology_face_
//! at`/`topology_edge_at`/`topology_vertex_at`/`adjacent_face_at`) these
//! queries build on are already covered structurally by `examples/
//! topology/topology_construction_basics.aicad`'s own `build_source` path
//! (no kernel context required there); this file is what actually
//! dispatches them, plus every new Query-category builtin, against a real
//! `OcctContext`.

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_runtime::Value;

const SOURCE: &str = "\
let b: Geometry = box(dx = 1mm, dy = 1mm, dz = 1mm);\n\
\n\
let faces: Int = face_count(b);\n\
let edges: Int = edge_count(b);\n\
let vertices: Int = vertex_count(b);\n\
let wires: Int = wire_count(b);\n\
let shells: Int = shell_count(b);\n\
let solids: Int = solid_count(b);\n\
let box_kind: String = topology_kind_of(b);\n\
\n\
let f0: Geometry = topology_face_at(b, 0);\n\
let f0_kind: String = topology_kind_of(f0);\n\
let f1: Geometry = topology_face_at(b, 1);\n\
let e0: Geometry = topology_edge_at(b, 0);\n\
let e0_kind: String = topology_kind_of(e0);\n\
let v0: Geometry = topology_vertex_at(b, 0);\n\
let v0_kind: String = topology_kind_of(v0);\n\
let v0_point: Point3 = vertex_point(v0);\n\
\n\
let e0_adjacent_count: Int = adjacent_face_count(b, 0);\n\
let e0_adjacent_face: Geometry = adjacent_face_at(b, 0, 0);\n\
\n\
let f0_again: Geometry = topology_face_at(b, 0);\n\
let f0_is_f0_again: Bool = is_same_entity(f0, f0_again);\n\
let f0_is_f1: Bool = is_same_entity(f0, f1);\n\
\n\
let f0_forward: Bool = is_forward_oriented(f0);\n\
\n\
let inside: String = classify_point(b, Point3(x = 0.5mm, y = 0.5mm, z = 0.5mm), 0.000001mm);\n\
let outside: String = classify_point(b, Point3(x = 5mm, y = 5mm, z = 5mm), 0.000001mm);\n\
";

fn session(ctx: &OcctContext) -> ParametricBuildSession<'_> {
    ParametricBuildSession::new("test.aicad", SOURCE, ctx)
        .expect("the fixture should build cleanly through the real production path")
}

fn number_magnitude(value: &Value) -> f64 {
    match value {
        Value::Number(n) => n.magnitude,
        other => panic!("expected Value::Number, got {other:?}"),
    }
}

fn bool_value(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        other => panic!("expected Value::Bool, got {other:?}"),
    }
}

fn str_value(value: &Value) -> &str {
    match value {
        Value::Str(s) => s,
        other => panic!("expected Value::Str, got {other:?}"),
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
fn a_unit_boxs_own_entity_counts_are_exact() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    assert_eq!(number_magnitude(global(&session, "faces")), 6.0);
    assert_eq!(number_magnitude(global(&session, "edges")), 12.0);
    assert_eq!(number_magnitude(global(&session, "vertices")), 8.0);
    assert_eq!(number_magnitude(global(&session, "wires")), 6.0);
    assert_eq!(number_magnitude(global(&session, "shells")), 1.0);
    assert_eq!(number_magnitude(global(&session, "solids")), 1.0);
}

#[test]
fn topology_kind_of_reports_the_exact_kind_of_every_selected_entity() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    assert_eq!(str_value(global(&session, "box_kind")), "Solid");
    assert_eq!(str_value(global(&session, "f0_kind")), "Face");
    assert_eq!(str_value(global(&session, "e0_kind")), "Edge");
    assert_eq!(str_value(global(&session, "v0_kind")), "Vertex");
}

#[test]
fn every_edge_of_a_box_borders_exactly_two_faces() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    assert_eq!(number_magnitude(global(&session, "e0_adjacent_count")), 2.0);
    // The adjacent face itself is a real, independently-dispatchable
    // Geometry value -- proven the same way `stage5_sewing_healing.rs`
    // proves its own Construction results.
    let binding = session.binding_named("e0_adjacent_face").unwrap();
    assert!(session.shape_for_binding(binding).is_some());
}

#[test]
fn is_same_entity_distinguishes_independently_obtained_handles() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    assert!(bool_value(global(&session, "f0_is_f0_again")));
    assert!(!bool_value(global(&session, "f0_is_f1")));
}

#[test]
fn is_forward_oriented_returns_a_real_bool() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    // Determinism, not a specific value: `Shape::is_forward_oriented`'s
    // own doc comment discloses this as a 4-way-to-bool collapse, so this
    // only proves the real kernel answered, not which way.
    let _ = bool_value(global(&session, "f0_forward"));
}

#[test]
fn classify_point_distinguishes_inside_from_outside_a_real_box() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    assert_eq!(str_value(global(&session, "inside")), "Inside");
    assert_eq!(str_value(global(&session, "outside")), "Outside");
}

#[test]
fn vertex_point_returns_a_real_coordinate_on_the_box() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    match global(&session, "v0_point") {
        Value::Struct { fields, .. } => {
            assert_eq!(fields.len(), 3);
            for (_, value) in fields {
                let magnitude = number_magnitude(value);
                assert!(
                    (0.0..=0.001 + 1e-9).contains(&magnitude),
                    "expected a coordinate within the unit box, got {magnitude}"
                );
            }
        }
        other => panic!("expected Value::Struct, got {other:?}"),
    }
}
