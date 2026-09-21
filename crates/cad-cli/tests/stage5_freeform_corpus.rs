//! `AICAD-127`: proves the frozen `project/benchmarks/stage5_freeform_corpus/`
//! difficult-freeform-geometry corpus through the real production path
//! (`ParametricBuildSession`), mirroring `stage4_reference_benchmark_fixtures.rs`'s
//! own "frozen corpus, real kernel, measured/independent evidence" discipline.
//! Each fixture's own `case.md` records the same evidence asserted here; a
//! future edit that silently changes a fixture's geometry will fail this
//! suite (the corpus's own freeze/change-control contract).
//!
//! Two `held_out` fixtures (`06`/`07`) exist because building the other
//! four fixtures first discovered a real capability gap: `make_edge`/
//! `make_face_on_surface` (topology construction, `AICAD-119`) only accept
//! the original Stage-2 analytic families (Line/Arc/Circle;
//! Plane/Cylinder/Cone/Sphere/Torus) -- a Bezier/B-spline curve or surface
//! (`AICAD-110`/`114`), or any `trim_surface` result (`AICAD-115`), is
//! rejected with an explicit `UNSUPPORTED_TOPOLOGY_CONSTRUCTION`
//! diagnostic, never silently. See `project/benchmarks/
//! stage5_freeform_corpus/README.md`'s "Discovered capability gap"
//! section and `project/reports/AICAD-127.md`.

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_runtime::Value;
use std::path::PathBuf;

fn fixture_source(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read fixture at {}: {err}", path.display()))
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

fn global<'a>(session: &'a ParametricBuildSession<'_>, name: &str) -> &'a Value {
    let binding = session
        .binding_named(name)
        .unwrap_or_else(|| panic!("'{name}' is a top-level let binding"));
    session
        .last_global(binding)
        .unwrap_or_else(|| panic!("'{name}' should have a computed value"))
}

fn struct_field<'a>(value: &'a Value, name: &str) -> &'a Value {
    match value {
        Value::Struct { fields, .. } => fields
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v)
            .unwrap_or_else(|| panic!("struct value has no field '{name}': {value:?}")),
        other => panic!("expected Value::Struct, got {other:?}"),
    }
}

fn point_xyz(value: &Value) -> (f64, f64, f64) {
    (
        number_magnitude(struct_field(value, "x")),
        number_magnitude(struct_field(value, "y")),
        number_magnitude(struct_field(value, "z")),
    )
}

fn eval_point(value: &Value) -> (f64, f64, f64) {
    point_xyz(struct_field(value, "point"))
}

fn close(a: (f64, f64, f64), b: (f64, f64, f64), tol: f64) -> bool {
    (a.0 - b.0).abs() < tol && (a.1 - b.1).abs() < tol && (a.2 - b.2).abs() < tol
}

fn build_ok(relative: &str) -> (OcctContext, String) {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let source = fixture_source(relative);
    (ctx, source)
}

/// A build expected to fail; returns the joined diagnostic messages.
fn build_err(relative: &str) -> String {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let source = fixture_source(relative);
    let diagnostics = match ParametricBuildSession::new("case.aicad", &source, &ctx) {
        Ok(_) => panic!("{relative}: expected this build to fail, but it succeeded"),
        Err(diagnostics) => diagnostics,
    };
    diagnostics
        .iter()
        .map(|d| d.message.as_str())
        .collect::<Vec<_>>()
        .join("; ")
}

// ---- public/01_spline_hook ----

