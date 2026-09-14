//! `AICAD-094`: real per-round evidence-capture helpers
//! [`ParametricBuildSession`](crate::parametric_build::ParametricBuildSession)
//! uses to give `cad-query`'s resolver ([`cad_query::ResolverContext`])
//! real candidate/lineage evidence sourced from the *actual* incremental
//! rebuild it just ran — never a hand-built stand-in operation, per this
//! campaign's own "reference replay must observe actual regenerated
//! semantic/lineage state" requirement.
//!
//! # Why this is a separate module from `parametric_build`
//!
//! These are pure functions over already-produced dispatch data (a
//! [`GeometryGraph`], a [`GraphResults`], a
//! [`cad_geometry_runtime::LineageTable`]) — they need no access to
//! [`ParametricBuildSession`]'s own private fields, so they are free
//! functions here rather than inherent methods, keeping that struct's own
//! module focused on orchestration (matching this crate's existing
//! `parametric_build`/`build`/`cli` module split).
//!
//! # Scope
//!
//! Only [`EntityKind::Face`] lineage is classified (matching
//! `cad_query::feature_lineage`'s own doc comment: "Faces and edges only,
//! matching `AICAD-086`'s own native capture scope" — this task captures
//! the Face half of that already-supported pair; Edge lineage capture is
//! not wired here, see `project/reports/AICAD-094.md`'s own
//! "Limitations"). Only a named top-level feature whose own *final* IR
//! node (the last [`GeomId`] in its `geom_range_for_call`) is directly one
//! of the five lineage-capable ops
//! (`Union`/`Cut`/`Intersect`/`Fillet`/`Chamfer`) is captured — a compound
//! builtin decomposed into multiple raw ops whose *final* node is not
//! itself lineage-capable (or an anonymous/nested-argument feature with no
//! [`FeatureAnchor::Named`] of its own) is silently skipped, never
//! guessed at: it simply contributes no entry to the returned index, and
//! `generated_by`/`modified_by` evidence for it stays `None`
//! ("no evidence"), matching `crate::eval::EvaluationEvidence`'s own
//! "never guess" contract one layer up.
//!
//! [`GeometryGraph`]: cad_geometry_api::GeometryGraph
//! [`GraphResults`]: cad_geometry_runtime::GraphResults

use std::collections::HashMap;
use std::ops::Range;

use cad_geometry_api::{GeomId, GeometryGraph, GeometryNodeKind, GeometryOp};
use cad_geometry_runtime::{GraphResults, LineageTable, NodeResult};
use cad_occt_bridge::Shape;
use cad_query::Candidate;
use cad_query::feature_lineage::{
    FeatureLineageError, FeatureLineageReport, classify_feature_lineage,
};
use cad_references::{EntityKind, FeatureAnchor};

/// Real, per-round `EntityKind::Face` lineage evidence for every named
/// top-level feature this session has ever captured it for — see this
/// module's own doc comment for exactly which features qualify. Keyed by
/// [`FeatureAnchor::Named`]; an entry, once captured, is only replaced
/// (never removed) by a later round that recomputes that same feature
/// again, so a feature the current round left untouched keeps reporting
/// its own last-real-rebuild evidence, exactly matching the real identity
/// continuity `cad_geometry_runtime::dispatch_graph_incremental`'s own
/// reuse path already guarantees for that feature's live `Shape`.
pub type FeatureLineageIndex<'ctx> = HashMap<FeatureAnchor, FeatureLineageReport<'ctx>>;

/// The [`GeomId`] operand a lineage-capable [`GeometryOp`] reads whose own
/// pre-operation entities `classify_feature_lineage` needs as its
/// `prior_entities` argument — for `Union`/`Cut`/`Intersect`, `lhs` only
/// (the shape the operation is conceptually applied *to*), never `rhs`
/// (the tool operand it is applied *with*) -- matching
/// `cad_query::feature_lineage`'s own already-established test precedent
/// (`a_straight_through_hole_marks_pierced_faces_modified_and_side_faces_
/// unchanged`/`every_result_face_is_accounted_for_as_new_merged_or_an_
/// ordinary_survivor`: both pass only the base shape's own prior faces,
/// never the cutting tool's). Including the tool operand's own faces here
/// would misclassify a genuinely new result face (e.g. a hole's own
/// cylindrical wall, generated from the tool's lateral face) as an
/// ordinary carry-forward of that tool face instead of `New` -- exactly
/// the silent-misclassification bug this function exists to avoid. Every
/// other op returns `None`, meaning "not lineage-capable," not "no
/// operands."
fn lineage_operand_id(op: &GeometryOp) -> Option<GeomId> {
    match op {
        GeometryOp::Union { lhs, .. }
        | GeometryOp::Cut { lhs, .. }
        | GeometryOp::Intersect { lhs, .. } => Some(*lhs),
        GeometryOp::Fillet { target, .. } | GeometryOp::Chamfer { target, .. } => Some(*target),
        _ => None,
    }
}

