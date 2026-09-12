//! Lowers a solved, closed [`cad_hir::sketch::Profile`] to an exact
//! kernel face (`AICAD-075`, "Lower solved closed sketch profiles to
//! exact faces", Batch S3-05).
//!
//! # Where this sits, and why here
//!
//! `docs/plan/22_REPOSITORY_WORK_PACKAGES.md`'s WP-05 ("Geometry
//! language API") owns "lowering to kernel bridge"; WP-08 ("Constraint
//! system") owns "sketch solver adapter". This module is the seam
//! between the two — exactly the role [`crate::bridge`] already plays for
//! `cad_runtime::value::NumberValue -> cad_geometry_api::Quantity` (that
//! module's own doc comment: converting an evaluated HIR/runtime value
//! into Geometry IR vocabulary). `cad-geometry-runtime` already depends
//! on `cad-runtime` (an HIR-adjacent, higher-layer crate) for precisely
//! this "seam" reason; adding `cad-hir`/`cad-constraints` as real
//! dependencies here (previously only `cad-hir` as a dev-dependency, for
//! tests) follows the same established precedent rather than inventing a
//! new architectural boundary.
//!
//! # What this module expects from its caller
//!
//! [`lower_profile_to_face`] takes an already-solved [`Sketch`] — the
//! caller applies a [`cad_constraints::SolveReport`]'s values first via
//! [`cad_constraints::apply_solved_values`], and decides for itself
//! whether a non-`Solved` [`cad_constraints::SolveStatus`] should still be
//! lowered (this module has no opinion on that policy; it only lowers the
//! geometry it is given). This mirrors `cad_hir::sketch`'s own precedent
//! of leaving numerical geometric-validity judgments to a later,
//! dedicated layer rather than duplicating them.
//!
//! # Closed-profile validation
//!
//! `cad_hir::sketch::Profile`'s own module doc comment explicitly defers
//! closure checking to this task ("Solved closed profile... AICAD-075's
//! own... territory"). This module checks exactly one thing: that
//! consecutive entities' endpoints coincide within the *same* tolerance
//! [`cad_constraints::sketch_solver::SketchSolverProfile`] already uses
//! to decide a constraint is satisfied
//! ([`SketchSolverProfile::v1`]`().position_tolerance`) — reusing the
//! solver's own already-evidenced convergence tolerance rather than
//! inventing a second, competing numeric-tolerance policy (the exact risk
//! `cad_hir::sketch`'s own doc comment warns against). A profile a
//! `SketchSolver` actually reports `Solved` will already close within
//! this tolerance; this check exists to fail explicitly (never silently
//! build a face from a gapped loop) when it does not.
//!
//! # Entity-kind -> kernel op mapping
//!
//! - `Line` -> `GeometryOp::LineEdge`.
//! - `Arc` -> `GeometryOp::ArcEdge` (`AICAD-075`'s own new capability —
//!   see that variant's doc comment for why a three-point arc edge, not
//!   a center/radius/angle one, was added: it needs no separate axis/
//!   sense parameter and mirrors `LineEdge`'s existing minimal style).
//! - `Circle` -> `GeometryOp::CircleWire`, but **only** as a profile's
//!   sole entity: a full circle is already closed by itself, so mixing
//!   it with any other entity in the same profile is a structural error
//!   ([`SketchLoweringError::CircleMustBeSoleProfileMember`]), not a
//!   geometry this module guesses how to chain.
//!
//! # Plane mapping
//!
//! `cad_hir::sketch::SketchPlane`'s own doc comment already fixes each
//! variant's normal (`WorldXy`: +Z, `WorldXz`: +Y, `WorldYz`: +X); this
//! module's [`to_point3`]/[`plane_normal`] implement exactly that
//! documented convention (`u`/`v` sketch coordinates map to the plane's
//! first/second named axis, the third axis held at zero), rather than
//! inventing a new one. `AICAD-075A` (still ahead) is charged with a
//! general `Frame3`-based plane; this module's mapping is deliberately
//! only as general as `SketchPlane`'s own current fixed-enum scope.