#[test]
fn spline_hook_endpoints_and_closest_point_query_are_exact() {
    let (ctx, source) =
        build_ok("project/benchmarks/stage5_freeform_corpus/public/01_spline_hook/case.aicad");
    let session = ParametricBuildSession::new("case.aicad", &source, &ctx)
        .expect("the spline-hook fixture should build cleanly through the real production path");

    // Clamped-B-spline endpoint identity: evaluate_curve(hook, 0.0)/(hook,
    // 1.0) must exactly reproduce the first/last control point.
    let start = eval_point(global(&session, "hook_start"));
    let end = eval_point(global(&session, "hook_end"));
    assert!(close(start, (0.0, 0.0, 0.0), 1e-12));
    assert!(close(end, (0.030, 0.015, 0.0), 1e-12));

    // closest_point_on_curve: the reported distance must equal the plain
    // Euclidean distance between the reported point and the obstacle
    // (an independent recomputation of the query's own arithmetic).
    let Value::List(results) = global(&session, "clearance_points") else {
        panic!("expected Value::List");
    };
    assert!(
        !results.is_empty(),
        "closest_point_on_curve found no solution"
    );
    let obstacle = (0.050, 0.030, 0.0);
    for result in results {
        let point = point_xyz(struct_field(result, "point"));
        let reported_distance = number_magnitude(struct_field(result, "distance"));
        let recomputed = ((point.0 - obstacle.0).powi(2)
            + (point.1 - obstacle.1).powi(2)
            + (point.2 - obstacle.2).powi(2))
        .sqrt();
        assert!(
            (reported_distance - recomputed).abs() < 1e-9,
            "reported distance {reported_distance} != recomputed {recomputed}"
        );
    }
}

// ---- public/02_twisted_variable_section_solid ----

#[test]
fn twisted_variable_section_lateral_surfaces_satisfy_bilinear_identities() {
    let (ctx, source) = build_ok(
        "project/benchmarks/stage5_freeform_corpus/public/02_twisted_variable_section_solid/case.aicad",
    );
    let session = ParametricBuildSession::new("case.aicad", &source, &ctx).expect(
        "the twisted variable-section fixture should build cleanly through the real production path",
    );

    let b0 = (10.0e-3, 10.0e-3, 0.0);
    let b1 = (-10.0e-3, 10.0e-3, 0.0);
    let half1 = 10.0e-3 / std::f64::consts::SQRT_2;
    let t0 = (0.0, half1, 30.0e-3);
    let t1 = (-half1, 0.0, 30.0e-3);

    // f(0.5, 0.5) is the exact average of all four bilinear control
    // corners (b0, b1, t0, t1).
    let expected_center = (
        (b0.0 + b1.0 + t0.0 + t1.0) / 4.0,
        (b0.1 + b1.1 + t0.1 + t1.1) / 4.0,
        (b0.2 + b1.2 + t0.2 + t1.2) / 4.0,
    );
    assert!(close(
        eval_point(global(&session, "side0_center")),
        expected_center,
        1e-12
    ));
    // f(0.0, 0.5) / f(1.0, 0.5) are the exact midpoints of the
    // bottom/top square edges this patch is ruled between.
    let expected_bottom_mid = (
        (b0.0 + b1.0) / 2.0,
        (b0.1 + b1.1) / 2.0,
        (b0.2 + b1.2) / 2.0,
    );
    let expected_top_mid = (
        (t0.0 + t1.0) / 2.0,
        (t0.1 + t1.1) / 2.0,
        (t0.2 + t1.2) / 2.0,
    );
    assert!(close(
        eval_point(global(&session, "side0_bottom_mid")),
        expected_bottom_mid,
        1e-12
    ));
    assert!(close(
        eval_point(global(&session, "side0_top_mid")),
        expected_top_mid,
        1e-12
    ));

    // The planar bottom/top caps (Line-family topology, actually
    // supported today) are real, valid kernel faces with exact area:
    // rotation does not change a square's own area.
    assert!(bool_value(global(&session, "bottom_face_valid")));
    assert!(bool_value(global(&session, "top_face_valid")));
    let bottom_area = number_magnitude(global(&session, "bottom_face_area"));
    let top_area = number_magnitude(global(&session, "top_face_area"));
    assert!(
        (bottom_area - 400.0e-6).abs() < 1e-12,
        "bottom area {bottom_area}"
    );
    assert!((top_area - 100.0e-6).abs() < 1e-12, "top area {top_area}");
}

// ---- public/03_twisted_blade_surface ----

