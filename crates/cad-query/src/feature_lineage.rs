//! Feature-level lineage classification (`AICAD-087`), per
//! `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §8:
//!
//! > Every topology-changing operation should report lineage if the
//! > kernel makes it available or the wrapper can infer it:
//! >
//! > ```text
//! > old entity -> unchanged/new/modified/split/merged/deleted -> new entities
//! > ```
//! >
//! > Store lineage at the feature node, not only in backend-native
//! > structures.
//!
//! `AICAD-086` (`cad_occt_bridge::Lineage`) captures raw per-entity
//! Generated/Modified/IsDeleted evidence for one operation's own two
//! original operands. This module is the classifier one layer up: given
//! that raw evidence plus the operation's own result shape, it sorts
//! every prior (pre-operation) entity into exactly one of
//! unchanged/modified/split/deleted, and every result (post-operation)
//! entity that is not an ordinary one-to-one carry-forward into new or
//! merged — the plan's own six-word vocabulary, never collapsed into an
//! arbitrary one-to-one mapping (a `Split`/`Merged` entity's own
//! successor/predecessor list may hold more than one entry).
//!
//! # Scope
//!
//! Faces and edges only, matching `AICAD-086`'s own native capture scope
//! (`EntityKind::Vertex`/`Wire`/`Shell`/`Solid` report
//! [`FeatureLineageError::UnsupportedEntityKind`] rather than a guessed
//! implementation).
//!
//! # What this module deliberately does not do
//!
//! It does not itself resolve a persistent [`cad_references::AnyRef`] —
//! every entity here is a live, epoch-bound `cad_occt_bridge::Shape`,
//! exactly like `crate::eval`'s own `Candidate` (see that module's own
//! doc comment for why). Turning this classification into a durable
//! `FeatureLineage { feature: FeatureAnchor, ... }` record addressed by
//! `FeatureAnchor`/`AnyRef` — the plan's own worked `housing.outer_wall`
//! example — is `AICAD-088`+'s resolver job, which is the first Stage-4
//! layer with an actual mechanism for turning a live candidate into a
//! persistent reference. This module only proves *that* the six-state
//! classification is computable from real evidence against a real build,
//! per `AGENTS.md`'s evidence rule.

use cad_geometry_api::OperationReport;
use cad_kernel_api::topology::ClassifiedShape;
use cad_kernel_api::{KernelError, KernelResult};
use cad_occt_bridge::{Lineage, OcctContext, Shape};
use cad_references::EntityKind;

/// The state a **prior** (pre-operation) entity is classified into,
/// relative to one feature operation — derived directly from
/// [`Lineage::is_deleted`]/[`Lineage::generated`]/[`Lineage::modified`],
/// never guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorEntityState {
    /// Not deleted, and `Lineage::generated`/`modified` are both empty —
    /// the entity survives with the same underlying topological identity
    /// (`Shape::is_same`) in the result.
    Unchanged,
    /// Survives as exactly one geometrically changed counterpart
    /// (`generated.len() + modified.len() == 1`).
    Modified,
    /// Survives as two or more counterparts
    /// (`generated.len() + modified.len() >= 2`) — never collapsed into
    /// an arbitrary one-to-one mapping; see [`PriorEntityRecord::successors`].
    Split,
    /// [`Lineage::is_deleted`] reports `true` — no surviving counterpart
    /// at all.
    Deleted,
}

/// How a **result** (post-operation) entity relates to the operation's
/// own prior entities, when that relationship is not an ordinary
/// one-to-one carry-forward (already fully described by the
/// corresponding [`PriorEntityRecord::state`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultEntityOrigin {
    /// No prior entity's own `generated`/`modified` evidence (nor a
    /// direct `is_same` match, for an `Unchanged` prior entity) names
    /// this result entity at all.
    New,
    /// Two or more distinct prior entities' own evidence names this same
    /// result entity — a genuine merge, not an arbitrary pick of one
    /// contributing prior entity.
    Merged,
}

/// One prior entity's own classification, plus which result entities (by
/// index into [`FeatureLineageReport::results`]) it survives as.
#[derive(Debug)]
pub struct PriorEntityRecord<'ctx> {
    pub entity: Shape<'ctx>,
    pub state: PriorEntityState,
    /// Indices into [`FeatureLineageReport::results`]: empty for
    /// `Deleted`; exactly one for `Unchanged`/`Modified`; two or more for
    /// `Split`.
    pub successors: Vec<usize>,
}

