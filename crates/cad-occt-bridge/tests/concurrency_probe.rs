//! Permanent concurrency-stress coverage for the kernel operations
//! AICAD-036's own investigation did not exercise.
//!
//! AICAD-036 (`project/reports/AICAD-036.md`) found and fixed a genuine
//! OCCT-internal concurrency defect in `BRepFilletAPI_MakeFillet`, but
//! its own isolation experiments only stress-tested `union`/`cut`/
//! `intersect`/`chamfer` alongside it (see that report's own
//! "Isolation" table). `sweep`, `loft`, `shell`, `offset`, `tessellate`,
//! and the topology-exploration functions (`edge_count`/`get_edge`/
//! `face_count`/`get_face`) were never put under the same methodology:
//! independent contexts on independent `std::thread::scope` threads,
//! validity-checked (not merely success-checked), the way the fillet
//! defect was actually found.
//!
//! `project/reports/reviews/STAGE1-INDEPENDENT-REVIEW.md` ran an
//! equivalent probe as one-off scratch evidence and found no failures;
//! this file commits that coverage permanently (per AGENTS.md's
//! "Autonomously allowed: unit/integration/property/fuzz tests" — this
//! is coverage-expanding, not a fix for a discovered bug) so future
//! changes to these operations, or to the underlying OCCT version, are
//! checked by CI rather than by a one-time audit run.
//!
//! Bounded, not exhaustive: 8 threads x 40 rounds (320 concurrent builds)
//! per operation on simple fixtures. This is real evidence, not a proof
//! of absence — the fillet defect itself needed a specific geometry
//! (concave/reentrant edge on a multi-boolean shape) to surface at all.

use cad_kernel_api::{Direction3, Point3};
use cad_occt_bridge::OcctContext;

fn run_concurrent<F>(threads: usize, rounds: usize, build: F) -> usize
where
    F: Fn() -> bool + Sync,
{
    let mut total_failures = 0;
    for _ in 0..rounds {
        let failures: usize = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..threads).map(|_| scope.spawn(&build)).collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .filter(|ok| !ok)
                .count()
        });
        total_failures += failures;
    }
    total_failures
}

#[test]
fn concurrent_shell_never_corrupts_result() {
    let failures = run_concurrent(8, 40, || {
        let context = OcctContext::new().unwrap();
        let b = context.create_box(10.0, 10.0, 10.0).unwrap();
        let face = b.get_face(0).unwrap();
        match b.shell(&[&face], -1.0) {
            Ok(shape) => shape.is_valid().unwrap_or(false),
            Err(_) => true, // failure is acceptable; corruption is not
        }
    });
    assert_eq!(
        failures, 0,
        "concurrent shell produced {failures} invalid results"
    );
}

#[test]
fn concurrent_offset_never_corrupts_result() {
    let failures = run_concurrent(8, 40, || {
        let context = OcctContext::new().unwrap();
        let b = context.create_box(10.0, 10.0, 10.0).unwrap();
        match b.offset(1.0) {
            Ok(shape) => shape.is_valid().unwrap_or(false),
            Err(_) => true,
        }
    });
    assert_eq!(
        failures, 0,
        "concurrent offset produced {failures} invalid results"
    );
}

#[test]
fn concurrent_sweep_never_corrupts_result() {
    let failures = run_concurrent(8, 40, || {
        let context = OcctContext::new().unwrap();
        let circle = context
            .make_circle_wire(Point3::ORIGIN, Direction3::Z, 2.0)
            .unwrap();
        let profile = circle.make_face().unwrap();
        let spine = context
            .make_circle_wire(Point3::new(0.0, 0.0, 10.0), Direction3::Z, 8.0)
            .unwrap();
        match profile.sweep(&spine) {
            Ok(shape) => shape.is_valid().unwrap_or(false),
            Err(_) => true,
        }
    });
    assert_eq!(
        failures, 0,
        "concurrent sweep produced {failures} invalid results"
    );
}

#[test]
fn concurrent_loft_never_corrupts_result() {
    let failures = run_concurrent(8, 40, || {
        let context = OcctContext::new().unwrap();
        let bottom = context
            .make_circle_wire(Point3::ORIGIN, Direction3::Z, 5.0)
            .unwrap();
        let top = context
            .make_circle_wire(Point3::new(0.0, 0.0, 10.0), Direction3::Z, 3.0)
            .unwrap();
        match context.loft(&[&bottom, &top]) {
            Ok(shape) => shape.is_valid().unwrap_or(false),
            Err(_) => true,
        }
    });
    assert_eq!(
        failures, 0,
        "concurrent loft produced {failures} invalid results"
    );
}

#[test]
fn concurrent_tessellate_never_corrupts_result() {
    let failures = run_concurrent(8, 40, || {
        let context = OcctContext::new().unwrap();
        let b = context.create_box(10.0, 10.0, 10.0).unwrap();
        match b.tessellate(0.1, 0.1) {
            Ok(mesh) => mesh.triangle_count() > 0,
            Err(_) => true,
        }
    });
    assert_eq!(
        failures, 0,
        "concurrent tessellate produced {failures} invalid results"
    );
}

#[test]
fn concurrent_topology_exploration_never_corrupts_result() {
    let failures = run_concurrent(8, 40, || {
        let context = OcctContext::new().unwrap();
        let a = context.create_box(10.0, 10.0, 10.0).unwrap();
        let bb = context.create_box(5.0, 5.0, 20.0).unwrap();
        let unioned = a.union(&bb).unwrap();
        let ec = unioned.edge_count().unwrap();
        let fc = unioned.face_count().unwrap();
        let vc = unioned.vertex_count().unwrap();
        ec > 0 && fc > 0 && vc > 0 && unioned.is_valid().unwrap_or(false)
    });
    assert_eq!(
        failures, 0,
        "concurrent topology exploration produced {failures} invalid results"
    );
}