use cad_constraints::sketch_solver::SketchSolverProfile;
use cad_diagnostics::{Diagnostic, DiagnosticCode, Position, Severity, SourceSpan};
use cad_geometry_api::ir::{GeomId, GeometryGraph, GeometryIrError, GeometryOp};
use cad_hir::sketch::{
    Point2, Profile, Quantity, Sketch, SketchEntity, SketchEntityId, SketchEntityKind, SketchId,
    SketchPlane,
};
use cad_kernel_api::{Direction3, Point3};

/// The result of lowering one solved profile: the [`GeometryGraph`] built
/// to construct it, and the [`GeomId`] of its final `MakeFace` node.
#[derive(Debug, Clone)]
pub struct LoweredFace {
    pub graph: GeometryGraph,
    pub face: GeomId,
}

/// Every way lowering a profile can fail. Mirrors `cad_hir::sketch::SketchIrError`'s/
/// `cad_geometry_api::ir::GeometryIrError`'s own restraint: every variant
/// is either a structural check (an id genuinely does not belong where
/// claimed) or the one numerical check this module owns (loop closure,
/// against the solver's own already-evidenced tolerance — see module doc
/// comment).
#[derive(Debug, Clone, PartialEq)]
pub enum SketchLoweringError {
    /// `profile.sketch` does not match the `sketch` passed to
    /// [`lower_profile_to_face`].
    ForeignProfile { expected: SketchId, found: SketchId },
    /// `profile` names an entity id `sketch` does not actually own (can
    /// only happen if `sketch` has fewer entities than when `profile`
    /// was built against its original sketch — e.g. a mismatched
    /// solved/original sketch pair).
    UnknownEntity { entity: SketchEntityId },
    /// A `Circle` entity appeared in a profile with more than one
    /// entity. A full circle is already closed on its own; chaining it
    /// with anything else is not a supported loop shape.
    CircleMustBeSoleProfileMember { entity: SketchEntityId },
    /// An `Arc` entity's `start_angle`/`end_angle` describe a zero (or
    /// numerically indistinguishable from zero) sweep, so no interior
    /// "mid" point exists to build a three-point arc edge from.
    DegenerateArc { entity: SketchEntityId },
    /// Two consecutive entities' endpoints do not coincide within the
    /// solver's own convergence tolerance — the profile is not actually
    /// closed.
    NotClosed {
        from: SketchEntityId,
        to: SketchEntityId,
        gap: f64,
        tolerance: f64,
    },
}

