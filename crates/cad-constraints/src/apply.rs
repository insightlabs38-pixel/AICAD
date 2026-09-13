//! Applies a [`SolveReport`]'s solved variable assignments back onto a
//! [`Sketch`], producing the concrete geometry `AICAD-075` ("Lower solved
//! closed sketch profiles to exact faces") needs to actually lower.
//!
//! `AICAD-074`'s own report left this "apply-back" step as an explicit
//! open judgment call for this task: `SketchSolver::solve` only ever
//! returns a [`SolveReport`] carrying a flat [`SketchVariable`] ->
//! magnitude map (`SolvedValues`) — nothing in `cad-constraints` itself
//! writes those magnitudes back onto a concrete [`Sketch`]'s entities.
//! [`apply_solved_values`] is that missing step.
//!
//! # Functional/value semantics (`D2`)
//!
//! [`apply_solved_values`] never mutates its input `sketch` — it builds
//! and returns a **new** [`Sketch`] value (same [`cad_hir::sketch::SketchId`],
//! entities re-pushed in the same order so every existing
//! [`cad_hir::sketch::SketchEntityId`]/[`crate::sketch_constraint::PointRef`]
//! still resolves against it identically), mirroring `Transform`'s own
//! `let moved = transform(part, t);` convention (`AICAD-075A`'s own
//! functional-semantics precedent, applied here one task early since this
//! module needed the same discipline first) rather than an in-place
//! `Sketch::apply_solved_values(&mut self, ...)` method.
//!
//! # Partial solutions are not an error here
//!
//! A [`SolveReport`] may be `Underconstrained` (some variables never
//! assigned) or `Overconstrained`/`Unsupported` (assignments only for the
//! constraints a backend could act on) — `values.get` returning `None`
//! for a given [`SketchVariable`] is an expected, routine outcome, not a
//! defect. This function falls back to the entity's own already-recorded
//! magnitude for any variable [`SolvedValues`] does not carry, so a
//! partial solve still produces a well-formed (if not fully constrained)
//! [`Sketch`] — never a panic or a fabricated value. Whether an
//! `Underconstrained`/`Overconstrained` sketch should actually be lowered
//! to a face at all is [`crate`]'s caller's own policy decision (`AICAD-075`'s
//! own `cad-geometry-runtime::sketch_lowering` consumer checks
//! `SolveStatus` before calling this function), not this module's.
//!
//! # Why this can still fail
//!
//! [`Sketch::add_circle`]/[`Sketch::add_arc`] independently re-validate
//! their own radius as strictly positive (`cad_hir::sketch`'s own
//! structural invariant, unrelated to solving) — a solver could in
//! principle drive a radius to zero/negative (e.g. an `Unsupported`
//! constraint left uncorrected on a badly-authored sketch). Re-running
//! that same check here (by re-using the entity constructors themselves,
//! not duplicating their validation) and surfacing
//! [`cad_hir::sketch::SketchIrError`] honestly is preferred over silently
//! clamping or accepting a degenerate radius.

use crate::sketch_constraint::{PointRef, SketchVariable, SolvedValues};
use cad_hir::sketch::{Point2, Quantity, Sketch, SketchEntityKind, SketchIrError};

fn apply_point(original: Point2, point: PointRef, values: &SolvedValues) -> Point2 {
    let x = values
        .get(SketchVariable::PointX(point))
        .unwrap_or(original.x);
    let y = values
        .get(SketchVariable::PointY(point))
        .unwrap_or(original.y);
    Point2::new(x, y)
}

fn apply_quantity(original: Quantity, variable: SketchVariable, values: &SolvedValues) -> Quantity {
    match values.get(variable) {
        Some(magnitude) => Quantity::new(magnitude, original.ty),
        None => original,
    }
}

