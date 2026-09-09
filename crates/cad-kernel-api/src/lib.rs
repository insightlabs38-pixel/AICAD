//! `cad-kernel-api` — backend-independent handle/error types (AICAD-017).
//!
//! Per `docs/plan/01_SYSTEM_ARCHITECTURE.md` §2.6 and
//! `project/DECISION_LOG.md#DL-5`/`#DL-6`, this crate is the
//! kernel-neutral surface every geometry backend (OCCT today, a future
//! kernel later) is validated against: it must not name, depend on, or
//! leak any backend-specific type. `Cargo.toml` accordingly declares no
//! dependencies at all — not even on `crates/cad-occt-bridge` — so a
//! backend dependency creeping in here would be a build-time
//! contradiction, not just a code-review concern.
//!
//! This task's scope is exactly its title: handle and error types. The
//! domain-level geometry *operations* (`create_box`, `boolean_union`,
//! ...) that will eventually consume/produce these handles are not
//! defined here — they arrive incrementally starting with Batch 1B
//! (AICAD-020+), each adding to whatever backend trait
//! `crates/cad-occt-bridge` (AICAD-018) exposes, once there is more than
//! one operation to generalize over. Defining that trait now, with a
//! single operation as its only evidence, would be exactly the kind of
//! premature abstraction `AGENTS.md` asks this agent to avoid.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// Marker types that parameterize [`KernelHandle`], so the Rust type
/// system distinguishes a [`KernelFace`] from a [`KernelSolid`] even
/// though both share the same runtime representation. None of these
/// types is ever instantiated.
pub mod kind {
    /// Marks a handle as referring to a general topological shape
    /// (the union of all other kinds below — the kind returned by an
    /// operation whose specific topological category is not yet known
    /// to the caller).
    #[derive(Debug)]
    pub struct Shape;
    /// Marks a handle as referring to a solid.
    #[derive(Debug)]
    pub struct Solid;
    /// Marks a handle as referring to a shell.
    #[derive(Debug)]
    pub struct Shell;
    /// Marks a handle as referring to a face.
    #[derive(Debug)]
    pub struct Face;
    /// Marks a handle as referring to a wire.
    #[derive(Debug)]
    pub struct Wire;
    /// Marks a handle as referring to an edge.
    #[derive(Debug)]
    pub struct Edge;
    /// Marks a handle as referring to a vertex.
    #[derive(Debug)]
    pub struct Vertex;
    /// Marks a handle as referring to a curve.
    #[derive(Debug)]
    pub struct Curve;
    /// Marks a handle as referring to a surface.
    #[derive(Debug)]
    pub struct Surface;
}

/// An opaque, epoch/build-local handle to a backend-owned geometry
/// entity (`AGENTS.md` non-negotiable "Raw topology is
/// ephemeral/unsafe and epoch-bound"; RFC-0002 §5).
///
/// `context_id`, `index`, and `generation` mirror `native/occt_bridge`'s
/// `AicadShapeHandle` (AICAD-016/019) field-for-field, because the
/// epoch/context-scoping concept is backend-independent even though the
/// *values* a specific backend assigns are not — a future non-OCCT
/// backend is free to assign these fields however it likes, as long as
/// it upholds the same contract: a handle is valid only against the
/// context that issued it, only while that context still considers the
/// underlying slot live, and only while `generation` matches the slot's
/// current generation (rejecting a handle whose slot was released and
/// possibly reused for an unrelated entity — AGENTS.md non-negotiable
/// "Released/stale handles must never accidentally alias newly created
/// geometry").
///
/// Never construct one of these from arbitrary numbers outside a
/// backend crate (`crates/cad-occt-bridge` and future backends). This
/// type is public so backend crates in this workspace can produce and
/// consume it; ordinary AICAD language/runtime code is never expected to
/// see it directly — that layer works with durable semantic references
/// (`crates/cad-references`), not raw kernel handles.
pub struct KernelHandle<Kind> {
    pub context_id: u64,
    pub index: u32,
    pub generation: u32,
    _kind: PhantomData<Kind>,
}

impl<Kind> KernelHandle<Kind> {
    /// Constructs a handle from its raw fields. Intended for backend
    /// crates translating a native/FFI result into this kernel-neutral
    /// type — not for constructing a handle speculatively.
    pub fn new(context_id: u64, index: u32, generation: u32) -> Self {
        Self {
            context_id,
            index,
            generation,
            _kind: PhantomData,
        }
    }
}

// Manual trait impls (rather than `#[derive(..)]`) so that `Kind` itself
// is never required to implement `Clone`/`Copy`/`PartialEq`/`Eq`/`Hash`/
// `Debug` — `#[derive]` would otherwise add a `Kind: Trait` bound to
// every impl even though `PhantomData<Kind>` needs no such bound, which
// would force every marker type in `kind` to derive traits it has no
// use for.

impl<Kind> Clone for KernelHandle<Kind> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Kind> Copy for KernelHandle<Kind> {}

impl<Kind> PartialEq for KernelHandle<Kind> {
    fn eq(&self, other: &Self) -> bool {
        self.context_id == other.context_id
            && self.index == other.index
            && self.generation == other.generation
    }
}

impl<Kind> Eq for KernelHandle<Kind> {}

