//! `cad-occt-bridge` — safe Rust wrapper around `native/occt_bridge`
//! (AICAD-018), implementing the kernel-neutral types from
//! `cad-kernel-api` against Open CASCADE Technology.
//!
//! Exposes only [`KernelContext`] and the handle/error types
//! re-exported from `cad-kernel-api` — never a raw FFI type or an OCCT
//! name. `ffi` is private: nothing outside this crate may call the
//! native ABI directly.

mod context;
mod ffi;

pub use cad_kernel_api::{KernelError, KernelResult, KernelSolid, RawHandle};
pub use context::KernelContext;
