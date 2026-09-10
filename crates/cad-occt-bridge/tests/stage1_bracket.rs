//! AICAD-034/AICAD-035: the Stage-1 proof.
//!
//! Builds a meaningfully nontrivial bracket-like part -- an L-shaped
//! mounting bracket combining a base flange, a perpendicular upright
//! wall (booleans + genuine 3D solid overlap, not merely a coincident
//! interface), four mounting through-holes (booleans against a
//! rigid-transformed cylinder, including one rotated onto a different
//! axis), a filleted interior root edge, and a chamfered exterior edge
//! -- directly through the Rust/native kernel API
//! (`cad_kernel_api`/`cad_occt_bridge`, no OCCT type ever named here).
//!
//! Verification is exact/analytic (B-rep validity, a closed-form
//! expected volume, an exact bounding box, an exact mirror-symmetry
//! invariant on the center of mass), never render-only, per AGENTS.md's
//! geometry correctness standard. The bracket is also exported to STEP
//! and, in a companion test, re-imported through this same bridge as one
//! (declared non-independent) layer of AICAD-035's STEP verification.
//!
//! See `project/reports/AICAD-034.md` and `project/reports/AICAD-035.md`
//! for the full geometry recipe, analytic derivations, and the
//! genuinely independent (non-OCCT) verification evidence gathered
//! alongside this test suite.

use cad_kernel_api::{Axis3, Direction3, KernelResult, Point3, Transform, Vector3};
use cad_occt_bridge::{OcctContext, Shape};

const BASE_DX: f64 = 80.0;
const BASE_DY: f64 = 60.0;
const BASE_DZ: f64 = 10.0;
const WALL_DX: f64 = 80.0;
const WALL_DY: f64 = 10.0;
const WALL_DZ: f64 = 60.0;

const HOLE_RADIUS: f64 = 4.0;
// The through-cut cylinders overshoot the plate/wall thickness by this
// much on each end so the cut is a clean full through-cut with no
// coincident end faces -- the overshoot lies entirely outside the solid
// on both sides, so it does not change the exact removed volume.
const HOLE_MARGIN: f64 = 1.0;

const FILLET_RADIUS: f64 = 4.0;
const CHAMFER_DISTANCE: f64 = 2.0;

// (x, y): two through holes in the base flange, drilled along +Z.
const BASE_HOLE_CENTERS: [(f64, f64); 2] = [(15.0, 45.0), (65.0, 45.0)];
// (x, z): two through holes in the upright wall, drilled along +Y.
const WALL_HOLE_CENTERS: [(f64, f64); 2] = [(20.0, 35.0), (60.0, 35.0)];

fn close(a: f64, b: f64) -> bool {
    close_tol(a, b, 1e-6)
}

fn close_tol(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
}