/// Builds a new [`Sketch`] with every entity's point/radius/angle fields
/// updated from `values` wherever `values` carries an assignment for the
/// corresponding [`SketchVariable`], and left as `sketch`'s own original
/// value otherwise. See module doc comment for the full functional-
/// semantics and partial-solution rationale.
pub fn apply_solved_values(
    sketch: &Sketch,
    values: &SolvedValues,
) -> Result<Sketch, SketchIrError> {
    let mut result = Sketch::new(sketch.id(), sketch.plane());
    for entity in sketch.entities() {
        match entity.kind {
            SketchEntityKind::Line { start, end } => {
                let start = apply_point(start, PointRef::LineStart(entity.id), values);
                let end = apply_point(end, PointRef::LineEnd(entity.id), values);
                result.add_line(start, end, entity.construction, entity.span);
            }
            SketchEntityKind::Circle { center, radius } => {
                let center = apply_point(center, PointRef::CircleCenter(entity.id), values);
                let radius = apply_quantity(radius, SketchVariable::Radius(entity.id), values);
                result.add_circle(center, radius, entity.construction, entity.span)?;
            }
            SketchEntityKind::Arc {
                center,
                radius,
                start_angle,
                end_angle,
                direction,
            } => {
                let center = apply_point(center, PointRef::ArcCenter(entity.id), values);
                let radius = apply_quantity(radius, SketchVariable::Radius(entity.id), values);
                let start_angle =
                    apply_quantity(start_angle, SketchVariable::StartAngle(entity.id), values);
                let end_angle =
                    apply_quantity(end_angle, SketchVariable::EndAngle(entity.id), values);
                result.add_arc(
                    center,
                    radius,
                    start_angle,
                    end_angle,
                    direction,
                    entity.construction,
                    entity.span,
                )?;
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sketch_constraint::sketch_variables;
    use cad_ast::Span;
    use cad_hir::sketch::{RotationDirection, SketchId, SketchPlane};
    use cad_types::Dimension;

    fn span() -> Span {
        Span::new(0, 1)
    }

    fn length(magnitude: f64) -> Quantity {
        Quantity::of(magnitude, Dimension::Length)
    }

    fn angle(magnitude: f64) -> Quantity {
        Quantity::of(magnitude, Dimension::Angle)
    }

    #[test]
    fn fully_solved_line_gets_exactly_the_solved_coordinates() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let line = sketch.add_line(Point2::ORIGIN, Point2::new(1.0, 1.0), false, span());

        let mut values = SolvedValues::new();
        values.set(SketchVariable::PointX(PointRef::LineStart(line)), 10.0);
        values.set(SketchVariable::PointY(PointRef::LineStart(line)), 20.0);
        values.set(SketchVariable::PointX(PointRef::LineEnd(line)), 30.0);
        values.set(SketchVariable::PointY(PointRef::LineEnd(line)), 40.0);

        let solved = apply_solved_values(&sketch, &values).unwrap();
        match solved.get(line).unwrap().kind {
            SketchEntityKind::Line { start, end } => {
                assert_eq!(start, Point2::new(10.0, 20.0));
                assert_eq!(end, Point2::new(30.0, 40.0));
            }
            other => panic!("expected Line, got {other:?}"),
        }
        // The original sketch must be untouched (D2 functional semantics).
        match sketch.get(line).unwrap().kind {
            SketchEntityKind::Line { start, end } => {
                assert_eq!(start, Point2::ORIGIN);
                assert_eq!(end, Point2::new(1.0, 1.0));
            }
            other => panic!("expected Line, got {other:?}"),
        }
    }

    #[test]
    fn a_partial_solve_falls_back_to_the_original_value_for_missing_variables() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let line = sketch.add_line(Point2::new(1.0, 2.0), Point2::new(3.0, 4.0), false, span());

        // Only the start point was assigned; the end point is untouched.
        let mut values = SolvedValues::new();
        values.set(SketchVariable::PointX(PointRef::LineStart(line)), 100.0);
        values.set(SketchVariable::PointY(PointRef::LineStart(line)), 200.0);

        let solved = apply_solved_values(&sketch, &values).unwrap();
        match solved.get(line).unwrap().kind {
            SketchEntityKind::Line { start, end } => {
                assert_eq!(start, Point2::new(100.0, 200.0));
                assert_eq!(end, Point2::new(3.0, 4.0));
            }
            other => panic!("expected Line, got {other:?}"),
        }
    }

    #[test]
    fn empty_solved_values_reproduces_the_original_sketch_exactly() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        sketch.add_line(Point2::ORIGIN, Point2::new(1.0, 0.0), false, span());
        sketch
            .add_circle(Point2::new(5.0, 5.0), length(2.0), false, span())
            .unwrap();

        let solved = apply_solved_values(&sketch, &SolvedValues::new()).unwrap();
        assert_eq!(solved, sketch);
    }

    #[test]
    fn circle_center_and_radius_apply_correctly() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let circle = sketch
            .add_circle(Point2::ORIGIN, length(1.0), false, span())
            .unwrap();

        let mut values = SolvedValues::new();
        values.set(SketchVariable::PointX(PointRef::CircleCenter(circle)), 7.0);
        values.set(SketchVariable::PointY(PointRef::CircleCenter(circle)), 8.0);
        values.set(SketchVariable::Radius(circle), 3.0);

        let solved = apply_solved_values(&sketch, &values).unwrap();
        match solved.get(circle).unwrap().kind {
            SketchEntityKind::Circle { center, radius } => {
                assert_eq!(center, Point2::new(7.0, 8.0));
                assert_eq!(radius.magnitude, 3.0);
                assert_eq!(radius.dimension(), Some(Dimension::Length));
            }
            other => panic!("expected Circle, got {other:?}"),
        }
    }

    #[test]
    fn arc_center_radius_and_angles_apply_correctly() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let arc = sketch
            .add_arc(
                Point2::ORIGIN,
                length(1.0),
                angle(0.0),
                angle(std::f64::consts::PI),
                RotationDirection::CounterClockwise,
                false,
                span(),
            )
            .unwrap();

        let mut values = SolvedValues::new();
        values.set(SketchVariable::Radius(arc), 5.0);
        values.set(SketchVariable::StartAngle(arc), 0.1);
        values.set(SketchVariable::EndAngle(arc), 2.0);

        let solved = apply_solved_values(&sketch, &values).unwrap();
        match solved.get(arc).unwrap().kind {
            SketchEntityKind::Arc {
                radius,
                start_angle,
                end_angle,
                direction,
                ..
            } => {
                assert_eq!(radius.magnitude, 5.0);
                assert_eq!(start_angle.magnitude, 0.1);
                assert_eq!(end_angle.magnitude, 2.0);
                assert_eq!(direction, RotationDirection::CounterClockwise);
            }
            other => panic!("expected Arc, got {other:?}"),
        }
    }

    #[test]
    fn a_solved_non_positive_radius_is_rejected_rather_than_silently_accepted() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let circle = sketch
            .add_circle(Point2::ORIGIN, length(1.0), false, span())
            .unwrap();
        let mut values = SolvedValues::new();
        values.set(SketchVariable::Radius(circle), -1.0);

        let err = apply_solved_values(&sketch, &values).unwrap_err();
        assert!(matches!(err, SketchIrError::NonPositiveLength { .. }));
    }

    #[test]
    fn entity_ids_from_the_original_sketch_still_resolve_against_the_solved_one() {
        let mut sketch = Sketch::new(SketchId::new(2), SketchPlane::WorldXy);
        let l0 = sketch.add_line(Point2::ORIGIN, Point2::new(1.0, 0.0), false, span());
        let l1 = sketch.add_line(Point2::new(1.0, 0.0), Point2::new(1.0, 1.0), false, span());

        let solved = apply_solved_values(&sketch, &SolvedValues::new()).unwrap();
        assert_eq!(solved.id(), sketch.id());
        assert!(solved.get(l0).is_some());
        assert!(solved.get(l1).is_some());
        // Every free variable the original sketch contributes still
        // resolves correctly against the solved sketch too.
        assert_eq!(
            sketch_variables(&sketch).len(),
            sketch_variables(&solved).len()
        );
    }
}
