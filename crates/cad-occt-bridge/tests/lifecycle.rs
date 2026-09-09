//! AICAD-019: kernel context lifecycle and shape-handle table —
//! dedicated ownership/lifecycle coverage through the safe wrapper.
//!
//! AICAD-016 implemented this contract natively and proved it directly
//! against the C ABI (`native/occt_bridge/tests/bridge_abi_tests.cpp`).
//! This suite proves the same contract holds when driven exclusively
//! through `cad-occt-bridge`'s safe Rust API (AICAD-018) with no
//! `unsafe` anywhere in this file, which is the Batch-1A checkpoint's
//! "Rust/native ownership behavior tested" evidence item.

use cad_occt_bridge::{KernelContext, KernelError, KernelSolid, RawHandle};

#[test]
fn stale_handle_is_rejected_after_destroy() {
    let mut ctx = KernelContext::new().unwrap();
    let solid = ctx.create_box(1.0, 1.0, 1.0).unwrap();

    ctx.destroy_solid(solid).unwrap();

    let err = ctx
        .shape_volume(solid)
        .expect_err("destroyed handle must not resolve");
    assert_eq!(err, KernelError::StaleHandle);
}

#[test]
fn double_destroy_is_rejected_as_stale_not_silently_accepted() {
    let mut ctx = KernelContext::new().unwrap();
    let solid = ctx.create_box(1.0, 1.0, 1.0).unwrap();

    ctx.destroy_solid(solid).unwrap();
    let err = ctx
        .destroy_solid(solid)
        .expect_err("destroying an already-destroyed handle must fail");
    assert_eq!(err, KernelError::StaleHandle);
}

#[test]
fn foreign_context_handle_is_rejected() {
    let mut ctx1 = KernelContext::new().unwrap();
    let mut ctx2 = KernelContext::new().unwrap();

    let solid_in_ctx1 = ctx1.create_box(1.0, 2.0, 3.0).unwrap();

    let err = ctx2
        .shape_volume(solid_in_ctx1)
        .expect_err("a handle from ctx1 must not resolve against ctx2");
    assert_eq!(err, KernelError::ForeignContext);

    let err = ctx2
        .destroy_solid(solid_in_ctx1)
        .expect_err("destroy must reject a foreign-context handle too");
    assert_eq!(err, KernelError::ForeignContext);
}

#[test]
fn out_of_range_slot_is_rejected_as_invalid_handle() {
    let mut ctx = KernelContext::new().unwrap();
    // Establish a real handle purely to learn this context's own id
    // (there is no other public accessor for it), then fabricate a
    // handle in the *same* context with a slot that was never
    // allocated, so FOREIGN_CONTEXT cannot mask the INVALID_HANDLE case
    // this test is for.
    let real = ctx.create_box(1.0, 1.0, 1.0).unwrap();
    let bogus = KernelSolid::from_raw(RawHandle::new(real.raw().context_id, 999_999, 1));

    let err = ctx
        .shape_volume(bogus)
        .expect_err("an out-of-range slot in the handle's own context must be rejected");
    assert_eq!(err, KernelError::InvalidHandle);
}

#[test]
fn released_slot_does_not_alias_newly_created_geometry() {
    let mut ctx = KernelContext::new().unwrap();

    let first = ctx.create_box(2.0, 2.0, 2.0).unwrap(); // volume 8
    ctx.destroy_solid(first).unwrap();

    let second = ctx.create_box(3.0, 3.0, 3.0).unwrap(); // volume 27, may reuse first's slot

    assert_ne!(
        first.raw(),
        second.raw(),
        "a freshly created handle must never equal a just-destroyed one"
    );

    let err = ctx
        .shape_volume(first)
        .expect_err("the destroyed handle must still not resolve after slot reuse");
    assert_eq!(err, KernelError::StaleHandle);

    let volume = ctx.shape_volume(second).unwrap();
    assert!(
        (volume - 27.0).abs() < 1e-9,
        "the new handle must resolve to its own (not the old) geometry, got {volume}"
    );
}

#[test]
fn handle_from_a_destroyed_context_is_rejected_by_a_later_context_not_aliased() {
    let mut ctx1 = KernelContext::new().unwrap();
    let solid = ctx1.create_box(1.0, 1.0, 1.0).unwrap();
    drop(ctx1); // destroys the native context; `solid` is now a dangling reference

    // A later, independently created context must never accidentally
    // resolve a handle minted by an earlier, already-destroyed context
    // -- context ids are a monotonic counter, never memory addresses,
    // specifically so this cannot happen by coincidental reuse.
    let ctx2 = KernelContext::new().unwrap();
    let err = ctx2
        .shape_volume(solid)
        .expect_err("a handle from a destroyed context must not resolve against a new one");
    assert_eq!(err, KernelError::ForeignContext);
}
