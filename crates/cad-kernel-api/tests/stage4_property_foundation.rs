//! AICAD-079C bounded property/adversarial foundation.
//!
//! These are deterministic semantic-invariant sweeps over already-implemented
//! spatial primitives. They deliberately avoid randomized geometry and add no
//! Stage-4 reference behavior.

use cad_kernel_api::{Frame3, Point3, Vector3};

const EPS: f64 = 1e-12;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= EPS
}

#[test]
fn every_bounded_nonzero_finite_vector_normalizes_to_unit_length() {
    for x in -3..=3 {
        for y in -3..=3 {
            for z in -3..=3 {
                if x == 0 && y == 0 && z == 0 {
                    continue;
                }
                let v = Vector3::new(x as f64, y as f64, z as f64);
                let d = v.normalize().expect("bounded nonzero finite vector");
                assert!(close(d.as_vector3().length(), 1.0), "vector={v:?}");
                for scale in [0.25, 2.0, 100.0] {
                    let scaled = (v * scale).normalize().expect("positive scaling stays valid");
                    assert!(
                        d.dot(scaled) > 1.0 - 1e-12,
                        "normalization changed direction: v={v:?}, scale={scale}"
                    );
                }
            }
        }
    }
}

#[test]
fn invalid_direction_inputs_fail_closed() {
    for v in [
        Vector3::ZERO,
        Vector3::new(f64::NAN, 0.0, 0.0),
        Vector3::new(f64::INFINITY, 0.0, 0.0),
        Vector3::new(1e-14, 0.0, 0.0),
    ] {
        assert!(v.normalize().is_none(), "invalid vector unexpectedly normalized: {v:?}");
    }
}

#[test]
fn derived_frames_are_orthonormal_and_right_handed_over_bounded_directions() {
    for x in -2..=2 {
        for y in -2..=2 {
            for z in -2..=2 {
                if x == 0 && y == 0 && z == 0 {
                    continue;
                }
                let direction = Vector3::new(x as f64, y as f64, z as f64)
                    .normalize()
                    .expect("bounded direction");
                for frame in [
                    Frame3::from_x(Point3::ORIGIN, direction),
                    Frame3::from_z(Point3::ORIGIN, direction),
                ] {
                    assert!(frame.x.dot(frame.y).abs() < 1e-12);
                    assert!(frame.x.dot(frame.z).abs() < 1e-12);
                    assert!(frame.y.dot(frame.z).abs() < 1e-12);
                    let cross = frame.x.cross(frame.y).expect("orthogonal axes cross");
                    assert!(cross.dot(frame.z) > 1.0 - 1e-12, "frame is not right-handed: {frame:?}");
                }
            }
        }
    }
}
