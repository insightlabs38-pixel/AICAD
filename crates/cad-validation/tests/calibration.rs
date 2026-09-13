//! `AICAD-064A`: the bounded multi-scale D5 v1 calibration corpus.
//!
//! Exercises the real kernel bridge (`cad_kernel_api`/`cad_occt_bridge`,
//! **dev-dependency only** — `crate::profile` itself has no such
//! dependency, see that module's own doc comment) across four
//! characteristic linear scales (1mm, 10mm, 100mm, 1000mm) and every
//! category `project/DECISION_LOG.md#DL-17` requires: primitives,
//! transforms, booleans, fillets/chamfers, analytically checkable area,
//! analytically checkable volume, center of mass, and repeated rebuilds.
//! Absolute error, relative error, and repeat-run numerical drift are
//! measured and printed separately (never conflated into one number) —
//! run with `cargo test -p cad-validation --test calibration --
//! --nocapture` to see the full measured table `project/reports/
//! AICAD-064A.md` transcribes verbatim.
//!
//! This is evidence-gathering, not the profile's enforcement: the
//! per-fixture assertions below use loose, generous sanity bounds (to
//! catch a genuine fixture bug, e.g. a wrong closed-form formula) — they
//! are deliberately **not** the derived v1 tolerances themselves, which
//! live in `crate::profile::ComparisonProfile::v1` once this evidence
//! justifies them.

use cad_kernel_api::{Axis3, Direction3, KernelResult, Point3, Transform, Vector3};
use cad_occt_bridge::{OcctContext, Shape};
use std::f64::consts::PI;

/// The four characteristic linear scales `DECISION_LOG.md#DL-17` requires
/// calibration to span, in millimetres (matching `AICAD-034`'s own raw
/// kernel-bridge convention — see `crate::profile`'s module doc comment).
const SCALES: [f64; 4] = [1.0, 10.0, 100.0, 1000.0];

#[derive(Debug, Clone, Copy)]
struct Measurement {
    scale: f64,
    fixture: &'static str,
    quantity: &'static str,
    measured: f64,
    expected: f64,
}

impl Measurement {
    fn abs_err(&self) -> f64 {
        (self.measured - self.expected).abs()
    }

    fn rel_err(&self) -> Option<f64> {
        if self.expected.abs() > 0.0 {
            Some(self.abs_err() / self.expected.abs())
        } else {
            None
        }
    }

    fn print(&self) {
        match self.rel_err() {
            Some(rel) => println!(
                "scale={:>7} fixture={:<20} quantity={:<10} measured={:.12e} expected={:.12e} \
                 abs_err={:.3e} rel_err={:.3e}",
                self.scale,
                self.fixture,
                self.quantity,
                self.measured,
                self.expected,
                self.abs_err(),
                rel
            ),
            None => println!(
                "scale={:>7} fixture={:<20} quantity={:<10} measured={:.12e} expected={:.12e} \
                 abs_err={:.3e} rel_err=N/A(expected=0)",
                self.scale,
                self.fixture,
                self.quantity,
                self.measured,
                self.expected,
                self.abs_err(),
            ),
        }
    }
}

fn build_box(ctx: &OcctContext, s: f64) -> KernelResult<Shape<'_>> {
    ctx.create_box(s, s, s)
}

/// A convex vertical edge of `build_box`'s own cube, found by its
/// bounding box (never by `get_edge`'s raw enumeration index) — mirrors
/// `crates/cad-occt-bridge/tests/stage1_bracket.rs`'s own established
/// edge-selection pattern.
fn find_vertical_edge_at_origin_corner<'ctx>(shape: &Shape<'ctx>) -> KernelResult<Shape<'ctx>> {
    find_vertical_edge_near(shape, 0.0, 0.0)
}