impl SketchLoweringError {
    pub fn code(&self) -> &'static str {
        match self {
            SketchLoweringError::ForeignProfile { .. } => "GEOM-E020",
            SketchLoweringError::UnknownEntity { .. } => "GEOM-E021",
            SketchLoweringError::CircleMustBeSoleProfileMember { .. } => "GEOM-E022",
            SketchLoweringError::DegenerateArc { .. } => "GEOM-E023",
            SketchLoweringError::NotClosed { .. } => "GEOM-E024",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            SketchLoweringError::ForeignProfile { .. } => "SKETCH_LOWERING_FOREIGN_PROFILE",
            SketchLoweringError::UnknownEntity { .. } => "SKETCH_LOWERING_UNKNOWN_ENTITY",
            SketchLoweringError::CircleMustBeSoleProfileMember { .. } => {
                "SKETCH_LOWERING_CIRCLE_MUST_BE_SOLE_PROFILE_MEMBER"
            }
            SketchLoweringError::DegenerateArc { .. } => "SKETCH_LOWERING_DEGENERATE_ARC",
            SketchLoweringError::NotClosed { .. } => "SKETCH_LOWERING_PROFILE_NOT_CLOSED",
        }
    }

    pub fn message(&self) -> String {
        match self {
            SketchLoweringError::ForeignProfile { expected, found } => {
                format!("profile belongs to {found}, but lowering was requested against {expected}")
            }
            SketchLoweringError::UnknownEntity { entity } => {
                format!("{entity} does not exist in the sketch being lowered")
            }
            SketchLoweringError::CircleMustBeSoleProfileMember { entity } => format!(
                "{entity} is a circle, which must be a profile's only entity, not one of several"
            ),
            SketchLoweringError::DegenerateArc { entity } => {
                format!("{entity}'s start and end angles describe a zero-sweep arc")
            }
            SketchLoweringError::NotClosed {
                from,
                to,
                gap,
                tolerance,
            } => format!(
                "profile is not closed: {from}'s end and {to}'s start are {gap} apart, \
                 exceeding the solver's own {tolerance} convergence tolerance"
            ),
        }
    }

    /// Builds this error's `cad_diagnostics::Diagnostic`, mirroring every
    /// other Stage-3 IR error's identical pattern. Every variant here has
    /// no natural surface-source span of its own (this module operates
    /// purely on already-lowered `Sketch`/`Profile` values, not source
    /// text), so this always reports the file's start position — matching
    /// `cad_geometry_api::ir::GeometryIrError`'s own precedent for
    /// backend-independent structural errors with no better span
    /// available.
    pub fn to_diagnostic(&self, file: &str) -> Diagnostic {
        let code = DiagnosticCode::parse(self.code())
            .expect("SketchLoweringError::code always returns a well-formed FAMILY-Exxx code");
        Diagnostic::new(
            code,
            Severity::Error,
            "sketch-lowering",
            self.title(),
            self.message(),
        )
        .expect("every SketchLoweringError code carries the 'E' severity letter")
        .with_source(SourceSpan {
            file: file.to_string(),
            start: Position::new(1, 1),
            end: Position::new(1, 1),
        })
    }
}

impl std::fmt::Display for SketchLoweringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for SketchLoweringError {}

/// Maps a sketch-local 2D coordinate into 3D via `plane`'s already-fixed
/// convention (`cad_hir::sketch::SketchPlane`'s own doc comment: `u`/`v`
/// map onto the plane's first/second named axis).
fn to_point3(plane: SketchPlane, p: Point2) -> Point3 {
    match plane {
        SketchPlane::WorldXy => Point3::new(p.x, p.y, 0.0),
        SketchPlane::WorldXz => Point3::new(p.x, 0.0, p.y),
        SketchPlane::WorldYz => Point3::new(0.0, p.x, p.y),
    }
}

/// `plane`'s fixed normal direction, per `SketchPlane`'s own doc comment.
fn plane_normal(plane: SketchPlane) -> Direction3 {
    match plane {
        SketchPlane::WorldXy => Direction3::Z,
        SketchPlane::WorldXz => Direction3::Y,
        SketchPlane::WorldYz => Direction3::X,
    }
}

/// Converts `cad_hir::sketch::Quantity` into `cad_geometry_api::ir::Quantity`
/// -- both are an independent, identically-shaped `{ magnitude: f64, ty:
/// OperandType }` re-implementation of the same typed-quantity concept
/// (per each module's own "no downward cross-layer dependency" doc
/// comment), so a direct field-copy conversion is the correct bridge,
/// mirroring [`crate::bridge::number_value_to_quantity`]'s own identical
/// pattern for `cad_runtime::value::NumberValue`.
fn to_ir_quantity(q: Quantity) -> cad_geometry_api::ir::Quantity {
    cad_geometry_api::ir::Quantity::new(q.magnitude, q.ty)
}

/// The point on a circle/arc of `radius` centered at `center`, at
/// `angle`. A small, independent re-implementation of exactly
/// `cad_constraints::sketch_constraint::arc_point`'s trigonometry — that
/// function is `pub(crate)` to its own crate, so this crate cannot import
/// it; re-deriving three lines of `sin`/`cos` here follows this
/// codebase's own established "independent re-implementation across a
/// layer boundary" precedent (`cad_hir::sketch`'s own module doc comment,
/// "Design decisions" #1/#2 in `project/reports/AICAD-072.md`) rather
/// than widening that function's visibility for one caller.
fn arc_point_2d(center: Point2, radius: Quantity, angle: Quantity) -> Point2 {
    let (sin, cos) = angle.magnitude.sin_cos();
    Point2::new(
        center.x + radius.magnitude * cos,
        center.y + radius.magnitude * sin,
    )
}

