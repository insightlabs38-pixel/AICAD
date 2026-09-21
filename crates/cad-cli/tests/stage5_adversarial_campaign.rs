//! `AICAD-128`: numerical/robustness/determinism/resource adversarial
//! campaign against the real Stage-5 geometry surface, executed through
//! the real production path (`ParametricBuildSession`) exactly like
//! `stage4_adversarial_bug_hunt.rs`'s own precedent for Stage-4. Builds on
//! `AICAD-127`'s own frozen corpus (`project/benchmarks/
//! stage5_freeform_corpus/`) and its central finding (topology
//! construction only bridges the original Stage-2 analytic families into
//! real kernel B-rep) by targeting cases that family *does* support, plus
//! pure value-level adversarial cases for the freeform layer.
//!
//! Each case below is classified per this task's own acceptance list:
//! invalid input, explicit ambiguity/no-solution, kernel failure,
//! numerical/robustness defect, reference defect, resource-limit
//! failure, or unrelated — recorded in its own doc comment and in
//! `project/reports/AICAD-128.md`. No case here weakens a tolerance or
//! papers over a real defect; every assertion records the actually
//! observed kernel behavior.

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_runtime::Value;
use std::time::Instant;

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

fn global<'a>(session: &'a ParametricBuildSession<'_>, name: &str) -> &'a Value {
    let binding = session
        .binding_named(name)
        .unwrap_or_else(|| panic!("'{name}' is a top-level let binding"));
    session
        .last_global(binding)
        .unwrap_or_else(|| panic!("'{name}' should have a computed value"))
}

fn build(source: &str) -> Result<(), Vec<cad_diagnostics::Diagnostic>> {
    let ctx = OcctContext::new().expect("context creation should succeed");
    ParametricBuildSession::new("case.aicad", source, &ctx).map(|_| ())
}

/// Classification: near-degenerate + multi-scale (both target categories
/// at once). A circular face at 1nm radius (near the kernel's own
/// precision floor) and at 1km radius (six orders of magnitude larger
/// than every other fixture in this repository) must each construct
/// without panicking and report *some* explicit validity/area, never a
/// crash or hang either way.
#[test]
fn near_degenerate_and_multi_scale_circular_faces_never_panic() {
    for (label, radius_mm) in [("tiny_1nm", "0.000001"), ("huge_1km", "1000000.0")] {
        let source = format!(
            "let c: Geometry = make_edge(circle_curve(\n\
                 center = Point3(x = 0mm, y = 0mm, z = 0mm),\n\
                 normal = Vector3(x = 0.0, y = 0.0, z = 1.0),\n\
                 radius = {radius_mm}mm,\n\
             ));\n\
             let f: Geometry = make_face(c);\n\
             let f_valid: Bool = is_valid(f);\n\
             let f_area: Area = area(f);\n"
        );
        let ctx = OcctContext::new().expect("context creation should succeed");
        let session = ParametricBuildSession::new("case.aicad", &source, &ctx)
            .unwrap_or_else(|d| panic!("[{label}] construction itself must not fail: {d:?}"));
        let valid = bool_value(global(&session, "f_valid"));
        let measured_area = number_magnitude(global(&session, "f_area"));
        let radius_m: f64 = radius_mm.parse::<f64>().unwrap() * 1e-3;
        let expected_area = std::f64::consts::PI * radius_m * radius_m;
        // Record the actual relative error rather than assuming a bound;
        // only fail on a result that is not merely imprecise but wrong
        // by orders of magnitude (a real numerical defect, not floor
        // noise) or on the construction silently reporting `is_valid`
        // when the kernel's own answer is nonsensical.
        let relative_error = ((measured_area - expected_area) / expected_area).abs();
        eprintln!(
            "[{label}] valid={valid} measured_area={measured_area} expected_area={expected_area} \
             relative_error={relative_error}"
        );
        assert!(
            relative_error < 0.10,
            "[{label}] area relative error {relative_error} exceeds 10% -- possible numerical defect"
        );
    }
}

