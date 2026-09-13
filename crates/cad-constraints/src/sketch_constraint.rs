//! Stage-3 solver-independent sketch constraint IR/adapter (`AICAD-073`,
//! Batch S3-05), per `docs/plan/04_HIGH_LEVEL_MODELING_API.md` §4's
//! baseline sketch-constraint catalogue, `docs/plan/
//! 08_CONSTRAINTS_REQUIREMENTS_TESTS.md` §4/§6, and
//! `project/DECISION_LOG.md#DL-20` (`D11`).
//!
//! # What `DL-20` requires, and how this module satisfies it
//!
//! `DL-20` requires the AICAD constraint IR — not a solver backend — to
//! be authoritative for every observable constraint semantic. It must
//! own: typed/dimensioned constraint variables ([`SketchVariable`]);
//! constraint kinds and their parameters ([`ConstraintKind`]); semantic
//! IDs "mirroring `DL-19`'s own sketch-entity-identity precedent"
//! ([`ConstraintId`], built the same way [`cad_hir::sketch::SketchEntityId`]
//! is: minted only by its owning [`ConstraintSet`], embedding that set's
//! own [`cad_hir::sketch::SketchId`] in its own shape); source/provenance
//! mapping ([`Constraint::span`]); the solve-status vocabulary
//! ([`SolveStatus`], at minimum solved/underconstrained/overconstrained);
//! and a structured diagnostic/evidence vocabulary ([`ConstraintIrError`]).
//! A solver backend ([`SketchSolver`]) owns numerical algorithms only —
//! this module defines that trait as the solver-independence boundary but
//! ships no concrete implementation of it (see "Scope cuts" below).
//!
//! # Why `cad-constraints` depends on `cad-hir` directly
//!
//! Unlike `cad-hir::sketch`'s own precedent of *not* depending downward on
//! `cad-geometry-api`/`cad-kernel-api` (a genuine cross-layer reach its own
//! module doc comment declines), `docs/plan/01_SYSTEM_ARCHITECTURE.md` §1's
//! own pipeline diagram places the constraint subsystem immediately after
//! "CAD-HIR" ("Parameter + constraint graph"), not below it — the
//! constraint subsystem is HIR-adjacent/downstream, not a lower
//! architectural layer HIR must not reach into. Reusing
//! [`cad_hir::sketch::SketchId`]/[`cad_hir::sketch::SketchEntityId`]/
//! [`cad_hir::sketch::Sketch`]/[`cad_hir::sketch::Quantity`] directly (a
//! real dependency, not another independent re-implementation) is also the
//! most literal way to satisfy `DL-20`'s "mirroring `DL-19`'s own
//! identity precedent... not a parallel identity scheme": there is
//! exactly one sketch-entity identity type in the whole workspace, and
//! this module references it rather than inventing a second one.
//!
//! # Scope cuts (deliberate — matching `AICAD-072`'s own precedent of
//! declaring an IR shape before a later task wires numeric behavior on
//! top of it)
//!
//! - **No numeric constraint solving.** [`SketchSolver`] is a trait
//!   (the solver-independence boundary `DL-20` requires) with no
//!   implementation shipped by this task — proving the interface is
//!   usable is left to this module's own tests (a trivial mock
//!   implementation, `#[cfg(test)]` only). `DL-20` permits "exactly one
//!   initial solver implementation... behind this interface"; that
//!   implementation is `AICAD-074`'s own scope ("Implement common sketch
//!   constraints needed by baseline parts"), not this task's.
//! - **[`ConstraintSet::add`] validates structure/dimension only** —
//!   operand ownership (an operand belongs to this set's own sketch),
//!   operand entity-kind correctness (e.g. `horizontal` requires a
//!   `Line`), and quantity dimension/positivity — mirroring
//!   `cad_hir::sketch::SketchIrError`'s identical restraint (never a
//!   numerical geometric-validity or satisfiability judgment; that is
//!   `SketchSolver`'s job).
//! - **`horizontal`/`vertical`/`parallel`/`perpendicular`/`angle`
//!   operate on `Line` entities only**, and **`concentric` operates on
//!   `Circle`/`Arc` entities only** — `docs/plan/
//!   04_HIGH_LEVEL_MODELING_API.md` §4's table also allows "axes" for
//!   each, but no `Axis` sketch-entity kind exists yet
//!   (`cad_hir::sketch::SketchEntityKind` has only `Line`/`Circle`/
//!   `Arc` — `AICAD-075A`, still ahead, owns the axis/frame foundation).
//!   Extending these kinds to accept an axis operand once one exists is
//!   forward-compatible (an additional match arm), not a breaking change.
//! - **`distance` is point-to-point only**, not the plan table's fuller
//!   "entities" (which also covers point-to-line and entity-to-entity
//!   distance in general CAD sketch tools). Point-to-point is the
//!   dimension actually needed to place `line`/`circle`/`arc` endpoints
//!   relative to one another for a baseline part; a point-to-line
//!   variant is a documented future extension, not a silently dropped
//!   requirement.
//! - **`tangent` requires at least one `Circle`/`Arc` operand** —
//!   line-to-line tangency is geometrically degenerate/undefined, so
//!   that specific pairing is a structural [`ConstraintIrError`], not a
//!   silently-accepted-then-unsatisfiable constraint.
//! - **No hard/soft constraint strength.** `docs/plan/
//!   08_CONSTRAINTS_REQUIREMENTS_TESTS.md` §5's hard/soft distinction
//!   belongs to that document's broader unified constraint/requirement
//!   engine (algebraic parameter constraints, engineering requirements),
//!   which is out of this task's 2D-sketch-constraint scope entirely —
//!   pulling it forward here would be exactly the kind of scope
//!   expansion the campaign brief's "do not pull forward... a full
//!   verification framework" line forbids.
//! - **No assembly/3D-geometric/algebraic-parameter/optimization
//!   constraint domains** (`docs/plan/
//!   08_CONSTRAINTS_REQUIREMENTS_TESTS.md` §2's other five domains) —
//!   assemblies are explicitly forbidden Stage-3 scope; the others are
//!   simply not what `AICAD-073`/`074`/`075` (2D sketch constraints,
//!   per their own titles) were asked to build.

use cad_ast::Span;
use cad_diagnostics::{Diagnostic, DiagnosticCode, Position, Severity, SourceSpan};
use cad_hir::sketch::{Point2, Quantity, Sketch, SketchEntityId, SketchEntityKind, SketchId};
use cad_types::Dimension;
use cad_units::OperandType;
use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------