/// Finds the box's vertical (Z-parallel) edge whose (x, y) location is
/// nearest `(target_x, target_y)` — used to fillet/chamfer two
/// *different* corners of the same shape (`AICAD-064A`'s repeated-rebuild
/// fixture combines both operations on one box: filleting the origin
/// corner, then chamfering the opposite corner, since the origin corner's
/// straight edge no longer exists once its own fillet replaces it with a
/// curved surface).
fn find_vertical_edge_near<'ctx>(
    shape: &Shape<'ctx>,
    target_x: f64,
    target_y: f64,
) -> KernelResult<Shape<'ctx>> {
    // OCCT's own edge bounding box carries a small fixed geometric
    // tolerance margin (observed ~1e-7, constant across every scale
    // tested here -- itself a data point for this task's own
    // calibration, see project/reports/AICAD-064A.md), so an exact-zero
    // width comparison is too strict; `1e-5` is a comfortable ~50x margin
    // over that observed floor while staying far tighter than any real
    // box edge length in this corpus (smallest scale is 1mm).
    const EDGE_TOLERANCE: f64 = 1e-5;
    let count = shape.edge_count()?;
    for i in 0..count {
        let edge = shape.get_edge(i)?;
        let bbox = edge.bounding_box()?;
        let dx = (bbox.max.x - bbox.min.x).abs();
        let dy = (bbox.max.y - bbox.min.y).abs();
        let is_vertical = dx < EDGE_TOLERANCE && dy < EDGE_TOLERANCE;
        let at_target = (bbox.min.x - target_x).abs() < EDGE_TOLERANCE
            && (bbox.min.y - target_y).abs() < EDGE_TOLERANCE;
        if is_vertical && at_target {
            return Ok(edge);
        }
    }
    panic!("expected to find a vertical edge near ({target_x}, {target_y})");
}

fn measure_primitives(ctx: &OcctContext, scale: f64, out: &mut Vec<Measurement>) {
    // Box: volume + total surface area + bounding box, per DL-17's
    // "primitives" and "analytically checkable area"/"analytically
    // checkable volume" categories.
    let cube = build_box(ctx, scale).expect("box should build");
    let volume = cube.volume().expect("box volume");
    out.push(Measurement {
        scale,
        fixture: "box",
        quantity: "volume",
        measured: volume,
        expected: scale.powi(3),
    });
    let area = cube.area().expect("box area");
    out.push(Measurement {
        scale,
        fixture: "box",
        quantity: "area",
        measured: area,
        expected: 6.0 * scale * scale,
    });
    let bbox = cube.bounding_box().expect("box bbox");
    for (axis, min_v, max_v) in [
        ("x", bbox.min.x, bbox.max.x),
        ("y", bbox.min.y, bbox.max.y),
        ("z", bbox.min.z, bbox.max.z),
    ] {
        out.push(Measurement {
            scale,
            fixture: "box",
            quantity: Box::leak(format!("bbox_min_{axis}").into_boxed_str()),
            measured: min_v,
            expected: 0.0,
        });
        out.push(Measurement {
            scale,
            fixture: "box",
            quantity: Box::leak(format!("bbox_max_{axis}").into_boxed_str()),
            measured: max_v,
            expected: scale,
        });
    }
    let com = cube.center_of_mass().expect("box center of mass");
    let expected_com = scale / 2.0;
    for (axis, v) in [("x", com.x), ("y", com.y), ("z", com.z)] {
        out.push(Measurement {
            scale,
            fixture: "box",
            quantity: Box::leak(format!("center_of_mass_{axis}").into_boxed_str()),
            measured: v,
            expected: expected_com,
        });
    }

    // Cylinder: volume + total surface area (side + two end caps).
    let radius = scale / 4.0;
    let height = scale;
    let cylinder = ctx
        .create_cylinder(radius, height)
        .expect("cylinder should build");
    let cyl_volume = cylinder.volume().expect("cylinder volume");
    out.push(Measurement {
        scale,
        fixture: "cylinder",
        quantity: "volume",
        measured: cyl_volume,
        expected: PI * radius * radius * height,
    });
    let cyl_area = cylinder.area().expect("cylinder area");
    out.push(Measurement {
        scale,
        fixture: "cylinder",
        quantity: "area",
        measured: cyl_area,
        expected: 2.0 * PI * radius * height + 2.0 * PI * radius * radius,
    });
}

