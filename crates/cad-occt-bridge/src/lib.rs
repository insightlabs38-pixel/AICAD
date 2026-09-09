//! `cad-occt-bridge` — safe Rust wrapper around
//! `native/occt_bridge`'s C ABI (AICAD-018).
//!
//! This is the **only** crate permitted to reference
//! `native/occt_bridge`/OCCT at all (RFC-0002 §3,
//! `project/DECISION_LOG.md#DL-5`). The [`ffi`] module holds the raw,
//! `unsafe`, one-to-one `extern "C"` declarations matching
//! `native/occt_bridge/include/aicad_occt_bridge.h` exactly; everything
//! else in this crate is a safe wrapper that never leaks an
//! `ffi`-module type, an OCCT type, or a raw pointer through its public
//! API. Handle identity above this module is expressed only in
//! `cad-kernel-api`'s kernel-neutral vocabulary
//! ([`cad_kernel_api::KernelShape`], [`cad_kernel_api::KernelId`]).

mod ffi;

use cad_kernel_api::{KernelError, KernelId, KernelResult, KernelShape};
use std::os::raw::c_int;

/// Converts one native `aicad_occt_status_t` value into a
/// [`KernelResult<()>`], per the one-to-one mapping documented on
/// [`cad_kernel_api::KernelError`] and `native/occt_bridge/include/aicad_occt_bridge.h`.
///
/// The native side is entirely under this workspace's control
/// (`native/occt_bridge/src/aicad_occt_bridge.cpp`), so every status
/// value it can ever produce is one of the eight enumerated here.
/// Nonetheless, any value this match does not recognize is treated as
/// [`KernelError::Internal`] rather than panicking or invoking undefined
/// behavior -- matching an unenumerated C `int` defensively is always
/// safe, whereas transmuting it into a Rust `#[repr(i32)]` enum would not
/// be.
fn status_result(raw: c_int) -> KernelResult<()> {
    match raw {
        0 => Ok(()),
        1 => Err(KernelError::InvalidArgument),
        2 => Err(KernelError::InvalidHandle),
        3 => Err(KernelError::StaleHandle),
        4 => Err(KernelError::ForeignContext),
        5 => Err(KernelError::WrongThread),
        6 => Err(KernelError::OperationFailed),
        _ => Err(KernelError::Internal), // 7 (ERR_INTERNAL), or anything unrecognized.
    }
}

fn id_to_handle(id: KernelId) -> ffi::aicad_shape_handle_t {
    ffi::aicad_shape_handle_t {
        context_id: id.context_id,
        slot: id.slot,
        generation: id.generation,
    }
}

fn handle_to_id(handle: ffi::aicad_shape_handle_t) -> KernelId {
    KernelId {
        context_id: handle.context_id,
        slot: handle.slot,
        generation: handle.generation,
    }
}

/// A safe, RAII-owning wrapper around one native OCCT kernel context.
///
/// Dropping an `OcctContext` destroys the underlying native context
/// (`aicad_occt_context_destroy`). Every [`Shape`] it produced borrows
/// this context for its own lifetime (`Shape<'ctx>`), so the borrow
/// checker rejects any attempt to drop the context while a `Shape` it
/// owns is still alive -- "released/stale handles must never accidentally
/// alias newly created geometry" and "kernel topology handles are
/// epoch/build-local" (Stage-1 kernel policies) hold at compile time
/// here, not only via the native bridge's own runtime checks
/// (`project/reports/AICAD-016.md`).
///
/// `OcctContext` is neither `Send` nor `Sync`: it holds a raw pointer,
/// so Rust does not implement either auto trait for it, matching the
/// native bridge's single-thread-affine contract (Stage-1 kernel policy
/// #9) at the type level, not only via the native bridge's own
/// `WRONG_THREAD` runtime check.
#[derive(Debug)]
pub struct OcctContext {
    raw: *mut ffi::aicad_occt_context_t,
}

impl OcctContext {
    /// Creates a new, independent kernel context.
    pub fn new() -> KernelResult<Self> {
        let mut raw: *mut ffi::aicad_occt_context_t = std::ptr::null_mut();
        // SAFETY: `&mut raw` is a valid, uniquely-owned `*mut *mut
        // aicad_occt_context_t` for the duration of this call, matching
        // the header's contract for `out_context`.
        let status = unsafe { ffi::aicad_occt_context_create(&mut raw) };
        status_result(status)?;
        debug_assert!(
            !raw.is_null(),
            "AICAD_OCCT_OK must set *out_context to a non-null value"
        );
        Ok(Self { raw })
    }