/// Identity of one constraint within its owning [`ConstraintSet`] — a
/// (`SketchId`, `index`) pair, mirroring
/// [`cad_hir::sketch::SketchEntityId`]'s exact shape and minting
/// discipline: only [`ConstraintSet::add`] ever produces one, and it
/// always embeds the [`SketchId`] of the sketch its constraints apply
/// to (`DL-20`'s "mirroring `DL-19`'s own sketch-entity-identity
/// precedent" requirement, satisfied structurally rather than by
/// caller convention).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConstraintId {
    sketch: SketchId,
    index: u32,
}

impl ConstraintId {
    pub fn sketch(self) -> SketchId {
        self.sketch
    }

    pub fn index(self) -> u32 {
        self.index
    }
}

impl fmt::Display for ConstraintId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/constraint#{}", self.sketch, self.index)
    }
}

// ---------------------------------------------------------------------
// Point references
// ---------------------------------------------------------------------

/// A reference to one specific point carried by a sketch entity — the
/// operand shape several baseline constraints need (`coincident`,
/// `distance`, `midpoint`) since they relate specific points, not whole
/// entities. Every variant carries the [`SketchEntityId`] of the entity
/// that owns the referenced point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointRef {
    LineStart(SketchEntityId),
    LineEnd(SketchEntityId),
    CircleCenter(SketchEntityId),
    ArcCenter(SketchEntityId),
    ArcStart(SketchEntityId),
    ArcEnd(SketchEntityId),
}

impl PointRef {
    pub fn entity(self) -> SketchEntityId {
        match self {
            PointRef::LineStart(e)
            | PointRef::LineEnd(e)
            | PointRef::CircleCenter(e)
            | PointRef::ArcCenter(e)
            | PointRef::ArcStart(e)
            | PointRef::ArcEnd(e) => e,
        }
    }

    fn matches_kind(self, kind: &SketchEntityKind) -> bool {
        matches!(
            (self, kind),
            (PointRef::LineStart(_), SketchEntityKind::Line { .. })
                | (PointRef::LineEnd(_), SketchEntityKind::Line { .. })
                | (PointRef::CircleCenter(_), SketchEntityKind::Circle { .. })
                | (PointRef::ArcCenter(_), SketchEntityKind::Arc { .. })
                | (PointRef::ArcStart(_), SketchEntityKind::Arc { .. })
                | (PointRef::ArcEnd(_), SketchEntityKind::Arc { .. })
        )
    }

    fn kind_name(self) -> &'static str {
        match self {
            PointRef::LineStart(_) => "line.start",
            PointRef::LineEnd(_) => "line.end",
            PointRef::CircleCenter(_) => "circle.center",
            PointRef::ArcCenter(_) => "arc.center",
            PointRef::ArcStart(_) => "arc.start",
            PointRef::ArcEnd(_) => "arc.end",
        }
    }
}

/// Resolves a [`PointRef`] to its current coordinate against `sketch`.
/// A purely structural lookup (not solving) — mirrors
/// [`cad_hir::sketch::Sketch::get`]'s own read-only-projection role.
/// Returns `None` only if `point`'s entity does not belong to `sketch`
/// (the same condition [`Sketch::get`] itself reports as `None`);
/// [`ConstraintSet::add`] already guarantees this never happens for any
/// `PointRef` stored inside a validated [`Constraint`].
pub fn resolve_point(sketch: &Sketch, point: PointRef) -> Option<Point2> {
    let entity = sketch.get(point.entity())?;
    match (point, entity.kind) {
        (PointRef::LineStart(_), SketchEntityKind::Line { start, .. }) => Some(start),
        (PointRef::LineEnd(_), SketchEntityKind::Line { end, .. }) => Some(end),
        (PointRef::CircleCenter(_), SketchEntityKind::Circle { center, .. }) => Some(center),
        (PointRef::ArcCenter(_), SketchEntityKind::Arc { center, .. }) => Some(center),
        (
            PointRef::ArcStart(_),
            SketchEntityKind::Arc {
                center,
                radius,
                start_angle,
                ..
            },
        ) => Some(arc_point(center, radius, start_angle)),
        (
            PointRef::ArcEnd(_),
            SketchEntityKind::Arc {
                center,
                radius,
                end_angle,
                ..
            },
        ) => Some(arc_point(center, radius, end_angle)),
        _ => None,
    }
}

/// `pub(crate)` (not private) so `sketch_solver`'s own working-copy point
/// resolution (`AICAD-074`) can reuse the exact same arc-endpoint
/// derivation as this module's own [`resolve_point`], rather than a
/// second re-implementation of the same trigonometry.
pub(crate) fn arc_point(center: Point2, radius: Quantity, angle: Quantity) -> Point2 {
    let (sin, cos) = angle.magnitude.sin_cos();
    Point2::new(
        center.x + radius.magnitude * cos,
        center.y + radius.magnitude * sin,
    )
}

// ---------------------------------------------------------------------
// Typed constraint variables (`DL-20`: "typed/dimensioned constraint
// variables")
// ---------------------------------------------------------------------

/// One scalar degree of freedom a sketch entity contributes — the
/// vocabulary a [`SketchSolver`] adjusts to satisfy a [`ConstraintSet`].
/// Purely structural (which scalars exist), never which values they
/// should take — that is entirely the solver's job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SketchVariable {
    PointX(PointRef),
    PointY(PointRef),
    Radius(SketchEntityId),
    StartAngle(SketchEntityId),
    EndAngle(SketchEntityId),
}

impl SketchVariable {
    /// The dimension a solver must respect when it assigns this
    /// variable's magnitude — `DL-20`'s "dimensioned" requirement.
    pub fn dimension(self) -> Dimension {
        match self {
            SketchVariable::PointX(_) | SketchVariable::PointY(_) | SketchVariable::Radius(_) => {
                Dimension::Length
            }
            SketchVariable::StartAngle(_) | SketchVariable::EndAngle(_) => Dimension::Angle,
        }
    }
}

