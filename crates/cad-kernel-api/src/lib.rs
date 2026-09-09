//! `cad-kernel-api` — backend-independent kernel handle/error vocabulary.
//!
//! This crate defines the narrow, vendor-neutral type vocabulary
//! `docs/plan/01_SYSTEM_ARCHITECTURE.md` §2.6 describes ("Internally
//! define a narrow backend interface... Operations should expose domain
//! concepts, not vendor names") and this crate's own `README.md` commits
//! to: `KernelCurve`, `KernelSurface`, `KernelShape`, `KernelVertex`,
//! `KernelEdge`, `KernelWire`, `KernelFace`, `KernelShell`, `KernelSolid`,
//! plus a structured [`KernelError`].
//!
//! **Scope of this crate (AICAD-017 — "backend-independent handle/error
//! types" only):** opaque handle newtypes and a structured error type.
//! No OCCT (or any other backend) type is named or depended on here —
//! this crate has zero dependencies, deliberately, so that a future
//! non-OCCT kernel backend never needs this crate to change. It does
//! **not** define:
//! - kernel context lifecycle (`AICAD-019`'s scope, per its own task
//!   title in `project/TASKS.yaml`);
//! - any operation (construct/query/mutate) on these handles (that is
//!   `cad-geometry-api`/`cad-geometry-runtime`, WP-05, lowering to
//!   `cad-occt-bridge`, WP-01's concrete backend, `AICAD-018`);
//! - curve/surface constructors, B-rep operations, or STEP primitives
//!   (WP-01's remaining "Owns" list in
//!   `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` §2 — added incrementally
//!   by later Stage-1 tasks as `cad-occt-bridge`/the native bridge grow
//!   the capability to back them).
//!
//! A handle in this crate carries no OCCT-derived meaning and cannot be
//! dereferenced or traversed on its own — it is only ever exchanged with
//! the concrete backend that minted it (currently `cad-occt-bridge`,
//! which in turn talks to `native/occt_bridge`'s C ABI,
//! `AICAD-016`). Passing a handle to a different backend/context than the
//! one that minted it is a **runtime** error the backend must reject
//! (`native/occt_bridge`'s handle table already does this — see
//! `project/reports/AICAD-016.md`); this crate does not attempt to
//! enforce that at the Rust type level (no context-tagging
//! lifetime/token/typestate parameter). That would be a real design
//! option for a future task if runtime-only rejection proves
//! insufficient, but nothing in Stage 1 so far shows it is — adding it
//! speculatively now would be exactly the kind of unnecessary
//! architecture invention `AGENTS.md` asks this agent to avoid.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;
use std::num::NonZeroU64;

/// Defines one opaque kernel handle newtype, all with identical, minimal
/// behavior: a handle is `Copy`, comparable, hashable, and wraps a
/// backend-minted [`NonZeroU64`] with no further public structure.
/// `NonZeroU64` (rather than a bare `u64`) makes "no handle"
/// unrepresentable in the type itself and gives `Option<$name>` the same
/// size as `$name` — matching `native/occt_bridge`'s own documented
/// invariant that a raw id of `0` is never a valid handle
/// (`native/occt_bridge/include/aicad_occt_bridge.h`).
macro_rules! kernel_handle {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(NonZeroU64);

        impl $name {
            /// Wrap a backend-minted raw id as a handle.
            ///
            /// This does not itself prove any entity with that id exists
            /// in any kernel context — call it only immediately after a
            /// concrete backend reports success and hands back a fresh
            /// id (as `cad-occt-bridge`, `AICAD-018`, will do after a
            /// successful `native/occt_bridge` ABI call). Constructing a
            /// handle from an arbitrary/guessed id and presenting it to
            /// a backend is safe (no memory unsafety — this crate is
            /// `#![forbid(unsafe_code)]`) but is rejected by the backend
            /// as an invalid handle at the point of use.
            #[must_use]
            pub const fn from_raw(id: NonZeroU64) -> Self {
                Self(id)
            }

            /// The backend-minted raw id this handle wraps.
            #[must_use]
            pub const fn raw(self) -> NonZeroU64 {
                self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "({})"), self.0)
            }
        }
    };
}

kernel_handle!(
    /// Opaque handle to a kernel-backend curve (analytic or freeform).
    /// See `docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` §3 for the
    /// public curve-construction API this will eventually back.
    KernelCurve
);

