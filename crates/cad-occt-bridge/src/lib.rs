//! `cad-occt-bridge` — safe Rust wrapper around `native/occt_bridge`'s
//! C ABI (AICAD-016), implemented for AICAD-018.
//!
//! Public API here is deliberately small: an [`OcctContext`] handle-out
//! type plus the two operations `native/occt_bridge` currently exposes
//! (`create_box`, `shape_volume`). Every `unsafe` FFI call is confined to
//! this crate; nothing downstream ever sees a raw pointer, an OCCT type,
//! or an `AicadStatus`/native error code — only [`cad_kernel_api`] types.
//!
//! Per `AGENTS.md`'s Stage-1 kernel policy #9 ("Treat KernelContext
//! conservatively as single-thread-affine initially"): [`OcctContext`]
//! wraps a raw pointer, and Rust raw pointers are `!Send`/`!Sync` by
//! default, so this type is already single-thread-affine without any
//! extra code — no `unsafe impl Send`/`Sync` is written here, and none
//! should be added without an approved architecture decision.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr::NonNull;

use cad_kernel_api::{KernelError, KernelResult, KernelSolid};

mod ffi {
    use std::os::raw::c_char;

    /// Opaque; matches `native/occt_bridge`'s `AicadOcctContext`. Never
    /// constructed or read from Rust — only ever passed back into the
    /// FFI functions below.
    #[repr(C)]
    pub struct AicadOcctContext {
        _private: [u8; 0],
    }

    pub const AICAD_STATUS_MESSAGE_CAPACITY: usize = 256;

    /// Mirrors `native/occt_bridge/include/aicad/occt_bridge.h`'s
    /// `AicadStatusCode`. `#[repr(i32)]` assumes this C++ compiler gives
    /// the C `enum AicadStatusCode` an `int`-sized representation, which
    /// `native/occt_bridge/src/occt_bridge.cpp`'s `static_assert`
    /// enforces at native-build time (added alongside this crate).
    // Every non-`Ok` variant is only ever produced by native code
    // crossing the FFI boundary (via `match status.code` in
    // `status_to_error`, which reads a raw C `enum` value into this
    // type) — Rust's own code never constructs them directly, which
    // dead-code analysis cannot see through, so it is allowed here
    // rather than worked around with an artificial construction site.
    #[allow(dead_code)]
    #[repr(i32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum AicadStatusCode {
        Ok = 0,
        InvalidArgument = 1,
        InvalidHandle = 2,
        ForeignContextHandle = 3,
        KernelFailure = 4,
        InternalError = 5,
    }

    #[repr(C)]
    pub struct AicadStatus {
        pub code: AicadStatusCode,
        pub message: [c_char; AICAD_STATUS_MESSAGE_CAPACITY],
    }

    impl AicadStatus {
        /// A status buffer to pass by mutable reference into an FFI
        /// call. Every native function this crate calls writes both
        /// fields on every return path (checked in AICAD-016's own
        /// source), so the placeholder values here are never read.
        pub fn placeholder() -> Self {
            Self {
                code: AicadStatusCode::Ok,
                message: [0; AICAD_STATUS_MESSAGE_CAPACITY],
            }
        }
    }

    /// Mirrors `native/occt_bridge`'s `AicadShapeHandle` (AICAD-019
    /// added `generation`, which native's shape-handle table bumps once
    /// per released slot to reject a stale handle even after its slot
    /// is reused for an unrelated shape).
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct AicadShapeHandle {
        pub context_id: u64,
        pub index: u32,
        pub generation: u32,
    }

    unsafe extern "C" {
        pub fn aicad_occt_context_create(out_status: *mut AicadStatus) -> *mut AicadOcctContext;
        pub fn aicad_occt_context_destroy(ctx: *mut AicadOcctContext);
        pub fn aicad_occt_create_box(
            ctx: *mut AicadOcctContext,
            dx: f64,
            dy: f64,
            dz: f64,
            out_handle: *mut AicadShapeHandle,
            out_status: *mut AicadStatus,
        );
        pub fn aicad_occt_shape_volume(
            ctx: *mut AicadOcctContext,
            handle: AicadShapeHandle,
            out_volume: *mut f64,
            out_status: *mut AicadStatus,
        );
        pub fn aicad_occt_release_shape(
            ctx: *mut AicadOcctContext,
            handle: AicadShapeHandle,
            out_status: *mut AicadStatus,
        );
    }
}