/// Enumerates every free scalar [`SketchVariable`] `sketch`'s own
/// entities contribute, in entity order. A structural fact about
/// `sketch` alone — independent of any [`ConstraintSet`] or
/// [`SketchSolver`] — so every solver implementation starts from the
/// same variable set (`DL-20`: a solver "must not redefine dimensional
/// semantics").
pub fn sketch_variables(sketch: &Sketch) -> Vec<SketchVariable> {
    let mut vars = Vec::new();
    for entity in sketch.entities() {
        match entity.kind {
            SketchEntityKind::Line { .. } => {
                vars.push(SketchVariable::PointX(PointRef::LineStart(entity.id)));
                vars.push(SketchVariable::PointY(PointRef::LineStart(entity.id)));
                vars.push(SketchVariable::PointX(PointRef::LineEnd(entity.id)));
                vars.push(SketchVariable::PointY(PointRef::LineEnd(entity.id)));
            }
            SketchEntityKind::Circle { .. } => {
                vars.push(SketchVariable::PointX(PointRef::CircleCenter(entity.id)));
                vars.push(SketchVariable::PointY(PointRef::CircleCenter(entity.id)));
                vars.push(SketchVariable::Radius(entity.id));
            }
            SketchEntityKind::Arc { .. } => {
                vars.push(SketchVariable::PointX(PointRef::ArcCenter(entity.id)));
                vars.push(SketchVariable::PointY(PointRef::ArcCenter(entity.id)));
                vars.push(SketchVariable::Radius(entity.id));
                vars.push(SketchVariable::StartAngle(entity.id));
                vars.push(SketchVariable::EndAngle(entity.id));
            }
        }
    }
    vars
}

// ---------------------------------------------------------------------
// Constraint kinds (`docs/plan/04_HIGH_LEVEL_MODELING_API.md` §4's
// baseline catalogue)
// ---------------------------------------------------------------------

/// One baseline sketch-constraint kind and its typed parameters.
/// See module doc comment ("Scope cuts") for exactly which entity kinds
/// each variant accepts in this task's implementation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstraintKind {
    Coincident {
        a: PointRef,
        b: PointRef,
    },
    Horizontal {
        line: SketchEntityId,
    },
    Vertical {
        line: SketchEntityId,
    },
    Parallel {
        a: SketchEntityId,
        b: SketchEntityId,
    },
    Perpendicular {
        a: SketchEntityId,
        b: SketchEntityId,
    },
    Tangent {
        a: SketchEntityId,
        b: SketchEntityId,
    },
    Concentric {
        a: SketchEntityId,
        b: SketchEntityId,
    },
    Equal {
        a: SketchEntityId,
        b: SketchEntityId,
    },
    Symmetric {
        a: SketchEntityId,
        b: SketchEntityId,
        about: SketchEntityId,
    },
    Distance {
        a: PointRef,
        b: PointRef,
        value: Quantity,
    },
    Angle {
        a: SketchEntityId,
        b: SketchEntityId,
        value: Quantity,
    },
    Radius {
        entity: SketchEntityId,
        value: Quantity,
    },
    Diameter {
        entity: SketchEntityId,
        value: Quantity,
    },
    Fixed {
        entity: SketchEntityId,
    },
    Midpoint {
        point: PointRef,
        line: SketchEntityId,
    },
}

impl ConstraintKind {
    /// The plan-doc constraint name (`docs/plan/
    /// 04_HIGH_LEVEL_MODELING_API.md` §4's table), used in diagnostics.
    pub fn name(&self) -> &'static str {
        match self {
            ConstraintKind::Coincident { .. } => "coincident",
            ConstraintKind::Horizontal { .. } => "horizontal",
            ConstraintKind::Vertical { .. } => "vertical",
            ConstraintKind::Parallel { .. } => "parallel",
            ConstraintKind::Perpendicular { .. } => "perpendicular",
            ConstraintKind::Tangent { .. } => "tangent",
            ConstraintKind::Concentric { .. } => "concentric",
            ConstraintKind::Equal { .. } => "equal",
            ConstraintKind::Symmetric { .. } => "symmetric",
            ConstraintKind::Distance { .. } => "distance",
            ConstraintKind::Angle { .. } => "angle",
            ConstraintKind::Radius { .. } => "radius",
            ConstraintKind::Diameter { .. } => "diameter",
            ConstraintKind::Fixed { .. } => "fixed",
            ConstraintKind::Midpoint { .. } => "midpoint",
        }
    }
}

/// One constraint owned by a [`ConstraintSet`]: its identity, kind, and
/// the source span it was built from (matching every other Stage-2/3
/// IR's span-fidelity convention).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Constraint {
    pub id: ConstraintId,
    pub kind: ConstraintKind,
    pub span: Span,
}

// ---------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------

/// Every way adding a constraint to a [`ConstraintSet`] can fail.
/// Mirrors `cad_hir::sketch::SketchIrError`'s own restraint exactly:
/// every variant is a static, structural, or dimensional check — never
/// a numerical satisfiability judgment (that is [`SketchSolver`]'s job).
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintIrError {
    /// An operand does not belong to this [`ConstraintSet`]'s own
    /// sketch (or names an entity/point that does not exist in it).
    ForeignOperand {
        context: &'static str,
        entity: SketchEntityId,
        expected_sketch: SketchId,
        span: Span,
    },
    /// An operand's entity/point kind does not match what `context`
    /// requires (e.g. `horizontal`'s operand must be a `Line`).
    WrongEntityKind {
        context: &'static str,
        expected: &'static str,
        found: &'static str,
        span: Span,
    },
    /// A [`Quantity`] parameter's dimension did not match what
    /// `context` requires.
    DimensionMismatch {
        context: &'static str,
        expected: Dimension,
        found: OperandType,
        span: Span,
    },
    /// A length-dimensioned parameter that must be strictly positive
    /// (`radius`/`diameter`'s target value) was zero or negative.
    NonPositiveLength {
        context: &'static str,
        found: f64,
        span: Span,
    },
    /// `equal`'s two operands are not the same comparable category
    /// (both lines, or both circle/arc).
    IncomparableEqualOperands {
        a: SketchEntityId,
        b: SketchEntityId,
        span: Span,
    },
    /// `tangent`'s two operands are both `Line`s — line-to-line
    /// tangency is geometrically undefined (see module doc comment,
    /// "Scope cuts").
    UnsupportedTangentPair {
        a: SketchEntityId,
        b: SketchEntityId,
        span: Span,
    },
}