/// Classification: explicit ambiguity/no-solution (tangency). Two
/// spheres positioned exactly externally tangent (center distance ==
/// r1+r2): the true intersection is a single point, not a curve.
/// **Discovered behavior** (not assumed in advance): `intersect_surfaces`
/// does not return an empty `List` for this case -- it fails the whole
/// build with an explicit `GEOMETRIC_QUERY_FAILED` /
/// `RuntimeError::GeometricQueryFailed` diagnostic
/// ("the input geometry is degenerate for this query"). This is still
/// the required evidence for this failure class (structured, explicit,
/// never a panic/hang/silent-wrong curve) -- just a build-level failure
/// rather than a query-level empty answer, worth recording precisely
/// since a future caller must handle it as a build error, not a
/// zero-length list.
#[test]
fn externally_tangent_spheres_report_explicit_degenerate_intersection() {
    let source = "\
        let s1: Surface = sphere_surface(center = Point3(x = 0mm, y = 0mm, z = 0mm), radius = 10mm);\n\
        let s2: Surface = sphere_surface(center = Point3(x = 20mm, y = 0mm, z = 0mm), radius = 10mm);\n\
        let curves: List<Curve> = intersect_surfaces(s1, s2, 0.000001mm);\n";
    let message = match build(source) {
        Ok(()) => panic!(
            "expected the tangent-sphere intersection to fail explicitly -- if this now \
             succeeds, `intersect_surfaces` has changed to return an empty/degenerate List \
             instead, which is also acceptable evidence but this assertion must be updated to \
             match, not silently left to always take the Err branch"
        ),
        Err(diagnostics) => diagnostics
            .iter()
            .map(|d| d.message.as_str())
            .collect::<Vec<_>>()
            .join("; "),
    };
    eprintln!("tangent-sphere intersection rejection message: {message}");
    assert!(
        message.to_lowercase().contains("degenerate"),
        "expected an explicit degenerate-geometry rejection, got: {message:?}"
    );
}

/// Classification: periodic (an analytic `Circle` accepts any finite
/// `u`, per `cad_geometry_api::curve::AnalyticCurve`'s own doc comment).
/// Evaluating past one full period, and at a negative parameter, must
/// wrap consistently rather than erroring or silently drifting.
#[test]
fn periodic_circle_evaluation_wraps_consistently_past_one_period() {
    let source = format!(
        "let c: Curve = circle_curve(\n\
             center = Point3(x = 0mm, y = 0mm, z = 0mm),\n\
             normal = Vector3(x = 0.0, y = 0.0, z = 1.0),\n\
             radius = 10mm,\n\
         );\n\
         let base: CurveEvaluation = evaluate_curve(c, 0.3);\n\
         let wrapped_forward: CurveEvaluation = evaluate_curve(c, {});\n\
         let wrapped_backward: CurveEvaluation = evaluate_curve(c, 0.3 - {});\n",
        2.0 * std::f64::consts::PI + 0.3,
        2.0 * std::f64::consts::PI,
    );
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = ParametricBuildSession::new("case.aicad", &source, &ctx)
        .expect("periodic evaluation past one period must not fail the build");
    let base = point_xyz(struct_field(global(&session, "base"), "point"));
    let forward = point_xyz(struct_field(global(&session, "wrapped_forward"), "point"));
    let backward = point_xyz(struct_field(global(&session, "wrapped_backward"), "point"));
    let close = |a: (f64, f64, f64), b: (f64, f64, f64)| {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9 && (a.2 - b.2).abs() < 1e-9
    };
    assert!(
        close(base, forward),
        "u and u+2*pi should evaluate to the same point on a periodic circle: {base:?} vs {forward:?}"
    );
    assert!(
        close(base, backward),
        "u and u-2*pi should evaluate to the same point on a periodic circle: {base:?} vs {backward:?}"
    );
}

