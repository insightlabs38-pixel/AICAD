//! Maintained user-facing example baseline.
//!
//! Every ACTIVE file in `examples/README.md` is exercised through the real
//! build path. Reference fixtures additionally prove the expected
//! fail-closed health outcome, and the canonical source reference is
//! re-resolved after a real parameter edit/rebuild.

use std::path::PathBuf;

use cad_cli::{BuildStatus, ParametricBuildSession, RefsCheckStatus};
use cad_occt_bridge::OcctContext;
use cad_query::ResolutionOutcome;
use cad_runtime::value::{NumberValue, Value};
use cad_types::Dimension;
use cad_units::OperandType;

const ACTIVE: &[&str] = &[
    "examples/getting_started/simple_box.aicad",
    "examples/parametric/derived_plate.aicad",
    "examples/brackets/stage2_mounting_plate.aicad",
    "examples/brackets/stage3_l_bracket.aicad",
    "examples/plates/stage3_bearing_mount.aicad",
    "examples/enclosures/stage3_enclosure.aicad",
    "examples/references/hole_wall_reference.aicad",
    "examples/references/ambiguous_reference.aicad",
    "examples/references/broken_reference.aicad",
];

fn example_source(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read ACTIVE example {}: {err}", path.display()))
}

fn length_value(metres: f64) -> Value {
    Value::Number(NumberValue {
        magnitude: metres,
        ty: OperandType::dimensional(Dimension::Length, None),
    })
}

#[test]
fn every_active_example_builds_cleanly() {
    for relative in ACTIVE {
        let source = example_source(relative);
        let report = cad_cli::build::build_source(relative, &source, None, None);
        assert_eq!(
            report.status,
            BuildStatus::Ok,
            "{relative}: {:?}",
            report.diagnostics
        );
        assert!(
            report.diagnostics.is_empty(),
            "{relative}: {:?}",
            report.diagnostics
        );
    }
}

fn assert_health(relative: &str, expected: (usize, usize, usize)) {
    let source = example_source(relative);
    let report = cad_cli::refs_check_source(relative, &source);
    assert_eq!(
        report.status,
        RefsCheckStatus::Ok,
        "{relative}: {:?}",
        report.diagnostics
    );
    let health = report.health.expect("successful refs check has health");
    assert_eq!(
        (health.resolved, health.ambiguous, health.broken),
        expected,
        "{relative}: unexpected reference health"
    );
}

#[test]
fn reference_examples_have_their_documented_fail_closed_outcomes() {
    assert_health("examples/references/hole_wall_reference.aicad", (1, 0, 0));
    assert_health("examples/references/ambiguous_reference.aicad", (0, 1, 0));
    assert_health("examples/references/broken_reference.aicad", (0, 0, 1));
}

#[test]
fn canonical_source_reference_survives_a_real_parameter_edit_and_rebuild() {
    let relative = "examples/references/hole_wall_reference.aicad";
    let source = example_source(relative);
    let ctx = OcctContext::new().expect("context creation should succeed");
    let mut session = ParametricBuildSession::new(relative, &source, &ctx)
        .expect("ACTIVE example initial build should succeed");

    let (_, reference) = session
        .source_references()
        .first()
        .expect("canonical reference example declares one source reference");
    let reference = reference.clone();

    let before = session
        .resolve_reference(&reference)
        .expect("initial reference resolution should not error");
    assert!(
        matches!(before.outcome, ResolutionOutcome::Resolved(ref c) if c.len() == 1),
        "canonical source reference must initially resolve uniquely"
    );
    drop(before);

    session
        .set_param("hole_diameter", length_value(0.008))
        .expect("parameter edit should be accepted");
    session.rebuild().expect("parameter rebuild should succeed");

    let after = session
        .resolve_reference(&reference)
        .expect("post-rebuild reference resolution should not error");
    assert!(
        matches!(after.outcome, ResolutionOutcome::Resolved(ref c) if c.len() == 1),
        "same persistent recipe must re-resolve uniquely after the parameter edit"
    );
}
