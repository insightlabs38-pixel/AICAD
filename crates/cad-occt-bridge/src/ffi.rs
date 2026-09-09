//! Raw, unsafe FFI declarations mirroring
//! `native/occt_bridge/include/aicad_occt_bridge.h` exactly.
//!
//! Nothing in this module is safe to call directly from outside this
//! crate; [`crate::context::KernelContext`] is the safe wrapper. Status
//! codes cross the FFI boundary as a raw `c_int` (not a Rust enum) and
//! are decoded on the Rust side by [`AicadStatus::from_raw`] --
//! `#[repr(C)]` enum layout for C's plain (non-fixed-width) `enum` is
//! platform/compiler-dependent, so trusting an FFI return value to
//! already be a well-formed Rust enum discriminant is not safe.

use std::os::raw::c_int;

/// Opaque native kernel context. Only ever used through a raw pointer.
#[repr(C)]
pub struct AicadKernelContext {
    _private: [u8; 0],
}

/// Mirrors `AicadShapeHandle` in the C header field-for-field.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AicadShapeHandle {
    pub context_id: u64,
    pub slot: u32,
    pub generation: u32,
}

impl AicadShapeHandle {
    pub const ZERO: AicadShapeHandle = AicadShapeHandle {
        context_id: 0,
        slot: 0,
        generation: 0,
    };
}

/// Decoded form of the raw `AicadStatus` C enum. See
/// `native/occt_bridge/include/aicad_occt_bridge.h` for the authoritative
/// numbering; kept in sync manually since there is deliberately no
/// codegen step (bindgen) for this small, hand-maintained ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AicadStatus {
    Ok,
    InvalidArgument,
    InvalidHandle,
    StaleHandle,
    ForeignContext,
    NativeException,
    UnknownError,
}

impl AicadStatus {
    pub fn from_raw(raw: c_int) -> Self {
        match raw {
            0 => AicadStatus::Ok,
            1 => AicadStatus::InvalidArgument,
            2 => AicadStatus::InvalidHandle,
            3 => AicadStatus::StaleHandle,
            4 => AicadStatus::ForeignContext,
            5 => AicadStatus::NativeException,
            // Any value the header does not define (including a future
            // addition this crate has not been updated for yet) is
            // treated as unknown rather than guessed at.
            _ => AicadStatus::UnknownError,
        }
    }
}

unsafe extern "C" {
    pub fn aicad_context_create(out_ctx: *mut *mut AicadKernelContext) -> c_int;
    pub fn aicad_context_destroy(ctx: *mut AicadKernelContext) -> c_int;
    pub fn aicad_create_box(
        ctx: *mut AicadKernelContext,
        dx: f64,
        dy: f64,
        dz: f64,
        out_handle: *mut AicadShapeHandle,
    ) -> c_int;
    pub fn aicad_shape_volume(
        ctx: *mut AicadKernelContext,
        handle: AicadShapeHandle,
        out_volume: *mut f64,
    ) -> c_int;
    pub fn aicad_shape_destroy(ctx: *mut AicadKernelContext, handle: AicadShapeHandle) -> c_int;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_raw_decodes_every_documented_status() {
        assert_eq!(AicadStatus::from_raw(0), AicadStatus::Ok);
        assert_eq!(AicadStatus::from_raw(1), AicadStatus::InvalidArgument);
        assert_eq!(AicadStatus::from_raw(2), AicadStatus::InvalidHandle);
        assert_eq!(AicadStatus::from_raw(3), AicadStatus::StaleHandle);
        assert_eq!(AicadStatus::from_raw(4), AicadStatus::ForeignContext);
        assert_eq!(AicadStatus::from_raw(5), AicadStatus::NativeException);
    }

    #[test]
    fn from_raw_treats_undocumented_values_as_unknown() {
        assert_eq!(AicadStatus::from_raw(6), AicadStatus::UnknownError);
        assert_eq!(AicadStatus::from_raw(999), AicadStatus::UnknownError);
        assert_eq!(AicadStatus::from_raw(-1), AicadStatus::UnknownError);
    }
}