impl ConstraintIrError {
    pub fn code(&self) -> &'static str {
        match self {
            ConstraintIrError::ForeignOperand { .. } => "CONSTRAINT-E001",
            ConstraintIrError::WrongEntityKind { .. } => "CONSTRAINT-E002",
            ConstraintIrError::DimensionMismatch { .. } => "CONSTRAINT-E003",
            ConstraintIrError::NonPositiveLength { .. } => "CONSTRAINT-E004",
            ConstraintIrError::IncomparableEqualOperands { .. } => "CONSTRAINT-E005",
            ConstraintIrError::UnsupportedTangentPair { .. } => "CONSTRAINT-E006",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            ConstraintIrError::ForeignOperand { .. } => "SKETCH_CONSTRAINT_FOREIGN_OPERAND",
            ConstraintIrError::WrongEntityKind { .. } => "SKETCH_CONSTRAINT_WRONG_ENTITY_KIND",
            ConstraintIrError::DimensionMismatch { .. } => "SKETCH_CONSTRAINT_DIMENSION_MISMATCH",
            ConstraintIrError::NonPositiveLength { .. } => "SKETCH_CONSTRAINT_NON_POSITIVE_LENGTH",
            ConstraintIrError::IncomparableEqualOperands { .. } => {
                "SKETCH_CONSTRAINT_INCOMPARABLE_EQUAL_OPERANDS"
            }
            ConstraintIrError::UnsupportedTangentPair { .. } => {
                "SKETCH_CONSTRAINT_UNSUPPORTED_TANGENT_PAIR"
            }
        }
    }

    pub fn message(&self) -> String {
        match self {
            ConstraintIrError::ForeignOperand {
                context,
                entity,
                expected_sketch,
                ..
            } => {
                format!("{context} references {entity}, which does not belong to {expected_sketch}")
            }
            ConstraintIrError::WrongEntityKind {
                context,
                expected,
                found,
                ..
            } => format!("{context} requires a {expected}, found a {found}"),
            ConstraintIrError::DimensionMismatch {
                context,
                expected,
                found,
                ..
            } => format!("{context} requires a {expected} quantity, found {found}"),
            ConstraintIrError::NonPositiveLength { context, found, .. } => {
                format!("{context} must be strictly positive, found {found}")
            }
            ConstraintIrError::IncomparableEqualOperands { a, b, .. } => format!(
                "equal requires two lines or two circle/arc entities, found {a} and {b} of \
                 different comparable categories"
            ),
            ConstraintIrError::UnsupportedTangentPair { a, b, .. } => format!(
                "tangent requires at least one circle/arc operand, found two lines ({a}, {b})"
            ),
        }
    }

    pub fn span(&self) -> Span {
        match self {
            ConstraintIrError::ForeignOperand { span, .. }
            | ConstraintIrError::WrongEntityKind { span, .. }
            | ConstraintIrError::DimensionMismatch { span, .. }
            | ConstraintIrError::NonPositiveLength { span, .. }
            | ConstraintIrError::IncomparableEqualOperands { span, .. }
            | ConstraintIrError::UnsupportedTangentPair { span, .. } => *span,
        }
    }

    /// Builds this error's `cad_diagnostics::Diagnostic`, mirroring
    /// `cad_hir::sketch::SketchIrError::to_diagnostic`'s identical
    /// pattern, reusing the `CONSTRAINT` family already registered in
    /// `cad_diagnostics::DIAGNOSTIC_FAMILIES` (a fixed RFC-0005
    /// taxonomy this task must not silently extend) with fresh codes
    /// `CONSTRAINT-E001`..`E006` (no other module has used this family
    /// yet).
    pub fn to_diagnostic(&self, file: &str, source: &str) -> Diagnostic {
        let span = self.span();
        let line_index = cad_ast::LineIndex::new(source);
        let start = line_index.line_column(source, span.start);
        let end = line_index.line_column(source, span.end);
        let code = DiagnosticCode::parse(self.code())
            .expect("ConstraintIrError::code always returns a well-formed FAMILY-Exxx code");
        Diagnostic::new(
            code,
            Severity::Error,
            "sketch-constraint-ir",
            self.title(),
            self.message(),
        )
        .expect("every ConstraintIrError code carries the 'E' severity letter")
        .with_source(SourceSpan {
            file: file.to_string(),
            start: Position::new(start.line, start.column),
            end: Position::new(end.line, end.column),
        })
    }
}

impl fmt::Display for ConstraintIrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for ConstraintIrError {}

// ---------------------------------------------------------------------
// ConstraintSet
// ---------------------------------------------------------------------

fn entity_kind_name(kind: &SketchEntityKind) -> &'static str {
    match kind {
        SketchEntityKind::Line { .. } => "line",
        SketchEntityKind::Circle { .. } => "circle",
        SketchEntityKind::Arc { .. } => "arc",
    }
}

/// An explicit, kernel-and-solver-independent set of constraints applied
/// to one [`Sketch`] (`DL-20`): an append-only, ordered list of
/// [`Constraint`]s this set alone owns. See module doc comment for the
/// identity/scope-cut rationale.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintSet {
    sketch: SketchId,
    constraints: Vec<Constraint>,
}

impl ConstraintSet {
    pub fn new(sketch: SketchId) -> ConstraintSet {
        ConstraintSet {
            sketch,
            constraints: Vec::new(),
        }
    }

    pub fn sketch(&self) -> SketchId {
        self.sketch
    }

    pub fn constraints(&self) -> &[Constraint] {
        &self.constraints
    }

    pub fn get(&self, id: ConstraintId) -> Option<&Constraint> {
        if id.sketch != self.sketch {
            return None;
        }
        self.constraints.get(id.index as usize)
    }

    fn next_id(&self) -> ConstraintId {
        ConstraintId {
            sketch: self.sketch,
            index: self.constraints.len() as u32,
        }
    }

    fn check_entity(
        &self,
        sketch: &Sketch,
        entity: SketchEntityId,
        context: &'static str,
        span: Span,
    ) -> Result<SketchEntityKind, ConstraintIrError> {
        sketch
            .get(entity)
            .map(|e| e.kind)
            .ok_or(ConstraintIrError::ForeignOperand {
                context,
                entity,
                expected_sketch: self.sketch,
                span,
            })
    }

    fn check_point(
        &self,
        sketch: &Sketch,
        point: PointRef,
        context: &'static str,
        span: Span,
    ) -> Result<(), ConstraintIrError> {
        let kind = self.check_entity(sketch, point.entity(), context, span)?;
        if !point.matches_kind(&kind) {
            return Err(ConstraintIrError::WrongEntityKind {
                context,
                expected: point.kind_name(),
                found: entity_kind_name(&kind),
                span,
            });
        }
        Ok(())
    }

    fn check_line(
        &self,
        sketch: &Sketch,
        entity: SketchEntityId,
        context: &'static str,
        span: Span,
    ) -> Result<(), ConstraintIrError> {
        let kind = self.check_entity(sketch, entity, context, span)?;
        if !matches!(kind, SketchEntityKind::Line { .. }) {
            return Err(ConstraintIrError::WrongEntityKind {
                context,
                expected: "line",
                found: entity_kind_name(&kind),
                span,
            });
        }
        Ok(())
    }

