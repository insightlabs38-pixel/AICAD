//! Geometry-backed proof that `AICAD-075A`'s spatial-value conversion
//! boundary (`cad_runtime::spatial`) produces `cad_kernel_api` values
//! that correctly drive real, representative Stage-3 modeling
//! preparation -- revolve, circular-pattern instance placement, a
//! mirror-plane distance query, and a general frame-based transform --
//! through the real kernel adapter, all built from the *same* shared
//! `Axis3`/`Frame3`/`Plane3` representation.
//!
//! Every value under test here starts life as an ordinary evaluated
//! `.aicad` struct value (`cad_hir::geometry_types`'s `Axis3`/`Frame3`/
//! `Plane`, run through a real `cad_runtime::interp::Interpreter`), not a
//! hand-built Rust struct -- exactly the shape an `AICAD-076`/`AICAD-077`
//! `RuntimeBuiltin` dispatch arm will actually receive as an argument.
//! Every assertion is exact-B-rep/closed-form/analytic (`AGENTS.md`'s
//! evidence rule), never a render-only check, mirroring
//! `cad-occt-bridge/tests/stage1_bracket.rs`'s own evidence style.
//!
//! `AICAD-076`/`AICAD-077` implement the actual `revolve`/`mirror`/
//! `radial_pattern` Safe CAD builtins that will call
//! `cad_runtime::spatial`'s conversion functions from a real
//! `dispatch_builtin` arm; this suite proves only that the shared
//! representation those builtins will consume already works correctly
//! end to end, per this task's own "do not duplicate full AICAD-076/077
//! feature implementation" scope limit.

use cad_hir::geometry_types::with_geometry_types;
use cad_kernel_api::{Axis3, Frame3, Plane3, Point3, Transform, Vector3};
use cad_occt_bridge::OcctContext;
use cad_runtime::Value;
use cad_runtime::interp::Interpreter;
use cad_runtime::spatial::{axis3_from_value, frame3_from_value, plane3_from_value};

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
}

/// Evaluates `fn f() -> <return_ty> { return <expr>; }`, with
/// `cad_hir::geometry_types` prepended, and returns the resulting runtime
/// [`Value`] -- a genuine evaluator-produced struct value, exactly as an
/// `AICAD-076`/`AICAD-077` builtin argument would arrive at
/// `dispatch_builtin`, never a hand-built [`Value::Struct`].
fn eval_geometry_value(return_ty: &str, expr: &str) -> Value {
    let source = format!("fn f() -> {return_ty} {{ return {expr}; }}");
    let (program, parse_diagnostics) = cad_parser::parse_program(&source, "test.aicad");
    assert!(parse_diagnostics.is_empty(), "{parse_diagnostics:?}");
    let program = with_geometry_types(&program);
    let lowered = cad_hir::lower::lower_program(&program, "test.aicad", &source);
    assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
    let checked =
        cad_hir::typeck::check_program(&lowered.program, &lowered.bindings, "test.aicad", &source);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
    interp
        .call_by_name("f", vec![])
        .expect("evaluation should succeed")
}

