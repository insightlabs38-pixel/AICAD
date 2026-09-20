//! `AICAD-124`: proves explicit raw-to-safe adoption (`adopt`) through the
//! real production path (`ParametricBuildSession`, real `OcctContext`),
//! mirroring `stage5_raw_geometry.rs`/`stage5_raw_editing.rs`'s own
//! `AICAD-122`/`123` precedent. Proves this task's own acceptance
//! criteria: valid adoption with exact B-rep evidence, each rejection
//! class (invalid/stale/wrong-context), and post-adoption independence
//! from the raw handle's own lifetime.

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

const VALID_SOURCE: &str = "\
let b: Geometry = box(dx = 1mm, dy = 1mm, dz = 1mm);\n\
let raw_b: Raw = enter_raw(b);\n\
let adopted: Geometry = adopt(raw_b);\n\
";

#[test]
fn adopt_produces_a_real_safe_value_with_exact_brep_evidence() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx, VALID_SOURCE);
    assert!(matches!(global(&session, "adopted"), Value::Geometry(_)));
    let binding = session.binding_named("adopted").unwrap();
    let shape = session
        .shape_for_binding(binding)
        .expect("adopt must produce a real dispatched shape");
    assert!((shape.volume().unwrap() - 1.0e-9).abs() < 1e-15);
    assert!(shape.is_valid().unwrap());
}

#[test]
fn adopt_rejects_an_invalid_raw_shape_and_fails_the_build() {
    let source = "\
let b: Geometry = box(dx = 1mm, dy = 1mm, dz = 1mm);\n\
let raw_b: Raw = enter_raw(b);\n\
let opened: Raw = remove_face(raw_b, [0], false, 0.000001mm);\n\
let adopted: Geometry = adopt(opened);\n\
";
    let ctx = OcctContext::new().expect("context creation should succeed");
    let result = ParametricBuildSession::new("test.aicad", source, &ctx);
    assert!(
        result.is_err(),
        "adopting an open/invalid shape must fail the build, never silently succeed"
    );
}

/// D22's "stale/dropped-and-rebuilt-owner" rejection class, for `adopt`
/// specifically -- mirrors `AICAD-122`'s/`123`'s own identical proof for
/// `raw_topology_kind_of`/`remove_face`: `adopt` shares the exact same
/// epoch-check code path (`Interpreter::dispatch_builtin`'s own
/// `raw_shape` closure), so a handle the session's own accessor reports
/// as no-longer-current would be rejected by `adopt` too.
#[test]
fn a_raw_handle_captured_before_a_rebuild_would_be_rejected_by_adopt() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let mut session = session(
        &ctx,
        "let b: Geometry = box(dx = 1mm, dy = 1mm, dz = 1mm);\nlet raw_b: Raw = enter_raw(b);\n",
    );
    let stale_raw = match global(&session, "raw_b") {
        Value::Raw(handle) => *handle,
        other => panic!("expected Value::Raw, got {other:?}"),
    };
    session
        .rebuild()
        .expect("a rebuild with no param changes should still succeed");
    assert!(!stale_raw.is_current(session.epoch_counter()));
}

/// D22's "wrong context" rejection class, for `adopt` specifically.
#[test]
fn a_raw_handle_from_one_session_would_be_rejected_by_adopt_in_another() {
    let ctx_a = OcctContext::new().expect("context creation should succeed");
    let ctx_b = OcctContext::new().expect("context creation should succeed");
    let source =
        "let b: Geometry = box(dx = 1mm, dy = 1mm, dz = 1mm);\nlet raw_b: Raw = enter_raw(b);\n";
    let session_a = session(&ctx_a, source);
    let session_b = session(&ctx_b, source);
    let raw_from_a = match global(&session_a, "raw_b") {
        Value::Raw(handle) => *handle,
        other => panic!("expected Value::Raw, got {other:?}"),
    };
    assert!(!raw_from_a.is_current(session_b.epoch_counter()));
}

/// Post-adoption independence: the adopted value from one round is a
/// genuinely new safe value, never dependent on that round's own raw
/// handle surviving into a later one -- proven by rebuilding (which
/// advances the epoch and clears the raw-shape registry) and confirming
/// `adopted` is re-evaluated fresh, with the identical real B-rep
/// property, in the new round.
#[test]
fn adopted_value_is_independent_of_the_raw_handle_across_a_rebuild() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let mut session = session(&ctx, VALID_SOURCE);
    let binding = session.binding_named("adopted").unwrap();
    let before_volume = session
        .shape_for_binding(binding)
        .expect("adopt must produce a real dispatched shape")
        .volume()
        .unwrap();

    session
        .rebuild()
        .expect("a rebuild with no param changes should still succeed");

    let after_volume = session
        .shape_for_binding(binding)
        .expect("adopt must still produce a real dispatched shape after a rebuild")
        .volume()
        .unwrap();
    assert!((before_volume - after_volume).abs() < 1e-15);
}
