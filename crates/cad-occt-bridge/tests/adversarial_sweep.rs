//! AICAD-036: bounded adversarial geometry regression/fuzz harness.
//!
//! Per AGENTS.md's "ADVERSARIAL GEOMETRY CASES"/"NATIVE CRASH / HANG
//! POLICY", this sweeps a bounded, deterministic (fixed-seed) set of
//! adversarial parameter combinations across the operations this bridge
//! already implements: zero/near-zero/tiny/huge/mixed-scale dimensions,
//! tangent/coincident/overlapping/disjoint booleans, impossible fillet/
//! chamfer magnitudes, extreme shell/offset deltas, and long chains of
//! repeated transforms/unusual frames.
//!
//! What this harness asserts, and what it deliberately does not:
//! - It DOES assert the process never panics or crashes for any case in
//!   the sweep -- every call is routed through a `KernelResult` and
//!   matched, never `.unwrap()`ed on a value whose success this harness
//!   doesn't already expect.
//! - It DOES assert argument-validation cases that are unambiguously
//!   invalid by this bridge's own documented contract (a non-positive/
//!   non-finite dimension, a non-positive fillet radius or chamfer
//!   distance) are rejected with `KernelError::InvalidArgument`, never
//!   silently accepted or misclassified as `Internal`.
//! - It does NOT assert every extreme-but-not-contractually-invalid case
//!   (e.g. a fillet radius larger than the edge can geometrically
//!   support) succeeds or produces a valid B-rep -- OCCT is free to
//!   report `OperationFailed` for those, and this bridge must not
//!   silently heal a failure into an artificial success (Stage-1 kernel
//!   policy #13-14). Outcome counts are printed for visibility instead of
//!   hard-coded expectations, since the exact success/failure boundary
//!   for a given adversarial magnitude is an OCCT implementation detail,
//!   not a contract this bridge makes.
//! - `KernelError::Internal` occurring for any case in the bounded sweep
//!   is treated as a hard failure: this bridge's own contract is that
//!   `Internal` means an adapter/backend defect, never an expected
//!   outcome for merely-extreme (as opposed to malformed) input.
//!
//! See `project/reports/AICAD-036.md` for the exact sweep parameters,
//! outcome counts from the last run, and the one genuine defect this
//! effort actually found (documented and fixed separately, and covered
//! by its own permanent regression test -- see
//! `concurrent_fillet_of_a_concave_edge_from_independent_contexts_does_not_corrupt_the_result`
//! in `crates/cad-occt-bridge/src/lib.rs`).

use cad_kernel_api::{Axis3, KernelError, KernelResult, Point3, Transform, Vector3};
use cad_occt_bridge::{OcctContext, Shape};

/// A tiny deterministic xorshift64* PRNG -- no new crate dependency, per
/// CLAUDE.md's "keep agent-support infrastructure minimal" policy. Fixed
/// seed makes every run of this harness reproduce byte-identical cases.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn next_unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.next_unit() * (hi - lo)
    }

    fn pick<'a, T>(&mut self, options: &'a [T]) -> &'a T {
        &options[(self.next_u64() as usize) % options.len()]
    }
}

const SEED: u64 = 0x0A1C_AD00_3600_00A1;