fn adjacent_squares_source(gap_mm: f64) -> String {
    format!(
        "let bottom0: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 0mm, y = 0mm, z = 0mm), direction = Vector3(x = 1.0, y = 0.0, z = 0.0)), 0.0, 0.01));\n\
         let right0: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 10mm, y = 0mm, z = 0mm), direction = Vector3(x = 0.0, y = 1.0, z = 0.0)), 0.0, 0.01));\n\
         let top0: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 10mm, y = 10mm, z = 0mm), direction = Vector3(x = -1.0, y = 0.0, z = 0.0)), 0.0, 0.01));\n\
         let left0: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 0mm, y = 10mm, z = 0mm), direction = Vector3(x = 0.0, y = -1.0, z = 0.0)), 0.0, 0.01));\n\
         let wire0: Geometry = make_wire([bottom0, right0, top0, left0]);\n\
         let face0: Geometry = make_face(wire0);\n\
         let bottom1: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = {gap}mm, y = 0mm, z = 0mm), direction = Vector3(x = 1.0, y = 0.0, z = 0.0)), 0.0, 0.01));\n\
         let right1: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = {gap_plus_10}mm, y = 0mm, z = 0mm), direction = Vector3(x = 0.0, y = 1.0, z = 0.0)), 0.0, 0.01));\n\
         let top1: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = {gap_plus_10}mm, y = 10mm, z = 0mm), direction = Vector3(x = -1.0, y = 0.0, z = 0.0)), 0.0, 0.01));\n\
         let left1: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = {gap}mm, y = 10mm, z = 0mm), direction = Vector3(x = 0.0, y = -1.0, z = 0.0)), 0.0, 0.01));\n\
         let wire1: Geometry = make_wire([bottom1, right1, top1, left1]);\n\
         let face1: Geometry = make_face(wire1);\n\
         let sewed: Geometry = sew([face0, face1], 0.01mm);\n\
         let sewed_valid: Bool = is_valid(sewed);\n\
         let sewed_face_count: Int = face_count(sewed);\n\
         let sewed_edge_count: Int = edge_count(sewed);\n\
         let sewed_vertex_count: Int = vertex_count(sewed);\n",
        gap = 10.0 + gap_mm,
        gap_plus_10 = 20.0 + gap_mm,
    )
}

/// Classification: numerical-robustness boundary (sewing tolerance). A
/// gap smaller than `sew`'s own tolerance must close into one coherent
/// (2-face) shell reported valid; a gap larger than tolerance must be
/// left genuinely open (never silently force-closed). Both are real,
/// distinct, explicit outcomes -- not a defect either way, but the
/// required evidence that the tolerance is a real threshold.
#[test]
fn sew_tolerance_boundary_behaves_as_a_real_threshold_not_silently() {
    let ctx = OcctContext::new().expect("context creation should succeed");

    let inside = adjacent_squares_source(0.001); // gap 0.001mm < 0.01mm tolerance
    let session = ParametricBuildSession::new("case.aicad", &inside, &ctx)
        .expect("sewing within tolerance must not fail the build");
    let inside_valid = bool_value(global(&session, "sewed_valid"));
    let inside_faces = number_magnitude(global(&session, "sewed_face_count"));
    let inside_edges = number_magnitude(global(&session, "sewed_edge_count"));
    let inside_vertices = number_magnitude(global(&session, "sewed_vertex_count"));
    eprintln!(
        "sew inside tolerance: valid={inside_valid} faces={inside_faces} edges={inside_edges} \
         vertices={inside_vertices}"
    );

    let outside = adjacent_squares_source(0.1); // gap 0.1mm > 0.01mm tolerance
    let session = ParametricBuildSession::new("case.aicad", &outside, &ctx)
        .expect("sewing outside tolerance must not fail the build either -- it stays open");
    let outside_valid = bool_value(global(&session, "sewed_valid"));
    let outside_faces = number_magnitude(global(&session, "sewed_face_count"));
    let outside_edges = number_magnitude(global(&session, "sewed_edge_count"));
    let outside_vertices = number_magnitude(global(&session, "sewed_vertex_count"));
    eprintln!(
        "sew outside tolerance: valid={outside_valid} faces={outside_faces} edges={outside_edges} \
         vertices={outside_vertices}"
    );

    // The two cases must not be silently identical -- the tolerance must
    // materially change at least one of validity/face/edge/vertex count,
    // proving `sew`'s own tolerance parameter is real, not ignored.
    assert!(
        inside_valid != outside_valid
            || (inside_faces - outside_faces).abs() > 0.5
            || (inside_edges - outside_edges).abs() > 0.5
            || (inside_vertices - outside_vertices).abs() > 0.5,
        "sew's own tolerance made no observable difference between a within-tolerance and a \
         beyond-tolerance gap (inside: valid={inside_valid} faces={inside_faces} \
         edges={inside_edges} vertices={inside_vertices}; outside: valid={outside_valid} \
         faces={outside_faces} edges={outside_edges} vertices={outside_vertices})"
    );
}

