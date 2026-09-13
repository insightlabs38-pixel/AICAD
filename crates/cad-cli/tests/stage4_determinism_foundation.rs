//! AICAD-079C determinism foundation for Stage 4.
//!
//! This deliberately tests AICAD-owned canonical state and D5/D19
//! engineering equivalence. It does not compare STEP/B-rep bytes or kernel
//! face/edge enumeration order, neither of which is canonical identity.

use cad_cli::{BuildStatus, build};
use cad_validation::profile::ComparisonProfile;
use std::path::{Path, PathBuf};

const FIXTURE: &str = "examples/brackets/stage3_l_bracket.aicad";

fn source(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn build_step(output: &Path) -> cad_cli::BuildReport {
    let _ = std::fs::remove_file(output);
    build::build_source(
        FIXTURE,
        &source(FIXTURE),
        Some(output),
        Some("LBracket.body"),
    )
}

#[test]
fn canonical_success_report_is_byte_identical_for_identical_input() {
    let src = source(FIXTURE);
    let first = build::build_source(FIXTURE, &src, None, None);
    let second = build::build_source(FIXTURE, &src, None, None);
    assert_eq!(first.status, BuildStatus::Ok);
    assert_eq!(second.status, BuildStatus::Ok);
    assert_eq!(
        first.to_json().to_canonical_string(),
        second.to_json().to_canonical_string(),
        "AICAD-owned canonical build-report serialization changed for identical input"
    );
}

#[test]
fn canonical_diagnostic_order_is_byte_identical_for_identical_invalid_input() {
    let invalid = "part Broken { let body: Geometry = ; let other: Length = 1mm + ; }";
    let first = build::build_source("determinism-invalid.aicad", invalid, None, None);
    let second = build::build_source("determinism-invalid.aicad", invalid, None, None);
    assert_eq!(first.status, BuildStatus::Failed);
    assert!(!first.diagnostics.is_empty());
    assert_eq!(
        first.to_json().to_canonical_string(),
        second.to_json().to_canonical_string(),
        "diagnostic content/order changed for identical invalid input"
    );
}

#[test]
fn repeated_exact_builds_are_engineering_equivalent_under_d5_v1() {
    let first_path = std::env::temp_dir().join("aicad_079c_determinism_first.step");
    let second_path = std::env::temp_dir().join("aicad_079c_determinism_second.step");
    let first = build_step(&first_path);
    let second = build_step(&second_path);
    assert_eq!(first.status, BuildStatus::Ok, "first build: {:?}", first.diagnostics);
    assert_eq!(second.status, BuildStatus::Ok, "second build: {:?}", second.diagnostics);

    let ctx = cad_occt_bridge::OcctContext::new().expect("context creation");
    let first_shape = ctx.import_step(&first_path).expect("first STEP re-import");
    let second_shape = ctx.import_step(&second_path).expect("second STEP re-import");
    assert!(first_shape.is_valid().expect("first validity"));
    assert!(second_shape.is_valid().expect("second validity"));

    let first_volume_mm3 = first_shape.volume().expect("first volume") * 1e9;
    let second_volume_mm3 = second_shape.volume().expect("second volume") * 1e9;
    let profile = ComparisonProfile::v1();
    assert!(
        profile.volume_matches(first_volume_mm3, second_volume_mm3, 60.0),
        "repeated build volumes differ beyond D5 v1: {first_volume_mm3} vs {second_volume_mm3} mm^3"
    );

    let _ = std::fs::remove_file(&first_path);
    let _ = std::fs::remove_file(&second_path);
}