/// Builds the Stage-1 proof bracket described in the module docs.
///
/// Geometry recipe (see `project/reports/AICAD-034.md` for the full
/// analytic derivation this test's own assertions are checked against):
/// 1. A base flange (`BASE_DX`x`BASE_DY`x`BASE_DZ`) and an upright wall
///    (`WALL_DX`x`WALL_DY`x`WALL_DZ`), both built at the same origin
///    corner so the wall genuinely, volumetrically overlaps the base
///    over `x in [0,80], y in [0,10], z in [0,10]` (a true 3D boolean
///    overlap, not a knife-edge coincident-face case) -- union is an
///    L-shaped solid.
/// 2. Four through holes, cut via boolean subtraction of a
///    rigid-transformed cylinder: two straight down through the base
///    flange, two rotated 90 degrees (mapping the cylinder's default +Z
///    axis onto +Y) and translated through the wall's thickness.
/// 3. A fillet on the interior (concave/reentrant) root edge where the
///    wall meets the base, found by its geometric bounding box (never by
///    raw enumeration index, per AGENTS.md's topology-naming guidance).
/// 4. A chamfer on an exterior (convex) top edge of the wall, found the
///    same way.
fn build_bracket(context: &OcctContext) -> KernelResult<Shape<'_>> {
    let base = context.create_box(BASE_DX, BASE_DY, BASE_DZ)?;
    let wall = context.create_box(WALL_DX, WALL_DY, WALL_DZ)?;
    let mut drilled = base.union(&wall)?;

    for &(x, y) in &BASE_HOLE_CENTERS {
        let raw = context.create_cylinder(HOLE_RADIUS, BASE_DZ + 2.0 * HOLE_MARGIN)?;
        let hole = raw.transform(&Transform::translation(Vector3::new(x, y, -HOLE_MARGIN)))?;
        drilled = drilled.cut(&hole)?;
    }

    // Rotating -90 degrees about world +X maps +Z onto +Y (Rodrigues:
    // (x,y,z) -> (x,z,-y) for this specific angle/axis), turning the
    // cylinder's default axis-along-+Z into axis-along-+Y so it can drill
    // through the wall's Y-thickness instead of the base's Z-thickness.
    let rotate_z_to_y = Transform::rotation(
        Axis3::new(Point3::ORIGIN, Direction3::X),
        -std::f64::consts::FRAC_PI_2,
    );
    for &(x, z) in &WALL_HOLE_CENTERS {
        let raw = context.create_cylinder(HOLE_RADIUS, WALL_DY + 2.0 * HOLE_MARGIN)?;
        let place =
            rotate_z_to_y.compose(&Transform::translation(Vector3::new(x, -HOLE_MARGIN, z)));
        let hole = raw.transform(&place)?;
        drilled = drilled.cut(&hole)?;
    }

    // The interior root edge: where the wall's back face (y = WALL_DY)
    // meets the base's top face (z = BASE_DZ), spanning the full X
    // extent -- a straight edge with a degenerate (single-point) Y/Z
    // bounding-box extent and the full X span.
    let root_edge = find_edge(&drilled, |b| {
        close(b.min.y, WALL_DY)
            && close(b.max.y, WALL_DY)
            && close(b.min.z, BASE_DZ)
            && close(b.max.z, BASE_DZ)
            && close(b.min.x, 0.0)
            && close(b.max.x, BASE_DX)
    })?
    .expect("the interior root edge must be found among the drilled bracket's edges");
    let filleted = drilled.fillet(&[&root_edge], FILLET_RADIUS)?;
    drop(root_edge);

    // The exterior top-front edge of the wall: z = WALL_DZ, y = 0,
    // spanning the full X extent.
    let top_edge = find_edge(&filleted, |b| {
        close(b.min.y, 0.0)
            && close(b.max.y, 0.0)
            && close(b.min.z, WALL_DZ)
            && close(b.max.z, WALL_DZ)
            && close(b.min.x, 0.0)
            && close(b.max.x, BASE_DX)
    })?
    .expect("the exterior top-front edge must be found among the filleted bracket's edges");
    let chamfered = filleted.chamfer(&[&top_edge], CHAMFER_DISTANCE)?;
    drop(top_edge);
    Ok(chamfered)
}

/// Finds the unique edge of `shape` whose bounding box satisfies
/// `predicate`, using `Shape::get_edge`'s own current raw enumeration
/// order only as an iteration mechanism -- never as identity. This is
/// the same geometric-selection pattern already established by
/// `cad-occt-bridge`'s own `chamfer_single_edge_matches_analytic_volume`
/// test (AICAD-027), applied here to a shape with holes/booleans instead
/// of a bare box.
fn find_edge<'ctx>(
    shape: &Shape<'ctx>,
    predicate: impl Fn(cad_occt_bridge::BoundingBox) -> bool,
) -> KernelResult<Option<Shape<'ctx>>> {
    let count = shape.edge_count()?;
    for i in 0..count {
        let edge = shape.get_edge(i)?;
        if predicate(edge.bounding_box()?) {
            return Ok(Some(edge));
        }
    }
    Ok(None)
}

