//! `cad-occt-bridge` — safe Rust wrapper around `native/occt_bridge`
//! (AICAD-015/AICAD-016), implementing `cad-kernel-api` (AICAD-017)
//! against Open CASCADE Technology.
//!
//! This is the **only** crate in the AICAD workspace allowed to link
//! against `native/occt_bridge`, per
//! `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-01 and this crate's own
//! `README.md`. Its public API exposes only [`cad_kernel_api`]'s
//! vendor-neutral handle types and [`cad_kernel_api::KernelError`] — no
//! OCCT type, and no raw pointer, appears in this crate's public
//! signatures. Internally, `mod ffi` holds the raw `unsafe extern "C"`
//! declarations mirroring `native/occt_bridge/include/aicad_occt_bridge.h`
//! exactly; every other item in this crate is safe Rust that uses `mod
//! ffi` behind ordinary (non-`unsafe`) method calls.
//!
//! **Units.** [`Context::create_box`] takes raw `f64` millimeters, not a
//! typed `cad-units` quantity. This is deliberate, not a violation of
//! `AGENTS.md`'s "Units are typed engineering quantities, not untyped
//! floats" non-negotiable: that rule governs the *public AICAD language*
//! surface. Nothing in the public language can reach this crate
//! directly — only `cad-geometry-api`/`cad-geometry-runtime` (WP-05) may
//! call it, and converting a typed `Length` quantity to a raw millimeter
//! `f64` at that boundary is exactly the "Geometry IR -> Kernel call
//! graph" lowering step `docs/plan/01_SYSTEM_ARCHITECTURE.md` §5
//! describes. Millimeters specifically because that is OCCT's own native
//! convention (confirmed in `project/reports/AICAD-015.md`), so no
//! implicit unit conversion happens inside this crate either.
//!
//! **Thread affinity.** [`Context`] wraps a raw pointer and is therefore
//! `!Send`/`!Sync` by Rust's ordinary auto-trait rules (no explicit
//! opt-out needed) — matching Stage-1 kernel policy #9 ("treat
//! `KernelContext` conservatively as single-thread-affine").

use std::ffi::CStr;
use std::num::NonZeroU64;

use cad_kernel_api::{KernelError, KernelResult, KernelShape};

/// Raw FFI declarations mirroring
/// `native/occt_bridge/include/aicad_occt_bridge.h` exactly. Nothing in
/// this module is exported from the crate; every public item elsewhere
/// in this crate wraps these in a safe API.
mod ffi {
    use std::ffi::c_char;

    /// Opaque native context type. Never constructed or read from Rust —
    /// only ever passed back to the functions below as the exact pointer
    /// `aicad_kernel_context_create` returned.
    #[repr(C)]
    pub struct AicadKernelContext {
        _private: [u8; 0],
    }

    /// Mirrors `AicadShapeHandle` (`struct { uint64_t id; }`) field for
    /// field.
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct AicadShapeHandle {
        pub id: u64,
    }

    /// Mirrors `AicadStatus`'s exact discriminant values.
    ///
    /// Every non-`Ok` variant is only ever produced by the native side
    /// crossing the ABI (never constructed by Rust code), so rustc's
    /// dead-code analysis cannot see them as "constructed" even though
    /// `error_for`'s `match` reads every one of them — hence
    /// `allow(dead_code)` here rather than at the crate level.
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[allow(dead_code)]
    pub enum AicadStatus {
        Ok = 0,
        NullContext = 1,
        InvalidArgument = 2,
        InvalidHandle = 3,
        KernelInternalError = 4,
        UnknownError = 5,
    }

    unsafe extern "C" {
        pub fn aicad_kernel_context_create() -> *mut AicadKernelContext;
        pub fn aicad_kernel_context_destroy(context: *mut AicadKernelContext);
        pub fn aicad_kernel_context_last_error(context: *mut AicadKernelContext) -> *const c_char;
        pub fn aicad_create_box(
            context: *mut AicadKernelContext,
            dx: f64,
            dy: f64,
            dz: f64,
            out_handle: *mut AicadShapeHandle,
        ) -> AicadStatus;
        pub fn aicad_shape_bbox_diagonal(
            context: *mut AicadKernelContext,
            handle: AicadShapeHandle,
            out_diagonal: *mut f64,
        ) -> AicadStatus;
        pub fn aicad_shape_release(
            context: *mut AicadKernelContext,
            handle: AicadShapeHandle,
        ) -> AicadStatus;
    }
}

