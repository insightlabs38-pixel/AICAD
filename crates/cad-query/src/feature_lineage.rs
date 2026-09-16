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

use cad_kernel_api::{KernelError, KernelResult};
use cad_occt_bridge::{Lineage, Shape};
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
}