/// One result entity's own classification, plus which prior entities (by
/// index into [`FeatureLineageReport::prior`]) it descends from.
#[derive(Debug)]
pub struct ResultEntityRecord<'ctx> {
    pub entity: Shape<'ctx>,
    /// `Some(New)` (zero predecessors), `Some(Merged)` (two or more), or
    /// `None` for an ordinary one-to-one carry-forward (exactly one
    /// predecessor — already fully described by that predecessor's own
    /// `Unchanged`/`Modified` state, so not separately flagged here).
    pub origin: Option<ResultEntityOrigin>,
    pub predecessors: Vec<usize>,
}

/// The full six-state classification for one feature operation's own
/// prior/result entities of one [`EntityKind`] (Face or Edge).
#[derive(Debug)]
pub struct FeatureLineageReport<'ctx> {
    pub prior: Vec<PriorEntityRecord<'ctx>>,
    pub results: Vec<ResultEntityRecord<'ctx>>,
}

/// Every way [`classify_feature_lineage`] can fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureLineageError {
    Kernel(KernelError),
    /// `AICAD-086`'s own native capture is Face/Edge-only — see this
    /// module's own doc comment "Scope".
    UnsupportedEntityKind(EntityKind),
}

impl From<KernelError> for FeatureLineageError {
    fn from(err: KernelError) -> Self {
        FeatureLineageError::Kernel(err)
    }
}

impl std::fmt::Display for FeatureLineageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeatureLineageError::Kernel(err) => write!(f, "{err:?}"),
            FeatureLineageError::UnsupportedEntityKind(kind) => {
                write!(
                    f,
                    "feature-lineage classification does not yet support {kind}"
                )
            }
        }
    }
}

impl std::error::Error for FeatureLineageError {}

type LResult<T> = Result<T, FeatureLineageError>;

/// Classifies `prior_entities` (every face/edge of one operation's own
/// original input shape(s), obtained *before* the operation ran) and
/// `result_shape`'s own current faces/edges into the plan §8 six-state
/// lineage model, using `lineage`'s already-captured (`AICAD-086`)
/// Generated/Modified/IsDeleted evidence.
///
/// `kind` selects Face or Edge (matching `AICAD-086`'s own scope); every
/// entity in `prior_entities` must be of that same kind (a mismatched
/// kind produces defective results from `Lineage`'s own kind-agnostic
/// queries, so callers must pass a homogeneous set — matching
/// `Candidate::new`'s own identical "caller states the kind" contract).
pub fn classify_feature_lineage<'ctx>(
    kind: EntityKind,
    prior_entities: Vec<Shape<'ctx>>,
    result_shape: &Shape<'ctx>,
    lineage: &Lineage<'ctx>,
) -> LResult<FeatureLineageReport<'ctx>> {
    let results = enumerate(kind, result_shape)?;

    let mut prior_records: Vec<PriorEntityRecord<'ctx>> = Vec::with_capacity(prior_entities.len());
    let mut result_predecessors: Vec<Vec<usize>> = vec![Vec::new(); results.len()];

    for (prior_index, entity) in prior_entities.into_iter().enumerate() {
        let deleted = lineage.is_deleted(&entity)?;
        let generated = if deleted {
            Vec::new()
        } else {
            lineage.generated(&entity)?
        };
        let modified = if deleted {
            Vec::new()
        } else {
            lineage.modified(&entity)?
        };

        let state = if deleted {
            PriorEntityState::Deleted
        } else if generated.is_empty() && modified.is_empty() {
            PriorEntityState::Unchanged
        } else if generated.len() + modified.len() == 1 {
            PriorEntityState::Modified
        } else {
            PriorEntityState::Split
        };

        let mut successors = Vec::new();
        match state {
            PriorEntityState::Deleted => {}
            // An `Unchanged` entity has no `generated`/`modified` entry
            // of its own to look up -- its own survival is proven by
            // finding itself, unaltered, among `results` directly.
            PriorEntityState::Unchanged => {
                successors.extend(matching_indices(&entity, &results)?);
            }
            PriorEntityState::Modified | PriorEntityState::Split => {
                for g in &generated {
                    successors.extend(matching_indices(g, &results)?);
                }
                for m in &modified {
                    successors.extend(matching_indices(m, &results)?);
                }
                successors.sort_unstable();
                successors.dedup();
            }
        }

        for &result_index in &successors {
            result_predecessors[result_index].push(prior_index);
        }

        prior_records.push(PriorEntityRecord {
            entity,
            state,
            successors,
        });
    }

    let result_records = results
        .into_iter()
        .zip(result_predecessors)
        .map(|(entity, predecessors)| {
            let origin = match predecessors.len() {
                0 => Some(ResultEntityOrigin::New),
                1 => None,
                _ => Some(ResultEntityOrigin::Merged),
            };
            ResultEntityRecord {
                entity,
                origin,
                predecessors,
            }
        })
        .collect();

    Ok(FeatureLineageReport {
        prior: prior_records,
        results: result_records,
    })
}