/// Converts a native `AicadStatus` (assumed non-`Ok`) into a
/// [`KernelError`], per `cad_kernel_api::KernelError`'s doc comment
/// ("mirrors `AicadStatusCode` one-to-one").
fn status_to_error(status: &ffi::AicadStatus) -> KernelError {
    // Safety: every native function that writes an AicadStatus
    // null-terminates `message` within its fixed capacity on every
    // return path (AICAD-016's `SetStatus`); `message.as_ptr()` is
    // therefore always a valid, null-terminated C string.
    let message = unsafe { CStr::from_ptr(status.message.as_ptr() as *const c_char) }
        .to_string_lossy()
        .into_owned();
    match status.code {
        ffi::AicadStatusCode::Ok => {
            unreachable!("status_to_error must only be called for a non-Ok status")
        }
        ffi::AicadStatusCode::InvalidArgument => KernelError::InvalidArgument(message),
        ffi::AicadStatusCode::InvalidHandle => KernelError::InvalidHandle(message),
        ffi::AicadStatusCode::ForeignContextHandle => KernelError::ForeignContextHandle(message),
        ffi::AicadStatusCode::KernelFailure => KernelError::Failure(message),
        ffi::AicadStatusCode::InternalError => KernelError::Internal(message),
    }
}

fn check_status(status: ffi::AicadStatus) -> KernelResult<()> {
    if status.code == ffi::AicadStatusCode::Ok {
        Ok(())
    } else {
        Err(status_to_error(&status))
    }
}

/// A safe, owning handle to one native OCCT kernel context. Dropping an
/// `OcctContext` destroys every shape it owns; every [`KernelSolid`]
/// handle it issued becomes invalid at that point (`native/occt_bridge`
/// cannot detect that after the fact — see AICAD-016's own report — so
/// callers must not retain a handle past its issuing context's
/// lifetime; a future task may add a checked-lifetime API on top of
/// this one, but this task does not, to stay within its own scope).
pub struct OcctContext {
    raw: NonNull<ffi::AicadOcctContext>,
}

impl OcctContext {
    /// Creates a new, independent kernel context.
    pub fn new() -> KernelResult<Self> {
        let mut status = ffi::AicadStatus::placeholder();
        // Safety: `aicad_occt_context_create` is documented to either
        // return a valid, newly-allocated context or a null pointer
        // with `*out_status` describing why; `&mut status` is a valid,
        // uniquely-owned `*mut AicadStatus` for the duration of the
        // call.
        let raw = unsafe { ffi::aicad_occt_context_create(&mut status) };
        match NonNull::new(raw) {
            Some(raw) => Ok(Self { raw }),
            None => Err(status_to_error(&status)),
        }
    }

    /// Constructs an exact-B-rep rectangular box solid of dimension
    /// `dx * dy * dz`, anchored at the origin. See
    /// `native/occt_bridge`'s `aicad_occt_create_box` for the exact
    /// validation rules (each dimension must be finite and strictly
    /// positive).
    pub fn create_box(&self, dx: f64, dy: f64, dz: f64) -> KernelResult<KernelSolid> {
        let mut handle = ffi::AicadShapeHandle {
            context_id: 0,
            index: 0,
            generation: 0,
        };
        let mut status = ffi::AicadStatus::placeholder();
        // Safety: `self.raw` is a live context for at least the
        // duration of this call (borrowed via `&self`); `&mut handle`
        // and `&mut status` are valid, uniquely-owned output pointers.
        unsafe {
            ffi::aicad_occt_create_box(self.raw.as_ptr(), dx, dy, dz, &mut handle, &mut status);
        }
        check_status(status)?;
        Ok(KernelSolid::new(
            handle.context_id,
            handle.index,
            handle.generation,
        ))
    }