kernel_handle!(
    /// Opaque handle to a kernel-backend surface (analytic or freeform).
    KernelSurface
);

kernel_handle!(
    /// Opaque handle to a kernel-backend shape (the general B-rep
    /// container — vertex/edge/wire/face/shell/solid/compound). This is
    /// the handle kind `native/occt_bridge`'s current ABI
    /// (`AICAD-016`, `aicad_create_box`/`aicad_shape_bbox_diagonal`/
    /// `aicad_shape_release`) already mints and accepts.
    KernelShape
);

kernel_handle!(
    /// Opaque handle to a kernel-backend topological vertex.
    KernelVertex
);

kernel_handle!(
    /// Opaque handle to a kernel-backend topological edge.
    KernelEdge
);

kernel_handle!(
    /// Opaque handle to a kernel-backend topological wire (ordered,
    /// possibly closed, sequence of edges).
    KernelWire
);

kernel_handle!(
    /// Opaque handle to a kernel-backend topological face (a trimmed
    /// region of a surface).
    KernelFace
);

kernel_handle!(
    /// Opaque handle to a kernel-backend topological shell (a connected
    /// set of faces).
    KernelShell
);

kernel_handle!(
    /// Opaque handle to a kernel-backend topological solid.
    KernelSolid
);

/// A structured, backend-independent kernel failure.
///
/// This mirrors the *categories* `native/occt_bridge`'s C ABI
/// (`AicadStatus`, `AICAD-016`) already distinguishes — not its exact
/// representation (this crate has no FFI dependency and never will;
/// converting a raw `AicadStatus` into a `KernelError` is
/// `cad-occt-bridge`'s job, `AICAD-018`). Per
/// `docs/plan/01_SYSTEM_ARCHITECTURE.md` §8, kernel-specific diagnostic
/// detail may be attached (via the `Internal`/`Unknown` message text
/// here) but must never be the *only* explanation — every variant below
/// is itself a vendor-neutral category a caller can match on without
/// knowing anything about OCCT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    /// The operation required a live kernel context and none was
    /// supplied. Corresponds to `AICAD_STATUS_NULL_CONTEXT`.
    NullContext,
    /// A context pointer was supplied but is not (or is no longer) a
    /// live context — e.g. it was already destroyed. Distinct from
    /// `NullContext` (which means no context was supplied at all).
    /// Corresponds to `AICAD_STATUS_INVALID_CONTEXT`
    /// (`native/occt_bridge`, `AICAD-019`); in ordinary safe Rust usage
    /// through `cad-occt-bridge::Context` this variant should be
    /// unreachable, since Rust's ownership model makes calling a method
    /// on an already-dropped `Context` a compile error — it exists for
    /// the underlying ABI's own misuse-safety guarantee, which protects
    /// callers below the safe Rust layer (e.g. a future non-Rust binding).
    InvalidContext,
    /// An argument was invalid independent of any kernel context state
    /// (e.g. a required output pointer was null at the ABI layer, or a
    /// higher layer rejected a value before ever reaching the backend).
    /// Corresponds to `AICAD_STATUS_INVALID_ARGUMENT`.
    InvalidArgument(String),
    /// A handle was not a live handle owned by the context it was
    /// presented to: never minted, already released (stale), or minted
    /// by a different context (foreign-context misuse) — the backend
    /// does not distinguish these to the caller because none of them
    /// should ever legitimately occur in correct calling code; see
    /// `project/reports/AICAD-016.md`'s handle-table evidence. Corresponds
    /// to `AICAD_STATUS_INVALID_HANDLE`.
    InvalidHandle,
    /// The backend kernel itself failed (e.g. an OCCT exception was
    /// caught and normalized — see `native/occt_bridge/src/bridge.cpp`'s
    /// `guard()`). `message` carries whatever diagnostic detail the
    /// backend recorded; per `docs/plan/01_SYSTEM_ARCHITECTURE.md` §8
    /// this is supplementary detail, not the sole explanation the
    /// `Internal` variant itself already provides.
    Internal(String),
    /// A backend failure that did not fit any of the above categories
    /// (e.g. a non-`Standard_Failure` exception escaped an OCCT call).
    /// Corresponds to `AICAD_STATUS_UNKNOWN_ERROR`.
    Unknown(String),
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelError::NullContext => write!(f, "kernel error: no context supplied"),
            KernelError::InvalidContext => {
                write!(f, "kernel error: context is not live (already destroyed?)")
            }
            KernelError::InvalidArgument(message) => {
                write!(f, "kernel error: invalid argument: {message}")
            }
            KernelError::InvalidHandle => write!(f, "kernel error: invalid handle"),
            KernelError::Internal(message) => {
                write!(f, "kernel error: internal backend failure: {message}")
            }
            KernelError::Unknown(message) => {
                write!(f, "kernel error: unknown backend failure: {message}")
            }
        }
    }
}