/// [`classify_feature_lineage`]'s own raw-tier counterpart (`AICAD-125`):
/// classifies `prior_entities` and `result_shape`'s own current
/// faces/edges into the same six-state model, sourced from a raw-edit
/// chain's own real `OperationReport<ClassifiedShape>` evidence
/// (`AICAD-122`-`124`, propagated across the whole chain by
/// `cad_geometry_runtime::raw_lineage::RawLineageIndex`) instead of a live
/// `cad_occt_bridge::Lineage` query — a raw edit chain has no native OCCT
/// history object of its own once it crosses back into the safe tier via
/// `adopt`, so there is nothing for a `Lineage`-backed
/// [`classify_feature_lineage`] call to query.
///
/// # Algorithm
///
/// For each `prior_entities` member, walks it forward through `steps` in
/// order: at each step, if the entity's own *current* live shape matches
/// (via [`Shape::is_same`]) an entry in that step's own `deleted` list, the
/// entity's own lineage strand ends there (no successor); a `modified`/
/// `split`/`merged` match replaces the current shape with its own
/// evidenced successor(s); no match at all carries the current shape
/// forward unchanged into the next step. An entity never mentioned by any
/// step is `Unchanged` (matching [`classify_feature_lineage`]'s own
/// identical convention: proven by finding itself, unaltered, among
/// `result_shape`'s own current entities, not merely by the absence of
/// evidence). One caveat this sharing does not have: an entity from
/// `prior_entities` that is genuinely outside the scope of every step
/// `steps` records evidence for (e.g. a face of the chain's own origin
/// shape that no raw edit in the chain ever selected, because a
/// `merge_faces`-shaped step only ever narrows to the faces it was asked
/// to merge, never the whole shape) legitimately classifies `Unchanged`
/// with **zero** successors in `result_shape` — honest ("this entity was
/// never part of what this chain's own adopted result narrowed down to"),
/// never a fabricated match.
///
/// `ctx` resolves every step's own [`ClassifiedShape`] payload back into a
/// live, comparable `Shape<'ctx>` (`cad_occt_bridge::Shape::resolve`) —
/// every resolve failure is a real [`FeatureLineageError::Kernel`], never
/// silently skipped, since a step's own just-produced evidence handle
/// failing to resolve would itself be a real bug in the raw tier, not a
/// legitimate "no evidence" outcome.
pub fn classify_raw_edit_lineage<'ctx>(
    ctx: &'ctx OcctContext,
    kind: EntityKind,
    prior_entities: Vec<Shape<'ctx>>,
    result_shape: &Shape<'ctx>,
    steps: &[OperationReport<ClassifiedShape>],
) -> LResult<FeatureLineageReport<'ctx>> {
    let results = enumerate(kind, result_shape)?;
    let resolved_steps: Vec<ResolvedStep<'ctx>> = steps
        .iter()
        .map(|step| resolve_step(ctx, step))
        .collect::<LResult<_>>()?;

    let mut prior_records: Vec<PriorEntityRecord<'ctx>> = Vec::with_capacity(prior_entities.len());
    let mut result_predecessors: Vec<Vec<usize>> = vec![Vec::new(); results.len()];

    for (prior_index, entity) in prior_entities.into_iter().enumerate() {
        let (deleted, successors) = walk_raw_chain(&entity, &resolved_steps)?;

        let state = if deleted {
            PriorEntityState::Deleted
        } else if successors.is_empty() {
            PriorEntityState::Unchanged
        } else if successors.len() == 1 {
            PriorEntityState::Modified
        } else {
            PriorEntityState::Split
        };

        let mut successor_indices = Vec::new();
        match state {
            PriorEntityState::Deleted => {}
            PriorEntityState::Unchanged => {
                successor_indices.extend(matching_indices(&entity, &results)?);
            }
            PriorEntityState::Modified | PriorEntityState::Split => {
                for successor in &successors {
                    successor_indices.extend(matching_indices(successor, &results)?);
                }
                successor_indices.sort_unstable();
                successor_indices.dedup();
            }
        }

        for &result_index in &successor_indices {
            result_predecessors[result_index].push(prior_index);
        }

        prior_records.push(PriorEntityRecord {
            entity,
            state,
            successors: successor_indices,
        });
    }

    let result_records = results
        .into_iter()
        .zip(result_predecessors)
        .map(|(entity, predecessors)| {
            let origin = match predecessors.len() {
                0 => Some(ResultEntityOrigin::New),
                1 => None,
                _ => Some(ResultEntityOrigin::Merged),
            };
            ResultEntityRecord {
                entity,
                origin,
                predecessors,
            }
        })
        .collect();

    Ok(FeatureLineageReport {
        prior: prior_records,
        results: result_records,
    })
}