/// Analytic expected volume, derived independently of the kernel
/// (`project/reports/AICAD-034.md` §"Analytic derivation" walks through
/// each term):
///
/// - base + wall union: `BASE_DX*BASE_DY*BASE_DZ + WALL_DX*WALL_DY*WALL_DZ -
///   overlap`, where `overlap = BASE_DX * WALL_DY * BASE_DZ` (the genuine
///   3D region both boxes occupy);
/// - minus 4 clean cylindrical through-holes, each removing exactly
///   `pi * HOLE_RADIUS^2 * thickness` (thickness = `BASE_DZ` for the base
///   holes, `WALL_DY` for the wall holes -- both happen to equal 10 here);
/// - plus the interior fillet, which on a concave/reentrant right-angle
///   edge *adds* material (a quarter-circle fills the notch): area added
///   per unit length = `r^2 * (1 - pi/4)`, times the edge's full
///   `BASE_DX` length;
/// - minus the exterior chamfer, which on a convex right-angle edge
///   *removes* a triangular prism: area removed per unit length =
///   `d^2/2`, times the edge's full `BASE_DX` length.
fn expected_volume() -> f64 {
    let pi = std::f64::consts::PI;
    let base_volume = BASE_DX * BASE_DY * BASE_DZ;
    let wall_volume = WALL_DX * WALL_DY * WALL_DZ;
    let overlap_volume = BASE_DX * WALL_DY * BASE_DZ;
    let union_volume = base_volume + wall_volume - overlap_volume;

    let holes_volume = 4.0 * pi * HOLE_RADIUS * HOLE_RADIUS * BASE_DZ;

    let fillet_added = BASE_DX * FILLET_RADIUS * FILLET_RADIUS * (1.0 - pi / 4.0);
    let chamfer_removed = BASE_DX * (CHAMFER_DISTANCE * CHAMFER_DISTANCE / 2.0);

    union_volume - holes_volume + fillet_added - chamfer_removed
}

#[test]
fn bracket_is_a_valid_exact_brep_matching_analytic_properties() {
    let context = OcctContext::new().expect("context creation should succeed");
    let bracket = build_bracket(&context).expect("the Stage-1 proof bracket should build");

    assert!(
        bracket.is_valid().unwrap(),
        "the bracket must be a valid B-rep, not merely constructible"
    );
    let report = bracket.validate().unwrap();
    assert!(
        report.is_valid,
        "the normalized validation report must agree"
    );
    assert_eq!(report.invalid_vertex_count, 0);
    assert_eq!(report.invalid_edge_count, 0);
    assert_eq!(report.invalid_wire_count, 0);
    assert_eq!(report.invalid_face_count, 0);

    let volume = bracket.volume().unwrap();
    let expected = expected_volume();
    assert!(
        (volume - expected).abs() < expected * 1e-3,
        "bracket volume {volume} did not match analytic expectation {expected} within 0.1%"
    );

    // Bounding box: the base sets x/y extent, the wall (taller than the
    // base) sets the z extent; holes/fillet/chamfer are all strictly
    // interior perturbations that never touch the outer envelope. A
    // looser tolerance than `close`'s 1e-6 is used here specifically:
    // OCCT's fillet/chamfer builders introduce curve-approximation
    // imprecision on the order of 1e-6 into the overall bounding box
    // (confirmed empirically -- the pre-fillet/chamfer `drilled` shape's
    // own bounding box matches to ~1.5e-7), so a strict 1e-6 comparison
    // is right at that noise floor.
    const BBOX_TOL: f64 = 1e-4;
    let bbox = bracket.bounding_box().unwrap();
    assert!(close_tol(bbox.min.x, 0.0, BBOX_TOL) && close_tol(bbox.max.x, BASE_DX, BBOX_TOL));
    assert!(close_tol(bbox.min.y, 0.0, BBOX_TOL) && close_tol(bbox.max.y, BASE_DY, BBOX_TOL));
    assert!(close_tol(bbox.min.z, 0.0, BBOX_TOL) && close_tol(bbox.max.z, WALL_DZ, BBOX_TOL));

    // Exact symmetry invariant: every feature (both hole pairs, the
    // full-length fillet, the full-length chamfer) is placed
    // mirror-symmetrically about the x = BASE_DX/2 plane, so the exact
    // center of mass must lie exactly on that plane regardless of the
    // fillet/chamfer/hole volumes' own exact values.
    let com = bracket.center_of_mass().unwrap();
    assert!(
        close(com.x, BASE_DX / 2.0),
        "center of mass x={} must sit exactly on the bracket's own mirror-symmetry plane at x={}",
        com.x,
        BASE_DX / 2.0
    );
    // Loose sanity range for y/z (not a claimed exact value -- the
    // fillet/chamfer/holes perturb it slightly from the pre-feature
    // L-cross-section centroid of ~(18.6, 18.6) computed in
    // project/reports/AICAD-034.md).
    assert!(
        com.y > 10.0 && com.y < 30.0,
        "center of mass y={} out of range",
        com.y
    );
    assert!(
        com.z > 10.0 && com.z < 30.0,
        "center of mass z={} out of range",
        com.z
    );

    // Nontrivial topology: strictly more faces/edges/vertices than a bare
    // box (6/12/8) -- not asserted as exact counts (fragile enumeration
    // ordering / OCCT-internal splitting choices), only as evidence this
    // is not secretly a renamed primitive.
    assert!(bracket.face_count().unwrap() > 6);
    assert!(bracket.edge_count().unwrap() > 12);
    assert!(bracket.vertex_count().unwrap() > 8);
}

