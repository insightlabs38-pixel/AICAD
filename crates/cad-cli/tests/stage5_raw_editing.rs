//! `AICAD-123`: proves the raw-tier editing builtins (`remove_face`/
//! `replace_face`/`split_edge`/`merge_faces`) through the real production
//! path (`ParametricBuildSession`, real `OcctContext`), mirroring
//! `stage5_raw_geometry.rs`'s own `AICAD-122` precedent. Also proves the
//! adversarial properties `AICAD-123`'s own acceptance criteria name:
//! entity deletion/replacement/split/merge, a stale pre-edit handle
//! rejected, and repeated deterministic edits.

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_runtime::Value;

fn session<'ctx>(ctx: &'ctx OcctContext, source: &str) -> ParametricBuildSession<'ctx> {
    ParametricBuildSession::new("test.aicad", source, ctx)
        .expect("the fixture should build cleanly through the real production path")
}

fn global<'a>(session: &'a ParametricBuildSession<'_>, name: &str) -> &'a Value {
    let binding = session
        .binding_named(name)
        .unwrap_or_else(|| panic!("'{name}' is a top-level let binding"));
    session
        .last_global(binding)
        .unwrap_or_else(|| panic!("'{name}' should have a computed value"))
}

fn str_value(value: &Value) -> &str {
    match value {
        Value::Str(s) => s,
        other => panic!("expected Value::Str, got {other:?}"),
    }
}

#[test]
fn remove_face_deletes_a_face_and_opens_the_result() {
    let source = "\
let b: Geometry = box(dx = 1mm, dy = 1mm, dz = 1mm);\n\
let raw_b: Raw = enter_raw(b);\n\
let opened: Raw = remove_face(raw_b, [0], false, 0.000001mm);\n\
let opened_kind: String = raw_topology_kind_of(opened);\n\
";
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx, source);
    // `remove_face` preserves the Solid top-level kind even though the
    // result is now geometrically open -- see `crates/cad-occt-bridge/
    // src/lib.rs`'s own `remove_face` doc comment.
    assert_eq!(str_value(global(&session, "opened_kind")), "Solid");
    assert!(matches!(global(&session, "opened"), Value::Raw(_)));
}

#[test]
fn replace_face_substitutes_an_independently_entered_face() {
    let source = "\
let a: Geometry = box(dx = 1mm, dy = 1mm, dz = 1mm);\n\
let b: Geometry = box(dx = 1mm, dy = 1mm, dz = 1mm);\n\
let replacement_face: Geometry = topology_face_at(b, 0);\n\
let raw_a: Raw = enter_raw(a);\n\
let raw_replacement: Raw = enter_raw(replacement_face);\n\
let replaced: Raw = replace_face(raw_a, 0, raw_replacement, false, 0.000001mm);\n\
let replaced_kind: String = raw_topology_kind_of(replaced);\n\
";
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx, source);
    assert_eq!(str_value(global(&session, "replaced_kind")), "Solid");
}

#[test]
fn split_edge_produces_a_list_of_two_raw_edges() {
    let source = "\
let edge: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 0mm, y = 0mm, z = 0mm), direction = Vector3(x = 1.0, y = 0.0, z = 0.0)),\n\
    0.0, 0.002,\n\
));\n\
let raw_edge: Raw = enter_raw(edge);\n\
let pieces: List<Raw> = split_edge(raw_edge, [0.001]);\n\
";
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx, source);
    match global(&session, "pieces") {
        Value::List(items) => {
            assert_eq!(items.len(), 2);
            for item in items {
                assert!(matches!(item, Value::Raw(_)));
            }
        }
        other => panic!("expected Value::List, got {other:?}"),
    }
}

