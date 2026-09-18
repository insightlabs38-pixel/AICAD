//! Kernel-neutral topology-kind classification (`AICAD-108`).
//!
//! Every [`KernelVertex`]/[`KernelEdge`]/.../[`KernelSolid`] handle
//! (`crate::lib`) already commits to its own specific topological kind at
//! the *type* level — a [`KernelFace`] can never be mistaken for a
//! [`KernelEdge`]. [`KernelShape`] deliberately does not: it is this
//! crate's own "unspecified/mixed topological kind until classified"
//! handle, returned by operations whose result is not yet known to be one
//! specific kind. Stage-5 topology inspection (`AICAD-119`-`121`) needs a
//! way to *report* which kind a [`KernelShape`] actually turned out to be
//! without narrowing the handle type itself (the shape may genuinely be a
//! compound of mixed kinds) — [`TopologyKind`] is that kernel-neutral tag.
//!
//! This module adds no inspection *operation* (no way to ask a real kernel
//! context "what kind is this shape") — only the kernel-neutral value a
//! future inspection operation's result carries, per this task's own
//! "topology... concepts needed by later tasks" scope (see `crate`'s own
//! module doc comment and `project/reports/AICAD-108.md` for the sibling
//! finding that D22's *raw-handle* tier needs no new type here at all,
//! `cad_references::raw_handle::RawHandle<T>` already covering it
//! generically).

use crate::{KernelEdge, KernelFace, KernelShape, KernelShell, KernelSolid, KernelVertex};
use std::fmt;

/// The six closed topological kinds a [`KernelShape`] may classify as —
/// deliberately the same six-way spelling `cad_references::EntityKind`
/// already uses one layer above the kernel adapter (a semantic-reference
/// candidate's own kind), kept as an independent type here rather than a
/// shared one: `cad-kernel-api` and `cad-references` are deliberately
/// unconnected crates (neither depends on the other — `cad-references` is
/// semantic-reference vocabulary above the kernel, `cad-kernel-api` is
/// kernel-adapter vocabulary below it, `project/DECISION_LOG.md#DL-5`), and
/// introducing a dependency between them purely to share a six-variant enum
/// would be a larger architectural change than this task's own scope —
/// mirrors `cad_feature_graph::cache`'s own established "duplicate a small,
/// independent concept rather than couple two layers that should stay
/// separate" precedent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TopologyKind {
    Vertex,
    Edge,
    Wire,
    Face,
    Shell,
    Solid,
}

impl fmt::Display for TopologyKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            TopologyKind::Vertex => "Vertex",
            TopologyKind::Edge => "Edge",
            TopologyKind::Wire => "Wire",
            TopologyKind::Face => "Face",
            TopologyKind::Shell => "Shell",
            TopologyKind::Solid => "Solid",
        };
        f.write_str(name)
    }
}

/// An opaque [`KernelShape`] paired with its own classified [`TopologyKind`]
/// — the value a future topology-inspection kernel operation returns once
/// it has determined which specific kind an otherwise-unclassified shape
/// actually is, without narrowing `shape` itself into one of the more
/// specific handle types (which would require the kernel adapter to
/// actually re-mint a differently-typed handle for the same underlying
/// entity — a real kernel-adapter design question `AICAD-119`-`121` owns,
/// not this task).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClassifiedShape {
    pub kind: TopologyKind,
    pub shape: KernelShape,
}

impl ClassifiedShape {
    pub fn new(kind: TopologyKind, shape: KernelShape) -> ClassifiedShape {
        ClassifiedShape { kind, shape }
    }
}

impl From<KernelVertex> for TopologyKind {
    fn from(_: KernelVertex) -> TopologyKind {
        TopologyKind::Vertex
    }
}
impl From<KernelEdge> for TopologyKind {
    fn from(_: KernelEdge) -> TopologyKind {
        TopologyKind::Edge
    }
}
impl From<KernelFace> for TopologyKind {
    fn from(_: KernelFace) -> TopologyKind {
        TopologyKind::Face
    }
}
impl From<KernelShell> for TopologyKind {
    fn from(_: KernelShell) -> TopologyKind {
        TopologyKind::Shell
    }
}
impl From<KernelSolid> for TopologyKind {
    fn from(_: KernelSolid) -> TopologyKind {
        TopologyKind::Solid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KernelId;

    fn sample_id() -> KernelId {
        KernelId {
            context_id: 1,
            slot: 0,
            generation: 1,
        }
    }

    #[test]
    fn every_kind_has_a_non_empty_display() {
        for kind in [
            TopologyKind::Vertex,
            TopologyKind::Edge,
            TopologyKind::Wire,
            TopologyKind::Face,
            TopologyKind::Shell,
            TopologyKind::Solid,
        ] {
            assert!(!kind.to_string().is_empty());
        }
    }

    #[test]
    fn classified_shape_pairs_kind_with_the_opaque_handle_unchanged() {
        let shape = KernelShape::from_id(sample_id());
        let classified = ClassifiedShape::new(TopologyKind::Face, shape);
        assert_eq!(classified.kind, TopologyKind::Face);
        assert_eq!(classified.shape, shape);
    }

    #[test]
    fn specific_handle_types_convert_to_their_own_matching_kind() {
        let id = sample_id();
        assert_eq!(
            TopologyKind::from(KernelFace::from_id(id)),
            TopologyKind::Face
        );
        assert_eq!(
            TopologyKind::from(KernelSolid::from_id(id)),
            TopologyKind::Solid
        );
        assert_eq!(
            TopologyKind::from(KernelVertex::from_id(id)),
            TopologyKind::Vertex
        );
    }

    #[test]
    fn classified_shapes_are_hashable_and_usable_as_set_keys() {
        let mut set = std::collections::HashSet::new();
        set.insert(ClassifiedShape::new(
            TopologyKind::Solid,
            KernelShape::from_id(sample_id()),
        ));
        set.insert(ClassifiedShape::new(
            TopologyKind::Solid,
            KernelShape::from_id(sample_id()),
        ));
        assert_eq!(set.len(), 1, "identical classified shapes deduplicate");
    }
}
