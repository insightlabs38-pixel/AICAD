//! `AICAD-118` (Checkpoint B): proves the Stage-5 multi-solution geometric-
//! query surface (`AICAD-117`) through the real production path
//! (`ParametricBuildSession`), mirroring `stage5_curves_checkpoint.rs`'s own
//! `AICAD-112` precedent exactly — an *independent* exact analytic check,
//! not merely "it built," and direct evidence that these queries never
//! touch the kernel (pure math over already-constructed `Curve`/`Surface`
//! values, exactly like curve/surface construction/evaluation before them).
//!
//! The central proof draws on curves, surfaces, and queries together (this
//! checkpoint's own acceptance line): a plane through a sphere's own center
//! gives back a great circle of the sphere's own radius
//! (`intersect_surfaces`), and a point outside the sphere projects onto it
//! (`project_point_to_surface`) at exactly the expected clearance distance —
//! both independently verifiable closed-form facts, not merely "whatever
//! this code happens to compute."

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_runtime::Value;

const SOURCE: &str = "\
let plane: Surface = plane_surface(\n\
    origin = Point3(x = 0m, y = 0m, z = 0m),\n\
    normal = Vector3(x = 0.0, y = 0.0, z = 1.0),\n\
);\n\
let globe: Surface = sphere_surface(\n\
    center = Point3(x = 0m, y = 0m, z = 0m),\n\
    radius = 5m,\n\
);\n\
let equator: List<Curve> = intersect_surfaces(plane, globe, 0.000001m);\n\
fn equator_radius_of(curves: List<Curve>) -> Length {\n\
    for c in curves {\n\
        let e: CurveEvaluation = evaluate_curve(c, 0.0);\n\
        return e.point.x;\n\
    }\n\
    return 0m - 1m;\n\
}\n\
let equator_radius: Length = equator_radius_of(equator);\n\
let probe: Point3 = Point3(x = 13m, y = 0m, z = 0m);\n\
let landing: List<SurfaceProjectionResult> = project_point_to_surface(globe, probe);\n\
fn clearance_of(results: List<SurfaceProjectionResult>) -> Length {\n\
    for r in results {\n\
        return r.distance;\n\
    }\n\
    return 0m - 1m;\n\
}\n\
let clearance: Length = clearance_of(landing);\n\
";

fn session(ctx: &OcctContext) -> ParametricBuildSession<'_> {
    ParametricBuildSession::new("test.aicad", SOURCE, ctx)
        .expect("the fixture should build cleanly through the real production path")
}

fn number_magnitude(value: &Value) -> f64 {
    match value {
        Value::Number(n) => n.magnitude,
        other => panic!("expected Value::Number, got {other:?}"),
    }
}

#[test]
fn plane_through_a_spheres_center_gives_back_the_exact_sphere_radius() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    let radius = session
        .binding_named("equator_radius")
        .and_then(|b| session.last_global(b))
        .expect("'equator_radius' is a top-level let binding");
    // Independent check: a plane through a sphere's own center always cuts
    // a great circle of exactly the sphere's own radius (5 m).
    assert!((number_magnitude(radius) - 5.0).abs() < 1e-9);
}

#[test]
fn projecting_a_point_outside_the_sphere_finds_the_exact_clearance() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    let clearance = session
        .binding_named("clearance")
        .and_then(|b| session.last_global(b))
        .expect("'clearance' is a top-level let binding");
    // Independent check: probe is at x = 13 m, sphere radius 5 m, so the
    // closest surface point is at x = 5 m and clearance = 13 - 5 = 8 m.
    assert!((number_magnitude(clearance) - 8.0).abs() < 1e-9);
}

#[test]
fn surface_surface_intersection_and_projection_never_touch_the_kernel() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    for name in [
        "plane",
        "globe",
        "equator",
        "equator_radius",
        "landing",
        "clearance",
    ] {
        let binding = session
            .binding_named(name)
            .unwrap_or_else(|| panic!("'{name}' is a top-level let binding"));
        assert!(
            session.shape_for_binding(binding).is_none(),
            "'{name}' is a Surface/Curve/List/Length value produced entirely by \
             cad_geometry_api::query — it must never have a kernel Shape, proving no OCCT call \
             happened to produce it"
        );
    }
}
