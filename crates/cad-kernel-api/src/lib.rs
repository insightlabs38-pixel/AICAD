//! `cad-kernel-api` — backend-independent kernel handle/error types
//! (AICAD-017).
//!
//! This crate defines the kernel-neutral vocabulary `cad-occt-bridge`
//! (AICAD-018) implements against and every layer above the kernel
//! adapter is allowed to see: opaque topology/geometry handles
//! ([`KernelVertex`], [`KernelEdge`], [`KernelWire`], [`KernelFace`],
//! [`KernelShell`], [`KernelSolid`], [`KernelShape`], [`KernelCurve`],
//! [`KernelSurface`]) and [`KernelError`], per
//! `docs/plan/01_SYSTEM_ARCHITECTURE.md` §2.6 and RFC-0002 §3
//! (`project/DECISION_LOG.md#DL-5`).
//!
//! No OCCT (or any other backend's) type or enum value appears anywhere
//! in this crate — that is the entire point of the kernel-independence
//! contract this crate exists to operationalize. It also owns no source
//! syntax, semantic references, user-facing package abstractions, or AI
//! logic (see `README.md`).
//!
//! # Not a semantic reference
//!
//! Every handle type here is **build/epoch-local** (Stage-1 kernel
//! policy #10 / RFC-0002 §4): it identifies an entity within one
//! geometry evaluation epoch of one kernel context and becomes invalid
//! after a topology-mutating operation affects it, or after the owning
//! context is destroyed. These types must never be treated as, or
//! substituted for, a durable AICAD semantic reference — that is
//! `cad-references`' job (RFC-0003), built *from* per-operation lineage
//! evidence the kernel adapter exposes, not from these handles
//! themselves.

#![forbid(unsafe_code)]

use std::fmt;

/// Backend-independent identity for one kernel-resident entity, scoped to
/// a single kernel context and shape-table generation.
///
/// This mirrors (but does not re-export or depend on)
/// `native/occt_bridge/include/aicad_occt_bridge.h`'s
/// `aicad_shape_handle_t` shape by design: `cad-occt-bridge` (AICAD-018)
/// is expected to construct a [`KernelId`] directly from the fields of an
/// `aicad_shape_handle_t` it receives from the native bridge, without
/// this crate ever depending on that header or on OCCT.
///
/// `context_id` distinguishes handles minted by different kernel
/// contexts (a handle from context A must never be accepted by context
/// B); `slot` addresses an entry in that context's shape table;
/// `generation` distinguishes a slot's successive occupants so a handle
/// captured before a release can never resolve to a later shape that
/// reused the same slot. See `project/reports/AICAD-016.md` for the
/// native-side proof of these properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KernelId {
    pub context_id: u64,
    pub slot: u32,
    pub generation: u32,
}

impl fmt::Display for KernelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KernelId(context={}, slot={}, generation={})",
            self.context_id, self.slot, self.generation
        )
    }
}

/// Defines one opaque, backend-independent kernel handle type wrapping a
/// [`KernelId`]. Each generated type is a distinct nominal type (not a
/// type alias) specifically so that, for example, a [`KernelFace`] and a
/// [`KernelEdge`] can never be accidentally interchanged at a call site,
/// even though both are structurally just a [`KernelId`].
macro_rules! kernel_handle {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub KernelId);

        impl $name {
            /// Constructs a handle from its backend-assigned identity.
            /// Only `cad-occt-bridge` (or another future kernel adapter)
            /// should call this — everything above the kernel adapter
            /// receives handles, it does not mint them.
            pub const fn from_id(id: KernelId) -> Self {
                Self(id)
            }

            /// Returns the backend-independent identity underlying this
            /// handle.
            pub const fn id(&self) -> KernelId {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "({})"), self.0)
            }
        }

        impl From<KernelId> for $name {
            fn from(id: KernelId) -> Self {
                Self::from_id(id)
            }
        }
    };
}

kernel_handle!(
    /// A single point-like topological entity (OCCT's `TopoDS_Vertex`,
    /// kept nameless here per the kernel-independence contract).
    KernelVertex
);
kernel_handle!(
    /// A single curve-bounded topological entity connecting two
    /// [`KernelVertex`] handles.
    KernelEdge
);
kernel_handle!(
    /// An ordered/connected collection of [`KernelEdge`] handles forming
    /// one boundary loop or open chain.
    KernelWire
);
kernel_handle!(
    /// A bounded region of one [`KernelSurface`], bounded by one or more
    /// [`KernelWire`] handles.
    KernelFace
);
kernel_handle!(
    /// A connected collection of [`KernelFace`] handles.
    KernelShell
);
kernel_handle!(
    /// A solid bounded by one or more [`KernelShell`] handles.
    KernelSolid
);
kernel_handle!(
    /// A general topological container that does not commit to a more
    /// specific kind above (compounds, or a shape whose kind the caller
    /// has not yet inspected). Domain-level operations should prefer the
    /// most specific handle type they can; `KernelShape` exists because
    /// some kernel adapter operations (e.g. `create_box`) legitimately
    /// return a result of unspecified/mixed topological kind until
    /// classified.
    KernelShape
);
kernel_handle!(
    /// A mathematical curve (line, circle, B-spline, ...) independent of
    /// any topological embedding.
    KernelCurve
);
kernel_handle!(
    /// A mathematical surface (plane, cylinder, B-spline, ...)
    /// independent of any topological embedding.
    KernelSurface
);