#[test]
fn bracket_exports_to_a_syntactically_valid_step_file() {
    let context = OcctContext::new().expect("context creation should succeed");
    let bracket = build_bracket(&context).expect("the Stage-1 proof bracket should build");

    let path = std::env::temp_dir().join("aicad_stage1_bracket.step");
    bracket
        .export_step(&path)
        .expect("export_step should succeed for the Stage-1 proof bracket");

    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.starts_with("ISO-10303-21;"));
    assert!(contents.contains("FILE_SCHEMA("));
    assert!(contents.contains("HEADER;") && contents.contains("DATA;"));
    assert!(
        contents.contains("MANIFOLD_SOLID_BREP(") || contents.contains("BREP_WITH_VOIDS("),
        "a solid with through holes must export as a manifold solid or brep-with-voids entity"
    );
    assert!(
        contents.contains("CYLINDRICAL_SURFACE("),
        "the four cylindrical holes plus the fillet's own cylindrical fillet surface must appear"
    );
    assert!(
        contents.contains("PLANE("),
        "the flat base/wall/chamfer faces must appear as PLANE entities"
    );

    std::fs::remove_file(&path).ok();
}

#[test]
fn bracket_survives_an_export_then_import_round_trip_through_this_bridge() {
    // AICAD-035's self-round-trip layer: NOT independent verification of
    // the exporter (both directions share this process's one OCCT
    // installation) -- see project/reports/AICAD-035.md for the
    // genuinely independent (non-OCCT) structural verification performed
    // alongside this. What this test does prove: the export/import
    // pipeline is internally self-consistent for a real multi-operation
    // shape (booleans + fillet + chamfer), not just for a bare box
    // (already covered by cad-occt-bridge's own unit tests).
    let context = OcctContext::new().expect("context creation should succeed");
    let bracket = build_bracket(&context).expect("the Stage-1 proof bracket should build");

    let path = std::env::temp_dir().join("aicad_stage1_bracket_round_trip.step");
    bracket.export_step(&path).unwrap();

    let reimported = context
        .import_step(&path)
        .expect("import_step should succeed for the file this bridge just exported");
    assert!(reimported.is_valid().unwrap());

    let original_volume = bracket.volume().unwrap();
    let reimported_volume = reimported.volume().unwrap();
    assert!(
        (original_volume - reimported_volume).abs() < original_volume * 1e-6,
        "re-imported volume {reimported_volume} should match the original {original_volume}"
    );

    let original_bbox = bracket.bounding_box().unwrap();
    let reimported_bbox = reimported.bounding_box().unwrap();
    assert!(close(original_bbox.min.x, reimported_bbox.min.x));
    assert!(close(original_bbox.max.x, reimported_bbox.max.x));
    assert!(close(original_bbox.min.y, reimported_bbox.min.y));
    assert!(close(original_bbox.max.y, reimported_bbox.max.y));
    assert!(close(original_bbox.min.z, reimported_bbox.min.z));
    assert!(close(original_bbox.max.z, reimported_bbox.max.z));

    assert_eq!(
        bracket.face_count().unwrap(),
        reimported.face_count().unwrap()
    );
    assert_eq!(
        bracket.edge_count().unwrap(),
        reimported.edge_count().unwrap()
    );

    std::fs::remove_file(&path).ok();
}
