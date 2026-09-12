//! `AICAD-063`: the complete Stage-2 end-to-end proof.
//!
//! Drives the real `cad build` pipeline (`cad_cli::build::build_source`
//! — the exact same code path `crates/cad-cli/src/main.rs` runs for a
//! real user, never a bypassed/alternate path) over
//! `examples/brackets/stage2_mounting_plate.aicad`, a fixture exercising:
//!
//! - **parameters** (`param ... : Length = ...mm;`) with **engineering
//!   units**;
//! - **derived expressions** (`hole_center_x`'s `margin + spacing * i`,
//!   `bracket`'s `dx - margin * 2`, `dx / 2`, ...);
//! - **functions** (`hole_center_x`/`make_through_hole`/
//!   `clamp_fillet_radius`/`rounded_plate`/`bracket`, each taking and
//!   returning typed values, `bracket` calling every other helper);
//! - **ordinary control flow** — `if` (`count <= 1`, `HOLE_COUNT > 0`),
//!   `while` (`clamp_fillet_radius`'s radius clamp), `for` (the
//!   hole-cutting loop over `0..HOLE_COUNT`), `match` (`BOSS_STYLE`), and
//!   `return`;
//! - **geometry operations** (`box`/`cylinder`/`transform`/`union`/
//!   `cut`/`fillet`/`chamfer`), every one an ordinary runtime-backed
//!   standard function call (`project/DECISION_LOG.md#DL-15`), never a
//!   compiler intrinsic or special AST node.
//!
//! This proves the full chain `project/CURRENT_STAGE.md`'s exit gate
//! names: `.aicad` source -> lexer/parser -> binding -> units/type
//! checking -> typed HIR -> execution/control flow -> Geometry IR ->
//! Stage-1 kernel API -> exact B-rep -> valid STEP. Every assertion below
//! is exact-B-rep-validity or a closed-form/analytic numeric comparison
//! against independently hand-derived expected values (`AGENTS.md`'s
//! evidence rule: "never mark geometry work complete because a render
//! looks right") — mirroring `crates/cad-occt-bridge/tests/
//! stage1_bracket.rs`'s own evidence style, but driven through the full
//! language/compiler/runtime stack instead of calling the kernel API
//! directly.
//!
//! # How the expected closed-form values were derived
//!
//! All magnitudes below are in the fixture's own millimetre units,
//! converted to this crate's SI-metre check values where noted. The
//! fixture, read literally:
//!
//! - a `plate_dx × plate_dy × plate_dz` = `80mm × 60mm × 10mm` base box;
//! - **one fillet** (radius clamped by `clamp_fillet_radius` from the
//!   `2mm` default down to `1mm`, since `2mm * 2 > plate_dz / 3 ≈
//!   3.33mm` on the first `while` iteration but `1mm * 2 = 2mm` is not on
//!   the second) on its front-top edge (length `80mm`), which **removes**
//!   `dx · r² · (1 − π/4)` of material (an outward/convex rounding);
//! - **one chamfer** (`2mm`) on its back-top edge (length `80mm`), which
//!   removes `dx · d²/2`;
//! - **one boss**: a `cylinder(8mm, 12mm)` translated to sit centered on
//!   top of the plate (`z = plate_dz`), unioned on — a flat-face contact
//!   with the plate, zero volumetric overlap, so its own closed-form
//!   cylinder volume `π·r²·h` adds exactly;
//! - **two through-holes** (`cylinder(4mm, plate_dz + 2mm)`, each
//!   overshooting the plate by `1mm` on both ends so the cut is clean),
//!   positioned by `hole_center_x` at `x = hole_margin = 15mm` and
//!   `x = plate_dx − hole_margin = 65mm` (both at `y = plate_dy/2 =
//!   30mm`) — each removes exactly `π·r²·plate_dz`, since neither
//!   overlaps the plate's own boundary, the boss (25mm center-to-center
//!   vs. an 8mm+4mm sum of radii), or each other (50mm apart vs. 8mm sum
//!   of radii).
//!
//! Net expected volume (mm³, then converted to this crate's m³ check
//! unit by `× 1e-9`):
//! `80·60·10 − 80·1²·(1 − π/4) − 80·2²/2 + π·8²·12 − 2·π·4²·10`
//! `≈ 48000 − 17.168 − 160 + 2412.743 − 1005.310 ≈ 49230.265 mm³`
//! `≈ 4.9230265e-5 m³` — independently confirmed to match the kernel's
//! own computed volume to better than 1e-4 relative error by a direct
//! (non-language) `cad-occt-bridge` reproduction of the identical
//! construction sequence during this task's own development (see
//! `project/reports/AICAD-063.md`).
//!
//! The fillet (front, `y=0`) and chamfer (back, `y=plate_dy`) are both
//! full-length (`x ∈ [0, plate_dx]`) and applied to the plate *before*
//! the off-center boss/holes are added, so the finished part is an exact
//! mirror about the `x = plate_dx/2 = 40mm` plane (both holes and the
//! boss are placed symmetrically about it) — `center_of_mass().x` must
//! equal `0.04` (metres) exactly, regardless of the fillet/chamfer/boss/
//! hole volumes' own exact values, mirroring `stage1_bracket.rs`'s own
//! "exact mirror-symmetry invariant" technique. `y`/`z` are only loosely
//! range-checked (the fillet removes less than the chamfer, and the boss
//! adds mass above the plate, so the true centroid is not at the
//! geometric center in those axes and was not worth hand-deriving
//! exactly for this task's own evidence requirements).

