//! AICAD-018 smoke test: proves the safe wrapper actually drives the
//! native ABI end to end (Rust -> FFI -> native OCCT bridge -> OCCT).
//! Comprehensive ownership/lifecycle coverage (foreign-context,
//! stale-handle, no-aliasing) is AICAD-019's dedicated test suite.

use cad_occt_bridge::KernelContext;

#[test]
fn create_box_query_volume_and_destroy_round_trip() {
    let mut ctx = KernelContext::new().expect("context creation must succeed");

    let solid = ctx
        .create_box(2.0, 3.0, 4.0)
        .expect("a 2x3x4 box must be constructible");

    let volume = ctx
        .shape_volume(solid)
        .expect("volume of a freshly created solid must be queryable");
    assert!(
        (volume - 24.0).abs() < 1e-9,
        "expected volume 24, got {volume}"
    );

    ctx.destroy_solid(solid)
        .expect("destroying a live handle must succeed");
}

#[test]
fn create_box_rejects_a_non_positive_dimension() {
    let mut ctx = KernelContext::new().expect("context creation must succeed");
    let err = ctx
        .create_box(-1.0, 1.0, 1.0)
        .expect_err("a negative dimension must be rejected");
    assert!(matches!(
        err,
        cad_occt_bridge::KernelError::InvalidArgument(_)
    ));
}
