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
//! # `Convex`/`Concave`/`Manifold`/`NonManifold`/`ConnectedTo`/`Contains`/
//! `Intersects` (`AICAD-100A`)
//!
//! These seven `TopologyPredicate` variants were originally left
//! `EvalError::NotYetSpecified` by `AICAD-083` (whose own task title named
//! only `generated_by`/`modified_by`/`descended_from`/`adjacent_to`/
//! `boundary`) pending a defined entity-kind restriction and geometric
//! test for each. `AICAD-100A` specifies and implements all seven with
//! explicit, tested topology-kind restrictions (see each `eval_*`
//! function's own doc comment for the exact kind/algorithm):
//! `Convex`/`Concave`/`Manifold`/`NonManifold` apply only to an
//! [`EntityKind::Edge`]; `ConnectedTo` only to [`EntityKind::Face`];
//! `Contains`/`Intersects` only to [`EntityKind::Solid`]. Every other
//! candidate kind reports `Ok(false)` ("does not apply"), never a guessed
//! match. `Convex`/`Concave`/`Manifold`/`NonManifold`/`ConnectedTo`
//! additionally need the candidate's own enclosing shape
//! ([`Candidate::with_root`]) to evaluate adjacency beyond the single
//! candidate — absent that, they report [`EvalError::NoEvidence`], never a
//! guess.
//!
//! # `SpatialPredicate::{NearestTo, FarthestFrom}` (`AICAD-100A`)
//!
//! These remain genuinely comparative across a candidate *set* — a single
//! candidate still has no boolean "is nearest" truth value in isolation,
//! so [`evaluate_spatial`] itself still reports `EvalError::
//! NotYetSpecified` for a *direct* call bypassing the resolver, honestly:
//! this per-candidate evaluator layer cannot decide them alone. Real
//! production semantics exist one layer up instead: `crate::resolve::
//! filter_and_rank` rewrites a `QueryClause::Spatial(SpatialPredicate::
//! NearestTo(target))`/`FarthestFrom(target)` clause into the equivalent
//! `RankingDirective::Nearest`/`Farthest` before evaluation — the exact
//! precedent `crate::resolve`'s own module doc comment already establishes
//! for `ConstructionStrategy::FeatureLineage`/`Ancestry` ("resolved by
//! rewriting the recipe into an equivalent ... query and reusing the exact
//! same evaluator/evidence path"), applied to a query clause instead of a
//! reference recipe. Every real query executed through `cad_query::
//! resolve_query`/`resolve_reference` (i.e. every production path in this
//! workspace) therefore gives `nearest_to`/`farthest_from` real,
//! already-tested comparative semantics via `crate::resolve::
//! keep_nearest`, never a placeholder.
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

/// A minimum-distance-from-the-tangent-plane threshold (kernel-internal
/// length units) below which two adjacent faces are treated as tangent
/// (neither convex nor concave) rather than guessing a sign from float
/// noise -- see `eval_convexity`'s own doc comment. Like
/// `POINT_CLASSIFY_TOLERANCE`, this is a numerical-robustness input to a
/// specific geometric algorithm, not a DL-26 query-matching tolerance
/// domain.
const CONVEXITY_EPSILON: f64 = 1e-9;

/// A minimum post-boolean-intersection volume (kernel-internal cubic
/// length units) above which two solids are considered to genuinely
/// intersect, rather than share only a float-noise-thin sliver an exact
/// boolean operation can produce for two solids that are actually merely
/// touching -- see `eval_intersects`'s own doc comment.
const INTERSECTION_VOLUME_EPSILON: f64 = 1e-12;

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
    /// The whole shape `shape` was enumerated from (`AICAD-100A`) --
    /// needed for predicates that require adjacency/connectivity context
    /// beyond the single candidate itself (`Convex`/`Concave`/`Manifold`/
    /// `NonManifold`/`ConnectedTo`). `None` for a candidate built without
    /// that context (e.g. every pre-`AICAD-100A` construction site/test
    /// fixture, and every candidate of a kind those predicates do not
    /// apply to) -- those predicates report [`EvalError::NoEvidence`]
    /// rather than guessing when this is absent, exactly like
    /// [`TopologyPredicate::Boundary`]'s own `parent_face`.
    root: Option<Shape<'ctx>>,
}