/// Revolve preparation (`AICAD-076`): an `Axis3` evaluated from source,
/// converted through the shared boundary, drives a real
/// `Shape::revolve` whose resulting solid's volume matches the
/// closed-form cylinder formula exactly (a rectangle profile with one
/// edge lying on the revolution axis sweeps out a solid cylinder).
#[test]
fn revolve_about_an_evaluated_axis3_value_matches_the_closed_form_cylinder_volume() {
    const WIDTH: f64 = 0.006; // metres
    const HEIGHT: f64 = 0.02;

    let axis_value = eval_geometry_value(
        "Axis3",
        "Axis3(origin = Point3(x = 0mm, y = 0mm, z = 0mm), \
                direction = Vector3(x = 0.0, y = 0.0, z = 1.0))",
    );
    let axis: Axis3 = axis3_from_value(&axis_value).expect("axis should convert");

    let ctx = OcctContext::new().expect("context creation should succeed");
    let p0 = Point3::new(0.0, 0.0, 0.0);
    let p1 = Point3::new(WIDTH, 0.0, 0.0);
    let p2 = Point3::new(WIDTH, 0.0, HEIGHT);
    let p3 = Point3::new(0.0, 0.0, HEIGHT);
    let e0 = ctx.make_line_edge(p0, p1).unwrap();
    let e1 = ctx.make_line_edge(p1, p2).unwrap();
    let e2 = ctx.make_line_edge(p2, p3).unwrap();
    let e3 = ctx.make_line_edge(p3, p0).unwrap();
    let wire = ctx.make_wire_from_edges(&[&e0, &e1, &e2, &e3]).unwrap();
    let face = wire.make_face().unwrap();

    let solid = face
        .revolve(axis, std::f64::consts::TAU)
        .expect("revolve should succeed");

    let expected_volume = std::f64::consts::PI * WIDTH * WIDTH * HEIGHT;
    let volume = solid.volume().expect("volume query should succeed");
    assert!(
        close(volume, expected_volume, expected_volume * 1e-6),
        "volume {volume} did not match closed-form {expected_volume}"
    );
}

/// Circular-pattern preparation (`AICAD-077`): the same `Axis3`
/// representation revolve just used also correctly drives `count`
/// evenly-spaced rigid rotations of a real kernel shape, landing each
/// instance's center of mass at the expected closed-form angular
/// position -- proving one `Axis3` serves both consumers without either
/// inventing its own convention.
#[test]
fn circular_pattern_positions_built_from_an_evaluated_axis3_value_land_at_the_expected_angles() {
    const RADIUS: f64 = 0.03;
    const BOX_SIZE: f64 = 0.002;
    const COUNT: usize = 4;

    let axis_value = eval_geometry_value(
        "Axis3",
        "Axis3(origin = Point3(x = 0mm, y = 0mm, z = 0mm), \
                direction = Vector3(x = 0.0, y = 0.0, z = 1.0))",
    );
    let axis: Axis3 = axis3_from_value(&axis_value).expect("axis should convert");

    let ctx = OcctContext::new().expect("context creation should succeed");
    let base = ctx
        .create_box(BOX_SIZE, BOX_SIZE, BOX_SIZE)
        .expect("box creation should succeed");
    // Place the box's own centroid at (RADIUS + BOX_SIZE/2, 0, BOX_SIZE/2)
    // -- exactly on the +X ray at distance RADIUS + BOX_SIZE/2 from the
    // axis -- so each pattern instance's expected position after
    // rotating by `angle` about the shared Z axis is the plain
    // `(r * cos(angle), r * sin(angle), z)` closed form below.
    let placed = base
        .transform(&Transform::translation(Vector3::new(
            RADIUS,
            -BOX_SIZE / 2.0,
            0.0,
        )))
        .expect("placement translation should succeed");

    let r = RADIUS + BOX_SIZE / 2.0;
    for k in 0..COUNT {
        let angle = std::f64::consts::TAU * (k as f64) / (COUNT as f64);
        let instance = placed
            .transform(&Transform::rotation(axis, angle))
            .expect("pattern rotation should succeed");
        let com = instance
            .center_of_mass()
            .expect("center-of-mass query should succeed");
        let expected_x = r * angle.cos();
        let expected_y = r * angle.sin();
        assert!(
            close(com.x, expected_x, 1e-9),
            "instance {k}: x {} != expected {expected_x}",
            com.x
        );
        assert!(
            close(com.y, expected_y, 1e-9),
            "instance {k}: y {} != expected {expected_y}",
            com.y
        );
        assert!(close(com.z, BOX_SIZE / 2.0, 1e-9));
    }
}

