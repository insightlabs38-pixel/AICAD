//! Predicate evaluation against a real build (`AICAD-082`/`083`/`084`).
//!
//! `crate::predicate` defines geometry/topology/spatial predicates as a
//! plain AST — this module is the first thing that actually inspects live
//! `cad-occt-bridge` geometry and decides whether one predicate holds for
//! one candidate entity. It is deliberately **not** a query executor: it
//! answers "does this predicate hold for this already-identified
//! candidate?", never "which entities does this whole query select?"
//! (candidate enumeration, ranking application, and cardinality
//! resolution remain `AICAD-088`+'s resolver).
//!
//! # Evidence this module does not itself produce
//!
//! Three predicate families need information no Stage-4 task has built a
//! production source for yet:
//!
//! - `generated_by`/`modified_by`/`descended_from` need feature lineage
//!   (`AICAD-085`..`087`);
//! - `adjacent_to`, `inside(volume)`, and `within(distance, ref)` need an
//!   already-*resolved* reference/query target (`AICAD-088`+'s resolver).
//!
//! Rather than fabricating either, this module takes them as an injected
//! [`EvaluationEvidence`] capability. Every method on that trait defaults
//! to returning `None` ("no evidence available"), which every evaluator
//! function here turns into [`EvalError::NoEvidence`] — an explicit,
//! propagated "cannot determine" outcome, never silently treated as
//! `false`. Silently mapping "unknown" to "does not match" would let a
//! predicate quietly exclude a candidate a resolver might otherwise have
//! reported `Ambiguous` or `Resolved` for — exactly the class of
//! silently-wrong outcome `AGENTS.md`'s semantic-reference outcome policy
//! forbids one layer up. `AICAD-088`+ decides what a `NoEvidence` result
//! ultimately means for the user-visible outcome (most likely `Broken`);
//! this module only refuses to guess.
//!
//! # Predicates not in scope
//!
//! `TopologyPredicate::{Convex, Concave, Manifold, NonManifold,
//! ConnectedTo, Contains, Intersects}` and `SpatialPredicate::{NearestTo,
//! FarthestFrom}` are already-shipped AST variants (`AICAD-081`) that
//! `AICAD-083`'s and `AICAD-084`'s own task titles do not name (`083`:
//! "generated_by/modified_by/descended_from/adjacent_to/boundary" only;
//! `084`: "baseline spatial predicates", which `nearest_to`/`farthest_from`
//! do not fit — see below). Every evaluator function here is exhaustive
//! over its predicate enum (so the match itself cannot silently drift out
//! of sync with a future AST addition), but these specific variants
//! return [`EvalError::NotYetSpecified`] rather than a guessed
//! implementation, matching `AICAD-081`'s own precedent for deferring
//! `curvature` (`crate::predicate`'s module doc comment) instead of
//! inventing comparison/traversal semantics no task has specified:
//!
//! - `Convex`/`Concave`: the plan doc (`docs/plan/06...` §6) names them
//!   with no definition of which entity kind they apply to or what
//!   geometric test decides them (dihedral-angle sign across an edge?
//!   surface curvature sign for a face? both are common CAD meanings).
//! - `Manifold`/`NonManifold`/`ConnectedTo`/`Contains`/`Intersects`: each
//!   needs either whole-shape topology-graph traversal (connectivity) or
//!   a defined aggregation policy this task's own scope does not cover.
//! - `NearestTo`/`FarthestFrom`: comparative across a candidate *set*
//!   (the plan's own `largest(area)`/`smallest(radius)`/`nearest(target)`
//!   precedent in the same §6 puts genuinely comparative operations under
//!   a separate "Ranking/disambiguation" heading) -- a single candidate
//!   has no boolean "is nearest" truth value in isolation. How a
//!   candidate-set-wide predicate like this composes with this module's
//!   per-candidate evaluator shape is a resolver-level design question,
//!   not a `GeometryPredicate`-style comparison this task can invent.
//!
//! # No invented numeric-tolerance policy
//!
//! `project/DECISION_LOG.md#DL-26` forbids a new numerical domain
//! silently borrowing another domain's tolerance (e.g. D5/D19's kernel
//! cross-build comparison profile) or inventing its own default. A
//! query-predicate `==`/`~=` "how close counts as equal" policy is
//! exactly such a new domain, and no task has named/owned one yet. This
//! module therefore never invents a "close enough" threshold:
//!
//! - [`Comparison::Eq`] against a [`Magnitude`] uses [`approx_eq`], a
//!   tiny *relative* floating-point-representation-noise allowance (not
//!   a geometric matching policy) -- the same kind of allowance ordinary
//!   floating-point code applies before comparing two independently
//!   computed reals for equality, sized only to absorb accumulated
//!   rounding, never to widen what counts as a geometric match.
//! - [`DirectionComparison`] with an explicit `tolerance` uses exactly
//!   that query-author-supplied value (never a value this module chose);
//!   with no `tolerance` given, direction comparison requires the two
//!   unit vectors to coincide up to the same representation-noise
//!   allowance as `approx_eq` -- i.e. "no tolerance stated" means
//!   "numerically exact," not "pick something reasonable."
//!
//! [`Shape::classify_point`]'s own `tolerance` parameter is a different
//! kind of number entirely -- an OCCT algorithm input required for the
//! classifier to run at all (every call needs *some* value), matching
//! this same native bridge's own precedent of hardcoding a small
//! geometric-construction tolerance directly (`native/occt_bridge/src/
//! aicad_occt_bridge.cpp`'s `aicad_occt_shell`/`aicad_occt_offset`, both
//! already pass a literal `1e-6` to their own OCCT join calls) -- not a
//! query-matching policy, so DL-26 does not apply to it.