    /// Returns the exact volume of the solid `handle` refers to. Fails
    /// with [`KernelError::ForeignContextHandle`] if `handle` was issued
    /// by a different `OcctContext`, or [`KernelError::InvalidHandle`]
    /// if it is out of range for this one, has been released, or is
    /// otherwise stale (its slot was reused after release — see
    /// [`Self::release_shape`]).
    pub fn shape_volume(&self, handle: KernelSolid) -> KernelResult<f64> {
        let ffi_handle = to_ffi_handle(handle);
        let mut volume = 0.0_f64;
        let mut status = ffi::AicadStatus::placeholder();
        // Safety: same reasoning as `create_box`; `handle` is passed by
        // value (POD), so there is no aliasing/lifetime concern beyond
        // the output pointers.
        unsafe {
            ffi::aicad_occt_shape_volume(self.raw.as_ptr(), ffi_handle, &mut volume, &mut status);
        }
        check_status(status)?;
        Ok(volume)
    }

    /// Releases the solid `handle` refers to, freeing its slot for
    /// reuse by a later [`Self::create_box`] call and permanently
    /// invalidating `handle` (and every other copy of it) — see
    /// `native/occt_bridge`'s `aicad_occt_release_shape` (AICAD-019).
    /// Releasing an already-released (or otherwise invalid) handle
    /// fails with [`KernelError::InvalidHandle`] rather than succeeding
    /// silently.
    pub fn release_shape(&self, handle: KernelSolid) -> KernelResult<()> {
        let ffi_handle = to_ffi_handle(handle);
        let mut status = ffi::AicadStatus::placeholder();
        // Safety: same reasoning as `shape_volume`.
        unsafe {
            ffi::aicad_occt_release_shape(self.raw.as_ptr(), ffi_handle, &mut status);
        }
        check_status(status)
    }
}

fn to_ffi_handle(handle: KernelSolid) -> ffi::AicadShapeHandle {
    ffi::AicadShapeHandle {
        context_id: handle.context_id,
        index: handle.index,
        generation: handle.generation,
    }
}