    fn check_circle_or_arc(
        &self,
        sketch: &Sketch,
        entity: SketchEntityId,
        context: &'static str,
        span: Span,
    ) -> Result<(), ConstraintIrError> {
        let kind = self.check_entity(sketch, entity, context, span)?;
        if !matches!(
            kind,
            SketchEntityKind::Circle { .. } | SketchEntityKind::Arc { .. }
        ) {
            return Err(ConstraintIrError::WrongEntityKind {
                context,
                expected: "circle or arc",
                found: entity_kind_name(&kind),
                span,
            });
        }
        Ok(())
    }

    fn check_dimension(
        quantity: &Quantity,
        expected: Dimension,
        context: &'static str,
        span: Span,
    ) -> Result<(), ConstraintIrError> {
        match quantity.ty {
            OperandType::Dimensional { dimension, .. } if dimension == expected => Ok(()),
            other => Err(ConstraintIrError::DimensionMismatch {
                context,
                expected,
                found: other,
                span,
            }),
        }
    }

    fn check_positive_length(
        quantity: &Quantity,
        context: &'static str,
        span: Span,
    ) -> Result<(), ConstraintIrError> {
        Self::check_dimension(quantity, Dimension::Length, context, span)?;
        if quantity.magnitude <= 0.0 {
            return Err(ConstraintIrError::NonPositiveLength {
                context,
                found: quantity.magnitude,
                span,
            });
        }
        Ok(())
    }

    /// Validates and appends one constraint, minting a fresh
    /// [`ConstraintId`]. `sketch` must be the same sketch this set was
    /// constructed for (checked per-operand: every operand not owned by
    /// `sketch` is a [`ConstraintIrError::ForeignOperand`]).
    pub fn add(
        &mut self,
        sketch: &Sketch,
        kind: ConstraintKind,
        span: Span,
    ) -> Result<ConstraintId, ConstraintIrError> {
        self.validate(sketch, &kind, span)?;
        let id = self.next_id();
        self.constraints.push(Constraint { id, kind, span });
        Ok(id)
    }