impl<'ctx> Candidate<'ctx> {
    pub fn new(kind: EntityKind, shape: Shape<'ctx>) -> Self {
        Candidate {
            kind,
            shape,
            parent_face: None,
            root: None,
        }
    }

    /// A Wire candidate together with the Face it bounds, needed to
    /// evaluate [`TopologyPredicate::Boundary`] against it.
    pub fn wire_with_parent_face(shape: Shape<'ctx>, parent_face: Shape<'ctx>) -> Self {
        Candidate {
            kind: EntityKind::Wire,
            shape,
            parent_face: Some(parent_face),
            root: None,
        }
    }

    /// A candidate built together with the whole shape it was enumerated
    /// from (`AICAD-100A`) -- required by `Convex`/`Concave`/`Manifold`/
    /// `NonManifold`/`ConnectedTo`; see [`Candidate::root`]'s own doc
    /// comment. `root` should be an independently-owned handle (e.g.
    /// `cad_occt_bridge::Shape::duplicate`), never the same handle another
    /// live `Candidate`/caller still holds an exclusive reference to.
    pub fn with_root(kind: EntityKind, shape: Shape<'ctx>, root: Shape<'ctx>) -> Self {
        Candidate {
            kind,
            shape,
            parent_face: None,
            root: Some(root),
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
        TopologyPredicate::Convex => eval_convexity(candidate, true),
        TopologyPredicate::Concave => eval_convexity(candidate, false),
        TopologyPredicate::Manifold => eval_manifold(candidate, true),
        TopologyPredicate::NonManifold => eval_manifold(candidate, false),
        TopologyPredicate::ConnectedTo(target) => eval_connected_to(candidate, target, evidence),
        TopologyPredicate::Contains(point) => eval_contains(candidate, point),
        TopologyPredicate::Intersects(target) => eval_intersects(candidate, target, evidence),
    }
}

/// `Convex`/`Concave` (`AICAD-100A`) -- defined only for an interior
/// manifold [`EntityKind::Edge`] (exactly two adjacent faces; any other
/// candidate kind, or an edge with a different adjacent-face count, does
/// not match either predicate -- `Ok(false)`, never a guess). Classifies
/// the edge via a standard tangent-plane probe: at each of the edge's two
/// adjacent faces' own representative point/outward-normal pair
/// ([`Shape::face_normal`]), the *other* face's own representative point
/// lies either on the material (inward) side of this face's tangent
/// plane -- a convex edge, material bulging outward, like an ordinary box
/// corner -- or on the non-material (outward) side -- a concave edge,
/// material recessing inward, like the inner corner of a pocket/notch.
/// Verified against both textbook cases (a box corner; the inner corner
/// of a rectangular notch) before being trusted here. Faces closer than
/// [`CONVEXITY_EPSILON`] to exactly tangent (coplanar) report `false` for
/// *both* `Convex` and `Concave`, never an arbitrary sign from float
/// noise.
fn eval_convexity(candidate: &Candidate<'_>, want_convex: bool) -> EvalResult<bool> {
    if candidate.kind != EntityKind::Edge {
        return Ok(false);
    }
    let root = candidate.root.as_ref().ok_or(EvalError::NoEvidence(
        "convex/concave requires the candidate's own enclosing root shape (Candidate::with_root)",
    ))?;
    let Some(edge_index) = find_edge_index(root, &candidate.shape)? else {
        return Ok(false);
    };
    if root.edge_adjacent_face_count(edge_index)? != 2 {
        // Not a well-defined interior manifold edge -- a free boundary
        // edge or a non-manifold junction has no convex/concave meaning.
        return Ok(false);
    }
    let f1 = root.edge_adjacent_face(edge_index, 0)?;
    let f2 = root.edge_adjacent_face(edge_index, 1)?;
    let (c1, n1) = f1.face_normal()?;
    let (c2, _) = f2.face_normal()?;
    let d = (c2 - c1).dot(n1.as_vector3());
    if d.abs() < CONVEXITY_EPSILON {
        return Ok(false);
    }
    let is_convex = d < 0.0;
    Ok(is_convex == want_convex)
}

/// Every index into `root`'s own [`Shape::edge_count`] enumeration whose
/// edge [`Shape::is_same`]-matches `edge` -- the parent-shape-relative
/// index [`Shape::edge_adjacent_face_count`]/[`Shape::edge_adjacent_face`]
/// need, since a bare edge [`Candidate`] carries no index of its own.
/// Returns the first match (an edge is never listed twice in one shape's
/// own unique-edge enumeration, per [`Shape::edge_count`]'s own doc
/// comment).
fn find_edge_index(root: &Shape<'_>, edge: &Shape<'_>) -> KernelResult<Option<usize>> {
    let count = root.edge_count()?;
    for i in 0..count {
        if root.get_edge(i)?.is_same(edge)? {
            return Ok(Some(i));
        }
    }
    Ok(None)
}

/// `Manifold`/`NonManifold` (`AICAD-100A`) -- defined only for
/// [`EntityKind::Edge`]: manifold means exactly two adjacent faces (an
/// ordinary interior edge); non-manifold means any other adjacent-face
/// count (one, a free/boundary edge; three or more, a non-manifold
/// junction). Any other candidate kind does not match either -- `Ok(false)`.
fn eval_manifold(candidate: &Candidate<'_>, want_manifold: bool) -> EvalResult<bool> {
    if candidate.kind != EntityKind::Edge {
        return Ok(false);
    }
    let root = candidate.root.as_ref().ok_or(EvalError::NoEvidence(
        "manifold/non_manifold requires the candidate's own enclosing root shape \
         (Candidate::with_root)",
    ))?;
    let Some(edge_index) = find_edge_index(root, &candidate.shape)? else {
        return Ok(false);
    };
    let adjacent = root.edge_adjacent_face_count(edge_index)?;
    Ok((adjacent == 2) == want_manifold)
}

/// `ConnectedTo` (`AICAD-100A`) -- defined only for [`EntityKind::Face`]:
/// true iff a path of face-to-face shared-edge adjacency (a real
/// breadth-first search over every face of the candidate's own enclosing
/// [`Candidate::root`] shape, reusing [`any_shared_edge`]) connects
/// `candidate` to *any* of `target`'s own resolved candidates -- a
/// genuine graph-connectivity test, never a coincidental geometric
/// heuristic. Any other candidate kind does not match -- `Ok(false)`.
fn eval_connected_to<'ctx>(
    candidate: &Candidate<'ctx>,
    target: &AdjacencyTarget,
    evidence: &dyn EvaluationEvidence<'ctx>,
) -> EvalResult<bool> {
    if candidate.kind != EntityKind::Face {
        return Ok(false);
    }
    let root = candidate.root.as_ref().ok_or(EvalError::NoEvidence(
        "connected_to requires the candidate's own enclosing root shape (Candidate::with_root)",
    ))?;
    let targets = evidence
        .resolve_target(target)
        .ok_or(EvalError::NoEvidence(
            "connected_to requires its target reference/query to already be resolved",
        ))?;

    let face_count = root.face_count()?;
    let mut faces = Vec::with_capacity(face_count);
    for i in 0..face_count {
        faces.push(root.get_face(i)?);
    }
    let Some(start) = faces
        .iter()
        .position(|f| f.is_same(&candidate.shape).unwrap_or(false))
    else {
        return Ok(false);
    };

    let mut visited = vec![false; face_count];
    let mut queue = std::collections::VecDeque::new();
    visited[start] = true;
    queue.push_back(start);
    while let Some(i) = queue.pop_front() {
        if targets
            .iter()
            .any(|t| t.shape().is_same(&faces[i]).unwrap_or(false))
        {
            return Ok(true);
        }
        for j in 0..face_count {
            if !visited[j] && any_shared_edge(&faces[i], &faces[j])? {
                visited[j] = true;
                queue.push_back(j);
            }
        }
    }
    Ok(false)
}