fn enumerate_faces<'ctx>(shape: &Shape<'ctx>) -> Result<Vec<Shape<'ctx>>, FeatureLineageError> {
    let count = shape.face_count().map_err(FeatureLineageError::from)?;
    (0..count)
        .map(|i| shape.get_face(i).map_err(FeatureLineageError::from))
        .collect()
}

/// For every `(anchor, geom_range)` pair naming one of this round's own
/// top-level features (every named feature, whether or not it was
/// recomputed this round — see this module's own doc comment for why
/// filtering happens naturally against `lineage_table` instead), captures
/// real `EntityKind::Face` lineage for the ones whose own final node both
/// (a) was actually recomputed this round (present in `lineage_table`,
/// which — per `dispatch_graph_incremental_with_lineage`'s own contract —
/// only ever contains recomputed nodes) and (b) is itself a
/// lineage-capable op. Every other feature contributes nothing to the
/// returned map (the caller merges this into its own longer-lived
/// [`FeatureLineageIndex`], so an untouched feature simply keeps its own
/// prior entry rather than losing it).
pub(crate) fn capture_named_feature_lineage<'ctx>(
    graph: &GeometryGraph,
    results: &GraphResults<'ctx>,
    lineage_table: &LineageTable<'ctx>,
    named_features: &[(FeatureAnchor, Range<u32>)],
) -> Result<FeatureLineageIndex<'ctx>, FeatureLineageError> {
    let mut out = FeatureLineageIndex::new();

    for (anchor, range) in named_features {
        let Some(last_index) = range.end.checked_sub(1).filter(|&i| i >= range.start) else {
            continue;
        };
        let Some(node) = graph.nodes().get(last_index as usize) else {
            continue;
        };
        let node_id = node.id;
        let Some((_, lineage)) = lineage_table.iter().find(|(id, _)| *id == node_id) else {
            continue;
        };
        let GeometryNodeKind::Construct(op) = &node.kind else {
            continue;
        };
        let Some(operand_id) = lineage_operand_id(op) else {
            continue;
        };

        let prior_faces = match results.get(operand_id.index() as usize) {
            Some(NodeResult::Shape(shape)) => enumerate_faces(shape)?,
            _ => continue,
        };
        let Some(NodeResult::Shape(result_shape)) = results.get(node_id.index() as usize) else {
            continue;
        };

        let report =
            classify_feature_lineage(EntityKind::Face, prior_faces, result_shape, lineage)?;
        out.insert(anchor.clone(), report);
    }

    Ok(out)
}

/// Every live candidate of `kind` among `shape`'s own immediate
/// sub-entities — the per-feature building block
/// [`cad_query::ResolverContext::candidates`] aggregates across every
/// named top-level feature's own current `Shape`. `Shell`/`Solid` return
/// empty (never a guess): `cad_occt_bridge::Shape` exposes no
/// `shell_count`/`solid_count`/`get_shell`/`get_solid` accessor to
/// enumerate sub-shells/sub-solids by, unlike `Vertex`/`Edge`/`Wire`/
/// `Face`, which each have a real `*_count`/`get_*` pair.
pub(crate) fn candidates_of_kind<'ctx>(
    shape: &Shape<'ctx>,
    kind: EntityKind,
) -> Vec<Candidate<'ctx>> {
    match kind {
        EntityKind::Face => (0..shape.face_count().unwrap_or(0))
            .filter_map(|i| shape.get_face(i).ok())
            .map(|entity| Candidate::new(EntityKind::Face, entity))
            .collect(),
        EntityKind::Edge => (0..shape.edge_count().unwrap_or(0))
            .filter_map(|i| shape.get_edge(i).ok())
            .map(|entity| Candidate::new(EntityKind::Edge, entity))
            .collect(),
        EntityKind::Wire => (0..shape.wire_count().unwrap_or(0))
            .filter_map(|i| shape.get_wire(i).ok())
            .map(|entity| Candidate::new(EntityKind::Wire, entity))
            .collect(),
        EntityKind::Vertex => (0..shape.vertex_count().unwrap_or(0))
            .filter_map(|i| shape.get_vertex(i).ok())
            .map(|entity| Candidate::new(EntityKind::Vertex, entity))
            .collect(),
        EntityKind::Shell | EntityKind::Solid => Vec::new(),
    }
}