use std::path::PathBuf;

fn fixture_source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/brackets/stage2_mounting_plate.aicad");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read fixture at {}: {err}", path.display()))
}

/// The complete pipeline (parse -> lower -> type check -> execute top-
/// level -> dispatch Geometry IR into the real kernel -> export STEP)
/// succeeds with zero diagnostics and writes a real STEP file, then that
/// file is re-imported through an independent `OcctContext` and verified
/// against the hand-derived closed-form properties above — never a
/// render-only check.
#[test]
fn stage2_bracket_fixture_builds_to_a_valid_exact_brep_and_step() {
    let source = fixture_source();
    let output_path = std::env::temp_dir().join("aicad_063_stage2_bracket.step");
    let _ = std::fs::remove_file(&output_path);

    let report =
        cad_cli::build::build_source("stage2_mounting_plate.aicad", &source, Some(&output_path));
    assert_eq!(
        report.status,
        cad_cli::BuildStatus::Ok,
        "build should succeed with zero diagnostics: {:?}",
        report.diagnostics
    );
    assert!(
        report.diagnostics.is_empty(),
        "a clean fixture should produce no diagnostics at all: {:?}",
        report.diagnostics
    );
    assert_eq!(report.artifacts, vec![output_path.clone()]);
    assert!(output_path.exists());

    // STEP-level sanity: a syntactically valid ISO-10303-21 file
    // containing exactly one manifold solid (this part is one
    // connected solid, not several disjoint ones merely unioned in the
    // same file) and no "with voids" variant (the through-holes are
    // open to the exterior, not enclosed internal cavities).
    let step_text = std::fs::read_to_string(&output_path).unwrap();
    assert!(step_text.starts_with("ISO-10303-21;"), "{step_text}");
    assert!(step_text.contains("HEADER;"));
    assert!(step_text.contains("DATA;"));
    assert_eq!(
        step_text.matches("MANIFOLD_SOLID_BREP(").count(),
        1,
        "expected exactly one solid in the exported STEP file"
    );
    assert_eq!(
        step_text.matches("MANIFOLD_SOLID_BREP_WITH_VOIDS(").count(),
        0,
        "the through-holes are open, not enclosed voids"
    );
    assert_eq!(step_text.matches("CLOSED_SHELL(").count(), 1);

    // Re-import through a fresh, independent kernel context — proving
    // the exported file is a real, valid B-rep on disk, not merely
    // self-consistent in the exporting process's own memory.
    let ctx = cad_occt_bridge::OcctContext::new().expect("context creation should succeed");
    let shape = ctx
        .import_step(&output_path)
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

    // Closed-form volume (see module doc comment), mm^3 -> m^3.
    const MM3_TO_M3: f64 = 1e-9;
    let expected_volume = (80.0 * 60.0 * 10.0
        - 80.0 * 1.0 * 1.0 * (1.0 - std::f64::consts::PI / 4.0)
        - 80.0 * 2.0 * 2.0 / 2.0
        + std::f64::consts::PI * 8.0 * 8.0 * 12.0
        - 2.0 * std::f64::consts::PI * 4.0 * 4.0 * 10.0)
        * MM3_TO_M3;
    let volume = shape.volume().expect("volume should succeed");
    assert!(
        (volume - expected_volume).abs() < expected_volume * 1e-3,
        "volume {volume} far from expected {expected_volume}"
    );

    // Bounding box: the plate's own (0,0,0)-(80,60,0) footprint, raised
    // to a height of boss_height + plate_dz = 22mm by the boss. A loose
    // (1e-4 m) tolerance covers the fillet/chamfer curve-approximation
    // noise `stage1_bracket.rs` also documents for the identical reason.
    const BBOX_TOL: f64 = 1e-4;
    let bbox = shape.bounding_box().expect("bounding_box should succeed");
    assert!((bbox.min.x - 0.0).abs() < BBOX_TOL);
    assert!((bbox.max.x - 0.08).abs() < BBOX_TOL);
    assert!((bbox.min.y - 0.0).abs() < BBOX_TOL);
    assert!((bbox.max.y - 0.06).abs() < BBOX_TOL);
    assert!((bbox.min.z - 0.0).abs() < BBOX_TOL);
    assert!((bbox.max.z - 0.022).abs() < BBOX_TOL);

    // Center of mass: exact mirror symmetry about x = plate_dx/2 (see
    // module doc comment); y/z only range-checked.
    let com = shape
        .center_of_mass()
        .expect("center_of_mass should succeed");
    assert!(
        (com.x - 0.04).abs() < 1e-6,
        "expected exact mirror symmetry, got com.x = {}",
        com.x
    );
    assert!(com.y > 0.0 && com.y < 0.06);
    assert!(com.z > 0.0 && com.z < 0.022);

    // Nontrivial topology: strictly more faces/edges/vertices than a
    // bare box (6/12/8) — evidence this is not secretly a renamed
    // primitive — deliberately not asserted as exact counts, per
    // `AGENTS.md`'s topological-naming guidance (OCCT's own internal
    // choices about face/edge splitting during boolean fusion are not a
    // semantic contract this bridge makes).
    assert!(shape.face_count().expect("face_count") > 6);
    assert!(shape.edge_count().expect("edge_count") > 12);
    assert!(shape.vertex_count().expect("vertex_count") > 8);

    let _ = std::fs::remove_file(&output_path);
}