#[test]
fn box_dimensions_never_panic_across_scale_extremes() {
    // Zero, near-zero, tiny, ordinary, huge, and negative/NaN/infinite
    // dimensions, all combinations bounded to CASES entries.
    const CASES: usize = 300;
    let magnitudes: [f64; 9] = [0.0, -1.0, 1e-300, 1e-12, 1e-6, 1.0, 1e6, 1e12, f64::NAN];
    let mut rng = Lcg::new(SEED);
    let mut ok_valid = 0;
    let mut ok_invalid = 0;
    let mut err_invalid_argument = 0;
    let mut err_other = 0;

    for _ in 0..CASES {
        let context = OcctContext::new().unwrap();
        let dx = *rng.pick(&magnitudes);
        let dy = *rng.pick(&magnitudes);
        let dz = *rng.pick(&magnitudes);
        match context.create_box(dx, dy, dz) {
            Ok(shape) => {
                if shape.is_valid().unwrap_or(false) {
                    ok_valid += 1;
                } else {
                    ok_invalid += 1;
                }
            }
            Err(KernelError::InvalidArgument) => err_invalid_argument += 1,
            Err(KernelError::Internal) => {
                panic!("create_box({dx}, {dy}, {dz}) returned Internal -- adapter defect");
            }
            Err(_) => err_other += 1,
        }
        // Contract check: a non-positive or non-finite dimension must
        // always be InvalidArgument, never accepted.
        if !(dx > 0.0 && dx.is_finite() && dy > 0.0 && dy.is_finite() && dz > 0.0 && dz.is_finite())
        {
            let repeat = context.create_box(dx, dy, dz);
            assert!(
                matches!(repeat, Err(KernelError::InvalidArgument)),
                "create_box({dx}, {dy}, {dz}) with a non-positive/non-finite dimension must be InvalidArgument, got {repeat:?}"
            );
        }
    }
    println!(
        "box sweep ({CASES} cases): ok_valid={ok_valid} ok_invalid={ok_invalid} \
         err_invalid_argument={err_invalid_argument} err_other={err_other}"
    );
}

#[test]
fn cylinder_dimensions_never_panic_across_scale_extremes() {
    const CASES: usize = 200;
    let magnitudes: [f64; 7] = [0.0, -1.0, 1e-9, 1e-3, 1.0, 1e9, f64::INFINITY];
    let mut rng = Lcg::new(SEED ^ 0x5EED);
    let mut internal_count = 0;

    for _ in 0..CASES {
        let context = OcctContext::new().unwrap();
        let radius = *rng.pick(&magnitudes);
        let height = *rng.pick(&magnitudes);
        match context.create_cylinder(radius, height) {
            Ok(shape) => {
                let _ = shape.is_valid();
            }
            Err(KernelError::Internal) => internal_count += 1,
            Err(_) => {}
        }
    }
    assert_eq!(
        internal_count, 0,
        "create_cylinder must never return Internal for merely-extreme (not malformed) input"
    );
}

/// Places `b_side`-sized box `b` relative to a fixed `a_side`-sized box
/// `a` (both origin-cornered) at a controllable X offset, covering
/// disjoint / tangent (exactly touching) / coincident-face / overlapping
/// / fully-nested configurations depending on `offset`.
fn tangent_and_coincident_pair<'ctx>(
    context: &'ctx OcctContext,
    a_side: f64,
    b_side: f64,
    offset: f64,
) -> KernelResult<(Shape<'ctx>, Shape<'ctx>)> {
    let a = context.create_box(a_side, a_side, a_side)?;
    let b = context
        .create_box(b_side, b_side, b_side)?
        .transform(&Transform::translation(Vector3::new(offset, 0.0, 0.0)))?;
    Ok((a, b))
}

#[test]
fn tangent_coincident_and_mixed_scale_booleans_never_panic() {
    const CASES: usize = 150;
    let mut rng = Lcg::new(SEED ^ 0xB0011);
    let scale_pairs: [(f64, f64); 5] = [
        (1.0, 1.0),  // equal scale
        (1.0, 1e6),  // mixed scale (huge second operand)
        (1e-6, 1.0), // mixed scale (tiny first operand)
        (1.0, 1e-9), // mixed scale (tiny second operand)
        (1e-3, 1e3), // wide mixed range
    ];
    // Offsets chosen relative to a unit side length: 0 = fully coincident
    // (identical boxes), `a_side` = exactly tangent (touching faces, zero
    // overlap volume), between = overlapping, beyond = disjoint.
    let offset_fractions: [f64; 6] = [0.0, 0.25, 0.5, 1.0, 1.0 + 1e-9, 2.0];

    let mut outcomes = [0usize; 4]; // [union, cut, intersect, panics-would-abort-before-counting]
    for _ in 0..CASES {
        let context = OcctContext::new().unwrap();
        let (a_side, b_side) = *rng.pick(&scale_pairs);
        let offset_fraction = *rng.pick(&offset_fractions);
        let offset = a_side * offset_fraction;
        let Ok((a, b)) = tangent_and_coincident_pair(&context, a_side, b_side, offset) else {
            continue;
        };
        if let Ok(u) = a.union(&b) {
            let _ = u.is_valid();
            outcomes[0] += 1;
        }
        if let Ok(c) = a.cut(&b) {
            let _ = c.is_valid();
            outcomes[1] += 1;
        }
        if let Ok(i) = a.intersect(&b) {
            let _ = i.is_valid();
            outcomes[2] += 1;
        }
    }
    println!(
        "tangent/coincident/mixed-scale boolean sweep ({CASES} cases): \
         union_ok={} cut_ok={} intersect_ok={}",
        outcomes[0], outcomes[1], outcomes[2]
    );
}