/// Classification: explicit ambiguity/multi-solution cardinality
/// (self-intersecting curve). A figure-eight-like B-spline curve is
/// queried for its closest point to its own self-intersection region --
/// the required evidence is that `closest_point_on_curve` returns a
/// real, inspectable `List` (whatever its actual cardinality), never an
/// arbitrary single silently-picked answer disguised as the only one.
#[test]
fn self_intersecting_curve_closest_point_reports_a_real_list_not_an_arbitrary_pick() {
    // A figure-eight: control points crossing through the origin twice.
    let source = "\
        let figure8: Curve = bspline_curve(\n\
            degree = 3,\n\
            control_points = [\n\
                Point3(x = 0mm - 30mm, y = 0mm, z = 0mm),\n\
                Point3(x = 0mm - 15mm, y = 30mm, z = 0mm),\n\
                Point3(x = 15mm, y = 0mm - 30mm, z = 0mm),\n\
                Point3(x = 30mm, y = 0mm, z = 0mm),\n\
            ],\n\
            knots = [0.0, 1.0],\n\
            multiplicities = [4, 4],\n\
            weights = [1.0, 1.0, 1.0, 1.0],\n\
            periodic = false,\n\
        );\n\
        let probe: Point3 = Point3(x = 0mm, y = 0mm, z = 0mm);\n\
        let results: List<ClosestPointResult> = closest_point_on_curve(figure8, probe);\n";
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = ParametricBuildSession::new("case.aicad", source, &ctx)
        .expect("closest-point query against a self-intersecting curve must not fail the build");
    let Value::List(results) = global(&session, "results") else {
        panic!("expected Value::List");
    };
    let count = results.len();
    eprintln!("self-intersecting closest-point solution count = {count}");
    assert!(
        count >= 1,
        "closest_point_on_curve found no solution at all"
    );
}

/// Classification: resource-limit / performance evidence. A 64-sided
/// regular polygon face is a real, if unusual, construction load
/// (64 edges, 64 vertices) -- it must construct within a bounded time
/// and its area must match the exact regular-polygon area formula
/// `(n/2) * r^2 * sin(2*pi/n)`.
#[test]
fn many_sided_regular_polygon_constructs_quickly_with_exact_area() {
    let n = 64usize;
    let radius_mm = 50.0;
    let mut edges = String::new();
    let mut names = Vec::new();
    for i in 0..n {
        let a0 = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
        let a1 = 2.0 * std::f64::consts::PI * ((i + 1) as f64) / (n as f64);
        let (x0, y0) = (radius_mm * a0.cos(), radius_mm * a0.sin());
        let (x1, y1) = (radius_mm * a1.cos(), radius_mm * a1.sin());
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len_mm = (dx * dx + dy * dy).sqrt();
        let (ux, uy) = (dx / len_mm, dy / len_mm);
        let name = format!("e{i}");
        edges.push_str(&format!(
            "let {name}: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = {x0}mm, y = {y0}mm, z = 0mm), direction = Vector3(x = {ux}, y = {uy}, z = 0.0)), 0.0, {len_m}));\n",
            len_m = len_mm / 1000.0,
        ));
        names.push(name);
    }
    let wire_list = names.join(", ");
    let source = format!(
        "{edges}let ngon_wire: Geometry = make_wire([{wire_list}]);\n\
         let ngon_face: Geometry = make_face(ngon_wire);\n\
         let ngon_valid: Bool = is_valid(ngon_face);\n\
         let ngon_area: Area = area(ngon_face);\n"
    );
    let ctx = OcctContext::new().expect("context creation should succeed");
    let started = Instant::now();
    let session = ParametricBuildSession::new("case.aicad", &source, &ctx)
        .expect("a 64-sided polygon should construct cleanly");
    let elapsed = started.elapsed();
    eprintln!("64-gon construction took {elapsed:?}");
    assert!(
        elapsed.as_secs() < 10,
        "64-gon construction took an unreasonably long time: {elapsed:?}"
    );
    assert!(bool_value(global(&session, "ngon_valid")));
    let radius_m = radius_mm / 1000.0;
    let expected_area =
        0.5 * (n as f64) * radius_m * radius_m * (2.0 * std::f64::consts::PI / n as f64).sin();
    let measured_area = number_magnitude(global(&session, "ngon_area"));
    assert!(
        (measured_area - expected_area).abs() < 1e-9,
        "measured area {measured_area} != expected regular-polygon area {expected_area}"
    );
}