use cad_kernel_api::{Direction3 as KernelDirection3, KernelError, KernelResult, Vector3};
use cad_occt_bridge::{PointClassification, Shape};
use cad_references::{AnyRef, EntityKind, FeatureAnchor};

use crate::predicate::{
    AdjacencyTarget, BoundaryKind, DirectionComparison, GeometryPredicate, RelativeDirection,
    SpatialPredicate, SpatialTarget, TopologyPredicate,
};
use crate::value::{Comparison, Direction3, Frame3, Magnitude, Point3 as QueryPoint3};

/// An OCCT-algorithm input for [`Shape::classify_point`], not a
/// query-matching tolerance policy -- see this module's own doc comment.
const POINT_CLASSIFY_TOLERANCE: f64 = 1e-7;

/// A relative floating-point-representation-noise allowance for
/// [`approx_eq`] -- see this module's own doc comment on why this is not
/// a DL-26 "tolerance domain."
const FLOAT_NOISE_RELATIVE: f64 = 1e-9;

/// One entity in a real build, already identified as belonging to
/// `kind`, that a predicate is evaluated against.
pub struct Candidate<'ctx> {
    kind: EntityKind,
    shape: Shape<'ctx>,
    /// The face `shape` (a Wire) bounds, when relevant to
    /// [`TopologyPredicate::Boundary`] -- a wire's own shape data alone
    /// never says which face it bounds. `None` for every other predicate
    /// and entity kind.
    parent_face: Option<Shape<'ctx>>,
}

impl<'ctx> Candidate<'ctx> {
    pub fn new(kind: EntityKind, shape: Shape<'ctx>) -> Self {
        Candidate {
            kind,
            shape,
            parent_face: None,
        }
    }

    /// A Wire candidate together with the Face it bounds, needed to
    /// evaluate [`TopologyPredicate::Boundary`] against it.
    pub fn wire_with_parent_face(shape: Shape<'ctx>, parent_face: Shape<'ctx>) -> Self {
        Candidate {
            kind: EntityKind::Wire,
            shape,
            parent_face: Some(parent_face),
        }
    }

    pub fn kind(&self) -> EntityKind {
        self.kind
    }

    pub fn shape(&self) -> &Shape<'ctx> {
        &self.shape
    }
}

/// Why a predicate could not be evaluated -- distinct from an ordinary
/// `false` (predicate evaluated cleanly and did not hold).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalError {
    /// The kernel bridge itself reported an error unrelated to any of
    /// the specific, documented "not applicable to this entity kind"
    /// cases this module handles as `Ok(false)` (e.g. a genuinely
    /// invalid handle, or a construction-time OCCT failure).
    Kernel(KernelError),
    /// Evaluating this predicate needs evidence
    /// [`EvaluationEvidence`] had no answer for -- see this module's own
    /// doc comment. Never treated as `false`.
    NoEvidence(&'static str),
    /// This predicate variant's evaluation semantics are not specified
    /// by any Stage-4 task yet -- see this module's own doc comment.
    NotYetSpecified(&'static str),
    /// The predicate's own authored data is malformed independent of any
    /// live geometry (e.g. a zero-length direction/axis) -- a query
    /// authoring error, not a "no match"/"no evidence" outcome.
    InvalidInput(&'static str),
}

impl From<KernelError> for EvalError {
    fn from(error: KernelError) -> Self {
        EvalError::Kernel(error)
    }
}

pub type EvalResult<T> = Result<T, EvalError>;

/// Injectable evidence [`evaluate_topology`]/[`evaluate_spatial`] consult
/// for the predicate families this module cannot itself resolve -- see
/// this module's own doc comment. Every method defaults to `None` ("no
/// evidence"); `NoEvidence` below is the zero-cost default implementation
/// for a caller (e.g. a test, or any context with no lineage/resolver
/// wired in yet) that supplies none of these.
pub trait EvaluationEvidence<'ctx> {
    fn generated_by(&self, _candidate: &Candidate<'ctx>, _anchor: &FeatureAnchor) -> Option<bool> {
        None
    }
    fn modified_by(&self, _candidate: &Candidate<'ctx>, _anchor: &FeatureAnchor) -> Option<bool> {
        None
    }
    fn descended_from(&self, _candidate: &Candidate<'ctx>, _ancestor: &AnyRef) -> Option<bool> {
        None
    }
    /// Resolves a bare reference (`inside(volume)`, `within(distance,
    /// ref)`) to the candidates it currently designates.
    fn resolve_ref(&self, _reference: &AnyRef) -> Option<Vec<Candidate<'ctx>>> {
        None
    }
    /// Resolves an [`AdjacencyTarget`] (`Ref` or nested `Query`) the same
    /// way.
    fn resolve_target(&self, _target: &AdjacencyTarget) -> Option<Vec<Candidate<'ctx>>> {
        None
    }
}