/// The angle of a point strictly between `start_angle` and `end_angle`
/// along the arc's actual traversal `direction` — the third point a
/// [`GeometryOp::ArcEdge`] needs to disambiguate which of the two
/// possible arcs between the endpoints is meant. Returns `None` for a
/// (numerically) zero sweep, which has no such interior point.
fn arc_mid_angle(
    start_angle: f64,
    end_angle: f64,
    direction: cad_hir::sketch::RotationDirection,
    angle_tolerance: f64,
) -> Option<f64> {
    use cad_hir::sketch::RotationDirection;
    use std::f64::consts::TAU;
    let raw_delta = end_angle - start_angle;
    // Normalized into (roughly) [0, TAU) for CCW, (-TAU, 0] for CW -- a
    // near-zero result unconditionally means a near-zero sweep (never
    // "actually a full circle"; an entity wanting a full circle is a
    // `Circle`, not a degenerate `Arc`), so it is never special-cased
    // back up to a full `TAU` here.
    let delta = match direction {
        RotationDirection::CounterClockwise => raw_delta.rem_euclid(TAU),
        RotationDirection::Clockwise => -(-raw_delta).rem_euclid(TAU),
    };
    if delta.abs() < angle_tolerance {
        None
    } else {
        Some(start_angle + delta / 2.0)
    }
}

/// The 2D (start, end) traversal endpoints of an entity that can
/// participate in a multi-entity chained profile (`Line`/`Arc`; `Circle`
/// has none, handled separately by the caller).
fn chain_endpoints(entity: &SketchEntity) -> Option<(Point2, Point2)> {
    match entity.kind {
        SketchEntityKind::Line { start, end } => Some((start, end)),
        SketchEntityKind::Arc {
            center,
            radius,
            start_angle,
            end_angle,
            ..
        } => Some((
            arc_point_2d(center, radius, start_angle),
            arc_point_2d(center, radius, end_angle),
        )),
        SketchEntityKind::Circle { .. } => None,
    }
}