fn measure_transform(ctx: &OcctContext, scale: f64, out: &mut Vec<Measurement>) {
    // A translated + rotated box must have *exactly* the same volume as
    // the untransformed box (a rigid transform never changes volume) --
    // this isolates transform-specific numerical noise from any
    // boolean/fillet/chamfer curve-fitting noise.
    let cube = build_box(ctx, scale).expect("box should build");
    let baseline_volume = cube.volume().expect("baseline volume");

    let translate = Transform::translation(Vector3::new(scale * 3.7, -scale * 2.1, scale * 15.0));
    let rotate = Transform::rotation(
        Axis3::new(Point3::ORIGIN, Direction3::Z),
        37.0_f64.to_radians(),
    );
    let composed = rotate.compose(&translate);
    let moved = cube.transform(&composed).expect("transform should succeed");
    let moved_volume = moved.volume().expect("transformed volume");

    out.push(Measurement {
        scale,
        fixture: "transform",
        quantity: "volume",
        measured: moved_volume,
        expected: baseline_volume,
    });
    out.push(Measurement {
        scale,
        fixture: "transform",
        quantity: "volume_vs_analytic",
        measured: moved_volume,
        expected: scale.powi(3),
    });
}

fn measure_booleans(ctx: &OcctContext, scale: f64, out: &mut Vec<Measurement>) {
    // Two identical cubes, the second shifted by half a scale along X --
    // a genuine 3D overlap of half the second cube's volume, mirroring
    // AICAD-034's own overlap-not-coincident convention.
    let a = build_box(ctx, scale).expect("box a");
    let b = build_box(ctx, scale).expect("box b");
    let b = b
        .transform(&Transform::translation(Vector3::new(scale / 2.0, 0.0, 0.0)))
        .expect("translate box b");

    let overlap_volume = (scale / 2.0) * scale * scale;

    let union = a.union(&b).expect("union should succeed");
    out.push(Measurement {
        scale,
        fixture: "union",
        quantity: "volume",
        measured: union.volume().expect("union volume"),
        expected: 2.0 * scale.powi(3) - overlap_volume,
    });

    let a2 = build_box(ctx, scale).expect("box a (fresh)");
    let b2 = build_box(ctx, scale)
        .expect("box b (fresh)")
        .transform(&Transform::translation(Vector3::new(scale / 2.0, 0.0, 0.0)))
        .expect("translate box b (fresh)");
    let intersection = a2.intersect(&b2).expect("intersect should succeed");
    out.push(Measurement {
        scale,
        fixture: "intersect",
        quantity: "volume",
        measured: intersection.volume().expect("intersection volume"),
        expected: overlap_volume,
    });

    // A clean through-cut hole -- boolean cut against a rigid-transformed
    // cylinder, overshooting the plate thickness on both ends so the cut
    // never leaves a coincident end face (AICAD-034's own convention).
    let plate = build_box(ctx, scale).expect("plate");
    let hole_radius = scale / 8.0;
    let margin = scale * 0.1;
    let raw_hole = ctx
        .create_cylinder(hole_radius, scale + 2.0 * margin)
        .expect("hole cylinder");
    let hole = raw_hole
        .transform(&Transform::translation(Vector3::new(
            scale / 2.0,
            scale / 2.0,
            -margin,
        )))
        .expect("place hole cylinder");
    let cut = plate.cut(&hole).expect("cut should succeed");
    out.push(Measurement {
        scale,
        fixture: "cut",
        quantity: "volume",
        measured: cut.volume().expect("cut volume"),
        expected: scale.powi(3) - PI * hole_radius * hole_radius * scale,
    });
}