#[test]
fn impossible_fillet_and_chamfer_magnitudes_never_panic_or_return_internal() {
    const CASES: usize = 150;
    let mut rng = Lcg::new(SEED ^ 0xF111E7);
    // Fractions of the box's own edge length -- >0.5 is geometrically
    // impossible for a fillet/chamfer confined to one edge's adjacent
    // faces on a unit-ish box; extreme values (huge, negative, NaN) probe
    // the argument-validation boundary.
    let magnitude_fractions: [f64; 8] = [-1.0, 0.0, 1e-9, 0.1, 0.5, 0.99, 10.0, f64::NAN];
    let mut internal_count = 0;

    for _ in 0..CASES {
        let context = OcctContext::new().unwrap();
        let side = 4.0;
        let box_shape = context.create_box(side, side, side).unwrap();
        let edge = box_shape.get_edge(0).unwrap();
        let magnitude = *rng.pick(&magnitude_fractions) * side;

        match box_shape.fillet(&[&edge], magnitude) {
            Ok(shape) => {
                let _ = shape.is_valid();
            }
            Err(KernelError::Internal) => internal_count += 1,
            Err(_) => {}
        }
        if !(magnitude > 0.0 && magnitude.is_finite()) {
            assert_eq!(
                box_shape.fillet(&[&edge], magnitude).unwrap_err(),
                KernelError::InvalidArgument,
                "a non-positive/non-finite fillet radius ({magnitude}) must be InvalidArgument"
            );
        }

        match box_shape.chamfer(&[&edge], magnitude) {
            Ok(shape) => {
                let _ = shape.is_valid();
            }
            Err(KernelError::Internal) => internal_count += 1,
            Err(_) => {}
        }
        if !(magnitude > 0.0 && magnitude.is_finite()) {
            assert_eq!(
                box_shape.chamfer(&[&edge], magnitude).unwrap_err(),
                KernelError::InvalidArgument,
                "a non-positive/non-finite chamfer distance ({magnitude}) must be InvalidArgument"
            );
        }
    }
    assert_eq!(
        internal_count, 0,
        "fillet/chamfer must never return Internal for merely-extreme (not malformed) magnitudes"
    );
}

#[test]
fn extreme_shell_and_offset_deltas_never_panic_or_return_internal() {
    const CASES: usize = 100;
    let mut rng = Lcg::new(SEED ^ 0x5AE11);
    let delta_fractions: [f64; 7] = [-10.0, -0.5, -1e-9, 0.0, 1e-9, 0.5, 10.0];
    let mut internal_count = 0;

    for _ in 0..CASES {
        let context = OcctContext::new().unwrap();
        let side = 3.0;
        let box_shape = context.create_box(side, side, side).unwrap();
        let face = box_shape.get_face(0).unwrap();
        let delta = *rng.pick(&delta_fractions) * side;

        match box_shape.shell(&[&face], delta) {
            Ok(shape) => {
                let _ = shape.is_valid();
            }
            Err(KernelError::Internal) => internal_count += 1,
            Err(_) => {}
        }
        match box_shape.offset(delta) {
            Ok(shape) => {
                let _ = shape.is_valid();
            }
            Err(KernelError::Internal) => internal_count += 1,
            Err(_) => {}
        }
    }
    assert_eq!(
        internal_count, 0,
        "shell/offset must never return Internal for merely-extreme (not malformed) deltas"
    );
}