/// Backend-independent kernel failure classification.
///
/// This is the neutral vocabulary every kernel adapter (starting with
/// `cad-occt-bridge`) must normalize its own failures into — no adapter
/// may surface a backend-specific status value above itself (Stage-1
/// kernel policy #6-7; RFC-0002 §3). Variant names intentionally mirror
/// `native/occt_bridge/include/aicad_occt_bridge.h`'s
/// `aicad_occt_status_t` (minus `OK`, which this crate represents as
/// `Result::Ok` instead) so that `cad-occt-bridge`'s translation from the
/// native status is a direct one-to-one mapping, not a redesign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KernelError {
    /// A caller-supplied argument was invalid (e.g. a non-positive
    /// dimension, a null/absent required value).
    InvalidArgument,
    /// The handle does not address any slot the kernel context has ever
    /// allocated.
    InvalidHandle,
    /// The handle addressed a slot that has since been released or
    /// superseded (a different generation now occupies it) — expected
    /// and routine after a topology-mutating operation, per RFC-0002 §4.
    StaleHandle,
    /// The handle belongs to a different kernel context than the one it
    /// was used with.
    ForeignContext,
    /// The kernel context was used from a thread other than the one that
    /// created it (kernel contexts are conservatively single-thread-affine,
    /// Stage-1 kernel policy #9).
    WrongThread,
    /// The kernel backend could not complete the requested operation for
    /// this input (e.g. a degenerate geometric configuration) — an
    /// expected outcome for some inputs, not necessarily an adapter
    /// defect.
    OperationFailed,
    /// An unexpected internal failure was caught and normalized at the
    /// kernel adapter boundary. Always indicates an adapter or backend
    /// defect, never an expected outcome.
    Internal,
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            KernelError::InvalidArgument => "invalid argument",
            KernelError::InvalidHandle => "handle does not address any allocated slot",
            KernelError::StaleHandle => "handle is stale (released or superseded)",
            KernelError::ForeignContext => "handle belongs to a different kernel context",
            KernelError::WrongThread => "kernel context used from a non-owning thread",
            KernelError::OperationFailed => "kernel backend could not complete the operation",
            KernelError::Internal => "internal kernel adapter failure",
        };
        f.write_str(message)
    }
}

impl std::error::Error for KernelError {}

/// Convenience alias for a kernel-adapter result.
pub type KernelResult<T> = Result<T, KernelError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn sample_id(slot: u32, generation: u32) -> KernelId {
        KernelId {
            context_id: 1,
            slot,
            generation,
        }
    }

    #[test]
    fn distinct_handle_types_are_not_interchangeable_at_the_type_level() {
        // This test's real assertion is that the file compiles: KernelFace
        // and KernelEdge are distinct nominal types even though both wrap
        // a KernelId with the same shape. `same_id` below intentionally
        // demonstrates that two DIFFERENT handle types constructed from
        // the same KernelId are not `==`-comparable to each other (no
        // blanket PartialEq across handle kinds exists), which is the
        // whole point of using distinct newtypes here.
        let same_id = sample_id(3, 1);
        let face = KernelFace::from_id(same_id);
        let edge = KernelEdge::from_id(same_id);
        assert_eq!(face.id(), same_id);
        assert_eq!(edge.id(), same_id);
        assert_eq!(face.id(), edge.id()); // ids are equal
        // `face == edge` would be a compile error here, which is the property under test.
    }

    #[test]
    fn handles_are_hashable_and_usable_as_set_keys() {
        let mut set: HashSet<KernelSolid> = HashSet::new();
        set.insert(KernelSolid::from_id(sample_id(0, 1)));
        set.insert(KernelSolid::from_id(sample_id(0, 1))); // duplicate
        set.insert(KernelSolid::from_id(sample_id(0, 2))); // different generation
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn stale_generation_is_a_distinct_id_even_in_the_same_slot() {
        let original = KernelVertex::from_id(sample_id(5, 1));
        let reused_slot = KernelVertex::from_id(sample_id(5, 2));
        assert_ne!(
            original, reused_slot,
            "a released slot's old handle must never equal a reused slot's new handle"
        );
    }

    #[test]
    fn display_impls_are_human_readable_and_non_empty() {
        let id = sample_id(7, 3);
        assert_eq!(format!("{id}"), "KernelId(context=1, slot=7, generation=3)");
        assert_eq!(
            format!("{}", KernelFace::from_id(id)),
            format!("KernelFace({id})")
        );
        for error in [
            KernelError::InvalidArgument,
            KernelError::InvalidHandle,
            KernelError::StaleHandle,
            KernelError::ForeignContext,
            KernelError::WrongThread,
            KernelError::OperationFailed,
            KernelError::Internal,
        ] {
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn kernel_error_implements_std_error() {
        fn assert_error<E: std::error::Error>(_: &E) {}
        assert_error(&KernelError::Internal);
    }

    #[test]
    fn kernel_result_alias_composes_normally() {
        fn maybe_fail(ok: bool) -> KernelResult<KernelSolid> {
            if ok {
                Ok(KernelSolid::from_id(sample_id(0, 1)))
            } else {
                Err(KernelError::OperationFailed)
            }
        }
        assert!(maybe_fail(true).is_ok());
        assert_eq!(maybe_fail(false).unwrap_err(), KernelError::OperationFailed);
    }
}
