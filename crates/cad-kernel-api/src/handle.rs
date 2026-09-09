//! Backend-independent kernel handles (AICAD-017).
//!
//! Mirrors `native/occt_bridge/include/aicad_occt_bridge.h`'s
//! `AicadShapeHandle` shape exactly (three plain integers: context id,
//! slot, generation) but this module itself does not depend on the
//! native bridge or on OCCT in any way -- it is pure, backend-neutral
//! data (Stage-1 kernel policy: the public kernel abstraction is
//! kernel-neutral; DL-5). `crates/cad-occt-bridge` (AICAD-018) is the
//! only crate that constructs these from a real native handle.

/// Raw handle identity: (owning context, slot, generation). Never a
/// pointer. Equality/hashing use all three fields, so a handle from one
/// context can never compare equal to a handle from another even if
/// their slot/generation happen to coincide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawHandle {
    pub context_id: u64,
    pub slot: u32,
    pub generation: u32,
}

impl RawHandle {
    pub const fn new(context_id: u64, slot: u32, generation: u32) -> Self {
        Self {
            context_id,
            slot,
            generation,
        }
    }
}

/// Declares a domain-shaped kernel handle newtype wrapping [`RawHandle`].
/// One newtype per concept in `docs/plan/01_SYSTEM_ARCHITECTURE.md`
/// §2.6's kernel-adapter list, so callers above this crate never see a
/// bare, kind-less handle and cannot pass a `KernelFace` where a
/// `KernelSolid` is expected.
macro_rules! kernel_handle {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(RawHandle);

        impl $name {
            pub const fn from_raw(raw: RawHandle) -> Self {
                Self(raw)
            }

            pub const fn raw(&self) -> RawHandle {
                self.0
            }
        }
    };
}

kernel_handle!(
    /// A shape of unspecified/mixed kind (compound, or not yet
    /// classified into a more specific handle below).
    KernelShape
);
kernel_handle!(
    /// A single point-like topological vertex.
    KernelVertex
);
kernel_handle!(
    /// A single topological edge.
    KernelEdge
);
kernel_handle!(
    /// An ordered/connected chain of edges.
    KernelWire
);
kernel_handle!(
    /// A bounded piece of surface.
    KernelFace
);
kernel_handle!(
    /// A connected set of faces.
    KernelShell
);
kernel_handle!(
    /// A solid (closed, manifold volume).
    KernelSolid
);
kernel_handle!(
    /// A standalone curve (not yet trimmed/bounded into an edge).
    KernelCurve
);
kernel_handle!(
    /// A standalone surface (not yet trimmed/bounded into a face).
    KernelSurface
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_from_distinct_contexts_are_never_equal() {
        let a = KernelSolid::from_raw(RawHandle::new(1, 0, 1));
        let b = KernelSolid::from_raw(RawHandle::new(2, 0, 1));
        assert_ne!(a, b);
    }

    #[test]
    fn handles_with_distinct_generations_are_never_equal() {
        let a = KernelSolid::from_raw(RawHandle::new(1, 0, 1));
        let b = KernelSolid::from_raw(RawHandle::new(1, 0, 2));
        assert_ne!(
            a, b,
            "a stale handle must not equal its slot's new occupant"
        );
    }

    #[test]
    fn round_trips_through_raw() {
        let raw = RawHandle::new(7, 3, 2);
        let h = KernelFace::from_raw(raw);
        assert_eq!(h.raw(), raw);
    }
}