/// A live native kernel context. Owns its native-side handle table; all
/// shapes it minted become invalid the moment this `Context` is dropped.
///
/// See the module-level doc for why this type is intentionally
/// `!Send`/`!Sync`.
pub struct Context {
    raw: *mut ffi::AicadKernelContext,
}

impl Context {
    /// Create a new, empty kernel context.
    pub fn new() -> KernelResult<Self> {
        // SAFETY: `aicad_kernel_context_create` takes no arguments and is
        // documented to never throw/abort, only to return NULL on
        // allocation failure (see aicad_occt_bridge.h).
        let raw = unsafe { ffi::aicad_kernel_context_create() };
        if raw.is_null() {
            return Err(KernelError::Internal(
                "native aicad_kernel_context_create returned null".to_string(),
            ));
        }
        Ok(Self { raw })
    }

    /// Reads the context's current last-error text. Only meaningful
    /// immediately after a non-OK status; see
    /// `aicad_kernel_context_last_error`'s documented pointer-lifetime
    /// contract in the C header, which this method's call site respects
    /// by copying the text out into an owned `String` before returning.
    fn last_error(&self) -> String {
        // SAFETY: `self.raw` is non-null for the lifetime of `self` (set
        // once in `new`, only freed in `Drop`, and `Context` is never
        // constructed with a null `raw` — see `new`'s null check above).
        // `aicad_kernel_context_last_error` is documented to always
        // return a valid, null-terminated pointer for a non-null
        // context.
        let ptr = unsafe { ffi::aicad_kernel_context_last_error(self.raw) };
        if ptr.is_null() {
            return String::new();
        }
        // SAFETY: the header guarantees the returned pointer is
        // null-terminated and valid until the next call on this context;
        // we immediately copy it into an owned String and make no
        // further native calls before doing so.
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }

    fn error_for(&self, status: ffi::AicadStatus) -> KernelError {
        match status {
            ffi::AicadStatus::Ok => {
                unreachable!("error_for must only be called with a non-OK status")
            }
            ffi::AicadStatus::NullContext => KernelError::NullContext,
            ffi::AicadStatus::InvalidArgument => KernelError::InvalidArgument(self.last_error()),
            ffi::AicadStatus::InvalidHandle => KernelError::InvalidHandle,
            ffi::AicadStatus::KernelInternalError => KernelError::Internal(self.last_error()),
            ffi::AicadStatus::UnknownError => KernelError::Unknown(self.last_error()),
        }
    }

    /// Construct an axis-aligned box of size `dx * dy * dz` millimeters at
    /// the origin. See the module doc for why the arguments are raw
    /// `f64` millimeters rather than a typed `cad-units` quantity.
    pub fn create_box(&mut self, dx: f64, dy: f64, dz: f64) -> KernelResult<KernelShape> {
        let mut out = ffi::AicadShapeHandle { id: 0 };
        // SAFETY: `self.raw` is a live context (see `last_error`'s
        // safety comment); `&mut out` is a valid, uniquely-owned
        // out-pointer for the duration of this call.
        let status = unsafe { ffi::aicad_create_box(self.raw, dx, dy, dz, &mut out) };
        if status != ffi::AicadStatus::Ok {
            return Err(self.error_for(status));
        }
        let id = NonZeroU64::new(out.id).unwrap_or_else(|| {
            unreachable!(
                "native/occt_bridge guarantees a non-zero handle id whenever \
                 aicad_create_box returns AICAD_STATUS_OK (see \
                 project/reports/AICAD-016.md)"
            )
        });
        Ok(KernelShape::from_raw(id))
    }