/// The zero-cost [`EvaluationEvidence`] implementation for a caller with
/// no lineage/resolver source wired in.
pub struct NoEvidence;

impl<'ctx> EvaluationEvidence<'ctx> for NoEvidence {}

// --- AICAD-082: geometry predicates ---

/// Evaluates a [`GeometryPredicate`] against `candidate`. Needs no
/// injected evidence: every geometry predicate is decidable from the
/// candidate's own kernel geometry alone.
pub fn evaluate_geometry(
    predicate: &GeometryPredicate,
    candidate: &Candidate<'_>,
) -> EvalResult<bool> {
    match predicate {
        GeometryPredicate::Planar => {
            is_surface_kind(candidate, cad_occt_bridge::SurfaceKind::Plane)
        }
        GeometryPredicate::Cylindrical => {
            is_surface_kind(candidate, cad_occt_bridge::SurfaceKind::Cylinder)
        }
        GeometryPredicate::Conical => {
            is_surface_kind(candidate, cad_occt_bridge::SurfaceKind::Cone)
        }
        GeometryPredicate::Spherical => {
            is_surface_kind(candidate, cad_occt_bridge::SurfaceKind::Sphere)
        }
        GeometryPredicate::Toroidal => {
            is_surface_kind(candidate, cad_occt_bridge::SurfaceKind::Torus)
        }
        GeometryPredicate::Bspline => {
            is_surface_kind(candidate, cad_occt_bridge::SurfaceKind::Bspline)
        }
        GeometryPredicate::Radius(cmp) => eval_radius(candidate, cmp),
        GeometryPredicate::Area(cmp) => Ok(compare_magnitude(cmp, candidate.shape.area()?)),
        GeometryPredicate::Length(cmp) => Ok(compare_magnitude(cmp, candidate.shape.length()?)),
        GeometryPredicate::Normal(dc) => eval_normal(candidate, dc),
        GeometryPredicate::Axis(dc) => eval_axis(candidate, dc),
    }
}

fn is_surface_kind(
    candidate: &Candidate<'_>,
    kind: cad_occt_bridge::SurfaceKind,
) -> EvalResult<bool> {
    if candidate.kind != EntityKind::Face {
        // Surface-family predicates are meaningless for a non-Face
        // candidate (a Vertex/Edge/Solid has no single "surface kind")
        // -- simply does not match, the same way an ordinary filter
        // predicate never matches an incomparable value.
        return Ok(false);
    }
    Ok(candidate.shape.surface_type()? == kind)
}

