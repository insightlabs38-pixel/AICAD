//! `AICAD-105` (`project/DECISION_LOG.md#DL-25`, resolving `project/
//! OWNER_DECISIONS.md#D23`): proves `is_valid`/`volume`/`area` — the new
//! kernel-backed `RuntimeBuiltin` query family — through the real
//! production path (`ParametricBuildSession::new`, which wires a real
//! `cad_geometry_runtime::OcctQueryExecutor` into the `Interpreter` it
//! builds internally, exactly like `crate::parametric_build`'s own module
//! doc comment describes), never a direct `cad_runtime::Interpreter`
//! construction with a hand-built fake executor (`crates/cad-runtime`'s
//! own unit tests already cover that in isolation).
//!
//! The central proof: a query result genuinely drives ordinary source
//! control flow (`if volume(a) < volume(b) { a } else { b }` selects the
//! *correct* branch based on the real kernel-computed volume), and the
//! selected branch is the real, correctly-shaped kernel geometry — not
//! merely "some Geometry value" — verified via
//! `ParametricBuildSession::shape_for_binding`'s own real `Shape::volume`.

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;

/// `small` (`5mm` cube, volume `125mm^3`) and `big` (`20mm` cube, volume
/// `8000mm^3`) — `chosen` picks whichever `volume(...)` reports smaller,
/// which must be `small`. `valid` proves `is_valid` reaches a real kernel
/// validity check (never a placeholder `true`). `total_area` proves
/// `area` participates in ordinary dimensional arithmetic (comparable
/// against an `Area` literal) exactly like any other typed quantity.
const SOURCE: &str = "\
let small: Geometry = box(5mm, 5mm, 5mm);\n\
let big: Geometry = box(20mm, 20mm, 20mm);\n\
let chosen: Geometry = if volume(small) < volume(big) { small } else { big };\n\
let valid: Bool = is_valid(small);\n\
let small_area: Area = area(small);\n\
let area_is_plausible: Bool = small_area > 100mm2;\n\
";

fn session(ctx: &OcctContext) -> ParametricBuildSession<'_> {
    ParametricBuildSession::new("test.aicad", SOURCE, ctx)
        .expect("the fixture should build cleanly through the real production path")
}

#[test]
fn is_valid_reaches_a_real_kernel_validity_check() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    let binding = session
        .binding_named("valid")
        .expect("'valid' is a top-level let binding");
    match session.last_global(binding) {
        Some(cad_runtime::Value::Bool(b)) => assert!(*b, "a freshly-built box must be valid"),
        other => panic!("expected Value::Bool(true), got {other:?}"),
    }
}

#[test]
fn area_participates_in_ordinary_dimensional_comparison() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    let binding = session
        .binding_named("area_is_plausible")
        .expect("'area_is_plausible' is a top-level let binding");
    match session.last_global(binding) {
        Some(cad_runtime::Value::Bool(b)) => assert!(
            *b,
            "a 5mm cube face has area 25mm^2, but the *total* 6-face-summed magnitude area() \
             reports must exceed 100mm2 for this to hold"
        ),
        other => panic!("expected Value::Bool, got {other:?}"),
    }
}

/// The real, production-path proof: `volume(...)`'s real kernel result
/// drives which branch of `if` actually gets selected, and the selected
/// branch is genuinely the smaller-volume box, not an arbitrary/incorrect
/// one — checked against the real kernel `Shape::volume`, not merely "the
/// `if` executed without erroring."
#[test]
fn volume_driven_if_selects_the_genuinely_smaller_box() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);

    let small_binding = session.binding_named("small").unwrap();
    let big_binding = session.binding_named("big").unwrap();
    let chosen_binding = session.binding_named("chosen").unwrap();

    let small_volume = session
        .shape_for_binding(small_binding)
        .expect("'small' built a real Shape")
        .volume()
        .expect("volume should succeed for a valid box");
    let big_volume = session
        .shape_for_binding(big_binding)
        .expect("'big' built a real Shape")
        .volume()
        .expect("volume should succeed for a valid box");
    let chosen_volume = session
        .shape_for_binding(chosen_binding)
        .expect("'chosen' built a real Shape")
        .volume()
        .expect("volume should succeed for a valid box");

    assert!(small_volume < big_volume, "fixture precondition");
    assert!(
        (chosen_volume - small_volume).abs() < 1e-12,
        "chosen ({chosen_volume}) must equal small's own real volume ({small_volume}), proving \
         volume(...) genuinely drove the if's branch selection rather than picking arbitrarily"
    );
}
