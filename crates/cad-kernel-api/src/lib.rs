//! `cad-kernel-api` — backend-independent kernel handle/error types
//! (AICAD-017).
//!
//! This crate owns exactly what its `README.md` says it owns: the
//! kernel-neutral handle types (`KernelCurve`, `KernelSurface`,
//! `KernelShape`, `KernelVertex`, `KernelEdge`, `KernelWire`,
//! `KernelFace`, `KernelShell`, `KernelSolid`) and the kernel error
//! type. It must never depend on `cad-occt-bridge`, on the native
//! bridge, or on OCCT — the dependency direction is the other way
//! (`cad-occt-bridge` implements against these types). Kernel context
//! lifecycle and the actual shape-handle table live in
//! `cad-occt-bridge` (AICAD-018/019), not here.

mod error;
mod handle;

pub use error::{KernelError, KernelResult};
pub use handle::{
    KernelCurve, KernelEdge, KernelFace, KernelShape, KernelShell, KernelSolid, KernelSurface,
    KernelVertex, KernelWire, RawHandle,
};