fn measure_fillet_chamfer(ctx: &OcctContext, scale: f64, out: &mut Vec<Measurement>) {
    let fillet_radius = scale / 10.0;
    let chamfer_distance = scale / 10.0;

    // Convex-edge fillet on a bare box removes material: area removed per
    // unit length = r^2 * (1 - pi/4) (AICAD-034's/AICAD-027's own
    // established convex-fillet formula), times the edge's full length.
    let cube = build_box(ctx, scale).expect("box for fillet");
    let edge = find_vertical_edge_at_origin_corner(&cube).expect("fillet edge");
    let filleted = cube.fillet(&[&edge], fillet_radius).expect("fillet");
    drop(edge);
    let removed = fillet_radius * fillet_radius * (1.0 - PI / 4.0) * scale;
    out.push(Measurement {
        scale,
        fixture: "fillet",
        quantity: "volume",
        measured: filleted.volume().expect("filleted volume"),
        expected: scale.powi(3) - removed,
    });

    // Convex-edge chamfer removes a triangular prism: area removed per
    // unit length = d^2/2, times the edge's full length.
    let cube2 = build_box(ctx, scale).expect("box for chamfer");
    let edge2 = find_vertical_edge_at_origin_corner(&cube2).expect("chamfer edge");
    let chamfered = cube2.chamfer(&[&edge2], chamfer_distance).expect("chamfer");
    drop(edge2);
    let removed2 = chamfer_distance * chamfer_distance / 2.0 * scale;
    out.push(Measurement {
        scale,
        fixture: "chamfer",
        quantity: "volume",
        measured: chamfered.volume().expect("chamfered volume"),
        expected: scale.powi(3) - removed2,
    });
}

/// A deliberately near-zero-volume fixture (a very thin flat box) --
/// `project/OWNER_DECISIONS.md#D19` explicitly flagged that no existing
/// evidence exercised this case, leaving `volume_abs` unsupported; this
/// closes that specific gap.
fn measure_near_zero_volume(ctx: &OcctContext, scale: f64, out: &mut Vec<Measurement>) {
    let thickness = scale * 1e-6;
    let thin = ctx
        .create_box(scale, scale, thickness)
        .expect("thin box should build");
    let volume = thin.volume().expect("thin box volume");
    out.push(Measurement {
        scale,
        fixture: "near_zero_volume",
        quantity: "volume",
        measured: volume,
        expected: scale * scale * thickness,
    });
}

fn build_fillet_chamfer_fixture(ctx: &OcctContext, scale: f64) -> f64 {
    let fillet_radius = scale / 10.0;
    let chamfer_distance = scale / 10.0;
    let cube = build_box(ctx, scale).expect("box for repeat-rebuild fixture");
    // Fillet the origin corner, then chamfer the *opposite* corner (the
    // origin corner's own straight vertical edge no longer exists once
    // its fillet replaces it with a curved surface).
    let edge = find_vertical_edge_near(&cube, 0.0, 0.0).expect("fillet edge");
    let filleted = cube.fillet(&[&edge], fillet_radius).expect("fillet");
    drop(edge);
    let edge2 = find_vertical_edge_near(&filleted, scale, scale).expect("chamfer edge");
    let chamfered = filleted
        .chamfer(&[&edge2], chamfer_distance)
        .expect("chamfer");
    drop(edge2);
    chamfered.volume().expect("rebuilt volume")
}

/// Repeated-rebuild numerical drift: the *same* fillet+chamfer fixture,
/// rebuilt from scratch `REPEATS` times in a fresh `OcctContext` each
/// time, at each scale -- measuring drift, never conflated with
/// absolute/relative error against the closed form (`DECISION_LOG.md
/// #DL-17`: "Measure separately: absolute error; relative error;
/// repeat-run numerical drift"). `DL-17` also requires not inferring
/// cross-platform behavior from same-machine repeated runs -- this test
/// makes no such claim, it only measures same-machine drift.
const REPEATS: usize = 5;