#[test]
fn twisted_blade_surface_reproduces_its_own_control_points_at_parameter_corners() {
    let (ctx, source) = build_ok(
        "project/benchmarks/stage5_freeform_corpus/public/03_twisted_blade_surface/case.aicad",
    );
    let session = ParametricBuildSession::new("case.aicad", &source, &ctx)
        .expect("the twisted-blade fixture should build cleanly through the real production path");

    let root_le = point_xyz(global(&session, "root_le"));
    let root_te = point_xyz(global(&session, "root_te"));
    let tip_le = point_xyz(global(&session, "tip_le"));
    let tip_te = point_xyz(global(&session, "tip_te"));

    assert!(close(
        eval_point(global(&session, "corner_00")),
        root_le,
        1e-9
    ));
    assert!(close(
        eval_point(global(&session, "corner_01")),
        root_te,
        1e-9
    ));
    assert!(close(
        eval_point(global(&session, "corner_10")),
        tip_le,
        1e-9
    ));
    assert!(close(
        eval_point(global(&session, "corner_11")),
        tip_te,
        1e-9
    ));

    let expected_center = (
        (root_le.0 + root_te.0 + tip_le.0 + tip_te.0) / 4.0,
        (root_le.1 + root_te.1 + tip_le.1 + tip_te.1) / 4.0,
        (root_le.2 + root_te.2 + tip_le.2 + tip_te.2) / 4.0,
    );
    assert!(close(
        eval_point(global(&session, "center")),
        expected_center,
        1e-9
    ));
}

// ---- public/04_trimmed_freeform_patch ----

#[test]
fn trimmed_freeform_patch_interior_point_matches_the_untrimmed_base_exactly() {
    let (ctx, source) = build_ok(
        "project/benchmarks/stage5_freeform_corpus/public/04_trimmed_freeform_patch/case.aicad",
    );
    let session = ParametricBuildSession::new("case.aicad", &source, &ctx)
        .expect("the trimmed-patch fixture should build cleanly through the real production path");

    // AICAD-115: `AnalyticSurface::evaluate`'s `Trimmed` arm delegates
    // directly to `base.evaluate(u, v)` once the point is confirmed
    // inside the outer loop -- so an in-domain point's coordinates must
    // be bit-identical (not merely tolerance-close) between the trimmed
    // and untrimmed evaluation.
    let tx = number_magnitude(global(&session, "trimmed_x"));
    let ty = number_magnitude(global(&session, "trimmed_y"));
    let tz = number_magnitude(global(&session, "trimmed_z"));
    let bx = number_magnitude(global(&session, "base_x"));
    let by = number_magnitude(global(&session, "base_y"));
    let bz = number_magnitude(global(&session, "base_z"));
    assert_eq!(tx, bx);
    assert_eq!(ty, by);
    assert_eq!(tz, bz);
}

// ---- held_out/05_out_of_domain_trim_rejected ----

#[test]
fn evaluating_outside_a_trim_loop_fails_the_build_explicitly_never_silently() {
    let message = build_err(
        "project/benchmarks/stage5_freeform_corpus/held_out/05_out_of_domain_trim_rejected/case.aicad",
    );
    assert!(
        message.to_lowercase().contains("domain") || message.to_lowercase().contains("evaluat"),
        "expected an explicit out-of-domain evaluation failure, got: {message:?}"
    );
}

// ---- held_out/06_near_degenerate_extreme_twist ----

#[test]
fn freeform_surface_topology_construction_is_explicitly_rejected_never_silent() {
    let message = build_err(
        "project/benchmarks/stage5_freeform_corpus/held_out/06_near_degenerate_extreme_twist/case.aicad",
    );
    assert!(
        message.contains("UNSUPPORTED") || message.to_lowercase().contains("does not yet support"),
        "expected an explicit unsupported-topology-construction rejection, got: {message:?}"
    );
}

// ---- held_out/07_spline_edge_construction_unsupported ----

#[test]
fn freeform_curve_edge_construction_is_explicitly_rejected_never_silent() {
    let message = build_err(
        "project/benchmarks/stage5_freeform_corpus/held_out/07_spline_edge_construction_unsupported/case.aicad",
    );
    assert!(
        message.contains("UNSUPPORTED") || message.to_lowercase().contains("does not yet support"),
        "expected an explicit unsupported-topology-construction rejection, got: {message:?}"
    );
}
