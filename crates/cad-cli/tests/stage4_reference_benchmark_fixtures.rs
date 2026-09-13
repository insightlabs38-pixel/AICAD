//! `AICAD-079A`: frozen-corpus buildability proof for `project/benchmarks/
//! stage4_semantic_reference/`.
//!
//! This is **not** a test of any Stage-4 semantic-reference resolution
//! behavior — no resolver exists yet (`AICAD-080` onward, forbidden until a
//! separate, later owner approval). It proves the one thing this task
//! actually implements: every frozen `baseline.aicad`/`perturbed.aicad`
//! fixture pair still builds, via the real `cad build` pipeline, to a
//! valid, `is_valid`/`validate`-passing exact B-rep with the exact
//! face/edge/vertex counts and volume each case's own `case.md` records as
//! measured evidence — the same discipline `stage3_ordinary_parts.rs`
//! established for the Stage-3 example parts (`AGENTS.md`'s evidence rule:
//! never a render-only claim). A future edit that silently changes one of
//! these fixtures (defeating the corpus's own "frozen" contract, `project/
//! benchmarks/stage4_semantic_reference/README.md`'s "Freezing and change
//! control" section) will fail this suite.

use std::path::{Path, PathBuf};

fn fixture_source(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read fixture at {}: {err}", path.display()))
}

/// Builds `relative`, exporting `name`, and asserts the build succeeds with
/// zero diagnostics and a real STEP artifact.
fn build_named(relative: &str, name: &str, output_path: &Path) {
    let source = fixture_source(relative);
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
        "{relative} (--name {name}): a clean fixture should produce no diagnostics: {:?}",
        report.diagnostics
    );
    assert_eq!(report.artifacts, vec![output_path.to_path_buf()]);
    assert!(output_path.exists());
}

/// Re-imports `step_path` through an independent kernel context and asserts
/// it matches the exact topology/volume evidence recorded in the owning
/// case's own `case.md` (a tolerance of `1e-2 mm^3` on volume accommodates
/// the kernel's own curve-tessellation noise floor already documented at
/// `project/OWNER_DECISIONS.md#D19`, not measurement laxity).
fn assert_matches_recorded_evidence(
    step_path: &Path,
    expected_faces: usize,
    expected_edges: usize,
    expected_vertices: usize,
    expected_volume_mm3: f64,
) {
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
    assert_eq!(shape.face_count().expect("face_count"), expected_faces);
    assert_eq!(shape.edge_count().expect("edge_count"), expected_edges);
    assert_eq!(
        shape.vertex_count().expect("vertex_count"),
        expected_vertices
    );
    let volume_mm3 = shape.volume().expect("volume should succeed") * 1e9;
    assert!(
        (volume_mm3 - expected_volume_mm3).abs() < 1e-2,
        "expected volume ~= {expected_volume_mm3} mm^3, got {volume_mm3} mm^3"
    );
}

/// Asserts `relative` fails to build with exactly `expected_code` (used for
/// case `06`'s own deliberately-infeasible perturbation). Geometry
/// dispatch to the real kernel only happens when an `--output` path is
/// given (`cad_cli::build::build_source`'s own lazy-graph design), so this
/// must pass one even though the artifact itself is never inspected.
fn assert_build_fails_with(relative: &str, expected_code: &str) {
    let source = fixture_source(relative);
    let output_path =
        std::env::temp_dir().join("aicad_079a_case06_fillet_viability_expected_failure.step");
    let _ = std::fs::remove_file(&output_path);
    let report = cad_cli::build::build_source(relative, &source, Some(&output_path), None);
    assert_eq!(
        report.status,
        cad_cli::BuildStatus::Failed,
        "{relative}: expected a failed build, got {:?}",
        report.diagnostics
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code.as_string() == expected_code),
        "{relative}: expected a {expected_code} diagnostic, got {:?}",
        report.diagnostics
    );
}

const BENCH: &str = "project/benchmarks/stage4_semantic_reference";