/// Mirror-plane preparation (`AICAD-077`): a `Plane` evaluated from
/// source, converted through the shared boundary into
/// [`cad_kernel_api::Plane3`], reports the correct closed-form signed
/// distance to a real kernel shape's own kernel-computed center of mass
/// -- the primitive a future mirror operation reflects a point with,
/// proven against real geometry rather than only against
/// `cad-kernel-api`'s own pure-math unit tests.
#[test]
fn mirror_plane_from_an_evaluated_plane_value_reports_the_correct_signed_distance_to_a_real_shapes_center_of_mass()
 {
    const OFFSET: f64 = 0.005;
    const BOX_SIZE: f64 = 0.002;

    let plane_value = eval_geometry_value(
        "Plane",
        "Plane(origin = Point3(x = 0mm, y = 0mm, z = 0mm), \
               normal = Vector3(x = 1.0, y = 0.0, z = 0.0))",
    );
    let plane: Plane3 = plane3_from_value(&plane_value).expect("plane should convert");

    let ctx = OcctContext::new().expect("context creation should succeed");
    let base = ctx
        .create_box(BOX_SIZE, BOX_SIZE, BOX_SIZE)
        .expect("box creation should succeed");
    let placed = base
        .transform(&Transform::translation(Vector3::new(OFFSET, 0.0, 0.0)))
        .expect("placement translation should succeed");
    let com = placed
        .center_of_mass()
        .expect("center-of-mass query should succeed");

    let expected_distance = OFFSET + BOX_SIZE / 2.0;
    let distance = plane.signed_distance(com);
    assert!(
        close(distance, expected_distance, 1e-9),
        "signed distance {distance} != expected {expected_distance}"
    );
}

/// Transformed-geometry preparation (general non-translation
/// transforms): a `Frame3` evaluated from source, converted through the
/// shared boundary, produces (via [`Transform::from_frames`]) a rigid
/// motion that moves a real kernel box to exactly the closed-form
/// expected bounding box -- proving the same `Frame3` representation
/// `AICAD-070` declared passively now has real, usable rotation
/// semantics behind it.
#[test]
fn frame_based_transform_from_an_evaluated_frame3_value_moves_a_real_kernel_box_to_the_expected_place()
 {
    const DX: f64 = 0.01;
    const DY: f64 = 0.006;
    const DZ: f64 = 0.003;

    // A frame whose x/y axes are world y/-x -- a 90-degree right-hand
    // rotation about world +Z, per this crate's own fixed convention
    // (`cad_kernel_api::geometry`'s own module doc comment).
    let frame_value = eval_geometry_value(
        "Frame3",
        "Frame3(origin = Point3(x = 20mm, y = 30mm, z = 0mm), \
                x_axis = Vector3(x = 0.0, y = 1.0, z = 0.0), \
                y_axis = Vector3(x = -1.0, y = 0.0, z = 0.0), \
                z_axis = Vector3(x = 0.0, y = 0.0, z = 1.0))",
    );
    let frame: Frame3 = frame3_from_value(&frame_value).expect("frame should convert");
    let transform = Transform::from_frames(Frame3::WORLD, frame);

    let ctx = OcctContext::new().expect("context creation should succeed");
    let base = ctx
        .create_box(DX, DY, DZ)
        .expect("box creation should succeed");
    let moved = base
        .transform(&transform)
        .expect("frame-based transform should succeed");

    let bbox = moved
        .bounding_box()
        .expect("bounding-box query should succeed");
    // `Transform::from_frames(WORLD, frame)` maps WORLD's +X onto
    // `frame.x` (world +Y) and +Y onto `frame.y` (world -X), so the box's
    // own local [0,DX]x[0,DY]x[0,DZ] extent becomes
    // [-DY,0]x[0,DX]x[0,DZ] before `frame.origin`'s translation is added.
    // `1e-6` (not a tighter tolerance) because a kernel bounding-box query
    // carries its own numerical noise floor at roughly this magnitude,
    // per `project/OWNER_DECISIONS.md#D19`'s own Stage-1 evidence.
    const TOL: f64 = 1e-6;
    assert!(close(bbox.min.x, frame.origin.x - DY, TOL));
    assert!(close(bbox.max.x, frame.origin.x, TOL));
    assert!(close(bbox.min.y, frame.origin.y, TOL));
    assert!(close(bbox.max.y, frame.origin.y + DX, TOL));
    assert!(close(bbox.min.z, frame.origin.z, TOL));
    assert!(close(bbox.max.z, frame.origin.z + DZ, TOL));
}