/// Lowers `profile` (already applied against `sketch`'s already-solved
/// entities — see module doc comment) into a [`GeometryGraph`] whose
/// final node builds the exact face the profile bounds.
pub fn lower_profile_to_face(
    sketch: &Sketch,
    profile: &Profile,
) -> Result<LoweredFace, SketchLoweringError> {
    if profile.sketch != sketch.id() {
        return Err(SketchLoweringError::ForeignProfile {
            expected: sketch.id(),
            found: profile.sketch,
        });
    }
    let entities: Vec<&SketchEntity> = profile
        .entities
        .iter()
        .map(|&id| {
            sketch
                .get(id)
                .ok_or(SketchLoweringError::UnknownEntity { entity: id })
        })
        .collect::<Result<_, _>>()?;

    let plane = sketch.plane();
    let mut graph = GeometryGraph::new();

    // A lone circle is already closed by itself.
    if let [entity] = entities.as_slice()
        && let SketchEntityKind::Circle { center, radius } = entity.kind
    {
        let wire = push(
            &mut graph,
            GeometryOp::CircleWire {
                center: to_point3(plane, center),
                normal: plane_normal(plane),
                radius: to_ir_quantity(radius),
            },
        );
        let face = push(&mut graph, GeometryOp::MakeFace { wire });
        return Ok(LoweredFace { graph, face });
    }

    let tolerance = SketchSolverProfile::v1();
    let mut chain = Vec::with_capacity(entities.len());
    for entity in &entities {
        match chain_endpoints(entity) {
            Some(endpoints) => chain.push((entity, endpoints)),
            None => {
                return Err(SketchLoweringError::CircleMustBeSoleProfileMember {
                    entity: entity.id,
                });
            }
        }
    }

    // Closure check: each entity's traversal end must meet the next
    // entity's traversal start (wrapping around to the first), within
    // the solver's own convergence tolerance (see module doc comment).
    for i in 0..chain.len() {
        let (entity, (_, end)) = chain[i];
        let (next_entity, (next_start, _)) = chain[(i + 1) % chain.len()];
        let gap = (end - next_start).length();
        if gap > tolerance.position_tolerance {
            return Err(SketchLoweringError::NotClosed {
                from: entity.id,
                to: next_entity.id,
                gap,
                tolerance: tolerance.position_tolerance,
            });
        }
    }

    let mut edges = Vec::with_capacity(chain.len());
    for (entity, _) in &chain {
        let edge = match entity.kind {
            SketchEntityKind::Line { start, end } => push(
                &mut graph,
                GeometryOp::LineEdge {
                    start: to_point3(plane, start),
                    end: to_point3(plane, end),
                },
            ),
            SketchEntityKind::Arc {
                center,
                radius,
                start_angle,
                end_angle,
                direction,
            } => {
                let mid_angle = arc_mid_angle(
                    start_angle.magnitude,
                    end_angle.magnitude,
                    direction,
                    tolerance.angle_tolerance,
                )
                .ok_or(SketchLoweringError::DegenerateArc { entity: entity.id })?;
                let mid = arc_point_2d(
                    center,
                    radius,
                    Quantity::of(mid_angle, cad_types::Dimension::Angle),
                );
                push(
                    &mut graph,
                    GeometryOp::ArcEdge {
                        start: to_point3(plane, arc_point_2d(center, radius, start_angle)),
                        mid: to_point3(plane, mid),
                        end: to_point3(plane, arc_point_2d(center, radius, end_angle)),
                    },
                )
            }
            SketchEntityKind::Circle { .. } => unreachable!(
                "chain_endpoints already rejected any Circle in a multi-entity profile"
            ),
        };
        edges.push(edge);
    }

    let wire = push(&mut graph, GeometryOp::WireFromEdges { edges });
    let face = push(&mut graph, GeometryOp::MakeFace { wire });
    Ok(LoweredFace { graph, face })
}