/// One raw-edit step's own [`OperationReport`], with every
/// [`ClassifiedShape`] payload already resolved into a live, comparable
/// `Shape<'ctx>` — see [`classify_raw_edit_lineage`]'s own doc comment.
struct ResolvedStep<'ctx> {
    deleted: Vec<Shape<'ctx>>,
    modified: Vec<(Shape<'ctx>, Shape<'ctx>)>,
    split: Vec<(Shape<'ctx>, Vec<Shape<'ctx>>)>,
    merged: Vec<(Vec<Shape<'ctx>>, Shape<'ctx>)>,
}

fn resolve_step<'ctx>(
    ctx: &'ctx OcctContext,
    step: &OperationReport<ClassifiedShape>,
) -> LResult<ResolvedStep<'ctx>> {
    let resolve = |classified: &ClassifiedShape| -> LResult<Shape<'ctx>> {
        Shape::resolve(ctx, classified.shape).map_err(FeatureLineageError::from)
    };
    Ok(ResolvedStep {
        deleted: step.deleted.iter().map(resolve).collect::<LResult<_>>()?,
        modified: step
            .modified
            .iter()
            .map(|(old, new)| Ok((resolve(old)?, resolve(new)?)))
            .collect::<LResult<_>>()?,
        split: step
            .split
            .iter()
            .map(|(old, pieces)| {
                Ok((
                    resolve(old)?,
                    pieces.iter().map(resolve).collect::<LResult<_>>()?,
                ))
            })
            .collect::<LResult<_>>()?,
        merged: step
            .merged
            .iter()
            .map(|(olds, result)| {
                Ok((
                    olds.iter().map(resolve).collect::<LResult<_>>()?,
                    resolve(result)?,
                ))
            })
            .collect::<LResult<_>>()?,
    })
}

/// Walks `start` forward through `steps` in order — see
/// [`classify_raw_edit_lineage`]'s own doc comment for the algorithm.
/// Returns `(true, [])` if `start`'s own lineage strand was deleted at any
/// step (whether directly, or because every one of its own successors was
/// itself later deleted); `(false, [])` if `start` was never mentioned by
/// any step at all (the `Unchanged` case); `(false, successors)` otherwise
/// (one entry: `Modified`; two or more: `Split`/collapsed-into-a-shared-
/// merge-result, both already meaning the same "more than one surviving
/// counterpart" to the caller).
fn walk_raw_chain<'ctx>(
    start: &Shape<'ctx>,
    steps: &[ResolvedStep<'ctx>],
) -> LResult<(bool, Vec<Shape<'ctx>>)> {
    let mut current: Vec<Shape<'ctx>> = vec![start.duplicate().map_err(FeatureLineageError::from)?];
    let mut touched = false;

    for step in steps {
        let mut next: Vec<Shape<'ctx>> = Vec::new();
        for candidate in &current {
            if step
                .deleted
                .iter()
                .any(|d| d.is_same(candidate).unwrap_or(false))
            {
                touched = true;
                continue;
            }
            if let Some((_, new)) = step
                .modified
                .iter()
                .find(|(old, _)| old.is_same(candidate).unwrap_or(false))
            {
                next.push(new.duplicate().map_err(FeatureLineageError::from)?);
                touched = true;
                continue;
            }
            if let Some((_, pieces)) = step
                .split
                .iter()
                .find(|(old, _)| old.is_same(candidate).unwrap_or(false))
            {
                for piece in pieces {
                    next.push(piece.duplicate().map_err(FeatureLineageError::from)?);
                }
                touched = true;
                continue;
            }
            if let Some((_, merged_result)) = step
                .merged
                .iter()
                .find(|(olds, _)| olds.iter().any(|o| o.is_same(candidate).unwrap_or(false)))
            {
                next.push(
                    merged_result
                        .duplicate()
                        .map_err(FeatureLineageError::from)?,
                );
                touched = true;
                continue;
            }
            next.push(candidate.duplicate().map_err(FeatureLineageError::from)?);
        }

        // Dedupe by `is_same` -- e.g. two merged sibling strands both
        // resolving to the exact same `merged_result` this step.
        let mut deduped: Vec<Shape<'ctx>> = Vec::with_capacity(next.len());
        for shape in next {
            let already_present = deduped
                .iter()
                .map(|d| d.is_same(&shape))
                .collect::<KernelResult<Vec<bool>>>()
                .map_err(FeatureLineageError::from)?
                .into_iter()
                .any(|same| same);
            if !already_present {
                deduped.push(shape);
            }
        }
        current = deduped;
        if current.is_empty() {
            return Ok((true, Vec::new()));
        }
    }

    if !touched {
        return Ok((false, Vec::new()));
    }
    Ok((false, current))
}