fn eval_radius(candidate: &Candidate<'_>, cmp: &Comparison<Magnitude>) -> EvalResult<bool> {
    let radius = match candidate.kind {
        EntityKind::Face => candidate.shape.face_radius(),
        EntityKind::Edge => candidate.shape.edge_radius(),
        _ => return Ok(false),
    };
    match radius {
        Ok(r) => Ok(compare_magnitude(cmp, r)),
        // No single well-defined radius for this face/edge's own
        // surface/curve kind (e.g. a planar face, or a conical face) --
        // simply does not match, not a hard error.
        Err(KernelError::InvalidArgument) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

fn eval_normal(candidate: &Candidate<'_>, dc: &DirectionComparison) -> EvalResult<bool> {
    if candidate.kind != EntityKind::Face {
        return Ok(false);
    }
    let (_, normal) = candidate.shape.face_normal()?;
    direction_matches(dc, normal)
}

fn eval_axis(candidate: &Candidate<'_>, dc: &DirectionComparison) -> EvalResult<bool> {
    let axis = match candidate.kind {
        EntityKind::Face => candidate.shape.face_axis(),
        EntityKind::Edge => candidate.shape.edge_axis(),
        _ => return Ok(false),
    };
    match axis {
        Ok(axis) => direction_matches(dc, axis.direction),
        Err(KernelError::InvalidArgument) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

// --- AICAD-083: topology predicates ---

/// Evaluates a [`TopologyPredicate`] against `candidate`, consulting
/// `evidence` for the predicate families this module cannot itself
/// resolve (see this module's own doc comment).
pub fn evaluate_topology<'ctx>(
    predicate: &TopologyPredicate,
    candidate: &Candidate<'ctx>,
    evidence: &dyn EvaluationEvidence<'ctx>,
) -> EvalResult<bool> {
    match predicate {
        TopologyPredicate::GeneratedBy(anchor) => {
            evidence
                .generated_by(candidate, anchor)
                .ok_or(EvalError::NoEvidence(
                    "generated_by requires feature lineage evidence (AICAD-085..087)",
                ))
        }
        TopologyPredicate::ModifiedBy(anchor) => {
            evidence
                .modified_by(candidate, anchor)
                .ok_or(EvalError::NoEvidence(
                    "modified_by requires feature lineage evidence (AICAD-085..087)",
                ))
        }
        TopologyPredicate::DescendedFrom(ancestor) => evidence
            .descended_from(candidate, ancestor)
            .ok_or(EvalError::NoEvidence(
                "descended_from requires feature lineage evidence (AICAD-085..087)",
            )),
        TopologyPredicate::AdjacentTo(target) => eval_adjacent_to(candidate, target, evidence),
        TopologyPredicate::Boundary(kind) => eval_boundary(candidate, *kind),
        TopologyPredicate::Convex
        | TopologyPredicate::Concave
        | TopologyPredicate::Manifold
        | TopologyPredicate::NonManifold
        | TopologyPredicate::ConnectedTo(_)
        | TopologyPredicate::Contains(_)
        | TopologyPredicate::Intersects(_) => Err(EvalError::NotYetSpecified(
            "not in AICAD-083's scope (generated_by/modified_by/descended_from/adjacent_to/\
             boundary only); no later Stage-4 task has specified this predicate's evaluation \
             semantics yet",
        )),
    }
}

fn eval_boundary(candidate: &Candidate<'_>, kind: BoundaryKind) -> EvalResult<bool> {
    if candidate.kind != EntityKind::Wire {
        return Ok(false);
    }
    let parent = candidate.parent_face.as_ref().ok_or(EvalError::NoEvidence(
        "boundary(outer|inner) requires the candidate wire's parent face, which the caller \
         did not supply (see Candidate::wire_with_parent_face)",
    ))?;
    let is_outer = parent.is_outer_wire(&candidate.shape)?;
    Ok(match kind {
        BoundaryKind::Outer => is_outer,
        BoundaryKind::Inner => !is_outer,
    })
}

fn eval_adjacent_to<'ctx>(
    candidate: &Candidate<'ctx>,
    target: &AdjacencyTarget,
    evidence: &dyn EvaluationEvidence<'ctx>,
) -> EvalResult<bool> {
    let targets = evidence
        .resolve_target(target)
        .ok_or(EvalError::NoEvidence(
            "adjacent_to requires its target reference/query to already be resolved (AICAD-088+)",
        ))?;
    for other in &targets {
        if shapes_are_adjacent(candidate, other)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Standard topological adjacency: two faces are adjacent iff they share
/// an edge; two edges are adjacent iff they share a vertex; a face and an
/// edge are adjacent iff the edge bounds the face. Every other
/// combination (a Vertex, Shell, or Solid on either side) is out of this
/// task's scope and reports `false` rather than guessing a definition.
fn shapes_are_adjacent(a: &Candidate<'_>, b: &Candidate<'_>) -> KernelResult<bool> {
    match (a.kind, b.kind) {
        (EntityKind::Face, EntityKind::Face) => any_shared_edge(&a.shape, &b.shape),
        (EntityKind::Edge, EntityKind::Edge) => any_shared_vertex(&a.shape, &b.shape),
        (EntityKind::Face, EntityKind::Edge) => face_has_edge(&a.shape, &b.shape),
        (EntityKind::Edge, EntityKind::Face) => face_has_edge(&b.shape, &a.shape),
        _ => Ok(false),
    }
}

fn any_shared_edge(a: &Shape<'_>, b: &Shape<'_>) -> KernelResult<bool> {
    for i in 0..a.edge_count()? {
        if face_has_edge(b, &a.get_edge(i)?)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn face_has_edge(face: &Shape<'_>, edge: &Shape<'_>) -> KernelResult<bool> {
    for i in 0..face.edge_count()? {
        if face.get_edge(i)?.is_same(edge)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn any_shared_vertex(a: &Shape<'_>, b: &Shape<'_>) -> KernelResult<bool> {
    let (a0, a1) = a.edge_vertices()?;
    let (b0, b1) = b.edge_vertices()?;
    Ok(a0.is_same(&b0)? || a0.is_same(&b1)? || a1.is_same(&b0)? || a1.is_same(&b1)?)
}

// --- AICAD-084: baseline spatial predicates ---

/// Evaluates a [`SpatialPredicate`] against `candidate`, consulting
/// `evidence` for reference resolution where needed.
pub fn evaluate_spatial<'ctx>(
    predicate: &SpatialPredicate,
    candidate: &Candidate<'ctx>,
    evidence: &dyn EvaluationEvidence<'ctx>,
) -> EvalResult<bool> {
    match predicate {
        SpatialPredicate::RelativeTo(direction, frame) => {
            eval_relative_to(candidate, *direction, frame)
        }
        SpatialPredicate::Inside(volume_ref) => eval_inside(candidate, volume_ref, evidence),
        SpatialPredicate::Within(distance, target) => {
            eval_within(candidate, distance, target, evidence)
        }
        SpatialPredicate::NearestTo(_) | SpatialPredicate::FarthestFrom(_) => {
            Err(EvalError::NotYetSpecified(
                "nearest_to/farthest_from are comparative across a candidate set, not a \
                 per-candidate boolean predicate; see this module's own doc comment",
            ))
        }
    }
}

fn eval_relative_to(
    candidate: &Candidate<'_>,
    direction: RelativeDirection,
    frame: &Frame3,
) -> EvalResult<bool> {
    let point = representative_point(candidate)?;
    let origin = to_kernel_point(frame.origin);
    let offset = point - origin;
    let up = to_kernel_direction(frame.z_axis).ok_or(EvalError::InvalidInput(
        "frame z_axis must be a nonzero direction",
    ))?;
    let right = to_kernel_direction(frame.x_axis).ok_or(EvalError::InvalidInput(
        "frame x_axis must be a nonzero direction",
    ))?;
    Ok(match direction {
        RelativeDirection::Above => offset.dot(up.as_vector3()) > 0.0,
        RelativeDirection::Below => offset.dot(up.as_vector3()) < 0.0,
        RelativeDirection::Right => offset.dot(right.as_vector3()) > 0.0,
        RelativeDirection::Left => offset.dot(right.as_vector3()) < 0.0,
    })
}

fn eval_inside<'ctx>(
    candidate: &Candidate<'ctx>,
    volume_ref: &AnyRef,
    evidence: &dyn EvaluationEvidence<'ctx>,
) -> EvalResult<bool> {
    let volumes = evidence
        .resolve_ref(volume_ref)
        .ok_or(EvalError::NoEvidence(
            "inside(volume) requires its target reference to already be resolved (AICAD-088+)",
        ))?;
    let point = representative_point(candidate)?;
    for volume in &volumes {
        let classification = volume
            .shape
            .classify_point(point, POINT_CLASSIFY_TOLERANCE)?;
        if matches!(
            classification,
            PointClassification::Inside | PointClassification::OnBoundary
        ) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn eval_within<'ctx>(
    candidate: &Candidate<'ctx>,
    distance: &Magnitude,
    target: &SpatialTarget,
    evidence: &dyn EvaluationEvidence<'ctx>,
) -> EvalResult<bool> {
    let point = representative_point(candidate)?;
    let target_points: Vec<cad_kernel_api::Point3> =
        match target {
            SpatialTarget::Point(p) => vec![to_kernel_point(*p)],
            SpatialTarget::Ref(reference) => {
                let candidates = evidence.resolve_ref(reference).ok_or(EvalError::NoEvidence(
                "within(distance, ref) requires its target reference to already be resolved \
                 (AICAD-088+)",
            ))?;
                candidates
                    .iter()
                    .map(representative_point)
                    .collect::<KernelResult<Vec<_>>>()?
            }
        };
    Ok(target_points
        .iter()
        .any(|target_point| (point - *target_point).length() <= distance.value))
}

// --- shared helpers ---

fn representative_point(candidate: &Candidate<'_>) -> KernelResult<cad_kernel_api::Point3> {
    if candidate.kind == EntityKind::Vertex {
        candidate.shape.vertex_point()
    } else {
        candidate.shape.center_of_mass()
    }
}

fn to_kernel_point(point: QueryPoint3) -> cad_kernel_api::Point3 {
    cad_kernel_api::Point3::new(point.x.value, point.y.value, point.z.value)
}

fn to_kernel_direction(direction: Direction3) -> Option<KernelDirection3> {
    Vector3::new(direction.x, direction.y, direction.z).normalize()
}

fn direction_matches(dc: &DirectionComparison, actual: KernelDirection3) -> EvalResult<bool> {
    let target = to_kernel_direction(dc.target).ok_or(EvalError::InvalidInput(
        "direction comparison target must be a nonzero direction",
    ))?;
    let dot = actual.dot(target).clamp(-1.0, 1.0);
    match dc.tolerance {
        // The query author's own explicit tolerance (assumed already
        // canonical-unit radians, matching this crate's `Magnitude`
        // convention), never a value this module chose.
        Some(tolerance) => Ok(dot >= tolerance.value.cos()),
        None => Ok(approx_eq(dot, 1.0)),
    }
}

fn compare_magnitude(cmp: &Comparison<Magnitude>, actual: f64) -> bool {
    match cmp {
        Comparison::Eq(m) => approx_eq(actual, m.value),
        Comparison::Lt(m) => actual < m.value,
        Comparison::Lte(m) => actual <= m.value,
        Comparison::Gt(m) => actual > m.value,
        Comparison::Gte(m) => actual >= m.value,
    }
}

/// A relative floating-point-representation-noise allowance -- see this
/// module's own doc comment on why this is not a DL-26 tolerance domain.
fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() <= a.abs().max(b.abs()) * FLOAT_NOISE_RELATIVE + f64::EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_occt_bridge::OcctContext;
    use cad_types::Dimension;
    use cad_units::OperandType;

    fn length(value: f64) -> Magnitude {
        Magnitude::new(value, OperandType::dimensional(Dimension::Length, None))
    }

    fn angle(value_radians: f64) -> Magnitude {
        Magnitude::new(
            value_radians,
            OperandType::dimensional(Dimension::Angle, None),
        )
    }

    fn faces<'ctx>(shape: &Shape<'ctx>) -> Vec<Candidate<'ctx>> {
        (0..shape.face_count().unwrap())
            .map(|i| Candidate::new(EntityKind::Face, shape.get_face(i).unwrap()))
            .collect()
    }

    fn edges<'ctx>(shape: &Shape<'ctx>) -> Vec<Candidate<'ctx>> {
        (0..shape.edge_count().unwrap())
            .map(|i| Candidate::new(EntityKind::Edge, shape.get_edge(i).unwrap()))
            .collect()
    }

    // --- AICAD-082 ---

    #[test]
    fn planar_matches_every_box_face_and_nothing_else() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        for candidate in faces(&cube) {
            assert!(evaluate_geometry(&GeometryPredicate::Planar, &candidate).unwrap());
            assert!(!evaluate_geometry(&GeometryPredicate::Cylindrical, &candidate).unwrap());
        }
        for candidate in edges(&cube) {
            assert!(!evaluate_geometry(&GeometryPredicate::Planar, &candidate).unwrap());
        }
    }

    #[test]
    fn cylindrical_matches_exactly_the_lateral_face_of_a_cylinder() {
        let context = OcctContext::new().unwrap();
        let cylinder = context.create_cylinder(2.0, 5.0).unwrap();
        let matches = faces(&cylinder)
            .into_iter()
            .filter(|c| evaluate_geometry(&GeometryPredicate::Cylindrical, c).unwrap())
            .count();
        assert_eq!(matches, 1);
    }

    #[test]
    fn radius_predicate_matches_the_cylinder_lateral_face() {
        let context = OcctContext::new().unwrap();
        let cylinder = context.create_cylinder(2.0, 5.0).unwrap();
        let lateral = faces(&cylinder)
            .into_iter()
            .find(|c| evaluate_geometry(&GeometryPredicate::Cylindrical, c).unwrap())
            .unwrap();
        let predicate = GeometryPredicate::Radius(Comparison::Eq(length(2.0)));
        assert!(evaluate_geometry(&predicate, &lateral).unwrap());
        let too_big = GeometryPredicate::Radius(Comparison::Gt(length(100.0)));
        assert!(!evaluate_geometry(&too_big, &lateral).unwrap());
    }

    #[test]
    fn radius_predicate_does_not_match_a_planar_face() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face = faces(&cube).into_iter().next().unwrap();
        let predicate = GeometryPredicate::Radius(Comparison::Gt(length(0.0)));
        assert!(!evaluate_geometry(&predicate, &face).unwrap());
    }

    #[test]
    fn area_predicate_matches_a_box_face_area() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 3.0, 4.0).unwrap();
        // Faces with area 2*3=6 exist (top/bottom), matching the plan's
        // own `area > 500mm^2`-shaped example.
        let has_area_6 = faces(&cube).into_iter().any(|c| {
            evaluate_geometry(&GeometryPredicate::Area(Comparison::Eq(length(6.0))), &c).unwrap()
        });
        assert!(has_area_6);
    }

    #[test]
    fn normal_predicate_matches_the_plus_z_box_face() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let predicate = GeometryPredicate::Normal(DirectionComparison {
            target: Direction3::POSITIVE_Z,
            tolerance: Some(angle(0.01)),
        });
        let matches = faces(&cube)
            .into_iter()
            .filter(|c| evaluate_geometry(&predicate, c).unwrap())
            .count();
        assert_eq!(
            matches, 1,
            "exactly one box face has +Z as its outward normal"
        );
    }

    #[test]
    fn axis_predicate_matches_the_cylinder_lateral_face_axis() {
        let context = OcctContext::new().unwrap();
        let cylinder = context.create_cylinder(2.0, 5.0).unwrap();
        let lateral = faces(&cylinder)
            .into_iter()
            .find(|c| evaluate_geometry(&GeometryPredicate::Cylindrical, c).unwrap())
            .unwrap();
        let predicate = GeometryPredicate::Axis(DirectionComparison {
            target: Direction3::POSITIVE_Z,
            tolerance: Some(angle(0.01)),
        });
        assert!(evaluate_geometry(&predicate, &lateral).unwrap());
    }

    // --- AICAD-083 ---

    #[test]
    fn generated_by_reports_no_evidence_without_a_lineage_source() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face = faces(&cube).into_iter().next().unwrap();
        let predicate = TopologyPredicate::GeneratedBy(FeatureAnchor::named("base"));
        assert_eq!(
            evaluate_topology(&predicate, &face, &NoEvidence).unwrap_err(),
            EvalError::NoEvidence(
                "generated_by requires feature lineage evidence (AICAD-085..087)"
            )
        );
    }

    struct FixedLineage(bool);
    impl<'ctx> EvaluationEvidence<'ctx> for FixedLineage {
        fn generated_by(&self, _c: &Candidate<'ctx>, _a: &FeatureAnchor) -> Option<bool> {
            Some(self.0)
        }
    }

    #[test]
    fn generated_by_uses_injected_lineage_evidence_when_available() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face = faces(&cube).into_iter().next().unwrap();
        let predicate = TopologyPredicate::GeneratedBy(FeatureAnchor::named("base"));
        assert!(evaluate_topology(&predicate, &face, &FixedLineage(true)).unwrap());
        assert!(!evaluate_topology(&predicate, &face, &FixedLineage(false)).unwrap());
    }

    /// Test-only evidence that resolves any [`AdjacencyTarget`] to a
    /// single face re-fetched (by index) from `root` on every call --
    /// `Shape` is an RAII handle with no `Clone`, so a fixed candidate
    /// set is represented as "how to re-derive it," not as stored data.
    struct OneFaceTarget<'a, 'ctx> {
        root: &'a Shape<'ctx>,
        index: usize,
    }
    impl<'a, 'ctx> EvaluationEvidence<'ctx> for OneFaceTarget<'a, 'ctx> {
        fn resolve_target(&self, _t: &AdjacencyTarget) -> Option<Vec<Candidate<'ctx>>> {
            Some(vec![Candidate::new(
                EntityKind::Face,
                self.root.get_face(self.index).unwrap(),
            )])
        }
    }

    fn any_ref_placeholder() -> AdjacencyTarget {
        AdjacencyTarget::Ref(cad_references::AnyRef::Face(
            cad_references::FaceRef::from_strategy(
                cad_references::ConstructionStrategy::StructuralRole("x".into()),
            ),
        ))
    }

    #[test]
    fn adjacent_to_reports_true_for_a_touching_face_and_false_for_a_disjoint_one() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face0 = Candidate::new(EntityKind::Face, cube.get_face(0).unwrap());
        let predicate = TopologyPredicate::AdjacentTo(any_ref_placeholder());
        // Every other face of a box either shares an edge with face 0 or
        // is the single opposite (parallel, non-touching) face -- try
        // all and confirm both outcomes occur, without hardcoding OCCT's
        // own face-enumeration order.
        let mut saw_true = false;
        let mut saw_false = false;
        for i in 1..cube.face_count().unwrap() {
            let evidence = OneFaceTarget {
                root: &cube,
                index: i,
            };
            if evaluate_topology(&predicate, &face0, &evidence).unwrap() {
                saw_true = true;
            } else {
                saw_false = true;
            }
        }
        assert!(
            saw_true,
            "a box face must be adjacent to at least one other face"
        );
        assert!(saw_false, "a box face is not adjacent to the opposite face");
    }

    #[test]
    fn adjacent_to_reports_no_evidence_without_a_resolver() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face0 = Candidate::new(EntityKind::Face, cube.get_face(0).unwrap());
        let predicate = TopologyPredicate::AdjacentTo(any_ref_placeholder());
        assert_eq!(
            evaluate_topology(&predicate, &face0, &NoEvidence).unwrap_err(),
            EvalError::NoEvidence(
                "adjacent_to requires its target reference/query to already be resolved (AICAD-088+)"
            )
        );
    }

    #[test]
    fn convex_and_nearest_to_are_explicitly_not_yet_specified() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let edge = Candidate::new(EntityKind::Edge, cube.get_edge(0).unwrap());
        assert!(matches!(
            evaluate_topology(&TopologyPredicate::Convex, &edge, &NoEvidence),
            Err(EvalError::NotYetSpecified(_))
        ));
        let face = Candidate::new(EntityKind::Face, cube.get_face(0).unwrap());
        assert!(matches!(
            evaluate_spatial(
                &SpatialPredicate::NearestTo(SpatialTarget::Point(QueryPoint3::new(
                    length(0.0),
                    length(0.0),
                    length(0.0)
                ))),
                &face,
                &NoEvidence
            ),
            Err(EvalError::NotYetSpecified(_))
        ));
    }

    #[test]
    fn boundary_outer_matches_a_box_faces_only_wire() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face = cube.get_face(0).unwrap();
        let wire = face.get_wire(0).unwrap();
        let candidate = Candidate::wire_with_parent_face(wire, cube.get_face(0).unwrap());
        assert!(
            evaluate_topology(
                &TopologyPredicate::Boundary(BoundaryKind::Outer),
                &candidate,
                &NoEvidence
            )
            .unwrap()
        );
        assert!(
            !evaluate_topology(
                &TopologyPredicate::Boundary(BoundaryKind::Inner),
                &candidate,
                &NoEvidence
            )
            .unwrap()
        );
    }

    #[test]
    fn boundary_without_a_parent_face_is_no_evidence() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let wire = cube.get_face(0).unwrap().get_wire(0).unwrap();
        let candidate = Candidate::new(EntityKind::Wire, wire);
        assert!(matches!(
            evaluate_topology(
                &TopologyPredicate::Boundary(BoundaryKind::Outer),
                &candidate,
                &NoEvidence
            ),
            Err(EvalError::NoEvidence(_))
        ));
    }

    // --- AICAD-084 ---

    #[test]
    fn relative_to_above_and_below_split_a_boxs_faces_around_its_center() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let frame = Frame3 {
            origin: QueryPoint3::new(length(1.0), length(1.0), length(1.0)),
            x_axis: Direction3::POSITIVE_X,
            y_axis: Direction3::POSITIVE_Y,
            z_axis: Direction3::POSITIVE_Z,
        };
        let above = faces(&cube)
            .into_iter()
            .filter(|c| {
                evaluate_spatial(
                    &SpatialPredicate::RelativeTo(RelativeDirection::Above, frame),
                    c,
                    &NoEvidence,
                )
                .unwrap()
            })
            .count();
        let below = faces(&cube)
            .into_iter()
            .filter(|c| {
                evaluate_spatial(
                    &SpatialPredicate::RelativeTo(RelativeDirection::Below, frame),
                    c,
                    &NoEvidence,
                )
                .unwrap()
            })
            .count();
        // A cube centered on the frame origin has exactly 1 face strictly
        // above (top, z-centroid 2 > origin's 1) and 1 strictly below
        // (bottom, z-centroid 0 < 1); the other 4 side faces each have a
        // z-centroid exactly equal to the origin's own (1), so their
        // offset's projection onto the "up" axis is exactly zero --
        // matching neither `Above` nor `Below`.
        assert_eq!(above, 1);
        assert_eq!(below, 1);
    }

    /// Test-only evidence resolving any [`AnyRef`] to a fresh 2x2x2 box
    /// solid built from `context` on every call -- `Shape` has no
    /// `Clone`, so "the same target volume" is represented as
    /// "reconstruct this same geometry," not as stored data.
    struct BoxVolumeTarget<'ctx> {
        context: &'ctx OcctContext,
    }
    impl<'ctx> EvaluationEvidence<'ctx> for BoxVolumeTarget<'ctx> {
        fn resolve_ref(&self, _reference: &AnyRef) -> Option<Vec<Candidate<'ctx>>> {
            let solid = self.context.create_box(2.0, 2.0, 2.0).unwrap();
            Some(vec![Candidate::new(EntityKind::Solid, solid)])
        }
    }

    #[test]
    fn inside_reports_true_for_a_point_at_the_box_center_and_false_far_away() {
        let context = OcctContext::new().unwrap();
        let probe = context.create_box(2.0, 2.0, 2.0).unwrap();
        let center = probe.center_of_mass().unwrap();
        let far = cad_kernel_api::Point3::new(center.x + 1000.0, center.y, center.z);

        let center_candidate = Candidate::new(EntityKind::Vertex, vertex_at(&context, center));
        let far_candidate = Candidate::new(EntityKind::Vertex, vertex_at(&context, far));

        let evidence = BoxVolumeTarget { context: &context };
        let volume_ref = AnyRef::Solid(cad_references::SolidRef::from_strategy(
            cad_references::ConstructionStrategy::StructuralRole("volume".into()),
        ));
        let predicate = SpatialPredicate::Inside(volume_ref);

        assert!(evaluate_spatial(&predicate, &center_candidate, &evidence).unwrap());
        assert!(!evaluate_spatial(&predicate, &far_candidate, &evidence).unwrap());
    }

    /// Builds a standalone vertex at exactly `p`, via a tiny line edge's
    /// own first endpoint -- there is no direct "make a vertex" kernel
    /// constructor, so this reuses `make_line_edge` + `edge_vertices`
    /// (both already proven elsewhere in `cad-occt-bridge`'s own test
    /// suite) as the minimal existing path to an isolated point-shaped
    /// candidate for these tests.
    fn vertex_at<'ctx>(context: &'ctx OcctContext, p: cad_kernel_api::Point3) -> Shape<'ctx> {
        let offset = p.translate(Vector3::new(0.01, 0.0, 0.0));
        let edge = context.make_line_edge(p, offset).unwrap();
        let (v0, _v1) = edge.edge_vertices().unwrap();
        v0
    }

    #[test]
    fn within_reports_true_for_a_near_point_and_false_for_a_far_one() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face = Candidate::new(EntityKind::Face, cube.get_face(0).unwrap());
        let point = representative_point(&face).unwrap();
        let near_target = SpatialTarget::Point(QueryPoint3::new(
            length(point.x),
            length(point.y),
            length(point.z),
        ));
        let near = SpatialPredicate::Within(length(0.001), near_target);
        assert!(evaluate_spatial(&near, &face, &NoEvidence).unwrap());

        let far_target = SpatialTarget::Point(QueryPoint3::new(
            length(point.x + 1000.0),
            length(point.y + 1000.0),
            length(point.z + 1000.0),
        ));
        let far = SpatialPredicate::Within(length(0.001), far_target);
        assert!(!evaluate_spatial(&far, &face, &NoEvidence).unwrap());
    }
}