impl<Kind> Hash for KernelHandle<Kind> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.context_id.hash(state);
        self.index.hash(state);
        self.generation.hash(state);
    }
}

impl<Kind> fmt::Debug for KernelHandle<Kind> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KernelHandle")
            .field("context_id", &self.context_id)
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish()
    }
}

/// A handle of unspecified/general topological kind.
pub type KernelShape = KernelHandle<kind::Shape>;
/// A handle known to refer to a solid.
pub type KernelSolid = KernelHandle<kind::Solid>;
/// A handle known to refer to a shell.
pub type KernelShell = KernelHandle<kind::Shell>;
/// A handle known to refer to a face.
pub type KernelFace = KernelHandle<kind::Face>;
/// A handle known to refer to a wire.
pub type KernelWire = KernelHandle<kind::Wire>;
/// A handle known to refer to an edge.
pub type KernelEdge = KernelHandle<kind::Edge>;
/// A handle known to refer to a vertex.
pub type KernelVertex = KernelHandle<kind::Vertex>;
/// A handle known to refer to a curve.
pub type KernelCurve = KernelHandle<kind::Curve>;
/// A handle known to refer to a surface.
pub type KernelSurface = KernelHandle<kind::Surface>;

/// A backend-neutral, structured failure result. Every backend crate
/// (starting with `crates/cad-occt-bridge`, AICAD-018) must normalize
/// its own native/FFI failures into one of these variants rather than
/// letting a backend-specific status code or exception propagate
/// upward, per `AGENTS.md` ("Normalize native failures into explicit
/// structured status/error results").
///
/// Variant shape intentionally mirrors `native/occt_bridge`'s
/// `AicadStatusCode` (AICAD-016) one-to-one, since every backend needs
/// to report the same *categories* of failure even though the concrete
/// message text is backend-specific.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    /// A caller-supplied argument was rejected before any backend/kernel
    /// call was attempted (e.g. a non-finite or non-positive dimension).
    InvalidArgument(String),
    /// A handle was well-formed for *some* context but out of range (or
    /// otherwise not recognized) for the context it was used against.
    InvalidHandle(String),
    /// A handle was issued by a different context than the one it was
    /// used against.
    ForeignContextHandle(String),
    /// The backend/kernel itself rejected or failed to complete an
    /// otherwise well-formed operation (e.g. a degenerate geometric
    /// result). Never conflated with `InvalidArgument`: this means the
    /// backend tried and failed, not that AICAD refused to try.
    Failure(String),
    /// A failure in this crate/backend's own plumbing, unrelated to the
    /// caller's input or the kernel's geometric behavior (e.g. an
    /// allocation failure).
    Internal(String),
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelError::InvalidArgument(message) => write!(f, "invalid argument: {message}"),
            KernelError::InvalidHandle(message) => write!(f, "invalid handle: {message}"),
            KernelError::ForeignContextHandle(message) => {
                write!(f, "foreign-context handle: {message}")
            }
            KernelError::Failure(message) => write!(f, "kernel failure: {message}"),
            KernelError::Internal(message) => write!(f, "internal error: {message}"),
        }
    }
}

impl std::error::Error for KernelError {}

/// Convenience alias for a kernel-neutral fallible result.
pub type KernelResult<T> = Result<T, KernelError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_with_equal_fields_are_equal_regardless_of_kind_type_identity() {
        let a: KernelSolid = KernelHandle::new(1, 2, 0);
        let b: KernelSolid = KernelHandle::new(1, 2, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn handles_with_different_fields_are_not_equal() {
        let a: KernelSolid = KernelHandle::new(1, 2, 0);
        let b: KernelSolid = KernelHandle::new(1, 3, 0);
        let c: KernelSolid = KernelHandle::new(2, 2, 0);
        let d: KernelSolid = KernelHandle::new(1, 2, 1);
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn handle_kind_is_a_compile_time_distinction_not_a_runtime_field() {
        // If this compiles, KernelFace and KernelSolid are genuinely
        // distinct Rust types even though their runtime shape (three
        // integers) is identical — the assertion below only needs the
        // shared fields to be readable on both.
        let face: KernelFace = KernelHandle::new(5, 0, 0);
        let solid: KernelSolid = KernelHandle::new(5, 0, 0);
        assert_eq!(face.context_id, solid.context_id);
        assert_eq!(face.index, solid.index);
        assert_eq!(face.generation, solid.generation);
    }

    #[test]
    fn kernel_handle_is_copy_and_clone() {
        fn assert_copy<T: Copy>(_: &T) {}
        fn clone_it<T: Clone>(value: &T) -> T {
            value.clone()
        }

        let a: KernelShape = KernelHandle::new(7, 1, 0);
        assert_copy(&a);
        let b = a;
        let c = clone_it(&a);
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn kernel_error_display_includes_message_and_category() {
        let error = KernelError::ForeignContextHandle("handle from a different context".into());
        let rendered = error.to_string();
        assert!(rendered.contains("foreign-context handle"));
        assert!(rendered.contains("handle from a different context"));
    }

    #[test]
    fn kernel_error_implements_std_error() {
        fn accepts_std_error(_: &dyn std::error::Error) {}
        let error = KernelError::Internal("boom".into());
        accepts_std_error(&error);
    }
}