#[test]
fn calibration_corpus_measures_error_and_drift_across_scales() {
    let mut measurements = Vec::new();

    for &scale in &SCALES {
        let ctx = OcctContext::new().expect("context creation should succeed");
        measure_primitives(&ctx, scale, &mut measurements);
        measure_transform(&ctx, scale, &mut measurements);
        measure_booleans(&ctx, scale, &mut measurements);
        measure_fillet_chamfer(&ctx, scale, &mut measurements);
        measure_near_zero_volume(&ctx, scale, &mut measurements);
    }

    println!("\n=== AICAD-064A calibration measurements ===");
    for m in &measurements {
        m.print();
    }

    // Loose sanity bounds only -- catch a genuinely broken fixture
    // (wrong closed-form formula, a kernel regression), not the derived
    // v1 tolerances themselves (those are documented in
    // project/reports/AICAD-064A.md and crate::profile::ComparisonProfile
    // once this evidence justifies them).
    for m in &measurements {
        if let Some(rel) = m.rel_err() {
            assert!(
                rel < 1e-3,
                "unexpectedly large relative error for {} {} at scale {}: {} (measured={}, \
                 expected={})",
                m.fixture,
                m.quantity,
                m.scale,
                rel,
                m.measured,
                m.expected
            );
        } else {
            assert!(
                m.abs_err() < 1e-6 * m.scale.max(1.0),
                "unexpectedly large absolute error for {} {} at scale {}: {} (measured={}, \
                 expected 0)",
                m.fixture,
                m.quantity,
                m.scale,
                m.abs_err(),
                m.measured
            );
        }
    }

    // Print per-quantity max relative error across all scales -- the
    // basis for deriving linear_rel/area_rel/volume_abs in
    // project/reports/AICAD-064A.md.
    println!("\n=== Max relative error by quantity kind ===");
    for kind in ["volume", "area"] {
        let max_rel = measurements
            .iter()
            .filter(|m| m.quantity == kind)
            .filter_map(Measurement::rel_err)
            .fold(0.0_f64, f64::max);
        println!("max rel_err for quantity={kind}: {max_rel:.3e}");
    }
    println!("\n=== Max absolute error for bbox/center-of-mass (linear quantities) ===");
    let max_abs_linear = measurements
        .iter()
        .filter(|m| m.quantity.starts_with("bbox_") || m.quantity.starts_with("center_of_mass_"))
        .map(Measurement::abs_err)
        .fold(0.0_f64, f64::max);
    println!("max abs_err for bbox/center-of-mass quantities: {max_abs_linear:.3e}");
    // Per-scale breakdown, to see whether the error grows with scale
    // (evidence for a nonzero linear_rel) or stays flat (evidence for a
    // pure absolute floor).
    for &scale in &SCALES {
        let max_at_scale = measurements
            .iter()
            .filter(|m| m.scale == scale)
            .filter(|m| {
                m.quantity.starts_with("bbox_") || m.quantity.starts_with("center_of_mass_")
            })
            .map(Measurement::abs_err)
            .fold(0.0_f64, f64::max);
        println!("scale={scale:>7} max linear abs_err={max_at_scale:.3e}");
    }
}

#[test]
fn repeated_rebuild_drift_across_scales() {
    println!("\n=== AICAD-064A repeated-rebuild drift (same machine only) ===");
    for &scale in &SCALES {
        let mut volumes = Vec::with_capacity(REPEATS);
        for _ in 0..REPEATS {
            let ctx = OcctContext::new().expect("context creation should succeed");
            volumes.push(build_fillet_chamfer_fixture(&ctx, scale));
        }
        let min = volumes.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = volumes.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let drift_abs = max - min;
        let drift_rel = if min.abs() > 0.0 {
            drift_abs / min.abs()
        } else {
            0.0
        };
        println!(
            "scale={scale:>7} repeats={REPEATS} volumes={volumes:?} drift_abs={drift_abs:.3e} \
             drift_rel={drift_rel:.3e}"
        );
        assert!(
            drift_rel < 1e-6,
            "unexpectedly large repeat-run drift at scale {scale}: {drift_rel} (volumes: \
             {volumes:?})"
        );
    }
}