macro_rules! fixture_test {
    ($test_name:ident, $dir:expr, $part_field:expr, $faces:expr, $edges:expr, $verts:expr, $volume:expr) => {
        mod $test_name {
            use super::*;

            #[test]
            fn baseline_builds_to_the_recorded_evidence() {
                let relative = format!("{BENCH}/{}/baseline.aicad", $dir);
                let output_path = std::env::temp_dir().join(concat!(
                    "aicad_079a_",
                    stringify!($test_name),
                    "_baseline.step"
                ));
                build_named(&relative, $part_field, &output_path);
                assert_matches_recorded_evidence(
                    &output_path,
                    $faces.0,
                    $edges.0,
                    $verts.0,
                    $volume.0,
                );
                let _ = std::fs::remove_file(&output_path);
            }

            #[test]
            fn perturbed_builds_to_the_recorded_evidence() {
                let relative = format!("{BENCH}/{}/perturbed.aicad", $dir);
                let output_path = std::env::temp_dir().join(concat!(
                    "aicad_079a_",
                    stringify!($test_name),
                    "_perturbed.step"
                ));
                build_named(&relative, $part_field, &output_path);
                assert_matches_recorded_evidence(
                    &output_path,
                    $faces.1,
                    $edges.1,
                    $verts.1,
                    $volume.1,
                );
                let _ = std::fs::remove_file(&output_path);
            }
        }
    };
}

// (faces, edges, vertices, volume_mm3) tuples are (baseline, perturbed),
// copied verbatim from each case's own `case.md` "Measured evidence" table.
fixture_test!(
    case01_topology_split_merge,
    "public/01_topology_split_merge",
    "Wall.body",
    (8usize, 9usize),
    (18usize, 21usize),
    (12usize, 14usize),
    (17434.51, 17457.02)
);
fixture_test!(
    case02_disappearing_entity,
    "public/02_disappearing_entity",
    "Corner.body",
    (8usize, 7usize),
    (18usize, 15usize),
    (12usize, 10usize),
    (7851.84, 5918.12)
);
fixture_test!(
    case04_pattern_count_change,
    "public/04_pattern_count_change",
    "Flange.body",
    (8usize, 9usize),
    (18usize, 21usize),
    (12usize, 14usize),
    (15205.31, 15104.78)
);
fixture_test!(
    case07_operation_reordering,
    "public/07_operation_reordering",
    "Plate.body",
    (8usize, 8usize),
    (18usize, 18usize),
    (12usize, 12usize),
    (14434.51, 14434.51)
);
fixture_test!(
    case08_upstream_suppression,
    "public/08_upstream_suppression",
    "Bracket.body",
    (8usize, 7usize),
    (18usize, 15usize),
    (12usize, 10usize),
    (7765.02, 7803.65)
);
fixture_test!(
    case09_changing_region,
    "public/09_changing_region",
    "Plate.body",
    (11usize, 11usize),
    (24usize, 24usize),
    (16usize, 16usize),
    (15808.00, 15040.00)
);
fixture_test!(
    case03_symmetric_candidates,
    "held_out/03_symmetric_candidates",
    "Plate.body",
    (8usize, 8usize),
    (18usize, 18usize),
    (12usize, 12usize),
    (14085.84, 14085.84)
);
fixture_test!(
    case05_boolean_topology_change,
    "held_out/05_boolean_topology_change",
    "Seam.body",
    (11usize, 13usize),
    (29usize, 31usize),
    (18usize, 18usize),
    (11214.60, 11214.60)
);
fixture_test!(
    case10_near_degenerate,
    "held_out/10_near_degenerate",
    "Block.body",
    (7usize, 7usize),
    (15usize, 15usize),
    (10usize, 10usize),
    (5892.70, 5571.65)
);

/// Case `06`'s own baseline is an ordinary positive build (folded into the
/// same evidence-matching shape as every other case); its perturbation is
/// a deliberate negative case (a real `GEOM-E005` kernel failure, not a
/// build to compare topology against) and is checked separately below.
#[test]
fn case06_fillet_viability_baseline_builds_to_the_recorded_evidence() {
    let relative = format!("{BENCH}/public/06_fillet_viability/baseline.aicad");
    let output_path = std::env::temp_dir().join("aicad_079a_case06_fillet_viability_baseline.step");
    build_named(&relative, "Block.body", &output_path);
    assert_matches_recorded_evidence(&output_path, 7, 15, 10, 3725.31);
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn case06_fillet_viability_perturbed_fails_with_a_real_kernel_geometry_error() {
    let relative = format!("{BENCH}/public/06_fillet_viability/perturbed.aicad");
    assert_build_fails_with(&relative, "GEOM-E005");
}