    fn validate(
        &self,
        sketch: &Sketch,
        kind: &ConstraintKind,
        span: Span,
    ) -> Result<(), ConstraintIrError> {
        match *kind {
            ConstraintKind::Coincident { a, b } => {
                self.check_point(sketch, a, "coincident.a", span)?;
                self.check_point(sketch, b, "coincident.b", span)?;
            }
            ConstraintKind::Horizontal { line } => {
                self.check_line(sketch, line, "horizontal.line", span)?;
            }
            ConstraintKind::Vertical { line } => {
                self.check_line(sketch, line, "vertical.line", span)?;
            }
            ConstraintKind::Parallel { a, b } => {
                self.check_line(sketch, a, "parallel.a", span)?;
                self.check_line(sketch, b, "parallel.b", span)?;
            }
            ConstraintKind::Perpendicular { a, b } => {
                self.check_line(sketch, a, "perpendicular.a", span)?;
                self.check_line(sketch, b, "perpendicular.b", span)?;
            }
            ConstraintKind::Tangent { a, b } => {
                let ka = self.check_entity(sketch, a, "tangent.a", span)?;
                let kb = self.check_entity(sketch, b, "tangent.b", span)?;
                let is_curve = |k: &SketchEntityKind| {
                    matches!(
                        k,
                        SketchEntityKind::Line { .. }
                            | SketchEntityKind::Circle { .. }
                            | SketchEntityKind::Arc { .. }
                    )
                };
                if !is_curve(&ka) {
                    return Err(ConstraintIrError::WrongEntityKind {
                        context: "tangent.a",
                        expected: "line, circle, or arc",
                        found: entity_kind_name(&ka),
                        span,
                    });
                }
                if !is_curve(&kb) {
                    return Err(ConstraintIrError::WrongEntityKind {
                        context: "tangent.b",
                        expected: "line, circle, or arc",
                        found: entity_kind_name(&kb),
                        span,
                    });
                }
                let is_curved = |k: &SketchEntityKind| {
                    matches!(
                        k,
                        SketchEntityKind::Circle { .. } | SketchEntityKind::Arc { .. }
                    )
                };
                if !is_curved(&ka) && !is_curved(&kb) {
                    return Err(ConstraintIrError::UnsupportedTangentPair { a, b, span });
                }
            }
            ConstraintKind::Concentric { a, b } => {
                self.check_circle_or_arc(sketch, a, "concentric.a", span)?;
                self.check_circle_or_arc(sketch, b, "concentric.b", span)?;
            }
            ConstraintKind::Equal { a, b } => {
                let ka = self.check_entity(sketch, a, "equal.a", span)?;
                let kb = self.check_entity(sketch, b, "equal.b", span)?;
                let is_line = matches!(ka, SketchEntityKind::Line { .. });
                let is_radial = |k: &SketchEntityKind| {
                    matches!(
                        k,
                        SketchEntityKind::Circle { .. } | SketchEntityKind::Arc { .. }
                    )
                };
                let comparable = (is_line && matches!(kb, SketchEntityKind::Line { .. }))
                    || (is_radial(&ka) && is_radial(&kb));
                if !comparable {
                    return Err(ConstraintIrError::IncomparableEqualOperands { a, b, span });
                }
            }
            ConstraintKind::Symmetric { a, b, about } => {
                self.check_entity(sketch, a, "symmetric.a", span)?;
                self.check_entity(sketch, b, "symmetric.b", span)?;
                self.check_line(sketch, about, "symmetric.about", span)?;
            }
            ConstraintKind::Distance { a, b, value } => {
                self.check_point(sketch, a, "distance.a", span)?;
                self.check_point(sketch, b, "distance.b", span)?;
                Self::check_dimension(&value, Dimension::Length, "distance.value", span)?;
            }
            ConstraintKind::Angle { a, b, value } => {
                self.check_line(sketch, a, "angle.a", span)?;
                self.check_line(sketch, b, "angle.b", span)?;
                Self::check_dimension(&value, Dimension::Angle, "angle.value", span)?;
            }
            ConstraintKind::Radius { entity, value } => {
                self.check_circle_or_arc(sketch, entity, "radius.entity", span)?;
                Self::check_positive_length(&value, "radius.value", span)?;
            }
            ConstraintKind::Diameter { entity, value } => {
                self.check_circle_or_arc(sketch, entity, "diameter.entity", span)?;
                Self::check_positive_length(&value, "diameter.value", span)?;
            }
            ConstraintKind::Fixed { entity } => {
                self.check_entity(sketch, entity, "fixed.entity", span)?;
            }
            ConstraintKind::Midpoint { point, line } => {
                self.check_point(sketch, point, "midpoint.point", span)?;
                self.check_line(sketch, line, "midpoint.line", span)?;
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------
// Solve status / solver adapter boundary
// ---------------------------------------------------------------------

/// The solve-status vocabulary `DL-20` requires "at minimum": solved,
/// underconstrained, or overconstrained. A [`SketchSolver`] backend may
/// never redefine what these mean or let an arbitrary internal branch
/// silently stand in for one of them.
#[derive(Debug, Clone, PartialEq)]
pub enum SolveStatus {
    /// Every constraint is satisfied and no free variable remains.
    Solved,
    /// At least one free variable remains after every constraint is
    /// satisfied. `remaining_dof` is the backend's own count of
    /// unresolved scalar degrees of freedom — evidence, not a claim of
    /// minimality.
    Underconstrained { remaining_dof: u32 },
    /// No assignment satisfies every constraint simultaneously.
    /// `conflicting` is structured evidence (every constraint the
    /// backend found in tension) — `DL-20` explicitly does not require
    /// this to be a provably minimal conflict set in the Stage-3
    /// baseline.
    Overconstrained { conflicting: Vec<ConstraintId> },
    /// A backend-specific extension beyond `DL-20`'s "at minimum" three
    /// statuses (added by `AICAD-074`'s own `RelaxationSolver`, per
    /// `crates/cad-constraints/src/sketch_solver.rs`'s own doc comment):
    /// this particular backend has no way to act on one or more of the
    /// listed constraints at all (e.g. both of a `coincident`'s operands
    /// are points this backend cannot move). Reported explicitly rather
    /// than silently folded into `Overconstrained` (which would wrongly
    /// imply the constraints are in tension, not merely unsupported) or
    /// `Solved` (which would wrongly imply the constraint was actually
    /// enforced). A different backend implementing this same trait may
    /// legitimately support what one backend reports `Unsupported` for.
    Unsupported { constraints: Vec<ConstraintId> },
}

/// A solved [`SketchVariable`] -> magnitude assignment a [`SketchSolver`]
/// produces. Magnitudes are already-canonical-unit `f64`s, matching
/// every sketch-entity field's own convention
/// (`cad_hir::sketch::Quantity`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SolvedValues(HashMap<SketchVariable, f64>);

impl SolvedValues {
    pub fn new() -> SolvedValues {
        SolvedValues(HashMap::new())
    }

    pub fn set(&mut self, variable: SketchVariable, magnitude: f64) {
        self.0.insert(variable, magnitude);
    }

    pub fn get(&self, variable: SketchVariable) -> Option<f64> {
        self.0.get(&variable).copied()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A [`SketchSolver`]'s complete answer for one solve attempt: its
/// classification ([`SolveStatus`]) plus whatever variable assignments
/// it was able to determine (may be partial, or empty, for a non-`Solved`
/// status).
#[derive(Debug, Clone, PartialEq)]
pub struct SolveReport {
    pub status: SolveStatus,
    pub values: SolvedValues,
}

/// The solver-independence boundary `DL-20` requires: a pluggable
/// backend that owns numerical algorithms only. Implementations must
/// not redefine [`ConstraintKind`]'s semantics, [`SolveStatus`]'s
/// vocabulary, or this trait's own contract — only *how* a satisfying
/// assignment (or a structured failure) is computed. See module doc
/// comment ("Scope cuts") for why this task ships the trait but no
/// implementation of it.
pub trait SketchSolver {
    fn solve(&self, sketch: &Sketch, constraints: &ConstraintSet) -> SolveReport;
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_hir::sketch::{RotationDirection, SketchPlane};
    use std::f64::consts::PI;

    fn span() -> Span {
        Span::new(0, 1)
    }

    fn length(magnitude: f64) -> Quantity {
        Quantity::of(magnitude, Dimension::Length)
    }

    fn angle(magnitude: f64) -> Quantity {
        Quantity::of(magnitude, Dimension::Angle)
    }

    fn sketch_with_two_lines() -> (Sketch, SketchEntityId, SketchEntityId) {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let l1 = sketch.add_line(Point2::ORIGIN, Point2::new(1.0, 0.0), false, span());
        let l2 = sketch.add_line(Point2::new(0.0, 1.0), Point2::new(1.0, 1.0), false, span());
        (sketch, l1, l2)
    }

    #[test]
    fn constraints_get_sequential_ids_scoped_to_their_sketch() {
        let (sketch, l1, l2) = sketch_with_two_lines();
        let mut set = ConstraintSet::new(sketch.id());
        let c1 = set
            .add(&sketch, ConstraintKind::Horizontal { line: l1 }, span())
            .unwrap();
        let c2 = set
            .add(&sketch, ConstraintKind::Horizontal { line: l2 }, span())
            .unwrap();
        assert_eq!(c1.sketch(), sketch.id());
        assert_eq!(c1.index(), 0);
        assert_eq!(c2.index(), 1);
        assert_eq!(set.constraints().len(), 2);
    }

    #[test]
    fn two_constraint_sets_never_share_id_space() {
        let (sketch_a, a0, _) = sketch_with_two_lines();
        let mut sketch_b = Sketch::new(SketchId::new(1), SketchPlane::WorldXy);
        let b0 = sketch_b.add_line(Point2::ORIGIN, Point2::new(1.0, 0.0), false, span());
        let mut set_a = ConstraintSet::new(sketch_a.id());
        let mut set_b = ConstraintSet::new(sketch_b.id());
        let ca = set_a
            .add(&sketch_a, ConstraintKind::Horizontal { line: a0 }, span())
            .unwrap();
        let cb = set_b
            .add(&sketch_b, ConstraintKind::Horizontal { line: b0 }, span())
            .unwrap();
        assert_eq!(ca.index(), cb.index());
        assert_ne!(ca.sketch(), cb.sketch());
        assert!(set_a.get(ca).is_some());
        assert!(set_a.get(cb).is_none());
    }

    #[test]
    fn horizontal_accepts_a_line() {
        let (sketch, l1, _) = sketch_with_two_lines();
        let mut set = ConstraintSet::new(sketch.id());
        assert!(
            set.add(&sketch, ConstraintKind::Horizontal { line: l1 }, span())
                .is_ok()
        );
    }

    #[test]
    fn horizontal_rejects_a_circle() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let circle = sketch
            .add_circle(Point2::ORIGIN, length(1.0), false, span())
            .unwrap();
        let mut set = ConstraintSet::new(sketch.id());
        let err = set
            .add(&sketch, ConstraintKind::Horizontal { line: circle }, span())
            .unwrap_err();
        match err {
            ConstraintIrError::WrongEntityKind {
                context,
                expected,
                found,
                ..
            } => {
                assert_eq!(context, "horizontal.line");
                assert_eq!(expected, "line");
                assert_eq!(found, "circle");
            }
            other => panic!("expected WrongEntityKind, got {other:?}"),
        }
    }

    #[test]
    fn operand_from_a_different_sketch_than_the_one_passed_is_rejected() {
        let (_sketch_a, a0, _) = sketch_with_two_lines();
        let sketch_b = Sketch::new(SketchId::new(1), SketchPlane::WorldXy);
        let mut set_b = ConstraintSet::new(sketch_b.id());
        let err = set_b
            .add(&sketch_b, ConstraintKind::Horizontal { line: a0 }, span())
            .unwrap_err();
        match err {
            ConstraintIrError::ForeignOperand {
                entity,
                expected_sketch,
                ..
            } => {
                assert_eq!(entity, a0);
                assert_eq!(expected_sketch, sketch_b.id());
            }
            other => panic!("expected ForeignOperand, got {other:?}"),
        }
    }

    #[test]
    fn parallel_requires_two_lines() {
        let (sketch, l1, l2) = sketch_with_two_lines();
        let mut set = ConstraintSet::new(sketch.id());
        assert!(
            set.add(&sketch, ConstraintKind::Parallel { a: l1, b: l2 }, span())
                .is_ok()
        );
    }

    #[test]
    fn concentric_requires_circle_or_arc() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let c1 = sketch
            .add_circle(Point2::ORIGIN, length(1.0), false, span())
            .unwrap();
        let c2 = sketch
            .add_circle(Point2::new(5.0, 0.0), length(2.0), false, span())
            .unwrap();
        let line = sketch.add_line(Point2::ORIGIN, Point2::new(1.0, 0.0), false, span());
        let mut set = ConstraintSet::new(sketch.id());
        assert!(
            set.add(&sketch, ConstraintKind::Concentric { a: c1, b: c2 }, span())
                .is_ok()
        );
        let err = set
            .add(
                &sketch,
                ConstraintKind::Concentric { a: c1, b: line },
                span(),
            )
            .unwrap_err();
        assert!(matches!(err, ConstraintIrError::WrongEntityKind { .. }));
    }

    #[test]
    fn equal_accepts_two_lines_or_two_radial_entities_but_not_a_mix() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let l1 = sketch.add_line(Point2::ORIGIN, Point2::new(1.0, 0.0), false, span());
        let l2 = sketch.add_line(Point2::new(0.0, 1.0), Point2::new(1.0, 1.0), false, span());
        let circle = sketch
            .add_circle(Point2::ORIGIN, length(1.0), false, span())
            .unwrap();
        let arc = sketch
            .add_arc(
                Point2::ORIGIN,
                length(1.0),
                angle(0.0),
                angle(PI),
                RotationDirection::CounterClockwise,
                false,
                span(),
            )
            .unwrap();
        let mut set = ConstraintSet::new(sketch.id());
        assert!(
            set.add(&sketch, ConstraintKind::Equal { a: l1, b: l2 }, span())
                .is_ok()
        );
        assert!(
            set.add(&sketch, ConstraintKind::Equal { a: circle, b: arc }, span())
                .is_ok()
        );
        let err = set
            .add(&sketch, ConstraintKind::Equal { a: l1, b: circle }, span())
            .unwrap_err();
        assert!(matches!(
            err,
            ConstraintIrError::IncomparableEqualOperands { .. }
        ));
    }

    #[test]
    fn tangent_rejects_two_lines_but_accepts_line_and_circle() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let l1 = sketch.add_line(Point2::ORIGIN, Point2::new(1.0, 0.0), false, span());
        let l2 = sketch.add_line(Point2::new(0.0, 1.0), Point2::new(1.0, 1.0), false, span());
        let circle = sketch
            .add_circle(Point2::new(0.5, 2.0), length(1.0), false, span())
            .unwrap();
        let mut set = ConstraintSet::new(sketch.id());
        let err = set
            .add(&sketch, ConstraintKind::Tangent { a: l1, b: l2 }, span())
            .unwrap_err();
        assert!(matches!(
            err,
            ConstraintIrError::UnsupportedTangentPair { .. }
        ));
        assert!(
            set.add(
                &sketch,
                ConstraintKind::Tangent { a: l1, b: circle },
                span()
            )
            .is_ok()
        );
    }

    #[test]
    fn distance_requires_a_length_value() {
        let (sketch, l1, l2) = sketch_with_two_lines();
        let a = PointRef::LineStart(l1);
        let b = PointRef::LineStart(l2);
        let mut set = ConstraintSet::new(sketch.id());
        let err = set
            .add(
                &sketch,
                ConstraintKind::Distance {
                    a,
                    b,
                    value: angle(1.0),
                },
                span(),
            )
            .unwrap_err();
        match err {
            ConstraintIrError::DimensionMismatch { context, .. } => {
                assert_eq!(context, "distance.value")
            }
            other => panic!("expected DimensionMismatch, got {other:?}"),
        }
        assert!(
            set.add(
                &sketch,
                ConstraintKind::Distance {
                    a,
                    b,
                    value: length(1.0),
                },
                span(),
            )
            .is_ok()
        );
    }

    #[test]
    fn point_ref_kind_mismatch_is_rejected() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let circle = sketch
            .add_circle(Point2::ORIGIN, length(1.0), false, span())
            .unwrap();
        let line = sketch.add_line(Point2::ORIGIN, Point2::new(1.0, 0.0), false, span());
        let mut set = ConstraintSet::new(sketch.id());
        // `LineStart` applied to a circle entity is a kind mismatch.
        let err = set
            .add(
                &sketch,
                ConstraintKind::Coincident {
                    a: PointRef::LineStart(circle),
                    b: PointRef::LineStart(line),
                },
                span(),
            )
            .unwrap_err();
        assert!(matches!(err, ConstraintIrError::WrongEntityKind { .. }));
    }

    #[test]
    fn radius_and_diameter_require_positive_length_on_circle_or_arc() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let circle = sketch
            .add_circle(Point2::ORIGIN, length(1.0), false, span())
            .unwrap();
        let mut set = ConstraintSet::new(sketch.id());
        assert!(
            set.add(
                &sketch,
                ConstraintKind::Radius {
                    entity: circle,
                    value: length(2.0),
                },
                span(),
            )
            .is_ok()
        );
        let err = set
            .add(
                &sketch,
                ConstraintKind::Diameter {
                    entity: circle,
                    value: length(0.0),
                },
                span(),
            )
            .unwrap_err();
        assert!(matches!(err, ConstraintIrError::NonPositiveLength { .. }));
    }