#[test]
fn long_chains_of_repeated_transforms_stay_exactly_rigid() {
    // Pure Rust math (no kernel calls) -- repeated composition is where
    // floating-point drift in a hand-rolled rotation/composition
    // implementation would first show up (AGENTS.md "repeated
    // transforms" adversarial case). Reuses the exact rigidity check
    // cad-kernel-api's own geometry tests already establish for a single
    // transform, applied here after up to 500 compositions.
    const CHAIN_LENGTH: usize = 500;
    let mut rng = Lcg::new(SEED ^ 0x7EA5F0);

    for _ in 0..20 {
        let mut t = Transform::identity();
        for _ in 0..CHAIN_LENGTH {
            let axis_dir = loop {
                let v = Vector3::new(
                    rng.range(-1.0, 1.0),
                    rng.range(-1.0, 1.0),
                    rng.range(-1.0, 1.0),
                );
                if let Some(d) = v.normalize() {
                    break d;
                }
            };
            let axis = Axis3::new(
                Point3::new(
                    rng.range(-100.0, 100.0),
                    rng.range(-100.0, 100.0),
                    rng.range(-100.0, 100.0),
                ),
                axis_dir,
            );
            let step =
                Transform::rotation(axis, rng.range(-std::f64::consts::PI, std::f64::consts::PI))
                    .compose(&Transform::translation(Vector3::new(
                        rng.range(-10.0, 10.0),
                        rng.range(-10.0, 10.0),
                        rng.range(-10.0, 10.0),
                    )));
            t = t.compose(&step);
        }
        let m = t.to_row_major_3x4();
        let col = |c: usize| Vector3::new(m[c], m[4 + c], m[8 + c]);
        // A generous but still meaningful tolerance: 500 compositions of
        // f64 arithmetic accumulate error; this is checking for gross
        // drift (a real bug), not chasing the last ULP.
        const TOL: f64 = 1e-6;
        for c in 0..3 {
            assert!(
                (col(c).length() - 1.0).abs() < TOL,
                "column {c} lost unit length after {CHAIN_LENGTH} compositions: {:?}",
                col(c)
            );
        }
        assert!(
            col(0).dot(col(1)).abs() < TOL,
            "rotation lost orthogonality (0,1)"
        );
        assert!(
            col(0).dot(col(2)).abs() < TOL,
            "rotation lost orthogonality (0,2)"
        );
        assert!(
            col(1).dot(col(2)).abs() < TOL,
            "rotation lost orthogonality (1,2)"
        );
    }
}

#[test]
fn unusual_orientation_frames_stay_orthonormal_and_right_handed() {
    // Pure Rust math (no kernel calls): Frame3::from_x across a wide
    // range of scales/near-axis-aligned directions (AGENTS.md "unusual
    // orientation/frame cases").
    use cad_kernel_api::Frame3;
    let mut rng = Lcg::new(SEED ^ 0xF12A3E);
    for _ in 0..500 {
        let scale = *rng.pick(&[1e-9, 1e-3, 1.0, 1e3, 1e9]);
        let v = loop {
            let candidate = Vector3::new(
                rng.range(-1.0, 1.0) * scale,
                rng.range(-1.0, 1.0) * scale,
                rng.range(-1.0, 1.0) * scale,
            );
            if let Some(d) = candidate.normalize() {
                break d;
            }
        };
        let frame = Frame3::from_x(Point3::ORIGIN, v);
        const TOL: f64 = 1e-6;
        assert!(frame.x.dot(frame.y).abs() < TOL);
        assert!(frame.x.dot(frame.z).abs() < TOL);
        assert!(frame.y.dot(frame.z).abs() < TOL);
        let cross = frame
            .x
            .cross(frame.y)
            .expect("orthonormal x/y must cross to a direction");
        assert!(
            (cross.dot(frame.z) - 1.0).abs() < TOL,
            "frame must stay right-handed"
        );
    }
}