/// `Contains(point)` (`AICAD-100A`) -- defined only for
/// [`EntityKind::Solid`], via exact point-vs-solid classification
/// ([`Shape::classify_point`], never a bounding-box/mesh approximation) --
/// the volumetric-containment counterpart to [`SpatialPredicate::Inside`]
/// (which tests whether a candidate's own representative point lies
/// inside an already-resolved *reference*; this tests whether the
/// candidate *itself* contains a literal authored point). Any other
/// candidate kind does not match -- `Ok(false)`.
fn eval_contains(candidate: &Candidate<'_>, point: &QueryPoint3) -> EvalResult<bool> {
    if candidate.kind != EntityKind::Solid {
        return Ok(false);
    }
    let kernel_point = to_kernel_point(*point);
    match candidate
        .shape
        .classify_point(kernel_point, POINT_CLASSIFY_TOLERANCE)
    {
        Ok(classification) => Ok(matches!(
            classification,
            PointClassification::Inside | PointClassification::OnBoundary
        )),
        Err(KernelError::InvalidArgument) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// `Intersects(target)` (`AICAD-100A`) -- defined only for a
/// [`EntityKind::Solid`] candidate against a `target` whose own resolved
/// candidates include at least one Solid: performs the real kernel
/// boolean intersection ([`Shape::intersect`]) and reports `true` iff the
/// result has non-negligible volume ([`INTERSECTION_VOLUME_EPSILON`]) --
/// two solids that merely touch along a zero-volume boundary (sharing a
/// face/edge/vertex only) are not "intersecting" in the volumetric sense
/// this predicate names. Any other candidate kind, or a target with no
/// Solid candidate, does not match -- `Ok(false)`, never a guess. A
/// kernel boolean failure against one target candidate is treated as "no
/// intersection with that candidate" (not every solid pair is booleanable
/// -- see [`Shape::intersect`]'s own doc comment) rather than aborting the
/// whole predicate.
fn eval_intersects<'ctx>(
    candidate: &Candidate<'ctx>,
    target: &AdjacencyTarget,
    evidence: &dyn EvaluationEvidence<'ctx>,
) -> EvalResult<bool> {
    if candidate.kind != EntityKind::Solid {
        return Ok(false);
    }
    let targets = evidence
        .resolve_target(target)
        .ok_or(EvalError::NoEvidence(
            "intersects requires its target reference/query to already be resolved",
        ))?;
    for other in &targets {
        if other.kind() != EntityKind::Solid {
            continue;
        }
        if let Ok(result) = candidate.shape.intersect(other.shape())
            && result.volume().unwrap_or(0.0) > INTERSECTION_VOLUME_EPSILON
        {
            return Ok(true);
        }
    }
    Ok(false)
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
    use cad_kernel_api::{Transform, Vector3};
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
    fn nearest_to_still_reports_not_yet_specified_for_a_direct_per_candidate_call() {
        // The real production semantics for `nearest_to`/`farthest_from`
        // now exist at the query/resolver level (`crate::resolve`'s own
        // clause-rewriting, `AICAD-100A`) -- a *direct* call to this bare
        // per-candidate evaluator still cannot decide them alone, honestly
        // (see this module's own doc comment).
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
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

    // --- AICAD-100A: Convex/Concave/Manifold/NonManifold/ConnectedTo/
    // Contains/Intersects ---

    fn edge_candidates_with_root<'ctx>(root: &'ctx Shape<'ctx>) -> Vec<Candidate<'ctx>> {
        (0..root.edge_count().unwrap())
            .map(|i| {
                Candidate::with_root(
                    EntityKind::Edge,
                    root.get_edge(i).unwrap(),
                    root.duplicate().unwrap(),
                )
            })
            .collect()
    }

    #[test]
    fn every_box_edge_is_convex_and_none_are_concave() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        for candidate in edge_candidates_with_root(&cube) {
            assert!(
                evaluate_topology(&TopologyPredicate::Convex, &candidate, &NoEvidence).unwrap(),
                "every ordinary box edge is convex"
            );
            assert!(
                !evaluate_topology(&TopologyPredicate::Concave, &candidate, &NoEvidence).unwrap()
            );
        }
    }

    /// A box with a rectangular notch cut from one corner has both convex
    /// edges (the box's own remaining outer corners) and exactly one
    /// concave edge (the inner corner of the notch, running the full
    /// height) -- the textbook counter-example `eval_convexity`'s own doc
    /// comment cites as having been checked before being trusted.
    #[test]
    fn a_notched_box_has_both_convex_and_concave_edges() {
        let context = OcctContext::new().unwrap();
        let base = context.create_box(10.0, 10.0, 10.0).unwrap();
        let notch_raw = context.create_box(5.0, 5.0, 12.0).unwrap();
        let notch = notch_raw
            .transform(&Transform::translation(Vector3::new(6.0, 6.0, -1.0)))
            .unwrap();
        let notched = base.cut(&notch).expect("cut should succeed");

        let mut saw_convex = false;
        let mut saw_concave = false;
        for candidate in edge_candidates_with_root(&notched) {
            if evaluate_topology(&TopologyPredicate::Convex, &candidate, &NoEvidence).unwrap() {
                saw_convex = true;
            }
            if evaluate_topology(&TopologyPredicate::Concave, &candidate, &NoEvidence).unwrap() {
                saw_concave = true;
            }
        }
        assert!(saw_convex, "the notched box's own outer corners are convex");
        assert!(
            saw_concave,
            "the notch's own inner corner edge must be classified concave"
        );
    }

    #[test]
    fn convex_and_concave_do_not_apply_to_a_non_edge_candidate() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face = Candidate::with_root(
            EntityKind::Face,
            cube.get_face(0).unwrap(),
            cube.duplicate().unwrap(),
        );
        assert!(!evaluate_topology(&TopologyPredicate::Convex, &face, &NoEvidence).unwrap());
        assert!(!evaluate_topology(&TopologyPredicate::Concave, &face, &NoEvidence).unwrap());
    }

    #[test]
    fn convex_without_a_root_shape_is_no_evidence() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let edge = Candidate::new(EntityKind::Edge, cube.get_edge(0).unwrap());
        assert!(matches!(
            evaluate_topology(&TopologyPredicate::Convex, &edge, &NoEvidence),
            Err(EvalError::NoEvidence(_))
        ));
    }

    #[test]
    fn every_box_edge_is_manifold_and_none_are_non_manifold() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        for candidate in edge_candidates_with_root(&cube) {
            assert!(
                evaluate_topology(&TopologyPredicate::Manifold, &candidate, &NoEvidence).unwrap()
            );
            assert!(
                !evaluate_topology(&TopologyPredicate::NonManifold, &candidate, &NoEvidence)
                    .unwrap()
            );
        }
    }

    /// A single standalone [`Shape`] of kind Face (detached from any
    /// solid -- e.g. one face lifted out of a box) has every one of its
    /// own boundary edges adjacent to exactly *one* face within its own
    /// enclosing shape (itself): a real, textbook free-boundary
    /// (non-manifold) edge, unlike the same edge back when it was part of
    /// a closed solid (adjacent to two faces there).
    #[test]
    fn a_standalone_faces_own_boundary_edges_are_non_manifold() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let face = cube.get_face(0).unwrap();
        assert_eq!(
            face.edge_count().unwrap(),
            4,
            "a box face has four boundary edges"
        );

        for candidate in edge_candidates_with_root(&face) {
            assert!(
                evaluate_topology(&TopologyPredicate::NonManifold, &candidate, &NoEvidence)
                    .unwrap(),
                "a standalone face's own boundary edge has only itself as an adjacent face"
            );
            assert!(
                !evaluate_topology(&TopologyPredicate::Manifold, &candidate, &NoEvidence).unwrap()
            );
        }
    }

    struct FixedTarget<'ctx>(Vec<Candidate<'ctx>>);
    impl<'ctx> EvaluationEvidence<'ctx> for FixedTarget<'ctx> {
        fn resolve_target(&self, _t: &AdjacencyTarget) -> Option<Vec<Candidate<'ctx>>> {
            Some(
                self.0
                    .iter()
                    .map(|c| Candidate::new(c.kind(), c.shape().duplicate().unwrap()))
                    .collect(),
            )
        }
    }

    #[test]
    fn connected_to_reaches_a_non_adjacent_face_of_the_same_solid() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let start = Candidate::with_root(
            EntityKind::Face,
            cube.get_face(0).unwrap(),
            cube.duplicate().unwrap(),
        );
        // The face directly opposite face 0 shares no edge with it, but is
        // still reachable via the other four faces -- a real transitive
        // BFS result, not a direct-adjacency check.
        let opposite = (1..cube.face_count().unwrap())
            .map(|i| cube.get_face(i).unwrap())
            .find(|f| !any_shared_edge(&cube.get_face(0).unwrap(), f).unwrap())
            .expect("a box has exactly one face not directly adjacent to face 0");
        let evidence = FixedTarget(vec![Candidate::new(EntityKind::Face, opposite)]);
        assert!(
            evaluate_topology(
                &TopologyPredicate::ConnectedTo(any_ref_placeholder()),
                &start,
                &evidence
            )
            .unwrap()
        );
    }

    #[test]
    fn connected_to_is_false_across_two_unrelated_solids() {
        let context = OcctContext::new().unwrap();
        let a = context.create_box(2.0, 2.0, 2.0).unwrap();
        let b = context.create_box(2.0, 2.0, 2.0).unwrap();
        let start = Candidate::with_root(
            EntityKind::Face,
            a.get_face(0).unwrap(),
            a.duplicate().unwrap(),
        );
        let evidence = FixedTarget(vec![Candidate::new(
            EntityKind::Face,
            b.get_face(0).unwrap(),
        )]);
        assert!(
            !evaluate_topology(
                &TopologyPredicate::ConnectedTo(any_ref_placeholder()),
                &start,
                &evidence
            )
            .unwrap()
        );
    }

    #[test]
    fn connected_to_does_not_apply_to_a_non_face_candidate() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let edge = Candidate::with_root(
            EntityKind::Edge,
            cube.get_edge(0).unwrap(),
            cube.duplicate().unwrap(),
        );
        let evidence = FixedTarget(vec![Candidate::new(
            EntityKind::Face,
            cube.get_face(0).unwrap(),
        )]);
        assert!(
            !evaluate_topology(
                &TopologyPredicate::ConnectedTo(any_ref_placeholder()),
                &edge,
                &evidence
            )
            .unwrap()
        );
    }

    #[test]
    fn contains_reports_true_for_the_box_center_and_false_far_away() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let solid = Candidate::new(EntityKind::Solid, cube);
        let center = QueryPoint3::new(length(1.0), length(1.0), length(1.0));
        let far = QueryPoint3::new(length(1000.0), length(1000.0), length(1000.0));
        assert!(
            evaluate_topology(&TopologyPredicate::Contains(center), &solid, &NoEvidence).unwrap()
        );
        assert!(
            !evaluate_topology(&TopologyPredicate::Contains(far), &solid, &NoEvidence).unwrap()
        );
    }

    #[test]
    fn contains_does_not_apply_to_a_face_candidate() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face = Candidate::new(EntityKind::Face, cube.get_face(0).unwrap());
        let point = QueryPoint3::new(length(0.5), length(0.5), length(0.5));
        assert!(
            !evaluate_topology(&TopologyPredicate::Contains(point), &face, &NoEvidence).unwrap()
        );
    }

    #[test]
    fn intersects_is_true_for_overlapping_solids_and_false_for_disjoint_ones() {
        let context = OcctContext::new().unwrap();
        let a = context.create_box(2.0, 2.0, 2.0).unwrap();
        let overlapping = context
            .create_box(2.0, 2.0, 2.0)
            .unwrap()
            .transform(&Transform::translation(Vector3::new(1.0, 0.0, 0.0)))
            .unwrap();
        let disjoint = context
            .create_box(2.0, 2.0, 2.0)
            .unwrap()
            .transform(&Transform::translation(Vector3::new(100.0, 0.0, 0.0)))
            .unwrap();

        let candidate = Candidate::new(EntityKind::Solid, a);
        let overlapping_evidence =
            FixedTarget(vec![Candidate::new(EntityKind::Solid, overlapping)]);
        assert!(
            evaluate_topology(
                &TopologyPredicate::Intersects(any_ref_placeholder()),
                &candidate,
                &overlapping_evidence
            )
            .unwrap()
        );

        let disjoint_evidence = FixedTarget(vec![Candidate::new(EntityKind::Solid, disjoint)]);
        assert!(
            !evaluate_topology(
                &TopologyPredicate::Intersects(any_ref_placeholder()),
                &candidate,
                &disjoint_evidence
            )
            .unwrap()
        );
    }

    #[test]
    fn intersects_does_not_apply_to_a_face_candidate() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let face = Candidate::new(EntityKind::Face, cube.get_face(0).unwrap());
        let evidence = FixedTarget(vec![Candidate::new(
            EntityKind::Solid,
            cube.duplicate().unwrap(),
        )]);
        assert!(
            !evaluate_topology(
                &TopologyPredicate::Intersects(any_ref_placeholder()),
                &face,
                &evidence
            )
            .unwrap()
        );
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