    #[test]
    fn symmetric_accepts_a_line_but_rejects_a_non_line_about_operand() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let l1 = sketch.add_line(Point2::ORIGIN, Point2::new(1.0, 0.0), false, span());
        let l2 = sketch.add_line(Point2::new(0.0, 1.0), Point2::new(1.0, 1.0), false, span());
        let circle = sketch
            .add_circle(Point2::new(5.0, 5.0), length(1.0), false, span())
            .unwrap();
        let mut set = ConstraintSet::new(sketch.id());
        assert!(
            set.add(
                &sketch,
                ConstraintKind::Symmetric {
                    a: l1,
                    b: l2,
                    about: l1,
                },
                span(),
            )
            .is_ok()
        );
        let err = set
            .add(
                &sketch,
                ConstraintKind::Symmetric {
                    a: l1,
                    b: l2,
                    about: circle,
                },
                span(),
            )
            .unwrap_err();
        assert!(matches!(err, ConstraintIrError::WrongEntityKind { .. }));
    }

    #[test]
    fn resolve_point_reads_line_and_circle_and_arc_points() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let line = sketch.add_line(Point2::new(1.0, 2.0), Point2::new(3.0, 4.0), false, span());
        let circle = sketch
            .add_circle(Point2::new(5.0, 6.0), length(1.0), false, span())
            .unwrap();
        let arc = sketch
            .add_arc(
                Point2::ORIGIN,
                length(2.0),
                angle(0.0),
                angle(std::f64::consts::FRAC_PI_2),
                RotationDirection::CounterClockwise,
                false,
                span(),
            )
            .unwrap();

        assert_eq!(
            resolve_point(&sketch, PointRef::LineStart(line)),
            Some(Point2::new(1.0, 2.0))
        );
        assert_eq!(
            resolve_point(&sketch, PointRef::LineEnd(line)),
            Some(Point2::new(3.0, 4.0))
        );
        assert_eq!(
            resolve_point(&sketch, PointRef::CircleCenter(circle)),
            Some(Point2::new(5.0, 6.0))
        );
        let start = resolve_point(&sketch, PointRef::ArcStart(arc)).unwrap();
        assert!((start.x - 2.0).abs() < 1e-9);
        assert!(start.y.abs() < 1e-9);
        let end = resolve_point(&sketch, PointRef::ArcEnd(arc)).unwrap();
        assert!(end.x.abs() < 1e-9);
        assert!((end.y - 2.0).abs() < 1e-9);
    }

    #[test]
    fn sketch_variables_counts_expected_dof_per_entity_kind() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        sketch.add_line(Point2::ORIGIN, Point2::new(1.0, 0.0), false, span());
        sketch
            .add_circle(Point2::ORIGIN, length(1.0), false, span())
            .unwrap();
        sketch
            .add_arc(
                Point2::ORIGIN,
                length(1.0),
                angle(0.0),
                angle(PI),
                RotationDirection::CounterClockwise,
                false,
                span(),
            )
            .unwrap();
        let vars = sketch_variables(&sketch);
        // line: 4, circle: 3, arc: 5 => 12 total.
        assert_eq!(vars.len(), 12);
        for v in &vars {
            let _ = v.dimension();
        }
    }

    #[test]
    fn every_error_variant_converts_to_a_well_formed_diagnostic() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let sketch_id = sketch.id();
        let dummy_entity = sketch.add_line(Point2::ORIGIN, Point2::new(1.0, 0.0), false, span());
        let errors = vec![
            ConstraintIrError::ForeignOperand {
                context: "horizontal.line",
                entity: dummy_entity,
                expected_sketch: sketch_id,
                span: span(),
            },
            ConstraintIrError::WrongEntityKind {
                context: "horizontal.line",
                expected: "line",
                found: "circle",
                span: span(),
            },
            ConstraintIrError::DimensionMismatch {
                context: "distance.value",
                expected: Dimension::Length,
                found: OperandType::dimensional(Dimension::Angle, None),
                span: span(),
            },
            ConstraintIrError::NonPositiveLength {
                context: "radius.value",
                found: -1.0,
                span: span(),
            },
            ConstraintIrError::IncomparableEqualOperands {
                a: dummy_entity,
                b: dummy_entity,
                span: span(),
            },
            ConstraintIrError::UnsupportedTangentPair {
                a: dummy_entity,
                b: dummy_entity,
                span: span(),
            },
        ];
        for err in errors {
            let code = err.code();
            assert!(code.starts_with("CONSTRAINT-E"));
            DiagnosticCode::parse(code).expect("every ConstraintIrError code must be well-formed");
            let diagnostic = err.to_diagnostic("test.aicad", "x");
            assert_eq!(diagnostic.category, "sketch-constraint-ir");
            assert!(!diagnostic.message.is_empty());
            assert!(!err.to_string().is_empty());
        }
    }

    #[test]
    fn constraint_id_display_is_stable() {
        let (sketch, l1, _) = sketch_with_two_lines();
        let mut set = ConstraintSet::new(sketch.id());
        let id = set
            .add(&sketch, ConstraintKind::Horizontal { line: l1 }, span())
            .unwrap();
        assert_eq!(id.to_string(), format!("{}/constraint#0", sketch.id()));
    }

    #[test]
    fn a_mock_solver_can_implement_the_adapter_trait() {
        struct AlwaysUnderconstrained;
        impl SketchSolver for AlwaysUnderconstrained {
            fn solve(&self, sketch: &Sketch, _constraints: &ConstraintSet) -> SolveReport {
                SolveReport {
                    status: SolveStatus::Underconstrained {
                        remaining_dof: sketch_variables(sketch).len() as u32,
                    },
                    values: SolvedValues::new(),
                }
            }
        }
        let (sketch, l1, _) = sketch_with_two_lines();
        let mut set = ConstraintSet::new(sketch.id());
        set.add(&sketch, ConstraintKind::Horizontal { line: l1 }, span())
            .unwrap();
        let solver = AlwaysUnderconstrained;
        let report = solver.solve(&sketch, &set);
        match report.status {
            SolveStatus::Underconstrained { remaining_dof } => assert_eq!(remaining_dof, 8),
            other => panic!("expected Underconstrained, got {other:?}"),
        }
        assert!(report.values.is_empty());
    }
}