impl Drop for OcctContext {
    fn drop(&mut self) {
        // Safety: `self.raw` was returned by `aicad_occt_context_create`
        // and has not been passed to `aicad_occt_context_destroy`
        // before (this is the only place that calls it, and `Drop::drop`
        // runs at most once).
        unsafe { ffi::aicad_occt_context_destroy(self.raw.as_ptr()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64, relative_tolerance: f64) {
        let scale = if expected.abs() > 0.0 {
            expected.abs()
        } else {
            1.0
        };
        assert!(
            (actual - expected).abs() <= relative_tolerance * scale,
            "expected {expected}, got {actual} (tolerance {relative_tolerance} relative)"
        );
    }

    #[test]
    fn create_box_and_volume_round_trip() {
        let ctx = OcctContext::new().expect("context creation should succeed");
        let handle = ctx
            .create_box(10.0, 20.0, 30.0)
            .expect("create_box(10, 20, 30) should succeed");
        let volume = ctx
            .shape_volume(handle)
            .expect("shape_volume should succeed for a handle this context issued");
        assert_close(volume, 6000.0, 1e-9);
    }

    #[test]
    fn zero_dimension_is_rejected_as_invalid_argument() {
        let ctx = OcctContext::new().expect("context creation should succeed");
        let result = ctx.create_box(0.0, 1.0, 1.0);
        assert!(matches!(result, Err(KernelError::InvalidArgument(_))));
    }

    #[test]
    fn negative_dimension_is_rejected_as_invalid_argument() {
        let ctx = OcctContext::new().expect("context creation should succeed");
        let result = ctx.create_box(-5.0, 1.0, 1.0);
        assert!(matches!(result, Err(KernelError::InvalidArgument(_))));
    }

    #[test]
    fn nan_dimension_is_rejected_as_invalid_argument() {
        let ctx = OcctContext::new().expect("context creation should succeed");
        let result = ctx.create_box(f64::NAN, 1.0, 1.0);
        assert!(matches!(result, Err(KernelError::InvalidArgument(_))));
    }

    #[test]
    fn very_small_and_very_large_boxes_have_dimensionally_correct_volume() {
        let ctx = OcctContext::new().expect("context creation should succeed");

        let tiny = ctx
            .create_box(1e-6, 1e-6, 1e-6)
            .expect("very small box should succeed");
        assert_close(ctx.shape_volume(tiny).unwrap(), 1e-18, 1e-6);

        let huge = ctx
            .create_box(1e6, 1e6, 1e6)
            .expect("very large box should succeed");
        assert_close(ctx.shape_volume(huge).unwrap(), 1e18, 1e-9);
    }

    #[test]
    fn handle_from_one_context_is_rejected_as_foreign_by_another() {
        let ctx1 = OcctContext::new().expect("context 1 creation should succeed");
        let ctx2 = OcctContext::new().expect("context 2 creation should succeed");
        let handle1 = ctx1
            .create_box(1.0, 1.0, 1.0)
            .expect("create_box on ctx1 should succeed");

        let result = ctx2.shape_volume(handle1);
        assert!(matches!(result, Err(KernelError::ForeignContextHandle(_))));
    }

    #[test]
    fn out_of_range_handle_is_rejected_as_invalid() {
        let ctx = OcctContext::new().expect("context creation should succeed");
        let handle = ctx
            .create_box(1.0, 1.0, 1.0)
            .expect("create_box should succeed");
        let out_of_range =
            KernelSolid::new(handle.context_id, handle.index + 9999, handle.generation);

        let result = ctx.shape_volume(out_of_range);
        assert!(matches!(result, Err(KernelError::InvalidHandle(_))));
    }

    #[test]
    fn two_contexts_do_not_interfere_with_each_others_shape_indices() {
        let ctx1 = OcctContext::new().expect("context 1 creation should succeed");
        let ctx2 = OcctContext::new().expect("context 2 creation should succeed");

        let handle1 = ctx1.create_box(2.0, 2.0, 2.0).unwrap();
        let handle2 = ctx2.create_box(3.0, 3.0, 3.0).unwrap();

        assert_close(ctx1.shape_volume(handle1).unwrap(), 8.0, 1e-9);
        assert_close(ctx2.shape_volume(handle2).unwrap(), 27.0, 1e-9);
    }

    #[test]
    fn dropping_a_context_does_not_panic_or_crash() {
        let ctx = OcctContext::new().expect("context creation should succeed");
        let _handle = ctx.create_box(1.0, 1.0, 1.0).unwrap();
        drop(ctx);
    }

    #[test]
    fn released_handle_is_rejected_and_double_release_fails() {
        let ctx = OcctContext::new().expect("context creation should succeed");
        let handle = ctx.create_box(4.0, 5.0, 6.0).unwrap();

        ctx.release_shape(handle)
            .expect("releasing a live handle should succeed");

        let result = ctx.shape_volume(handle);
        assert!(matches!(result, Err(KernelError::InvalidHandle(_))));

        let double_release = ctx.release_shape(handle);
        assert!(matches!(double_release, Err(KernelError::InvalidHandle(_))));
    }

    #[test]
    fn stale_handle_never_aliases_a_reused_slot() {
        let ctx = OcctContext::new().expect("context creation should succeed");
        let released = ctx.create_box(4.0, 5.0, 6.0).unwrap();
        ctx.release_shape(released).unwrap();

        let reused = ctx
            .create_box(7.0, 8.0, 9.0)
            .expect("a new box should be able to reuse the released slot");
        assert_eq!(
            reused.index, released.index,
            "the free list should have reused the released slot's index"
        );
        assert_ne!(
            reused.generation, released.generation,
            "the reused slot must carry a different generation than the released handle"
        );

        assert_close(ctx.shape_volume(reused).unwrap(), 504.0, 1e-9);

        // The critical property: the OLD (pre-release) handle must stay
        // rejected forever, never silently resolving to the new box
        // that now occupies its slot.
        let result = ctx.shape_volume(released);
        assert!(matches!(result, Err(KernelError::InvalidHandle(_))));
    }
}
