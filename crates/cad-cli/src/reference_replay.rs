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
    FeatureLineageError, FeatureLineageReport, PriorEntityState, ResultEntityOrigin,
    classify_feature_lineage,
};
use cad_references::{EntityKind, FeatureAnchor, LineageRole};

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

/// `target_report`'s own result entities actually attributable to `role`
/// (`Generated` -> classified `New`; `Modified` -> real, non-ordinary-
/// carry-forward change, matching `cad_cli::parametric_build::
/// ParametricBuildSession`'s `generated_by`/`modified_by` `Evaluation
/// Evidence` methods exactly) — shared by those two methods and by
/// [`descended_from_closure`]'s own base case below, so "descended from
/// feature X" starts from the same real entity/entities `generated_by(X)`/
/// `modified_by(X)` would themselves report, never the looser "anything
/// currently among X's results, including untouched sibling faces X never
/// actually touched" (which `Ancestry`'s own "descendant of a stable prior
/// entity" contract — `crate::recipe::ConstructionStrategy::Ancestry`'s own
/// doc comment — does not intend).
pub(crate) fn feature_result_shapes_for_role<'a, 'ctx>(
    report: &'a FeatureLineageReport<'ctx>,
    role: LineageRole,
) -> Vec<&'a Shape<'ctx>> {
    match role {
        LineageRole::Generated => report
            .results
            .iter()
            .filter(|r| r.origin == Some(ResultEntityOrigin::New))
            .map(|r| &r.entity)
            .collect(),
        LineageRole::Modified => report
            .results
            .iter()
            .filter(|r| {
                let is_ordinary_carry_forward = r.origin.is_none()
                    && r.predecessors
                        .first()
                        .map(|&i| report.prior[i].state == PriorEntityState::Unchanged)
                        .unwrap_or(false);
                !is_ordinary_carry_forward && r.origin != Some(ResultEntityOrigin::New)
            })
            .map(|r| &r.entity)
            .collect(),
    }
}

/// Computes the transitive closure of every currently-captured shape
/// "descended from" `target`'s own feature operation under `role`
/// (`AICAD-100A`, `D31`/`TopologyPredicate::DescendedFrom`/
/// `ConstructionStrategy::Ancestry`): the entities `target`'s own report
/// actually attributes to `role` (base case — see
/// [`feature_result_shapes_for_role`]'s own doc comment for exactly which
/// ones that is), plus, transitively, every result entity any *other*
/// captured feature's own report links via a real predecessor edge to an
/// already-known-descended entity. Composed strictly from real
/// per-operation Generated/Modified/Split/Merged evidence this session has
/// actually captured (`cad_query::feature_lineage::classify_feature_
/// lineage`'s own six-state model) — never synthesized from geometric
/// coincidence (`AGENTS.md`'s evidence rule).
///
/// Terminates: `feature_lineage` is finite and each pass only ever adds
/// entities, never removes any, so the fixed point is reached in at most
/// `feature_lineage.len()` passes.
///
/// `None` when `target` itself has no captured report at all (no evidence
/// — same "never guess" contract as every other evidence hook).
pub(crate) fn descended_from_closure<'a, 'ctx>(
    feature_lineage: &'a FeatureLineageIndex<'ctx>,
    target: &FeatureAnchor,
    role: LineageRole,
) -> Option<Vec<&'a Shape<'ctx>>> {
    let target_report = feature_lineage.get(target)?;
    let mut known: Vec<&Shape<'ctx>> = feature_result_shapes_for_role(target_report, role);

    loop {
        let mut added = false;
        for report in feature_lineage.values() {
            for prior in &report.prior {
                if known
                    .iter()
                    .any(|k| prior.entity.is_same(k).unwrap_or(false))
                {
                    for &succ_idx in &prior.successors {
                        let succ_shape = &report.results[succ_idx].entity;
                        if !known.iter().any(|k| succ_shape.is_same(k).unwrap_or(false)) {
                            known.push(succ_shape);
                            added = true;
                        }
                    }
                }
            }
        }
        if !added {
            break;
        }
    }

    Some(known)
}

/// Every live candidate of `kind` among `shape`'s own immediate
/// sub-entities — the per-feature building block
/// [`cad_query::ResolverContext::candidates`] aggregates across every
/// named top-level feature's own current `Shape`. `Shell`/`Solid` use the
/// same `*_count`/`get_*` pair `cad_occt_bridge::Shape` now exposes for
/// them (`AICAD-100A`, mirroring `Vertex`/`Edge`/`Wire`/`Face`'s own
/// identical shape) — previously always empty, since no bridge accessor
/// existed to enumerate a shape's own sub-shells/sub-solids at all,
/// meaning a `ShellRef`/`SolidRef` query against real geometry could never
/// find a candidate regardless of whether one genuinely existed.
pub(crate) fn candidates_of_kind<'ctx>(
    shape: &Shape<'ctx>,
    kind: EntityKind,
) -> Vec<Candidate<'ctx>> {
    match kind {
        // `Face`/`Edge` candidates carry their own enclosing `shape` as
        // `root` (`AICAD-100A`, `Candidate::with_root`) — required by
        // `ConnectedTo` (Face) and `Convex`/`Concave`/`Manifold`/
        // `NonManifold` (Edge). A duplicate that fails to fetch (should
        // not happen for a live `shape`) simply skips that one candidate
        // rather than aborting the whole enumeration, matching this
        // function's own existing `filter_map`/`unwrap_or(0)` "skip what
        // fails, never guess" convention.
        EntityKind::Face => (0..shape.face_count().unwrap_or(0))
            .filter_map(|i| shape.get_face(i).ok().zip(shape.duplicate().ok()))
            .map(|(entity, root)| Candidate::with_root(EntityKind::Face, entity, root))
            .collect(),
        EntityKind::Edge => (0..shape.edge_count().unwrap_or(0))
            .filter_map(|i| shape.get_edge(i).ok().zip(shape.duplicate().ok()))
            .map(|(entity, root)| Candidate::with_root(EntityKind::Edge, entity, root))
            .collect(),
        EntityKind::Wire => (0..shape.wire_count().unwrap_or(0))
            .filter_map(|i| shape.get_wire(i).ok())
            .map(|entity| Candidate::new(EntityKind::Wire, entity))
            .collect(),
        EntityKind::Vertex => (0..shape.vertex_count().unwrap_or(0))
            .filter_map(|i| shape.get_vertex(i).ok())
            .map(|entity| Candidate::new(EntityKind::Vertex, entity))
            .collect(),
        EntityKind::Shell => (0..shape.shell_count().unwrap_or(0))
            .filter_map(|i| shape.get_shell(i).ok())
            .map(|entity| Candidate::new(EntityKind::Shell, entity))
            .collect(),
        EntityKind::Solid => (0..shape.solid_count().unwrap_or(0))
            .filter_map(|i| shape.get_solid(i).ok())
            .map(|entity| Candidate::new(EntityKind::Solid, entity))
            .collect(),
    }
}