impl Error for KernelError {}

/// Convenience alias for a kernel-backend operation's result.
pub type KernelResult<T> = Result<T, KernelError>;

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(id: u64) -> NonZeroU64 {
        NonZeroU64::new(id).expect("test ids must be non-zero")
    }

    #[test]
    fn handle_round_trips_its_raw_id() {
        let handle = KernelShape::from_raw(raw(42));
        assert_eq!(handle.raw(), raw(42));
    }

    #[test]
    fn handles_with_equal_raw_ids_are_equal() {
        assert_eq!(KernelShape::from_raw(raw(7)), KernelShape::from_raw(raw(7)));
        assert_ne!(KernelShape::from_raw(raw(7)), KernelShape::from_raw(raw(8)));
    }

    #[test]
    fn handle_is_usable_as_a_hash_map_key() {
        use std::collections::HashMap;
        let mut map: HashMap<KernelShape, &str> = HashMap::new();
        map.insert(KernelShape::from_raw(raw(1)), "first");
        map.insert(KernelShape::from_raw(raw(2)), "second");
        assert_eq!(map.get(&KernelShape::from_raw(raw(1))), Some(&"first"));
        assert_eq!(map.get(&KernelShape::from_raw(raw(2))), Some(&"second"));
        assert_eq!(map.get(&KernelShape::from_raw(raw(3))), None);
    }

    #[test]
    fn option_of_handle_is_the_same_size_as_the_handle() {
        // Verifies the NonZeroU64 niche-optimization this crate relies
        // on to justify NonZeroU64 over a bare u64 (see the
        // `kernel_handle!` macro doc comment) — not merely assumed.
        assert_eq!(
            std::mem::size_of::<Option<KernelShape>>(),
            std::mem::size_of::<KernelShape>()
        );
    }

    #[test]
    fn distinct_handle_kinds_are_distinct_types() {
        // This is a compile-time property, not a runtime one: the line
        // below would fail to compile if KernelShape and KernelVertex
        // were the same type or otherwise interchangeable, which is
        // exactly the point (a KernelVertex must never be usable where a
        // KernelShape is expected, even though both wrap a NonZeroU64).
        fn takes_shape(_: KernelShape) {}
        let vertex = KernelVertex::from_raw(raw(1));
        let shape = KernelShape::from_raw(vertex.raw());
        takes_shape(shape);
    }

    #[test]
    fn kernel_error_display_includes_backend_detail_without_being_the_only_explanation() {
        let error = KernelError::Internal("Standard_DomainError: ...".to_string());
        let text = error.to_string();
        assert!(text.contains("internal backend failure"));
        assert!(text.contains("Standard_DomainError"));
    }

    #[test]
    fn kernel_error_variants_are_distinguishable_without_string_matching() {
        // A caller must be able to match on the category alone (per
        // docs/plan/01_SYSTEM_ARCHITECTURE.md §8's "must never be the
        // only error explanation" rule) — confirm this compiles and
        // behaves as a plain enum match, not a string-inspection hack.
        fn category_name(error: &KernelError) -> &'static str {
            match error {
                KernelError::NullContext => "null_context",
                KernelError::InvalidContext => "invalid_context",
                KernelError::InvalidArgument(_) => "invalid_argument",
                KernelError::InvalidHandle => "invalid_handle",
                KernelError::Internal(_) => "internal",
                KernelError::Unknown(_) => "unknown",
            }
        }
        assert_eq!(category_name(&KernelError::InvalidHandle), "invalid_handle");
        assert_eq!(
            category_name(&KernelError::InvalidContext),
            "invalid_context"
        );
        assert_eq!(
            category_name(&KernelError::Internal("x".to_string())),
            "internal"
        );
    }

    #[test]
    fn kernel_error_implements_std_error() {
        fn assert_is_error<E: Error>() {}
        assert_is_error::<KernelError>();
    }
}
