//! Safe Rust wrapper around the native kernel context (AICAD-018).

use cad_kernel_api::{KernelError, KernelResult, KernelSolid, RawHandle};

use crate::ffi::{self, AicadShapeHandle, AicadStatus};

/// A live native kernel context and everything it owns.
///
/// Deliberately holds a raw pointer and therefore is neither `Send` nor
/// `Sync` (Rust infers this automatically from the `*mut` field; no
/// `unsafe impl` is given). Stage-1 kernel policy #9 requires treating
/// the kernel context as single-thread-affine until an approved
/// architecture decision says otherwise — this makes that the type
/// system's problem, not a documentation-only convention.
pub struct KernelContext {
    ptr: *mut ffi::AicadKernelContext,
}

impl KernelContext {
    /// Creates a new, independent native kernel context.
    pub fn new() -> KernelResult<Self> {
        let mut ptr: *mut ffi::AicadKernelContext = std::ptr::null_mut();
        // SAFETY: `&mut ptr` is a valid, uniquely-owned `*mut *mut
        // AicadKernelContext` for the duration of this call, matching
        // the header's documented contract.
        let raw_status = unsafe { ffi::aicad_context_create(&mut ptr) };
        match AicadStatus::from_raw(raw_status) {
            AicadStatus::Ok => {
                if ptr.is_null() {
                    return Err(KernelError::Unknown(
                        "aicad_context_create reported OK but returned a null context".into(),
                    ));
                }
                Ok(Self { ptr })
            }
            other => Err(status_to_error(other, "aicad_context_create")),
        }
    }

    /// Constructs an axis-aligned box of extents `dx * dy * dz`.
    ///
    /// Dimensions are plain `f64`, not a typed `Length` — `cad-units`
    /// (AICAD-047-049) does not exist yet, and the native ABI itself is
    /// explicitly unit-unaware (see the header's doc comment); typed-unit
    /// integration at this boundary is future work, not Stage 1's.
    pub fn create_box(&mut self, dx: f64, dy: f64, dz: f64) -> KernelResult<KernelSolid> {
        let mut handle = AicadShapeHandle::ZERO;
        // SAFETY: `self.ptr` is non-null and was created by
        // `aicad_context_create` and not yet destroyed (destruction only
        // happens in `Drop`, which consumes `self`). `&mut handle` is a
        // valid out-param pointer for the duration of this call.
        let raw_status = unsafe { ffi::aicad_create_box(self.ptr, dx, dy, dz, &mut handle) };
        match AicadStatus::from_raw(raw_status) {
            AicadStatus::Ok => Ok(KernelSolid::from_raw(to_raw_handle(handle))),
            other => Err(status_to_error(other, "aicad_create_box")),
        }
    }

    /// Exact volume of the solid `handle` currently refers to.
    pub fn shape_volume(&self, handle: KernelSolid) -> KernelResult<f64> {
        let mut volume = 0.0_f64;
        // SAFETY: `self.ptr` is non-null and live; `&mut volume` is a
        // valid out-param pointer for the duration of this call.
        let raw_status = unsafe {
            ffi::aicad_shape_volume(self.ptr, from_raw_handle(handle.raw()), &mut volume)
        };
        match AicadStatus::from_raw(raw_status) {
            AicadStatus::Ok => Ok(volume),
            other => Err(status_to_error(other, "aicad_shape_volume")),
        }
    }

    /// Releases `handle`. After this call, `handle` (and any copy of it)
    /// is stale: the underlying slot may be reused for new geometry, but
    /// the native layer's generation bump guarantees this handle will
    /// never resolve to that new geometry.
    pub fn destroy_solid(&mut self, handle: KernelSolid) -> KernelResult<()> {
        // SAFETY: `self.ptr` is non-null and live.
        let raw_status =
            unsafe { ffi::aicad_shape_destroy(self.ptr, from_raw_handle(handle.raw())) };
        match AicadStatus::from_raw(raw_status) {
            AicadStatus::Ok => Ok(()),
            other => Err(status_to_error(other, "aicad_shape_destroy")),
        }
    }
}

impl Drop for KernelContext {
    fn drop(&mut self) {
        // SAFETY: `self.ptr` was created by `aicad_context_create` and
        // has not been destroyed yet (Drop runs at most once).
        // `aicad_context_destroy` treats a null pointer as a no-op, so
        // this is safe even if construction somehow left it null.
        unsafe {
            ffi::aicad_context_destroy(self.ptr);
        }
    }
}

fn to_raw_handle(h: AicadShapeHandle) -> RawHandle {
    RawHandle::new(h.context_id, h.slot, h.generation)
}

fn from_raw_handle(h: RawHandle) -> AicadShapeHandle {
    AicadShapeHandle {
        context_id: h.context_id,
        slot: h.slot,
        generation: h.generation,
    }
}

fn status_to_error(status: AicadStatus, op: &str) -> KernelError {
    match status {
        AicadStatus::Ok => unreachable!("status_to_error must not be called with Ok"),
        AicadStatus::InvalidArgument => {
            KernelError::InvalidArgument(format!("{op} rejected an argument"))
        }
        AicadStatus::InvalidHandle => KernelError::InvalidHandle,
        AicadStatus::StaleHandle => KernelError::StaleHandle,
        AicadStatus::ForeignContext => KernelError::ForeignContext,
        AicadStatus::NativeException => {
            KernelError::NativeFailure(format!("{op} raised a native (OCCT) exception"))
        }
        AicadStatus::UnknownError => {
            KernelError::Unknown(format!("{op} returned an unrecognized status code"))
        }
    }
}
