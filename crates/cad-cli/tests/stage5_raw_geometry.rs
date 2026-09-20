//! `AICAD-122`: proves the controlled raw/unsafe geometry tier
//! (`enter_raw`/`raw_topology_kind_of`) through the real production path
//! (`ParametricBuildSession`, real `OcctContext`), mirroring
//! `stage5_topology_inspection.rs`'s own `AICAD-121` precedent for the
//! positive path -- plus the D22 adversarial properties `AICAD-122`'s own
//! acceptance criteria name explicitly: a raw handle captured from one
//! session/round is rejected by a *different* session's own epoch (wrong
//! context) and by the *same* session's own next `rebuild` round (dropped/
//! rebuilt owner, handle reuse). Neither adversarial case can be expressed
//! in `.aicad` source itself (a program has no way to persist a value
//! across separate sessions or rebuild rounds) -- both use
//! `ParametricBuildSession::epoch_counter`'s own real, public accessor
//! directly, exactly the only way an embedding caller could misuse a raw
//! handle in practice.

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_runtime::Value;

const SOURCE: &str = "\
let b: Geometry = box(dx = 1mm, dy = 1mm, dz = 1mm);\n\
let r: Raw = enter_raw(b);\n\
let kind: String = raw_topology_kind_of(r);\n\
";

fn session(ctx: &OcctContext) -> ParametricBuildSession<'_> {
    ParametricBuildSession::new("test.aicad", SOURCE, ctx)
        .expect("the fixture should build cleanly through the real production path")
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
fn enter_raw_and_raw_topology_kind_of_round_trip_against_a_real_box() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    assert!(matches!(global(&session, "r"), Value::Raw(_)));
    assert_eq!(str_value(global(&session, "kind")), "Solid");
}

/// D22's "dropped/rebuilt owner" and "handle reuse" cases: a `Raw` value
/// captured after one build round is rejected by this exact same
/// session's own current epoch once a later `rebuild` round runs --
/// `ParametricBuildSession::rebuild` unconditionally advances its own
/// `EpochCounter` at the start of every round (including a round where no
/// parameter actually changed), matching `AICAD-093`'s/`AICAD-094`'s own
/// "connecting one `EpochCounter` per build session... calling `advance`
/// exactly on regeneration" integration.
#[test]
fn a_raw_handle_captured_before_a_rebuild_is_rejected_by_the_next_round() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let mut session = session(&ctx);
    let stale_raw = match global(&session, "r") {
        Value::Raw(handle) => *handle,
        other => panic!("expected Value::Raw, got {other:?}"),
    };
    assert!(
        stale_raw.is_current(session.epoch_counter()),
        "the handle is current immediately after its own build round"
    );

    session
        .rebuild()
        .expect("a rebuild with no param changes should still succeed");

    assert!(
        !stale_raw.is_current(session.epoch_counter()),
        "a handle captured before a rebuild must be rejected by the session's own new epoch, \
         even though nothing about the handle's own Rust value changed"
    );
    assert!(stale_raw.get(session.epoch_counter()).is_err());

    // Using the handle again after this rejection (handle reuse) still
    // fails identically -- rejection is not a one-time consumption.
    assert!(stale_raw.get(session.epoch_counter()).is_err());
}

/// D22's "wrong context" case: a `Raw` handle minted by one
/// `ParametricBuildSession` (its own real kernel context and its own real
/// `EpochCounter`) is rejected outright by a completely independent
/// session's own epoch, even though neither session's counter has ever
/// advanced.
#[test]
fn a_raw_handle_from_one_session_is_rejected_by_a_different_sessions_epoch() {
    let ctx_a = OcctContext::new().expect("context creation should succeed");
    let ctx_b = OcctContext::new().expect("context creation should succeed");
    let session_a = session(&ctx_a);
    let session_b = session(&ctx_b);

    let raw_from_a = match global(&session_a, "r") {
        Value::Raw(handle) => *handle,
        other => panic!("expected Value::Raw, got {other:?}"),
    };
    assert!(raw_from_a.is_current(session_a.epoch_counter()));
    assert!(
        !raw_from_a.is_current(session_b.epoch_counter()),
        "a handle minted by session A's own counter must never be accepted by session B's, \
         even though both are freshly built and neither has advanced"
    );
    assert!(raw_from_a.get(session_b.epoch_counter()).is_err());
}