    /// Constructs an axis-aligned box of the given dimensions and returns
    /// a [`Shape`] owning it, borrowed from this context.
    pub fn create_box(&self, dx: f64, dy: f64, dz: f64) -> KernelResult<Shape<'_>> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `self.raw` is a valid context for `self`'s lifetime
        // (invariant maintained by this type); `&mut handle` is a valid
        // out-param per the header's contract.
        let status = unsafe { ffi::aicad_occt_create_box(self.raw, dx, dy, dz, &mut handle) };
        status_result(status)?;
        Ok(Shape {
            context: self,
            id: handle_to_id(handle),
        })
    }
}

impl Drop for OcctContext {
    fn drop(&mut self) {
        // SAFETY: `self.raw` was created by `aicad_occt_context_create`
        // and has not yet been destroyed (this is the only place that
        // destroys it, and it runs at most once per `OcctContext`). Every
        // `Shape` referencing this context has already been dropped by
        // the time this runs -- the borrow checker enforces that via
        // `Shape<'ctx>`'s lifetime.
        let status = unsafe { ffi::aicad_occt_context_destroy(self.raw) };
        debug_assert_eq!(
            status, 0,
            "aicad_occt_context_destroy failed during OcctContext::drop"
        );
    }
}

/// A shape owned by one [`OcctContext`], automatically released
/// (`aicad_occt_release_shape`) when dropped.
#[derive(Debug)]
pub struct Shape<'ctx> {
    context: &'ctx OcctContext,
    id: KernelId,
}

impl<'ctx> Shape<'ctx> {
    /// The backend-independent handle this shape can be exchanged as
    /// through `cad-kernel-api`'s kernel-neutral vocabulary. The returned
    /// [`KernelShape`] carries no lifetime and does not keep this
    /// `Shape`'s native resource alive by itself -- it is a value-only
    /// identity, matching the rest of `cad-kernel-api`.
    pub fn handle(&self) -> KernelShape {
        KernelShape::from_id(self.id)
    }

    /// Whether OCCT's own topology checker (`BRepCheck_Analyzer`)
    /// considers this shape valid.
    pub fn is_valid(&self) -> KernelResult<bool> {
        let mut is_valid: c_int = 0;
        // SAFETY: `self.context.raw` is valid for at least `'ctx`
        // (enforced by the borrow this `Shape` holds); `self.raw_handle()`
        // addresses a slot this `Shape` owns and has not yet released;
        // `&mut is_valid` is a valid out-param.
        let status = unsafe {
            ffi::aicad_occt_shape_is_valid(self.context.raw, self.raw_handle(), &mut is_valid)
        };
        status_result(status)?;
        Ok(is_valid != 0)
    }

    /// This shape's volume, per OCCT's `BRepGProp::VolumeProperties`, in
    /// the kernel's internal linear unit (unit semantics belong to
    /// `cad-units` above this crate, not here).
    pub fn volume(&self) -> KernelResult<f64> {
        let mut volume: f64 = 0.0;
        // SAFETY: see `is_valid` above; identical argument.
        let status = unsafe {
            ffi::aicad_occt_shape_volume(self.context.raw, self.raw_handle(), &mut volume)
        };
        status_result(status)?;
        Ok(volume)
    }

    fn raw_handle(&self) -> ffi::aicad_shape_handle_t {
        id_to_handle(self.id)
    }
}