    /// Diagonal length of `shape`'s axis-aligned bounding box.
    pub fn shape_bbox_diagonal(&mut self, shape: KernelShape) -> KernelResult<f64> {
        let mut out = 0.0_f64;
        let handle = ffi::AicadShapeHandle {
            id: shape.raw().get(),
        };
        // SAFETY: see `create_box`'s safety comment; `handle` is a POD
        // value, not a pointer, so no aliasing concern beyond the
        // context pointer itself.
        let status = unsafe { ffi::aicad_shape_bbox_diagonal(self.raw, handle, &mut out) };
        if status != ffi::AicadStatus::Ok {
            return Err(self.error_for(status));
        }
        Ok(out)
    }

    /// Release `shape`. After this call, `shape`'s id is never reissued
    /// by any context (see `native/occt_bridge`'s handle-id policy,
    /// `project/reports/AICAD-016.md`) — using it again always yields
    /// [`KernelError::InvalidHandle`].
    pub fn release_shape(&mut self, shape: KernelShape) -> KernelResult<()> {
        let handle = ffi::AicadShapeHandle {
            id: shape.raw().get(),
        };
        // SAFETY: see `create_box`'s safety comment.
        let status = unsafe { ffi::aicad_shape_release(self.raw, handle) };
        if status != ffi::AicadStatus::Ok {
            return Err(self.error_for(status));
        }
        Ok(())
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: `self.raw` was obtained from `aicad_kernel_context_create`
        // in `new` and has not been freed before (Drop::drop runs at
        // most once); `aicad_kernel_context_destroy` is documented safe
        // to call on any context exactly once, releasing every shape it
        // still owns.
        unsafe { ffi::aicad_kernel_context_destroy(self.raw) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_kernel_api::KernelError;

    #[test]
    fn create_query_release_round_trip() {
        let mut context = Context::new().expect("context creation should succeed");
        let shape = context
            .create_box(10.0, 20.0, 30.0)
            .expect("create_box(10,20,30) should succeed");
        let diagonal = context
            .shape_bbox_diagonal(shape)
            .expect("bbox_diagonal on a live shape should succeed");
        let expected = (10.0_f64 * 10.0 + 20.0 * 20.0 + 30.0 * 30.0).sqrt();
        assert!(
            (diagonal - expected).abs() < 1e-9,
            "diagonal={diagonal} expected={expected}"
        );
        context
            .release_shape(shape)
            .expect("releasing a live shape should succeed");
    }

    #[test]
    fn released_handle_is_rejected_not_aliased() {
        let mut context = Context::new().unwrap();
        let shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        context.release_shape(shape).unwrap();

        let result = context.shape_bbox_diagonal(shape);
        assert_eq!(result, Err(KernelError::InvalidHandle));

        let result = context.release_shape(shape);
        assert_eq!(result, Err(KernelError::InvalidHandle));
    }

    #[test]
    fn foreign_context_handle_is_rejected() {
        let mut context_a = Context::new().unwrap();
        let mut context_b = Context::new().unwrap();
        let shape_from_a = context_a.create_box(2.0, 2.0, 2.0).unwrap();

        let result = context_b.shape_bbox_diagonal(shape_from_a);
        assert_eq!(result, Err(KernelError::InvalidHandle));
    }

    #[test]
    fn degenerate_box_is_a_contained_internal_error_not_a_panic() {
        let mut context = Context::new().unwrap();
        let result = context.create_box(0.0, 20.0, 30.0);
        match result {
            Err(KernelError::Internal(message)) => {
                assert!(
                    !message.is_empty(),
                    "expected a non-empty diagnostic message from the native bridge"
                );
            }
            other => panic!("expected KernelError::Internal(_), got {other:?}"),
        }
    }

    #[test]
    fn two_contexts_never_mint_colliding_handle_ids() {
        let mut context_a = Context::new().unwrap();
        let mut context_b = Context::new().unwrap();
        let shape_a = context_a.create_box(1.0, 1.0, 1.0).unwrap();
        let shape_b = context_b.create_box(1.0, 1.0, 1.0).unwrap();
        assert_ne!(shape_a.raw(), shape_b.raw());
    }

    #[test]
    fn dropping_a_context_with_outstanding_shapes_does_not_panic_or_abort() {
        let mut context = Context::new().unwrap();
        let _shape = context.create_box(5.0, 5.0, 5.0).unwrap();
        drop(context); // must not panic/abort; native side cleans up.
    }
}