fn enumerate<'ctx>(kind: EntityKind, shape: &Shape<'ctx>) -> LResult<Vec<Shape<'ctx>>> {
    match kind {
        EntityKind::Face => {
            let count = shape.face_count()?;
            (0..count)
                .map(|i| shape.get_face(i).map_err(Into::into))
                .collect()
        }
        EntityKind::Edge => {
            let count = shape.edge_count()?;
            (0..count)
                .map(|i| shape.get_edge(i).map_err(Into::into))
                .collect()
        }
        other => Err(FeatureLineageError::UnsupportedEntityKind(other)),
    }
}

/// Every index into `haystack` whose own `Shape::is_same` matches
/// `needle` — a `Vec`, not an `Option`, since two structurally distinct
/// (but topologically-identical-by-`IsSame`) result entities are not
/// definitionally impossible; ordinary usage returns exactly one match.
fn matching_indices(needle: &Shape<'_>, haystack: &[Shape<'_>]) -> KernelResult<Vec<usize>> {
    let mut out = Vec::new();
    for (index, candidate) in haystack.iter().enumerate() {
        if needle.is_same(candidate)? {
            out.push(index);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_kernel_api::{Transform, Vector3};
    use cad_occt_bridge::OcctContext;

    #[test]
    fn a_straight_through_hole_marks_pierced_faces_modified_and_side_faces_unchanged() {
        let context = OcctContext::new().unwrap();
        let a = context.create_box(10.0, 10.0, 10.0).unwrap();
        let cyl_raw = context.create_cylinder(2.0, 20.0).unwrap();
        let cyl = cyl_raw
            .transform(&Transform::translation(Vector3::new(5.0, 5.0, -5.0)))
            .unwrap();
        let prior: Vec<_> = (0..a.face_count().unwrap())
            .map(|i| a.get_face(i).unwrap())
            .collect();
        let (result, lineage) = a.cut_with_lineage(&cyl).expect("cut should succeed");

        let report = classify_feature_lineage(EntityKind::Face, prior, &result, &lineage).unwrap();

        assert_eq!(report.prior.len(), 6);
        let unchanged = report
            .prior
            .iter()
            .filter(|r| r.state == PriorEntityState::Unchanged)
            .count();
        let modified_or_split = report
            .prior
            .iter()
            .filter(|r| {
                matches!(
                    r.state,
                    PriorEntityState::Modified | PriorEntityState::Split
                )
            })
            .count();
        assert_eq!(unchanged, 4, "the four side faces the hole never reaches");
        assert_eq!(
            modified_or_split, 2,
            "the top and bottom faces the hole pierces"
        );
        assert!(
            report
                .prior
                .iter()
                .all(|r| r.state != PriorEntityState::Deleted)
        );

        // Every `Unchanged` prior entity's own successor is a real,
        // present result entity, and that result entity carries no
        // `origin` flag of its own (an ordinary one-to-one survivor, not
        // "new").
        for (prior_index, record) in report.prior.iter().enumerate() {
            if record.state != PriorEntityState::Unchanged {
                continue;
            }
            assert_eq!(record.successors.len(), 1);
            let successor = &report.results[record.successors[0]];
            assert_eq!(successor.origin, None);
            assert_eq!(successor.predecessors, vec![prior_index]);
        }
    }

    #[test]
    fn a_wholly_swallowed_face_is_deleted_with_no_successors() {
        let context = OcctContext::new().unwrap();
        let a = context.create_box(10.0, 10.0, 10.0).unwrap();
        let tool_raw = context.create_box(20.0, 20.0, 10.0).unwrap();
        let tool = tool_raw
            .transform(&Transform::translation(Vector3::new(-5.0, -5.0, 5.0)))
            .unwrap();
        let prior: Vec<_> = (0..a.face_count().unwrap())
            .map(|i| a.get_face(i).unwrap())
            .collect();
        let (result, lineage) = a.cut_with_lineage(&tool).expect("cut should succeed");

        let report = classify_feature_lineage(EntityKind::Face, prior, &result, &lineage).unwrap();

        let deleted: Vec<_> = report
            .prior
            .iter()
            .filter(|r| r.state == PriorEntityState::Deleted)
            .collect();
        assert_eq!(deleted.len(), 1, "exactly the wholly-swallowed top face");
        assert!(deleted[0].successors.is_empty());
    }

    #[test]
    fn every_result_face_is_accounted_for_as_new_merged_or_an_ordinary_survivor() {
        // Sanity: no result entity is silently dropped from the report --
        // this is really just a structural well-formedness check
        // (`results.len()` always equals `result_shape.face_count()`),
        // but it directly guards against a classifier that quietly loses
        // entities instead of reporting them.
        let context = OcctContext::new().unwrap();
        let a = context.create_box(10.0, 10.0, 10.0).unwrap();
        let cyl_raw = context.create_cylinder(2.0, 20.0).unwrap();
        let cyl = cyl_raw
            .transform(&Transform::translation(Vector3::new(5.0, 5.0, -5.0)))
            .unwrap();
        let prior: Vec<_> = (0..a.face_count().unwrap())
            .map(|i| a.get_face(i).unwrap())
            .collect();
        let (result, lineage) = a.cut_with_lineage(&cyl).expect("cut should succeed");
        let expected_result_count = result.face_count().unwrap();

        let report = classify_feature_lineage(EntityKind::Face, prior, &result, &lineage).unwrap();
        assert_eq!(report.results.len(), expected_result_count);
        assert!(
            report
                .results
                .iter()
                .any(|r| r.origin == Some(ResultEntityOrigin::New)),
            "the hole's own new cylindrical wall face must be classified New"
        );
    }

    #[test]
    fn unsupported_entity_kinds_are_a_structured_error_not_a_guess() {
        let context = OcctContext::new().unwrap();
        let a = context.create_box(10.0, 10.0, 10.0).unwrap();
        let cyl_raw = context.create_cylinder(2.0, 20.0).unwrap();
        let cyl = cyl_raw
            .transform(&Transform::translation(Vector3::new(5.0, 5.0, -5.0)))
            .unwrap();
        let (result, lineage) = a.cut_with_lineage(&cyl).expect("cut should succeed");
        let err =
            classify_feature_lineage(EntityKind::Solid, Vec::new(), &result, &lineage).unwrap_err();
        assert_eq!(
            err,
            FeatureLineageError::UnsupportedEntityKind(EntityKind::Solid)
        );
    }

    // --- AICAD-125: `classify_raw_edit_lineage` ---

    fn classified(shape: &Shape<'_>) -> ClassifiedShape {
        ClassifiedShape::new(shape.topology_kind().unwrap(), shape.handle())
    }

    #[test]
    fn a_single_remove_face_step_deletes_exactly_the_removed_face() {
        let context = OcctContext::new().unwrap();
        let base = context.create_box(1.0, 1.0, 1.0).unwrap();
        let prior: Vec<_> = (0..base.face_count().unwrap())
            .map(|i| base.get_face(i).unwrap())
            .collect();
        let removed_face = base.get_face(0).unwrap();
        let removed = classified(&removed_face);
        let result = base.remove_face(&[0], false, 1e-6).unwrap();

        let mut report = OperationReport::empty();
        report.deleted.push(removed);

        let classification =
            classify_raw_edit_lineage(&context, EntityKind::Face, prior, &result, &[report])
                .unwrap();

        assert_eq!(classification.prior.len(), 6);
        let deleted: Vec<_> = classification
            .prior
            .iter()
            .filter(|r| r.state == PriorEntityState::Deleted)
            .collect();
        assert_eq!(deleted.len(), 1);
        assert!(deleted[0].successors.is_empty());
        let unchanged = classification
            .prior
            .iter()
            .filter(|r| r.state == PriorEntityState::Unchanged)
            .count();
        assert_eq!(unchanged, 5, "every untouched face survives unchanged");
        // Every surviving face is real: `results` still has all 5.
        assert_eq!(classification.results.len(), 5);
        assert!(
            classification.results.iter().all(|r| r.origin.is_none()),
            "no ordinary carry-forward face is misclassified New/Merged"
        );
    }

    #[test]
    fn a_two_step_chain_composes_evidence_across_both_steps() {
        let context = OcctContext::new().unwrap();
        let base = context.create_box(1.0, 1.0, 1.0).unwrap();
        let prior: Vec<_> = (0..base.face_count().unwrap())
            .map(|i| base.get_face(i).unwrap())
            .collect();

        let removed_first_face = base.get_face(0).unwrap();
        let removed_first = classified(&removed_first_face);
        let mut step_a = OperationReport::empty();
        step_a.deleted.push(removed_first);
        let after_first = base.remove_face(&[0], false, 1e-6).unwrap();

        let removed_second_face = after_first.get_face(0).unwrap();
        let removed_second = classified(&removed_second_face);
        let mut step_b = OperationReport::empty();
        step_b.deleted.push(removed_second);
        let after_second = after_first.remove_face(&[0], false, 1e-6).unwrap();

        let classification = classify_raw_edit_lineage(
            &context,
            EntityKind::Face,
            prior,
            &after_second,
            &[step_a, step_b],
        )
        .unwrap();

        let deleted = classification
            .prior
            .iter()
            .filter(|r| r.state == PriorEntityState::Deleted)
            .count();
        assert_eq!(deleted, 2, "both removed faces are traced across the chain");
        let unchanged = classification
            .prior
            .iter()
            .filter(|r| r.state == PriorEntityState::Unchanged)
            .count();
        assert_eq!(unchanged, 4);
        assert_eq!(classification.results.len(), 4);
    }

    #[test]
    fn a_replace_face_step_reports_the_old_face_as_modified() {
        let context = OcctContext::new().unwrap();
        let base = context.create_box(1.0, 1.0, 1.0).unwrap();
        let replacement_source = context.create_box(2.0, 2.0, 2.0).unwrap();
        let replacement = replacement_source.get_face(0).unwrap();

        let prior: Vec<_> = (0..base.face_count().unwrap())
            .map(|i| base.get_face(i).unwrap())
            .collect();
        let old_face_shape = base.get_face(0).unwrap();
        let old_face = classified(&old_face_shape);
        let replacement_classified = classified(&replacement);
        let result = base.replace_face(0, &replacement, false, 1e-6).unwrap();

        let mut report = OperationReport::empty();
        report.modified.push((old_face, replacement_classified));

        let classification =
            classify_raw_edit_lineage(&context, EntityKind::Face, prior, &result, &[report])
                .unwrap();

        let modified: Vec<_> = classification
            .prior
            .iter()
            .filter(|r| r.state == PriorEntityState::Modified)
            .collect();
        assert_eq!(modified.len(), 1);
        assert_eq!(modified[0].successors.len(), 1);
    }

    #[test]
    fn a_face_never_mentioned_by_any_step_is_unchanged_with_no_successors_when_absent_from_the_result()
     {
        // A `merge_faces`-shaped step only ever narrows evidence to the
        // faces it was actually asked to merge -- an unselected face of
        // the chain's own origin shape is legitimately `Unchanged` (never
        // mentioned) even though it does not appear at all in a `result_
        // shape` that only contains the merged output (see this module's
        // own doc comment, "one caveat this sharing does not have").
        let context = OcctContext::new().unwrap();
        let base = context.create_box(1.0, 1.0, 1.0).unwrap();
        let prior: Vec<_> = (0..base.face_count().unwrap())
            .map(|i| base.get_face(i).unwrap())
            .collect();
        // A result shape with no faces in common with `base` at all --
        // stands in for "the adopted result narrowed down to something
        // this face was never part of."
        let unrelated_result = context.create_box(5.0, 5.0, 5.0).unwrap();

        let classification =
            classify_raw_edit_lineage(&context, EntityKind::Face, prior, &unrelated_result, &[])
                .unwrap();

        assert!(
            classification
                .prior
                .iter()
                .all(|r| r.state == PriorEntityState::Unchanged && r.successors.is_empty())
        );
    }

    /// A `merge_faces`-shaped step's own `merged` evidence: both
    /// contributing prior faces classify `Modified` (one real successor
    /// each — the same shared merged face), and the merged result itself
    /// classifies `ResultEntityOrigin::Merged` (two real predecessors) —
    /// never an arbitrary pick of one contributing face, matching
    /// `classify_feature_lineage`'s own established `Merged` semantics one
    /// evidence source over.
    #[test]
    fn a_merge_step_classifies_both_contributors_modified_and_the_result_merged() {
        use cad_geometry_api::{FaceOrientation, GeometryGraph, GeometryOp, SurfaceSpec};
        use cad_geometry_runtime::{NodeResult, dispatch_graph};
        use cad_kernel_api::{Direction3, Point3};

        fn edge_adjacent_square(
            graph: &mut GeometryGraph,
            x_offset: f64,
        ) -> cad_geometry_api::GeomId {
            let mut edge = |x0: f64, y0: f64, x1: f64, y1: f64| {
                graph
                    .push_op(
                        GeometryOp::LineEdge {
                            start: Point3::new(x0 + x_offset, y0, 0.0),
                            end: Point3::new(x1 + x_offset, y1, 0.0),
                        },
                        cad_ast::Span::new(0, 1),
                    )
                    .unwrap()
            };
            let e0 = edge(0.0, 0.0, 1.0, 0.0);
            let e1 = edge(1.0, 0.0, 1.0, 1.0);
            let e2 = edge(1.0, 1.0, 0.0, 1.0);
            let e3 = edge(0.0, 1.0, 0.0, 0.0);
            let wire = graph
                .push_op(
                    GeometryOp::WireFromEdges {
                        edges: vec![e0, e1, e2, e3],
                    },
                    cad_ast::Span::new(0, 1),
                )
                .unwrap();
            graph
                .push_op(
                    GeometryOp::MakeFaceOnSurface {
                        surface: SurfaceSpec::Plane {
                            origin: Point3::ORIGIN,
                            normal: Direction3::Z,
                        },
                        outer: wire,
                        holes: vec![],
                        orientation: FaceOrientation::Forward,
                    },
                    cad_ast::Span::new(0, 1),
                )
                .unwrap()
        }

        let context = OcctContext::new().unwrap();
        let mut graph = GeometryGraph::new();
        let face1 = edge_adjacent_square(&mut graph, 0.0);
        let face2 = edge_adjacent_square(&mut graph, 1.0);
        let sewn_id = graph
            .push_op(
                GeometryOp::Sew {
                    shapes: vec![face1, face2],
                    tolerance: cad_geometry_api::Quantity::of(1e-6, cad_types::Dimension::Length),
                },
                cad_ast::Span::new(0, 1),
            )
            .unwrap();
        let results = dispatch_graph(&graph, &context).unwrap();
        let NodeResult::Shape(sewn) = &results[sewn_id.index() as usize] else {
            panic!("expected a Shape result");
        };
        assert_eq!(sewn.face_count().unwrap(), 2, "not yet merged");

        let prior_f0 = sewn.get_face(0).unwrap();
        let prior_f1 = sewn.get_face(1).unwrap();
        let input0 = classified(&prior_f0);
        let input1 = classified(&prior_f1);
        let merged_shape = sewn.merge_faces(&[0, 1]).unwrap();
        assert_eq!(
            merged_shape.face_count().unwrap(),
            1,
            "two coplanar adjacent faces fully merge into one"
        );
        let merged_face = merged_shape.get_face(0).unwrap();
        let merged_classified = classified(&merged_face);

        let mut report = OperationReport::empty();
        report
            .merged
            .push((vec![input0, input1], merged_classified));

        let prior_entities = vec![sewn.get_face(0).unwrap(), sewn.get_face(1).unwrap()];
        let classification = classify_raw_edit_lineage(
            &context,
            EntityKind::Face,
            prior_entities,
            &merged_shape,
            &[report],
        )
        .unwrap();

        assert_eq!(classification.prior.len(), 2);
        assert!(
            classification
                .prior
                .iter()
                .all(|r| r.state == PriorEntityState::Modified && r.successors == vec![0]),
            "both contributing faces must classify Modified with the same single successor"
        );
        assert_eq!(classification.results.len(), 1);
        assert_eq!(
            classification.results[0].origin,
            Some(ResultEntityOrigin::Merged)
        );
        assert_eq!(classification.results[0].predecessors, vec![0, 1]);
    }
}
