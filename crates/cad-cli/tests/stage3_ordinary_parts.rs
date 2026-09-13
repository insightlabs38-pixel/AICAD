//! `AICAD-079`: Stage-3 "ordinary-part examples" end-to-end proof.
//!
//! Drives the real `cad build` pipeline (`cad_cli::build::build_source`,
//! the exact same code path `crates/cad-cli/src/main.rs` runs for a real
//! user) over three new Stage-3 example parts under `examples/`, each
//! built exclusively from the Stage-3 Safe CAD builtin catalogue
//! (`hole`/`pocket`/`mirror`/`linear_pattern`/`radial_pattern`/`shell`,
//! alongside the Stage-2 `box`/`cylinder`/`union`/`cut`/`fillet`/
//! `chamfer`) and each exposing its final solid as a named `part` output
//! (`AICAD-071`/`AICAD-079`), exported here by name (`--name`'s own
//! library entry point, `resolve_named_output`) rather than by the
//! Stage-2-only "last geometry node" default.
//!
//! Unlike `stage2_end_to_end.rs`'s own exhaustive hand-derived-volume
//! proof (already established once, for one fixture, at Stage 2), each
//! example here is checked by real exact-B-rep validity/topology evidence
//! — re-imported through an independent kernel context, `is_valid`/
//! `validate`, a sanity-checked bounding box, and a face/edge/vertex count
//! strictly above a bare box's own (6/12/8) — per `AGENTS.md`'s evidence
//! rule ("never mark geometry work complete because a render looks
//! right"), without re-deriving a full closed-form volume for every
//! example (Stage 2 already proves the pipeline can be checked that
//! exhaustively; repeating it three more times here would not add new
//! evidence about anything this task actually changed).

use std::path::{Path, PathBuf};

fn example_source(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read example at {}: {err}", path.display()))
}

/// Builds `relative` (an `examples/...` path), exporting `name` (`--name`'s
/// own `<binding>[.<field>]` argument), and asserts the build succeeds
/// with zero diagnostics and a real STEP artifact.
fn build_named(relative: &str, name: &str, output_path: &Path) {
    let source = example_source(relative);
    let _ = std::fs::remove_file(output_path);
    let report = cad_cli::build::build_source(relative, &source, Some(output_path), Some(name));
    assert_eq!(
        report.status,
        cad_cli::BuildStatus::Ok,
        "{relative} (--name {name}): {:?}",
        report.diagnostics
    );
    assert!(
        report.diagnostics.is_empty(),
        "{relative} (--name {name}): a clean example should produce no diagnostics: {:?}",
        report.diagnostics
    );
    assert_eq!(report.artifacts, vec![output_path.to_path_buf()]);
    assert!(output_path.exists());
}

/// Re-imports `step_path` through an independent kernel context and
/// asserts it is a valid, nontrivial (more topology than a bare box)
/// exact B-rep solid — never a render-only check.
fn assert_valid_nontrivial_solid(step_path: &Path) {
    let ctx = cad_occt_bridge::OcctContext::new().expect("context creation should succeed");
    let shape = ctx
        .import_step(step_path)
        .expect("the exported STEP file must re-import");
    assert!(shape.is_valid().expect("is_valid should succeed"));
    let report = shape.validate().expect("validate should succeed");
    assert!(
        report.is_valid
            && report.invalid_vertex_count == 0
            && report.invalid_edge_count == 0
            && report.invalid_wire_count == 0
            && report.invalid_face_count == 0,
        "expected a fully valid B-rep, got {report:?}"
    );
    assert!(shape.face_count().expect("face_count") > 6);
    assert!(shape.edge_count().expect("edge_count") > 12);
    assert!(shape.vertex_count().expect("vertex_count") > 8);
    let volume = shape.volume().expect("volume should succeed");
    assert!(volume > 0.0, "expected positive volume, got {volume}");
}