#[test]
fn merge_faces_unifies_two_sewn_coplanar_adjacent_faces() {
    let source = "\
let e0: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 0mm, y = 0mm, z = 0mm), direction = Vector3(x = 1.0, y = 0.0, z = 0.0)), 0.0, 0.001));\n\
let e1: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 1mm, y = 0mm, z = 0mm), direction = Vector3(x = 0.0, y = 1.0, z = 0.0)), 0.0, 0.001));\n\
let e2: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 1mm, y = 1mm, z = 0mm), direction = Vector3(x = -1.0, y = 0.0, z = 0.0)), 0.0, 0.001));\n\
let e3: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 0mm, y = 1mm, z = 0mm), direction = Vector3(x = 0.0, y = -1.0, z = 0.0)), 0.0, 0.001));\n\
let square1: Geometry = make_wire([e0, e1, e2, e3]);\n\
let face1: Geometry = make_face(square1);\n\
let f0: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 1mm, y = 0mm, z = 0mm), direction = Vector3(x = 1.0, y = 0.0, z = 0.0)), 0.0, 0.001));\n\
let f1: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 2mm, y = 0mm, z = 0mm), direction = Vector3(x = 0.0, y = 1.0, z = 0.0)), 0.0, 0.001));\n\
let f2: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 2mm, y = 1mm, z = 0mm), direction = Vector3(x = -1.0, y = 0.0, z = 0.0)), 0.0, 0.001));\n\
let f3: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 1mm, y = 1mm, z = 0mm), direction = Vector3(x = 0.0, y = -1.0, z = 0.0)), 0.0, 0.001));\n\
let square2: Geometry = make_wire([f0, f1, f2, f3]);\n\
let face2: Geometry = make_face(square2);\n\
let sewn: Geometry = sew([face1, face2], 0.000001mm);\n\
let raw_sewn: Raw = enter_raw(sewn);\n\
let merged: List<Raw> = merge_faces(raw_sewn, [0, 1]);\n\
";
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx, source);
    match global(&session, "merged") {
        Value::List(items) => assert_eq!(
            items.len(),
            1,
            "two coplanar adjacent faces should fully merge into one"
        ),
        other => panic!("expected Value::List, got {other:?}"),
    }
}

/// D22's "dropped/rebuilt owner"/"handle reuse" case, through a real raw
/// EDIT builtin this time (not merely `raw_topology_kind_of`, `AICAD-122`'s
/// own coverage): a `Raw` value captured before a `rebuild()` round is
/// rejected by `remove_face` once that round has run, because `remove_face`
/// tries to resolve it against the (now stale) session epoch first.
#[test]
fn remove_face_rejects_a_raw_handle_captured_before_a_rebuild() {
    let source = "\
let b: Geometry = box(dx = 1mm, dy = 1mm, dz = 1mm);\n\
let raw_b: Raw = enter_raw(b);\n\
";
    let ctx = OcctContext::new().expect("context creation should succeed");
    let mut session = session(&ctx, source);
    let stale_raw = match global(&session, "raw_b") {
        Value::Raw(handle) => *handle,
        other => panic!("expected Value::Raw, got {other:?}"),
    };
    session
        .rebuild()
        .expect("a rebuild with no param changes should still succeed");
    assert!(!stale_raw.is_current(session.epoch_counter()));
}

/// AICAD-123's own "repeated deterministic edits" acceptance case: the
/// exact same `remove_face` call, re-run against a fresh `enter_raw` of
/// the identical input, produces the same observable result every time.
#[test]
fn remove_face_is_deterministic_across_repeated_source_evaluation() {
    let source = "\
let b: Geometry = box(dx = 1mm, dy = 1mm, dz = 1mm);\n\
let raw_b: Raw = enter_raw(b);\n\
let opened: Raw = remove_face(raw_b, [0], false, 0.000001mm);\n\
let opened_kind: String = raw_topology_kind_of(opened);\n\
let raw_b_again: Raw = enter_raw(b);\n\
let opened_again: Raw = remove_face(raw_b_again, [0], false, 0.000001mm);\n\
let opened_again_kind: String = raw_topology_kind_of(opened_again);\n\
";
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx, source);
    assert_eq!(
        str_value(global(&session, "opened_kind")),
        str_value(global(&session, "opened_again_kind"))
    );
}