impl<'ctx> Drop for Shape<'ctx> {
    fn drop(&mut self) {
        // SAFETY: see `is_valid`'s SAFETY comment; identical argument.
        // The result is intentionally ignored: `Drop::drop` cannot
        // propagate an error, and a release failing here would indicate a
        // defect this crate's own tests are responsible for catching
        // directly (e.g. a double-release bug), not something to panic
        // over during unwind-sensitive drop code.
        let _ = unsafe { ffi::aicad_occt_release_shape(self.context.raw, self.raw_handle()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_create_and_destroy_succeeds() {
        let context = OcctContext::new().expect("context creation should succeed");
        drop(context);
    }

    #[test]
    fn create_box_is_valid_and_has_the_expected_volume() {
        let context = OcctContext::new().unwrap();
        let shape = context
            .create_box(1.0, 2.0, 3.0)
            .expect("create_box should succeed");
        assert!(
            shape.is_valid().unwrap(),
            "a freshly created box must be a valid B-rep"
        );
        let volume = shape.volume().unwrap();
        assert!(
            (volume - 6.0).abs() < 1e-9,
            "expected volume 6.0, got {volume}"
        );
    }

    #[test]
    fn create_box_rejects_invalid_dimensions() {
        let context = OcctContext::new().unwrap();
        assert_eq!(
            context.create_box(0.0, 1.0, 1.0).unwrap_err(),
            KernelError::InvalidArgument
        );
        assert_eq!(
            context.create_box(-1.0, 1.0, 1.0).unwrap_err(),
            KernelError::InvalidArgument
        );
        assert_eq!(
            context.create_box(f64::NAN, 1.0, 1.0).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn multiple_shapes_in_one_context_have_independent_lifecycles() {
        let context = OcctContext::new().unwrap();
        let a = context.create_box(1.0, 1.0, 1.0).unwrap(); // volume 1
        let b = context.create_box(2.0, 2.0, 2.0).unwrap(); // volume 8

        assert!((a.volume().unwrap() - 1.0).abs() < 1e-9);
        assert!((b.volume().unwrap() - 8.0).abs() < 1e-9);

        // Dropping `a` (which releases its native handle) must not affect
        // `b`, which occupies a different slot in the same context's
        // shape table.
        drop(a);
        assert!(
            (b.volume().unwrap() - 8.0).abs() < 1e-9,
            "releasing one shape must not disturb an unrelated shape in the same context"
        );
    }

    #[test]
    fn dropping_and_recreating_reuses_the_slot_without_aliasing() {
        let context = OcctContext::new().unwrap();
        let first = context.create_box(1.0, 1.0, 1.0).unwrap(); // volume 1
        let first_handle = first.handle();
        drop(first); // releases the native handle

        let second = context.create_box(5.0, 5.0, 5.0).unwrap(); // volume 125, likely reuses the freed slot
        let second_handle = second.handle();

        assert_ne!(
            first_handle, second_handle,
            "a released handle's identity must never equal a new shape's handle, even if the slot was reused"
        );
        assert!((second.volume().unwrap() - 125.0).abs() < 1e-9);
    }

    #[test]
    fn two_contexts_are_fully_independent() {
        let context_a = OcctContext::new().unwrap();
        let context_b = OcctContext::new().unwrap();
        let shape_a = context_a.create_box(3.0, 3.0, 3.0).unwrap(); // volume 27
        let shape_b = context_b.create_box(4.0, 4.0, 4.0).unwrap(); // volume 64
        assert!((shape_a.volume().unwrap() - 27.0).abs() < 1e-9);
        assert!((shape_b.volume().unwrap() - 64.0).abs() < 1e-9);
    }

    /// AICAD-019: proves `OcctContext` is safe to use concurrently across
    /// real OS threads as long as each thread owns its own context (the
    /// pattern the single-thread-affine design is meant to support).
    /// Each `OcctContext` is created *inside* its owning thread's closure
    /// -- `OcctContext` is neither `Send` nor `Sync`, so the type system
    /// would reject any attempt to create one thread-side and move or
    /// share it into another, which is exactly the property under test.
    #[test]
    fn many_contexts_are_safe_across_real_threads() {
        const THREAD_COUNT: usize = 8;
        const SHAPES_PER_THREAD: usize = 50;

        let results: Vec<bool> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..THREAD_COUNT)
                .map(|t| {
                    scope.spawn(move || {
                        let context = OcctContext::new()
                            .expect("context creation should succeed on a worker thread");
                        for i in 0..SHAPES_PER_THREAD {
                            let side = 1.0 + (t as f64) * 0.01 + (i as f64) * 0.0001;
                            let shape = context
                                .create_box(side, side, side)
                                .expect("create_box should succeed on a worker thread");
                            let expected = side * side * side;
                            let volume = shape
                                .volume()
                                .expect("volume should succeed on a worker thread");
                            if (volume - expected).abs() >= 1e-6 {
                                return false;
                            }
                        }
                        true
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

        assert!(
            results.iter().all(|&ok| ok),
            "every worker thread's independently-owned context must produce correct, non-aliased results"
        );
    }
}