/// Pushes `op` with a placeholder span (this module has no source span of
/// its own — see [`SketchLoweringError::to_diagnostic`]'s identical
/// note) and panics on `GeometryIrError`, which is unreachable here: every
/// operand this module ever constructs is a node this same function just
/// pushed into the same graph, so [`GeometryGraph::push_op`]'s own
/// structural checks can never fail against it.
fn push(graph: &mut GeometryGraph, op: GeometryOp) -> GeomId {
    graph
        .push_op(op, cad_ast::Span::new(0, 0))
        .unwrap_or_else(|err: GeometryIrError| {
            panic!("sketch_lowering built a structurally invalid GeometryOp: {err}")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::{NodeResult, dispatch_graph};
    use cad_ast::Span;
    use cad_constraints::sketch_constraint::{ConstraintKind, ConstraintSet};
    use cad_constraints::{RelaxationSolver, SketchSolver, apply_solved_values};
    use cad_geometry_api::ir::GeometryQuery;
    use cad_hir::sketch::{RotationDirection, SketchId, SketchPlane};
    use cad_occt_bridge::OcctContext;
    use cad_types::Dimension;

    fn span() -> Span {
        Span::new(0, 1)
    }

    fn length(magnitude: f64) -> Quantity {
        Quantity::of(magnitude, Dimension::Length)
    }

    fn area_of(graph: &GeometryGraph, face: GeomId, ctx: &OcctContext) -> f64 {
        let mut graph = graph.clone();
        let area_q = graph.push_query(GeometryQuery::Area(face), span()).unwrap();
        let results = dispatch_graph(&graph, ctx).expect("dispatch should succeed");
        match &results[area_q.index() as usize] {
            NodeResult::Number(area) => *area,
            other => panic!("expected Number, got {other:?}"),
        }
    }

    fn is_valid(graph: &GeometryGraph, face: GeomId, ctx: &OcctContext) -> bool {
        let mut graph = graph.clone();
        let valid_q = graph
            .push_query(GeometryQuery::IsValid(face), span())
            .unwrap();
        let results = dispatch_graph(&graph, ctx).expect("dispatch should succeed");
        match &results[valid_q.index() as usize] {
            NodeResult::Bool(valid) => *valid,
            other => panic!("expected Bool, got {other:?}"),
        }
    }

    #[test]
    fn a_solved_rectangle_lowers_to_a_valid_face_with_the_exact_area() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let profile = sketch
            .add_rectangle(
                (length(4.0), length(2.0)),
                Point2::ORIGIN,
                Quantity::of(0.0, Dimension::Angle),
                span(),
            )
            .unwrap();

        // A rectangle from `add_rectangle` is already exactly closed (no
        // constraint solving needed) -- this proves the geometry-only
        // path (closure check + LineEdge chain + WireFromEdges + MakeFace)
        // independent of the solver.
        let lowered = lower_profile_to_face(&sketch, &profile).unwrap();
        let ctx = OcctContext::new().unwrap();
        assert!(is_valid(&lowered.graph, lowered.face, &ctx));
        let area = area_of(&lowered.graph, lowered.face, &ctx);
        assert!((area - 8.0).abs() < 1e-6, "area = {area}");
    }

    #[test]
    fn a_solved_slot_with_arcs_lowers_to_a_valid_face_with_the_exact_area() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let profile = sketch
            .add_slot(Point2::ORIGIN, Point2::new(10.0, 0.0), length(4.0), span())
            .unwrap();

        let lowered = lower_profile_to_face(&sketch, &profile).unwrap();
        let ctx = OcctContext::new().unwrap();
        assert!(is_valid(&lowered.graph, lowered.face, &ctx));
        let area = area_of(&lowered.graph, lowered.face, &ctx);
        // A stadium: a 10x4 rectangle plus a full circle of radius 2.
        let expected = 10.0 * 4.0 + std::f64::consts::PI * 2.0 * 2.0;
        assert!(
            (area - expected).abs() < expected * 1e-6,
            "area = {area}, expected {expected}"
        );
    }

    #[test]
    fn a_lone_circle_profile_lowers_to_a_valid_face_with_the_exact_area() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let circle = sketch
            .add_circle(Point2::ORIGIN, length(3.0), false, span())
            .unwrap();
        let profile = sketch.make_profile(vec![circle], span()).unwrap();

        let lowered = lower_profile_to_face(&sketch, &profile).unwrap();
        let ctx = OcctContext::new().unwrap();
        assert!(is_valid(&lowered.graph, lowered.face, &ctx));
        let area = area_of(&lowered.graph, lowered.face, &ctx);
        let expected = std::f64::consts::PI * 3.0 * 3.0;
        assert!((area - expected).abs() < expected * 1e-6);
    }

    #[test]
    fn a_genuinely_constraint_solved_rectangle_lowers_correctly() {
        // A rough, unclosed hand-placed rectangle sketch, closed and
        // squared up entirely by the solver (mirrors
        // `sketch_solver::tests::rectangle_from_rough_sketch_converges_to_exact_rectangle`),
        // then applied back and lowered -- proving the full
        // solve -> apply_solved_values -> lower_profile_to_face pipeline.
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        // `bottom` is seeded exactly at its final position/shape (mirroring
        // `sketch_solver::tests::rectangle_from_rough_sketch_converges_to_exact_rectangle`'s
        // own precedent) so that `Fixed(bottom)` alone pins the whole
        // loop's absolute position, orientation, and length in one shot;
        // the other three sides stay rough/hand-placed.
        let bottom = sketch.add_line(Point2::new(0.0, 0.0), Point2::new(10.0, 0.0), false, span());
        let right = sketch.add_line(Point2::new(10.2, 0.1), Point2::new(9.7, 4.9), false, span());
        let top = sketch.add_line(
            Point2::new(10.1, 5.2),
            Point2::new(-0.3, 4.8),
            false,
            span(),
        );
        let left = sketch.add_line(
            Point2::new(0.2, 5.1),
            Point2::new(-0.1, -0.1),
            false,
            span(),
        );
        let profile = sketch
            .make_profile(vec![bottom, right, top, left], span())
            .unwrap();

        let mut set = ConstraintSet::new(sketch.id());
        // `Fixed { bottom }` alone pins bottom's own two endpoints (and
        // therefore its position/orientation/length all at once) --
        // mirroring `sketch_solver::tests::
        // rectangle_from_rough_sketch_converges_to_exact_rectangle`'s own
        // proven fixture exactly. Adding `Horizontal`/`Distance` on top of
        // an already-`Fixed` entity is redundant, and the solver's own
        // (documented, non-minimal) global DOF count reports that as
        // `Overconstrained` rather than silently accepting it -- so this
        // fixture does not add them, matching that same precedent.
        set.add(&sketch, ConstraintKind::Fixed { entity: bottom }, span())
            .unwrap();
        set.add(&sketch, ConstraintKind::Vertical { line: right }, span())
            .unwrap();
        set.add(&sketch, ConstraintKind::Horizontal { line: top }, span())
            .unwrap();
        set.add(&sketch, ConstraintKind::Vertical { line: left }, span())
            .unwrap();
        set.add(
            &sketch,
            ConstraintKind::Coincident {
                a: cad_constraints::sketch_constraint::PointRef::LineEnd(bottom),
                b: cad_constraints::sketch_constraint::PointRef::LineStart(right),
            },
            span(),
        )
        .unwrap();
        set.add(
            &sketch,
            ConstraintKind::Coincident {
                a: cad_constraints::sketch_constraint::PointRef::LineEnd(right),
                b: cad_constraints::sketch_constraint::PointRef::LineStart(top),
            },
            span(),
        )
        .unwrap();
        set.add(
            &sketch,
            ConstraintKind::Coincident {
                a: cad_constraints::sketch_constraint::PointRef::LineEnd(top),
                b: cad_constraints::sketch_constraint::PointRef::LineStart(left),
            },
            span(),
        )
        .unwrap();
        set.add(
            &sketch,
            ConstraintKind::Coincident {
                a: cad_constraints::sketch_constraint::PointRef::LineEnd(left),
                b: cad_constraints::sketch_constraint::PointRef::LineStart(bottom),
            },
            span(),
        )
        .unwrap();
        set.add(
            &sketch,
            ConstraintKind::Distance {
                a: cad_constraints::sketch_constraint::PointRef::LineStart(right),
                b: cad_constraints::sketch_constraint::PointRef::LineEnd(right),
                value: length(5.0),
            },
            span(),
        )
        .unwrap();

        let solver = RelaxationSolver::new();
        let report = solver.solve(&sketch, &set);
        assert!(
            matches!(report.status, cad_constraints::SolveStatus::Solved),
            "expected Solved, got {:?}",
            report.status
        );

        let solved_sketch = apply_solved_values(&sketch, &report.values).unwrap();
        let lowered = lower_profile_to_face(&solved_sketch, &profile).unwrap();
        let ctx = OcctContext::new().unwrap();
        assert!(is_valid(&lowered.graph, lowered.face, &ctx));
        let area = area_of(&lowered.graph, lowered.face, &ctx);
        assert!((area - 50.0).abs() < 1e-6, "area = {area}");
    }

    #[test]
    fn a_gapped_profile_is_rejected_as_not_closed() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let l0 = sketch.add_line(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), false, span());
        // Deliberately does not start where l0 ends.
        let l1 = sketch.add_line(Point2::new(2.0, 0.0), Point2::new(0.0, 0.0), false, span());
        let profile = sketch.make_profile(vec![l0, l1], span()).unwrap();

        let err = lower_profile_to_face(&sketch, &profile).unwrap_err();
        assert!(matches!(err, SketchLoweringError::NotClosed { .. }));
    }

    #[test]
    fn a_circle_mixed_with_another_entity_is_rejected() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let circle = sketch
            .add_circle(Point2::ORIGIN, length(1.0), false, span())
            .unwrap();
        let line = sketch.add_line(Point2::ORIGIN, Point2::new(1.0, 0.0), false, span());
        let profile = sketch.make_profile(vec![circle, line], span()).unwrap();

        let err = lower_profile_to_face(&sketch, &profile).unwrap_err();
        assert!(matches!(
            err,
            SketchLoweringError::CircleMustBeSoleProfileMember { .. }
        ));
    }

    #[test]
    fn a_profile_from_a_foreign_sketch_is_rejected() {
        let mut sketch_a = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let profile_a = sketch_a
            .add_rectangle(
                (length(1.0), length(1.0)),
                Point2::ORIGIN,
                Quantity::of(0.0, Dimension::Angle),
                span(),
            )
            .unwrap();
        let sketch_b = Sketch::new(SketchId::new(1), SketchPlane::WorldXy);

        let err = lower_profile_to_face(&sketch_b, &profile_a).unwrap_err();
        assert!(matches!(err, SketchLoweringError::ForeignProfile { .. }));
    }

    #[test]
    fn a_zero_sweep_arc_is_rejected_as_degenerate() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        // A "closed" 2-entity loop where the arc contributes zero sweep:
        // both of its endpoints coincide with each other (and with the
        // degenerate line's own single point), so the closure check
        // passes and lowering reaches (and rejects) the degenerate arc
        // itself, rather than failing on non-closure first.
        let point_on_arc = arc_point_2d(
            Point2::ORIGIN,
            length(1.0),
            Quantity::of(0.5, Dimension::Angle),
        );
        let arc = sketch
            .add_arc(
                Point2::ORIGIN,
                length(1.0),
                Quantity::of(0.5, Dimension::Angle),
                Quantity::of(0.5, Dimension::Angle),
                RotationDirection::CounterClockwise,
                false,
                span(),
            )
            .unwrap();
        let line = sketch.add_line(point_on_arc, point_on_arc, false, span());
        let profile = sketch.make_profile(vec![arc, line], span()).unwrap();

        let err = lower_profile_to_face(&sketch, &profile).unwrap_err();
        assert!(matches!(err, SketchLoweringError::DegenerateArc { .. }));
    }

    #[test]
    fn every_error_variant_converts_to_a_well_formed_diagnostic() {
        let sketch_id = SketchId::new(0);
        let mut sketch = Sketch::new(sketch_id, SketchPlane::WorldXy);
        let entity = sketch.add_line(Point2::ORIGIN, Point2::new(1.0, 0.0), false, span());
        let errors = vec![
            SketchLoweringError::ForeignProfile {
                expected: sketch_id,
                found: SketchId::new(1),
            },
            SketchLoweringError::UnknownEntity { entity },
            SketchLoweringError::CircleMustBeSoleProfileMember { entity },
            SketchLoweringError::DegenerateArc { entity },
            SketchLoweringError::NotClosed {
                from: entity,
                to: entity,
                gap: 1.0,
                tolerance: 1e-9,
            },
        ];
        for err in errors {
            let code = err.code();
            assert!(code.starts_with("GEOM-E"));
            DiagnosticCode::parse(code)
                .expect("every SketchLoweringError code must be well-formed");
            let diagnostic = err.to_diagnostic("test.aicad");
            assert_eq!(diagnostic.category, "sketch-lowering");
            assert!(!diagnostic.message.is_empty());
            assert!(!err.to_string().is_empty());
        }
    }
}