/// The same fixture, run twice independently end to end, must produce
/// the same semantic/numerical verification outcome both times — `D5`'s
/// own Level-1/Level-2 determinism contract (`project/DECISION_LOG.md
/// #DL-12`): not byte-identical B-rep, but the same validity and the
/// same volume to the same tolerance. Guards against nondeterminism
/// entering through iteration order, hashing, or environment-dependent
/// behavior anywhere in the pipeline this task's own fixture exercises.
#[test]
fn stage2_bracket_fixture_is_deterministic_across_independent_runs() {
    let source = fixture_source();

    let mut volumes = Vec::new();
    for i in 0..3 {
        let output_path = std::env::temp_dir().join(format!("aicad_063_determinism_{i}.step"));
        let _ = std::fs::remove_file(&output_path);
        let report = cad_cli::build::build_source(
            "stage2_mounting_plate.aicad",
            &source,
            Some(&output_path),
        );
        assert_eq!(
            report.status,
            cad_cli::BuildStatus::Ok,
            "{:?}",
            report.diagnostics
        );

        let ctx = cad_occt_bridge::OcctContext::new().expect("context creation should succeed");
        let shape = ctx
            .import_step(&output_path)
            .expect("the exported STEP file must re-import");
        assert!(shape.is_valid().expect("is_valid should succeed"));
        volumes.push(shape.volume().expect("volume should succeed"));
        let _ = std::fs::remove_file(&output_path);
    }

    for volume in &volumes[1..] {
        assert!(
            (volume - volumes[0]).abs() < volumes[0].abs() * 1e-9,
            "volumes diverged across independent runs: {volumes:?}"
        );
    }
}

/// A deliberately broken variant of the fixture (a dimensionally invalid
/// expression appended at the end: `5mm + 2kg`, mirroring
/// `crates/cad-cli/src/build.rs`'s own
/// `a_type_error_stops_the_pipeline_before_execution` test, though that
/// test's own mismatch is an *argument* mismatch, `TYPE-E418`; a bare
/// `+` between incompatible dimensions is `cad_units`'s own
/// `UNIT-E104` `DIMENSIONAL_ARITHMETIC_ERROR`, reused verbatim by
/// `cad-hir`'s type checker per that crate's established "reuse, not
/// re-derivation" pattern) must fail type checking and never reach
/// execution/dispatch/STEP export — proving the pipeline's ordinary
/// error path still holds for a fixture this much larger than
/// `cad-cli`'s own small smoke tests.
#[test]
fn a_dimensionally_invalid_variant_of_the_fixture_is_rejected_before_execution() {
    let mut source = fixture_source();
    source.push_str("\nlet broken = 5mm + 2kg;\n");
    let output_path = std::env::temp_dir().join("aicad_063_broken_variant.step");
    let _ = std::fs::remove_file(&output_path);

    let report = cad_cli::build::build_source("broken.aicad", &source, Some(&output_path));

    assert_eq!(report.status, cad_cli::BuildStatus::Failed);
    assert!(report.artifacts.is_empty());
    assert!(!output_path.exists());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code.as_string() == "UNIT-E104"),
        "{:?}",
        report.diagnostics
    );
}