#[test]
fn l_bracket_body_builds_to_a_valid_solid() {
    let output_path = std::env::temp_dir().join("aicad_079_l_bracket_body.step");
    build_named(
        "examples/brackets/stage3_l_bracket.aicad",
        "LBracket.body",
        &output_path,
    );
    assert_valid_nontrivial_solid(&output_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn l_bracket_mirrored_output_also_builds_to_a_valid_solid() {
    // Proves `--name` can select *either* of a part's two `Geometry`
    // fields, not merely the one that happens to be the graph's own last
    // node.
    let output_path = std::env::temp_dir().join("aicad_079_l_bracket_mirrored.step");
    build_named(
        "examples/brackets/stage3_l_bracket.aicad",
        "LBracket.mirrored",
        &output_path,
    );
    assert_valid_nontrivial_solid(&output_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn l_bracket_body_is_ambiguous_without_a_field_name() {
    // `LBracket` exposes two `Geometry` fields (`body`, `mirrored`) plus
    // its own scalar parameters — `--name LBracket` alone must be
    // rejected as ambiguous, not silently guess one.
    let source = example_source("examples/brackets/stage3_l_bracket.aicad");
    let output_path = std::env::temp_dir().join("aicad_079_l_bracket_ambiguous.step");
    let _ = std::fs::remove_file(&output_path);
    let report = cad_cli::build::build_source(
        "stage3_l_bracket.aicad",
        &source,
        Some(&output_path),
        Some("LBracket"),
    );
    assert_eq!(report.status, cad_cli::BuildStatus::Failed);
    assert!(!output_path.exists());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code.as_string() == "EXPORT-E002"
                && d.message.contains("body")
                && d.message.contains("mirrored")),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn bearing_mount_body_builds_to_a_valid_solid() {
    let output_path = std::env::temp_dir().join("aicad_079_bearing_mount.step");
    build_named(
        "examples/plates/stage3_bearing_mount.aicad",
        "BearingMount.body",
        &output_path,
    );
    assert_valid_nontrivial_solid(&output_path);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn enclosure_body_builds_to_a_valid_solid() {
    let output_path = std::env::temp_dir().join("aicad_079_enclosure.step");
    build_named(
        "examples/enclosures/stage3_enclosure.aicad",
        "Enclosure.body",
        &output_path,
    );
    assert_valid_nontrivial_solid(&output_path);
    let _ = std::fs::remove_file(&output_path);
}

// --- Benchmark task 5 ("fix a compiler error") fixtures --------------

#[test]
fn fix_compiler_error_task_broken_fixture_fails_with_the_expected_diagnostic() {
    let source =
        example_source("project/benchmarks/stage3_core_skill/broken/simple_plate_broken.aicad");
    let report = cad_cli::build::build_source("simple_plate_broken.aicad", &source, None, None);
    assert_eq!(report.status, cad_cli::BuildStatus::Failed);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code.as_string() == "UNIT-E110"),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn fix_compiler_error_task_fixed_fixture_builds_cleanly() {
    // The reference fix's own `body` is a plain box (no other feature) —
    // `assert_valid_nontrivial_solid`'s strict "more topology than a bare
    // box" check does not apply here; a plain valid box is exactly what
    // this fixture is supposed to produce.
    let output_path = std::env::temp_dir().join("aicad_079_fix_compiler_error_fixed.step");
    build_named(
        "project/benchmarks/stage3_core_skill/broken/simple_plate_fixed.aicad",
        "SimplePlate.body",
        &output_path,
    );
    let ctx = cad_occt_bridge::OcctContext::new().expect("context creation should succeed");
    let shape = ctx
        .import_step(&output_path)
        .expect("the exported STEP file must re-import");
    assert!(shape.is_valid().expect("is_valid should succeed"));
    assert_eq!(shape.face_count().expect("face_count"), 6);
    let _ = std::fs::remove_file(&output_path);
}