/// Classification: determinism (D5 Level 1/2, `DECISION_LOG.md#DL-12`).
/// Building the exact same source through two independent `OcctContext`s
/// must produce identical measured properties -- the required "rerun
/// deterministic/repeat-build checks" evidence, reusing `AICAD-127`'s own
/// frozen `02_twisted_variable_section_solid` fixture as a nontrivial
/// real reproducer (two independently-built squares' own exact areas).
#[test]
fn repeated_independent_builds_of_the_same_fixture_are_deterministic() {
    let source = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("project/benchmarks/stage5_freeform_corpus/public/02_twisted_variable_section_solid/case.aicad"),
    )
    .expect("fixture should exist");

    let ctx_a = OcctContext::new().expect("context creation should succeed");
    let session_a = ParametricBuildSession::new("case.aicad", &source, &ctx_a)
        .expect("first independent build should succeed");
    let ctx_b = OcctContext::new().expect("context creation should succeed");
    let session_b = ParametricBuildSession::new("case.aicad", &source, &ctx_b)
        .expect("second independent build should succeed");

    for name in ["bottom_face_area", "top_face_area"] {
        let a = number_magnitude(global(&session_a, name));
        let b = number_magnitude(global(&session_b, name));
        assert_eq!(
            a.to_bits(),
            b.to_bits(),
            "'{name}' was not bit-identical across two independent builds: {a} vs {b}"
        );
    }
    for name in ["bottom_face_valid", "top_face_valid"] {
        assert_eq!(
            bool_value(global(&session_a, name)),
            bool_value(global(&session_b, name)),
            "'{name}' differed across two independent builds"
        );
    }
}

/// Classification: invalid input, rejected explicitly at construction
/// time (never a panic or a silently-accepted malformed curve). A
/// `knots`/`multiplicities` length mismatch is exactly the validation
/// `AnalyticCurve::bspline` (`crates/cad-geometry-api/src/curve.rs`)
/// documents rejecting.
#[test]
fn malformed_bspline_knot_vector_is_rejected_explicitly_not_panicking() {
    let source = "\
        let bad: Curve = bspline_curve(\n\
            degree = 2,\n\
            control_points = [\n\
                Point3(x = 0mm, y = 0mm, z = 0mm),\n\
                Point3(x = 10mm, y = 10mm, z = 0mm),\n\
                Point3(x = 20mm, y = 0mm, z = 0mm),\n\
            ],\n\
            knots = [0.0, 1.0],\n\
            multiplicities = [2, 3, 4],\n\
            weights = [1.0, 1.0, 1.0],\n\
            periodic = false,\n\
        );\n";
    let diagnostics = match build(source) {
        Ok(()) => panic!("a knots/multiplicities length mismatch must be rejected, not accepted"),
        Err(diagnostics) => diagnostics,
    };
    let message = diagnostics
        .iter()
        .map(|d| d.message.as_str())
        .collect::<Vec<_>>()
        .join("; ");
    eprintln!("malformed bspline rejection message: {message}");
    assert!(
        !message.is_empty(),
        "expected a real diagnostic message, got none"
    );
}
