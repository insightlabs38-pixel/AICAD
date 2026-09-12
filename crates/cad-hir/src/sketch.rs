//! Stage-3 minimal sketch entity IR (`AICAD-072`, Batch S3-04), per
//! `docs/plan/04_HIGH_LEVEL_MODELING_API.md` §3's `sketch`/`line`/
//! `circle`/`arc`/`rectangle`/`polygon`/`slot` feature-catalogue entries
//! and `project/DECISION_LOG.md#DL-19` (`D3`).
//!
//! # What `DL-19` requires, and how this module satisfies it
//!
//! `DL-19` requires an explicit, kernel-independent semantic [`Sketch`]
//! object that owns its plane/frame, explicit entity nodes, deterministic
//! *local* semantic entity identities "mirroring `param`'s own
//! `BindingId`-based identity precedent... not a parallel identity
//! scheme", and that no entity's existence depend on an implicit,
//! unobservable side effect.
//!
//! This module gives that precedent's *mechanism*, not its literal type:
//! exactly like [`crate::ids::BindingId`] is minted only by
//! [`crate::lower::Lowerer`]'s own counter (never invented by a
//! consumer) and exactly like `cad_geometry_api::ir::GeomId` is minted
//! only by its owning `GeometryGraph` in strictly increasing order, a
//! [`SketchEntityId`] is minted only by [`Sketch::add_line`]/
//! [`Sketch::add_circle`]/[`Sketch::add_arc`] (and the composite
//! constructors built on them), always embeds the [`SketchId`] of the
//! [`Sketch`] that minted it (satisfying "identify both the owning
//! sketch and the entity itself" directly in the id's own shape, not by
//! convention), and is never constructible from an arbitrary integer by
//! a caller. There is no global mutable registry anywhere in this
//! module: a [`Sketch`] is an ordinary owned value: two `Sketch`es never
//! share entity-id space, and nothing here reaches outside a `Sketch`'s
//! own fields to decide an entity's identity.
//!
//! # Scope cuts (deliberate, matching `AICAD-070`/`AICAD-071`'s own
//! precedent of declaring a data model before wiring it to source syntax)
//!
//! - **No grammar/lowering integration.** No `sketch { ... }` block
//!   syntax, builtin-function signature, or runtime `Value` variant
//!   exists yet for any of this module's types — this task is the IR
//!   shape only, exactly as `AICAD-070`'s `geometry_types.rs` landed six
//!   struct *declarations* with no runtime construction path until a
//!   separate task closed that gap. Whether a future task wires
//!   `sketch`/`line`/`circle`/... as Safe CAD builtins, as ordinary
//!   language structs, or some other mechanism is explicitly left open.
//! - **[`SketchPlane`] is a fixed enum of the three world reference
//!   planes only** (`WorldXy`/`WorldXz`/`WorldYz`), not a general
//!   `Frame3`/`FaceRef`-parameterized plane as `docs/plan/
//!   04_HIGH_LEVEL_MODELING_API.md`'s `sketch.plane: Plane|FaceRef|Frame3`
//!   signature ultimately wants. `AICAD-075A` (Batch S3-06, still ahead
//!   of this task) is explicitly charged with establishing "one coherent
//!   minimal axis/frame/rotation foundation for revolve, transform,
//!   mirror, and circular patterns" — inventing this sketch's own
//!   independent arbitrary-frame math *now*, before that foundation
//!   exists, would be exactly the "each feature invents independent
//!   spatial semantics" outcome that task is charged with preventing.
//!   `FaceRef`-based planes additionally require Stage-4 semantic
//!   references, out of scope entirely before `AICAD-079A`/Stage 4.
//! - **No closed-profile / geometric-validity checking.** A [`Profile`]
//!   here is only an explicit, ordered aggregation of entity ids this
//!   module's own [`Sketch`] already owns — it does not check that the
//!   entities actually form a closed loop, that they don't self-
//!   intersect, or any other numerical-tolerance geometric property.
//!   "Solved closed profile" is `AICAD-073`/`AICAD-074`/`AICAD-075`'s own
//!   constraint-solving territory (Batch S3-05, still ahead); introducing
//!   a tolerance-based closure check here would risk a second, competing
//!   tolerance policy ahead of that work, which `AGENTS.md`'s D5
//!   precedent (`project/OWNER_DECISIONS.md#D19`) treats as exactly the
//!   kind of numeric-tolerance decision that must not be made twice.
//!   This module validates only structural/dimensional well-formedness —
//!   the same restraint [`cad_geometry_api::ir::GeometryGraph`] already
//!   exercises (operand existence, dimension correctness, non-empty
//!   operand lists; never numerical geometric validity, which is the
//!   kernel's/solver's job).
//! - **`rectangle`'s `corner_radius` is not implemented.** `docs/plan/
//!   04_HIGH_LEVEL_MODELING_API.md`'s own table defaults it to `0mm`;
//!   [`Sketch::add_rectangle`] only ever builds the sharp-cornered
//!   rectangle. Rounding a `Profile`'s corners is a fillet-shaped
//!   operation on entities, not a distinct entity kind, and does not
//!   block any Batch S3-04/S3-05 dependency.

use cad_ast::{LineIndex, Span};
use cad_diagnostics::{Diagnostic, DiagnosticCode, Position, Severity, SourceSpan};
use cad_types::Dimension;
use cad_units::OperandType;
use std::f64::consts::PI;
use std::fmt;
use std::ops::{Add, Sub};

// ---------------------------------------------------------------------
// Pure 2D math primitives
// ---------------------------------------------------------------------
//
// Mirrors `cad_kernel_api::geometry`'s `Point3`/`Vector3`/`Direction3`
// exactly (same shape, same "pure Rust, no dimension/unit tag" raw-`f64`
// convention for a *point's own* coordinates — see that module's own doc
// comment: "typed engineering quantities... are a later stage['s job],
// not this one"). Defined independently here rather than imported: this
// crate must not depend on `cad-kernel-api` (a lower architectural layer
// — `docs/plan/01_SYSTEM_ARCHITECTURE.md` §5's Source AST -> HIR ->
// Feature IR -> Geometry IR -> Kernel call graph ordering — HIR must not
// reach past Feature IR/Geometry IR into kernel-adjacent vocabulary), and
// no shared "pure 2D/3D math" crate exists to hold a common definition
// (adding one is exactly the kind of new-crate/new-abstraction move
// `AGENTS.md`'s "do not add a crate without a plan-doc entry" and "no
// premature abstraction" both rule out for what one task needs).

/// A point in the sketch's own 2D coordinate system (its owning
/// [`Sketch`]'s plane). Coordinates are already-resolved canonical-unit
/// `Length` magnitudes, exactly like [`cad_kernel_api::Point3`]'s own
/// convention for the analogous 3D case.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

impl Point2 {
    pub const ORIGIN: Point2 = Point2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Point2 {
        Point2 { x, y }
    }

    pub fn translate(self, v: Vector2) -> Point2 {
        Point2::new(self.x + v.x, self.y + v.y)
    }
}

impl Sub for Point2 {
    type Output = Vector2;
    /// The vector from `other` to `self`.
    fn sub(self, other: Point2) -> Vector2 {
        Vector2::new(self.x - other.x, self.y - other.y)
    }
}

impl Add<Vector2> for Point2 {
    type Output = Point2;
    fn add(self, v: Vector2) -> Point2 {
        self.translate(v)
    }
}

/// A free vector in the sketch's own 2D coordinate system (no fixed
/// location, unlike [`Point2`]) — mirrors [`cad_kernel_api::Vector3`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector2 {
    pub x: f64,
    pub y: f64,
}

impl Vector2 {
    pub const ZERO: Vector2 = Vector2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Vector2 {
        Vector2 { x, y }
    }

    pub fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn scale(self, s: f64) -> Vector2 {
        Vector2::new(self.x * s, self.y * s)
    }

    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// Normalizes into a [`Direction2`]. Returns `None` if this vector is
    /// not finite or too close to zero to normalize reliably — same
    /// `1e-12` threshold as [`cad_kernel_api::Vector3::normalize`].
    pub fn normalize(self) -> Option<Direction2> {
        if !self.is_finite() || self.length() < 1e-12 {
            return None;
        }
        let len = self.length();
        Some(Direction2(Vector2::new(self.x / len, self.y / len)))
    }
}

/// A unit-length 2D direction. The only way to construct one is
/// [`Vector2::normalize`], exactly mirroring
/// [`cad_kernel_api::Direction3`]'s identical invariant-by-construction
/// design.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Direction2(Vector2);

impl Direction2 {
    pub fn as_vector2(self) -> Vector2 {
        self.0
    }

    /// This direction rotated +90 degrees (counter-clockwise) — the left
    /// perpendicular, used by [`Sketch::add_slot`] to offset a
    /// centerline into the slot's two straight edges.
    pub fn perp(self) -> Direction2 {
        Direction2(Vector2::new(-self.0.y, self.0.x))
    }

    /// This direction's angle from the +X axis, in radians (the same
    /// canonical unit [`Quantity::of`]`(_, Dimension::Angle)` uses).
    pub fn angle(self) -> f64 {
        self.0.y.atan2(self.0.x)
    }
}

// ---------------------------------------------------------------------
// Typed quantities (`AGENTS.md`: "never a bare float")
// ---------------------------------------------------------------------

/// A typed magnitude used by a sketch-entity parameter (a circle's
/// radius, an arc's angles, a slot's width, ...).
///
/// Deliberately an independent, minimal re-implementation of exactly
/// [`cad_geometry_api::ir::Quantity`]'s shape (`magnitude: f64, ty:
/// OperandType`) and [`cad_runtime::value::NumberValue`]'s identical
/// shape — not a shared dependency on either crate. This mirrors this
/// crate's own [`crate::ids`] module doc comment precedent exactly
/// ("small, independent re-implementation... scoped to exactly what
/// [this layer] needs" rather than a cross-layer dependency): `cad-hir`
/// sits *above* `cad-geometry-api` in `docs/plan/
/// 01_SYSTEM_ARCHITECTURE.md` §5's pipeline (Source AST -> HIR ->
/// Feature IR -> **Geometry IR** -> Kernel call graph), so depending on
/// it here would be a downward reach across a layer this task does not
/// need to cross.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantity {
    pub magnitude: f64,
    pub ty: OperandType,
}

impl Quantity {
    pub fn new(magnitude: f64, ty: OperandType) -> Quantity {
        Quantity { magnitude, ty }
    }

    /// Convenience constructor for a non-affine dimension (`Length`,
    /// `Angle` — every dimension a sketch-entity parameter uses).
    pub fn of(magnitude: f64, dimension: Dimension) -> Quantity {
        Quantity {
            magnitude,
            ty: OperandType::dimensional(dimension, None),
        }
    }

    pub fn dimension(&self) -> Option<Dimension> {
        match self.ty {
            OperandType::Dimensional { dimension, .. } => Some(dimension),
            OperandType::Scalar(_) => None,
        }
    }
}

/// `arc`'s sweep orientation (`docs/plan/04_HIGH_LEVEL_MODELING_API.md`
/// §3's `arc.direction: RotationDirection`, default CCW).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationDirection {
    CounterClockwise,
    Clockwise,
}

// ---------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------

/// Identity of one [`Sketch`] value. See module doc comment
/// ("What `DL-19` requires") for why this crate does not (yet) mint
/// these from a single authoritative counter the way
/// [`crate::ids::BindingId`]/`GeomId` are minted from their own owning
/// container: no task has yet built the "multiple sketches in one
/// lowering context" registry `BindingId`'s `Lowerer`/`GeomId`'s
/// `GeometryGraph` play that role for. A `SketchId` is a plain,
/// publicly-constructible identity for now — the entity-local identity
/// scoping *within* one sketch ([`SketchEntityId`], minted only by that
/// sketch's own [`Sketch::add_line`]/[`Sketch::add_circle`]/
/// [`Sketch::add_arc`]) is what `DL-19` actually requires this task to
/// get right, and does not depend on how `SketchId`s themselves end up
/// minted once that future registry exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SketchId(u32);

impl SketchId {
    pub fn new(index: u32) -> SketchId {
        SketchId(index)
    }

    pub fn index(self) -> u32 {
        self.0
    }
}

impl fmt::Display for SketchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sketch#{}", self.0)
    }
}

/// Identity of one entity within its owning [`Sketch`] — a
/// (`SketchId`, `index`) pair, mirroring `cad_geometry_api::ir::GeomId`'s
/// "SSA-style identity... minted by the graph itself in strictly
/// increasing order" precedent, except the owning container's own id is
/// carried *in* the identity (`GeomId` does not need this, since a
/// `GeomId` is only ever meaningful against the one `GeometryGraph`
/// already in hand — see that type's own doc comment, "a `GeomId` from
/// one `GeometryGraph` is meaningless... against a different graph").
/// `DL-19` explicitly requires identifying "both the owning sketch and
/// the entity itself", so this type does that structurally rather than
/// by caller convention. Only [`Sketch`]'s own entity constructors ever
/// produce one — there is no public constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SketchEntityId {
    sketch: SketchId,
    index: u32,
}

impl SketchEntityId {
    pub fn sketch(self) -> SketchId {
        self.sketch
    }

    pub fn index(self) -> u32 {
        self.index
    }
}

impl fmt::Display for SketchEntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/entity#{}", self.sketch, self.index)
    }
}

// ---------------------------------------------------------------------
// Plane
// ---------------------------------------------------------------------

/// The support plane a [`Sketch`]'s 2D coordinates are interpreted
/// against. See module doc comment ("Scope cuts") for why this is a
/// fixed enum of the world reference planes rather than a general
/// `Frame3`/`FaceRef`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SketchPlane {
    /// The X/Y plane (normal +Z) — `docs/plan/
    /// 04_HIGH_LEVEL_MODELING_API.md`'s own default construction plane
    /// (see `plate`'s `frame: Frame3 = XY`).
    WorldXy,
    /// The X/Z plane (normal +Y).
    WorldXz,
    /// The Y/Z plane (normal +X).
    WorldYz,
}

// ---------------------------------------------------------------------
// Entities
// ---------------------------------------------------------------------

/// One sketch entity's geometric shape, per `docs/plan/
/// 04_HIGH_LEVEL_MODELING_API.md` §3's `line`/`circle`/`arc` entries
/// (the three primitive entity kinds that entity — as opposed to
/// composite `Profile` — constructors produce; see [`Sketch::add_rectangle`]/
/// [`Sketch::add_polygon`]/[`Sketch::add_slot`] for how the composite
/// shapes lower to sequences of these).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SketchEntityKind {
    Line {
        start: Point2,
        end: Point2,
    },
    Circle {
        center: Point2,
        radius: Quantity,
    },
    Arc {
        center: Point2,
        radius: Quantity,
        start_angle: Quantity,
        end_angle: Quantity,
        direction: RotationDirection,
    },
}

/// One entity owned by a [`Sketch`]: its identity, shape, construction-
/// geometry flag (`docs/plan/04_HIGH_LEVEL_MODELING_API.md` §3's
/// `line`/`circle`/`arc.construction: Bool`), and the source span it was
/// built from (threaded through for diagnostics, matching every other
/// Stage-2/3 IR's own span-fidelity convention —
/// `cad_geometry_api::ir::GeometryNode` in particular).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SketchEntity {
    pub id: SketchEntityId,
    pub kind: SketchEntityKind,
    pub construction: bool,
    pub span: Span,
}

/// An explicit, ordered aggregation of entity ids belonging to one
/// [`Sketch`] — the IR shape `rectangle`/`polygon`/`slot` return
/// (`docs/plan/04_HIGH_LEVEL_MODELING_API.md` §3: all three return
/// `Profile`, not `SketchEntity`). See module doc comment ("Scope
/// cuts") for why this does *not* itself check closure/validity.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub sketch: SketchId,
    pub entities: Vec<SketchEntityId>,
    pub span: Span,
}

// ---------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------

/// Every way constructing a [`Sketch`]'s entities/profiles can fail.
/// Mirrors `cad_geometry_api::ir::GeometryIrError`'s own restraint
/// exactly: every variant is a static, structural or dimensional check —
/// never a numerical geometric-validity judgment (see module doc comment,
/// "Scope cuts").
#[derive(Debug, Clone, PartialEq)]
pub enum SketchIrError {
    /// An entity id passed to [`Sketch::make_profile`] (or produced
    /// internally by a composite constructor) belongs to a different
    /// [`Sketch`] than the one being built against.
    WrongSketch {
        expected: SketchId,
        found: SketchEntityId,
        span: Span,
    },
    /// A [`Quantity`] operand's dimension did not match what the entity
    /// parameter named by `context` requires.
    DimensionMismatch {
        context: &'static str,
        expected: Dimension,
        found: OperandType,
        span: Span,
    },
    /// A length-dimensioned parameter that must be strictly positive
    /// (a circle/arc radius, a slot width) was zero or negative.
    NonPositiveLength {
        context: &'static str,
        found: f64,
        span: Span,
    },
    /// [`Sketch::make_profile`] was given an empty entity list.
    EmptyProfile { span: Span },
    /// [`Sketch::add_polygon`] was given fewer than two points.
    TooFewPoints {
        minimum: usize,
        found: usize,
        span: Span,
    },
    /// [`Sketch::add_slot`]'s `start` and `end` coincide (or are too
    /// close to normalize a centerline direction from), so no
    /// perpendicular offset can be computed.
    DegenerateSlot { span: Span },
}

impl SketchIrError {
    pub fn code(&self) -> &'static str {
        match self {
            SketchIrError::WrongSketch { .. } => "GEOM-E010",
            SketchIrError::DimensionMismatch { .. } => "GEOM-E011",
            SketchIrError::NonPositiveLength { .. } => "GEOM-E012",
            SketchIrError::EmptyProfile { .. } => "GEOM-E013",
            SketchIrError::TooFewPoints { .. } => "GEOM-E014",
            SketchIrError::DegenerateSlot { .. } => "GEOM-E015",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            SketchIrError::WrongSketch { .. } => "SKETCH_ENTITY_WRONG_SKETCH",
            SketchIrError::DimensionMismatch { .. } => "SKETCH_ENTITY_DIMENSION_MISMATCH",
            SketchIrError::NonPositiveLength { .. } => "SKETCH_ENTITY_NON_POSITIVE_LENGTH",
            SketchIrError::EmptyProfile { .. } => "SKETCH_PROFILE_EMPTY",
            SketchIrError::TooFewPoints { .. } => "SKETCH_POLYGON_TOO_FEW_POINTS",
            SketchIrError::DegenerateSlot { .. } => "SKETCH_SLOT_DEGENERATE_CENTERLINE",
        }
    }

    pub fn message(&self) -> String {
        match self {
            SketchIrError::WrongSketch {
                expected, found, ..
            } => format!(
                "{found} belongs to a different sketch than {expected}, and cannot be used in \
                 one of its profiles"
            ),
            SketchIrError::DimensionMismatch {
                context,
                expected,
                found,
                ..
            } => format!("{context} requires a {expected} quantity, found {found}"),
            SketchIrError::NonPositiveLength { context, found, .. } => {
                format!("{context} must be strictly positive, found {found}")
            }
            SketchIrError::EmptyProfile { .. } => {
                "a profile requires at least one entity, found none".to_string()
            }
            SketchIrError::TooFewPoints { minimum, found, .. } => {
                format!("a polygon requires at least {minimum} points, found {found}")
            }
            SketchIrError::DegenerateSlot { .. } => {
                "a slot's start and end points must not coincide".to_string()
            }
        }
    }

    pub fn span(&self) -> Span {
        match self {
            SketchIrError::WrongSketch { span, .. }
            | SketchIrError::DimensionMismatch { span, .. }
            | SketchIrError::NonPositiveLength { span, .. }
            | SketchIrError::EmptyProfile { span }
            | SketchIrError::TooFewPoints { span, .. }
            | SketchIrError::DegenerateSlot { span } => *span,
        }
    }

    /// Builds this error's `cad_diagnostics::Diagnostic`, mirroring
    /// `cad_geometry_api::ir::GeometryIrError::to_diagnostic`'s identical
    /// pattern (same `GEOM` family — sketch entities are a geometry-IR-
    /// adjacent domain, and `cad_diagnostics::DIAGNOSTIC_FAMILIES` is a
    /// fixed RFC-0005 taxonomy this task must not silently extend).
    pub fn to_diagnostic(&self, file: &str, source: &str) -> Diagnostic {
        let span = self.span();
        let line_index = LineIndex::new(source);
        let start = line_index.line_column(source, span.start);
        let end = line_index.line_column(source, span.end);
        let code = DiagnosticCode::parse(self.code())
            .expect("SketchIrError::code always returns a well-formed FAMILY-Exxx code");
        Diagnostic::new(
            code,
            Severity::Error,
            "sketch-entity-ir",
            self.title(),
            self.message(),
        )
        .expect("every SketchIrError code carries the 'E' severity letter")
        .with_source(SourceSpan {
            file: file.to_string(),
            start: Position::new(start.line, start.column),
            end: Position::new(end.line, end.column),
        })
    }
}

impl fmt::Display for SketchIrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for SketchIrError {}

// ---------------------------------------------------------------------
// Sketch
// ---------------------------------------------------------------------

/// An explicit, kernel-independent semantic sketch (`DL-19`): a plane
/// plus an append-only, ordered list of entities it alone owns. See
/// module doc comment for the identity/scope-cut rationale.
#[derive(Debug, Clone, PartialEq)]
pub struct Sketch {
    id: SketchId,
    plane: SketchPlane,
    entities: Vec<SketchEntity>,
}

impl Sketch {
    pub fn new(id: SketchId, plane: SketchPlane) -> Sketch {
        Sketch {
            id,
            plane,
            entities: Vec::new(),
        }
    }

    pub fn id(&self) -> SketchId {
        self.id
    }

    pub fn plane(&self) -> SketchPlane {
        self.plane
    }

    pub fn entities(&self) -> &[SketchEntity] {
        &self.entities
    }

    pub fn get(&self, id: SketchEntityId) -> Option<&SketchEntity> {
        if id.sketch != self.id {
            return None;
        }
        self.entities.get(id.index as usize)
    }

    fn next_entity_id(&self) -> SketchEntityId {
        SketchEntityId {
            sketch: self.id,
            index: self.entities.len() as u32,
        }
    }

    fn push_entity(
        &mut self,
        kind: SketchEntityKind,
        construction: bool,
        span: Span,
    ) -> SketchEntityId {
        let id = self.next_entity_id();
        self.entities.push(SketchEntity {
            id,
            kind,
            construction,
            span,
        });
        id
    }

    fn check_dimension(
        quantity: &Quantity,
        expected: Dimension,
        context: &'static str,
        span: Span,
    ) -> Result<(), SketchIrError> {
        match quantity.ty {
            OperandType::Dimensional { dimension, .. } if dimension == expected => Ok(()),
            other => Err(SketchIrError::DimensionMismatch {
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
    ) -> Result<(), SketchIrError> {
        Self::check_dimension(quantity, Dimension::Length, context, span)?;
        if quantity.magnitude <= 0.0 {
            return Err(SketchIrError::NonPositiveLength {
                context,
                found: quantity.magnitude,
                span,
            });
        }
        Ok(())
    }

    /// Adds a `line` entity (`docs/plan/04_HIGH_LEVEL_MODELING_API.md`
    /// §3's `line`). Two coordinates can never fail a structural check,
    /// so unlike the other entity constructors this one is infallible.
    pub fn add_line(
        &mut self,
        start: Point2,
        end: Point2,
        construction: bool,
        span: Span,
    ) -> SketchEntityId {
        self.push_entity(SketchEntityKind::Line { start, end }, construction, span)
    }

    /// Adds a `circle` entity (`docs/plan/04_HIGH_LEVEL_MODELING_API.md`
    /// §3's `circle`). `radius` must be a strictly positive `Length`.
    pub fn add_circle(
        &mut self,
        center: Point2,
        radius: Quantity,
        construction: bool,
        span: Span,
    ) -> Result<SketchEntityId, SketchIrError> {
        Self::check_positive_length(&radius, "circle.radius", span)?;
        Ok(self.push_entity(
            SketchEntityKind::Circle { center, radius },
            construction,
            span,
        ))
    }

    /// Adds an `arc` entity (`docs/plan/04_HIGH_LEVEL_MODELING_API.md`
    /// §3's `arc`). `radius` must be a strictly positive `Length`;
    /// `start_angle`/`end_angle` must be `Angle`.
    #[allow(clippy::too_many_arguments)]
    pub fn add_arc(
        &mut self,
        center: Point2,
        radius: Quantity,
        start_angle: Quantity,
        end_angle: Quantity,
        direction: RotationDirection,
        construction: bool,
        span: Span,
    ) -> Result<SketchEntityId, SketchIrError> {
        Self::check_positive_length(&radius, "arc.radius", span)?;
        Self::check_dimension(&start_angle, Dimension::Angle, "arc.start_angle", span)?;
        Self::check_dimension(&end_angle, Dimension::Angle, "arc.end_angle", span)?;
        Ok(self.push_entity(
            SketchEntityKind::Arc {
                center,
                radius,
                start_angle,
                end_angle,
                direction,
            },
            construction,
            span,
        ))
    }

    /// Builds a [`Profile`] from entities this sketch already owns.
    /// Every id must belong to `self`, and the list must be non-empty.
    pub fn make_profile(
        &self,
        entities: Vec<SketchEntityId>,
        span: Span,
    ) -> Result<Profile, SketchIrError> {
        if entities.is_empty() {
            return Err(SketchIrError::EmptyProfile { span });
        }
        for &entity in &entities {
            if entity.sketch != self.id {
                return Err(SketchIrError::WrongSketch {
                    expected: self.id,
                    found: entity,
                    span,
                });
            }
        }
        Ok(Profile {
            sketch: self.id,
            entities,
            span,
        })
    }

    /// Adds a `rectangle` profile (`docs/plan/
    /// 04_HIGH_LEVEL_MODELING_API.md` §3's `rectangle`): four `line`
    /// entities forming a closed loop, in `size`/`center`/`rotation`
    /// order. `size` (width, height) must both be strictly positive
    /// `Length`s. See module doc comment ("Scope cuts") for why
    /// `corner_radius` is not implemented.
    pub fn add_rectangle(
        &mut self,
        size: (Quantity, Quantity),
        center: Point2,
        rotation: Quantity,
        span: Span,
    ) -> Result<Profile, SketchIrError> {
        let (width, height) = size;
        Self::check_positive_length(&width, "rectangle.size.x", span)?;
        Self::check_positive_length(&height, "rectangle.size.y", span)?;
        Self::check_dimension(&rotation, Dimension::Angle, "rectangle.rotation", span)?;

        let hw = width.magnitude / 2.0;
        let hh = height.magnitude / 2.0;
        let (sin, cos) = rotation.magnitude.sin_cos();
        let rotate = |x: f64, y: f64| -> Point2 {
            Point2::new(center.x + x * cos - y * sin, center.y + x * sin + y * cos)
        };
        let corners = [
            rotate(-hw, -hh),
            rotate(hw, -hh),
            rotate(hw, hh),
            rotate(-hw, hh),
        ];

        let mut ids = Vec::with_capacity(4);
        for i in 0..4 {
            let start = corners[i];
            let end = corners[(i + 1) % 4];
            ids.push(self.add_line(start, end, false, span));
        }
        self.make_profile(ids, span)
    }

    /// Adds a `polygon` profile (`docs/plan/
    /// 04_HIGH_LEVEL_MODELING_API.md` §3's `polygon`): one `line` entity
    /// per consecutive point pair, closing the loop back to the first
    /// point when `closed` is `true`. Requires at least two points.
    pub fn add_polygon(
        &mut self,
        points: &[Point2],
        closed: bool,
        span: Span,
    ) -> Result<Profile, SketchIrError> {
        if points.len() < 2 {
            return Err(SketchIrError::TooFewPoints {
                minimum: 2,
                found: points.len(),
                span,
            });
        }
        let segment_count = if closed {
            points.len()
        } else {
            points.len() - 1
        };
        let mut ids = Vec::with_capacity(segment_count);
        for i in 0..segment_count {
            let start = points[i];
            let end = points[(i + 1) % points.len()];
            ids.push(self.add_line(start, end, false, span));
        }
        self.make_profile(ids, span)
    }

    /// Adds a `slot` profile (`docs/plan/04_HIGH_LEVEL_MODELING_API.md`
    /// §3's `slot`): a stadium shape — two straight edges parallel to
    /// the `start`/`end` centerline, offset by `width / 2`, capped by a
    /// semicircular `arc` at each end. `width` must be a strictly
    /// positive `Length`; `start`/`end` must not coincide.
    pub fn add_slot(
        &mut self,
        start: Point2,
        end: Point2,
        width: Quantity,
        span: Span,
    ) -> Result<Profile, SketchIrError> {
        Self::check_positive_length(&width, "slot.width", span)?;
        let along = (end - start)
            .normalize()
            .ok_or(SketchIrError::DegenerateSlot { span })?;
        let half_width = width.magnitude / 2.0;
        let offset = along.perp().as_vector2().scale(half_width);

        let p1 = start.translate(offset);
        let p2 = end.translate(offset);
        let p3 = end.translate(offset.scale(-1.0));
        let p4 = start.translate(offset.scale(-1.0));

        let radius = Quantity::of(half_width, Dimension::Length);
        // Both caps sweep exactly PI radians, bulging away from the slot
        // body — see this module's own derivation in its implementation
        // report (`project/reports/AICAD-072.md`). That derivation's own
        // arithmetic is correct, but its direction was not: `start_angle`
        // -> `end_angle` here is a +PI increase, and *increasing* angle is
        // `Direction2::perp`'s own established counter-clockwise
        // convention (`+90 degrees (counter-clockwise)`) — so traversing
        // this specific pair of angles counter-clockwise sweeps through
        // the angle *between* them (e.g. `end_cap_start_angle + PI/2`,
        // which points back toward the slot body's own centerline), not
        // away from it. `AICAD-075`'s own geometry-backed lowering tests
        // caught this: the resulting profile had exactly `2 * (half_width
        // circle area)` less area than the correct stadium, i.e. each cap
        // was cut *into* the body instead of extending past it.
        // `Clockwise` (decreasing angle) sweeps the correct, outward-
        // bulging PI/2 half of the circle instead.
        let end_cap_start_angle = along.angle() + std::f64::consts::FRAC_PI_2;
        let start_cap_start_angle = end_cap_start_angle + PI;

        let top = self.add_line(p1, p2, false, span);
        let end_cap = self.add_arc(
            end,
            radius,
            Quantity::of(end_cap_start_angle, Dimension::Angle),
            Quantity::of(end_cap_start_angle + PI, Dimension::Angle),
            RotationDirection::Clockwise,
            false,
            span,
        )?;
        let bottom = self.add_line(p3, p4, false, span);
        let start_cap = self.add_arc(
            start,
            radius,
            Quantity::of(start_cap_start_angle, Dimension::Angle),
            Quantity::of(start_cap_start_angle + PI, Dimension::Angle),
            RotationDirection::Clockwise,
            false,
            span,
        )?;

        self.make_profile(vec![top, end_cap, bottom, start_cap], span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn line_entities_get_sequential_ids_scoped_to_their_sketch() {
        let mut sketch = Sketch::new(SketchId::new(3), SketchPlane::WorldXy);
        let a = sketch.add_line(Point2::ORIGIN, Point2::new(1.0, 0.0), false, span());
        let b = sketch.add_line(Point2::new(1.0, 0.0), Point2::new(1.0, 1.0), false, span());
        assert_eq!(a.sketch(), SketchId::new(3));
        assert_eq!(a.index(), 0);
        assert_eq!(b.index(), 1);
        assert_eq!(sketch.entities().len(), 2);
    }

    #[test]
    fn two_sketches_never_share_entity_id_space() {
        let mut sketch_a = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let mut sketch_b = Sketch::new(SketchId::new(1), SketchPlane::WorldXy);
        let a0 = sketch_a.add_line(Point2::ORIGIN, Point2::new(1.0, 0.0), false, span());
        let b0 = sketch_b.add_line(Point2::ORIGIN, Point2::new(1.0, 0.0), false, span());
        assert_eq!(a0.index(), b0.index());
        assert_ne!(a0.sketch(), b0.sketch());
        assert!(sketch_a.get(a0).is_some());
        assert!(sketch_a.get(b0).is_none());
    }

    #[test]
    fn circle_accepts_a_positive_radius() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let id = sketch
            .add_circle(Point2::ORIGIN, length(2.0), false, span())
            .unwrap();
        match sketch.get(id).unwrap().kind {
            SketchEntityKind::Circle { radius, .. } => assert_eq!(radius.magnitude, 2.0),
            other => panic!("expected Circle, got {other:?}"),
        }
    }

    #[test]
    fn circle_rejects_a_zero_radius() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let err = sketch
            .add_circle(Point2::ORIGIN, length(0.0), false, span())
            .unwrap_err();
        assert!(matches!(err, SketchIrError::NonPositiveLength { .. }));
    }

    #[test]
    fn circle_rejects_a_negative_radius() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let err = sketch
            .add_circle(Point2::ORIGIN, length(-1.0), false, span())
            .unwrap_err();
        assert!(matches!(err, SketchIrError::NonPositiveLength { .. }));
    }

    #[test]
    fn circle_rejects_an_angle_where_a_length_radius_is_required() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let err = sketch
            .add_circle(Point2::ORIGIN, angle(1.0), false, span())
            .unwrap_err();
        match err {
            SketchIrError::DimensionMismatch {
                context, expected, ..
            } => {
                assert_eq!(context, "circle.radius");
                assert_eq!(expected, Dimension::Length);
            }
            other => panic!("expected DimensionMismatch, got {other:?}"),
        }
    }

    #[test]
    fn arc_requires_angle_dimensioned_start_and_end_angles() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let err = sketch
            .add_arc(
                Point2::ORIGIN,
                length(1.0),
                length(0.0), // wrong: should be Angle
                angle(PI),
                RotationDirection::CounterClockwise,
                false,
                span(),
            )
            .unwrap_err();
        match err {
            SketchIrError::DimensionMismatch {
                context, expected, ..
            } => {
                assert_eq!(context, "arc.start_angle");
                assert_eq!(expected, Dimension::Angle);
            }
            other => panic!("expected DimensionMismatch, got {other:?}"),
        }
    }

    #[test]
    fn arc_entity_records_its_own_fields() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let id = sketch
            .add_arc(
                Point2::ORIGIN,
                length(1.0),
                angle(0.0),
                angle(PI),
                RotationDirection::Clockwise,
                true,
                span(),
            )
            .unwrap();
        let entity = sketch.get(id).unwrap();
        assert!(entity.construction);
        match entity.kind {
            SketchEntityKind::Arc {
                direction,
                start_angle,
                end_angle,
                ..
            } => {
                assert_eq!(direction, RotationDirection::Clockwise);
                assert_eq!(start_angle.magnitude, 0.0);
                assert_eq!(end_angle.magnitude, PI);
            }
            other => panic!("expected Arc, got {other:?}"),
        }
    }

    #[test]
    fn make_profile_rejects_an_empty_entity_list() {
        let sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let err = sketch.make_profile(vec![], span()).unwrap_err();
        assert!(matches!(err, SketchIrError::EmptyProfile { .. }));
    }

    #[test]
    fn make_profile_rejects_an_entity_from_a_different_sketch() {
        let mut sketch_a = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let sketch_b = Sketch::new(SketchId::new(1), SketchPlane::WorldXy);
        let foreign = sketch_a.add_line(Point2::ORIGIN, Point2::new(1.0, 0.0), false, span());
        let err = sketch_b.make_profile(vec![foreign], span()).unwrap_err();
        assert!(matches!(err, SketchIrError::WrongSketch { .. }));
    }

    #[test]
    fn rectangle_produces_four_closed_lines_of_the_right_size() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let profile = sketch
            .add_rectangle(
                (length(4.0), length(2.0)),
                Point2::ORIGIN,
                angle(0.0),
                span(),
            )
            .unwrap();
        assert_eq!(profile.entities.len(), 4);
        assert_eq!(profile.sketch, SketchId::new(0));

        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for &id in &profile.entities {
            if let SketchEntityKind::Line { start, end } = sketch.get(id).unwrap().kind {
                for p in [start, end] {
                    min_x = min_x.min(p.x);
                    max_x = max_x.max(p.x);
                    min_y = min_y.min(p.y);
                    max_y = max_y.max(p.y);
                }
            } else {
                panic!("rectangle must only produce Line entities");
            }
        }
        assert!((max_x - min_x - 4.0).abs() < 1e-9);
        assert!((max_y - min_y - 2.0).abs() < 1e-9);
    }

    #[test]
    fn rectangle_rejects_a_non_positive_size() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let err = sketch
            .add_rectangle(
                (length(0.0), length(2.0)),
                Point2::ORIGIN,
                angle(0.0),
                span(),
            )
            .unwrap_err();
        assert!(matches!(err, SketchIrError::NonPositiveLength { .. }));
    }

    #[test]
    fn rotated_rectangle_corners_rotate_rigidly() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let profile = sketch
            .add_rectangle(
                (length(2.0), length(2.0)),
                Point2::ORIGIN,
                angle(std::f64::consts::FRAC_PI_2),
                span(),
            )
            .unwrap();
        // A square rotated 90 degrees about its own center has the same
        // bounding box as the unrotated square (side 2, centered on 0).
        for &id in &profile.entities {
            if let SketchEntityKind::Line { start, end } = sketch.get(id).unwrap().kind {
                for p in [start, end] {
                    assert!(p.x.abs() < 1.0 + 1e-9);
                    assert!(p.y.abs() < 1.0 + 1e-9);
                }
            }
        }
    }

    #[test]
    fn polygon_closed_wraps_back_to_the_first_point() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let points = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ];
        let profile = sketch.add_polygon(&points, true, span()).unwrap();
        assert_eq!(profile.entities.len(), 3);
        let last = sketch.get(profile.entities[2]).unwrap();
        if let SketchEntityKind::Line { end, .. } = last.kind {
            assert_eq!(end, points[0]);
        } else {
            panic!("expected Line");
        }
    }

    #[test]
    fn polygon_open_does_not_wrap() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let points = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ];
        let profile = sketch.add_polygon(&points, false, span()).unwrap();
        assert_eq!(profile.entities.len(), 2);
    }

    #[test]
    fn polygon_rejects_fewer_than_two_points() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let points = [Point2::ORIGIN];
        let err = sketch.add_polygon(&points, true, span()).unwrap_err();
        assert!(matches!(err, SketchIrError::TooFewPoints { .. }));
    }

    #[test]
    fn slot_produces_two_lines_and_two_semicircular_caps() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let profile = sketch
            .add_slot(Point2::ORIGIN, Point2::new(10.0, 0.0), length(4.0), span())
            .unwrap();
        assert_eq!(profile.entities.len(), 4);

        let mut line_count = 0;
        let mut arc_count = 0;
        for &id in &profile.entities {
            match sketch.get(id).unwrap().kind {
                SketchEntityKind::Line { start, end } => {
                    line_count += 1;
                    // Each straight edge is offset by half the width (2.0)
                    // from the centerline and has the same length as it.
                    assert!((start.y.abs() - 2.0).abs() < 1e-9);
                    assert!((end.y.abs() - 2.0).abs() < 1e-9);
                    assert!(((end.x - start.x).abs() - 10.0).abs() < 1e-9);
                }
                SketchEntityKind::Arc {
                    center,
                    radius,
                    start_angle,
                    end_angle,
                    direction,
                } => {
                    arc_count += 1;
                    assert_eq!(radius.magnitude, 2.0);
                    assert!((end_angle.magnitude - start_angle.magnitude - PI).abs() < 1e-9);
                    assert_eq!(direction, RotationDirection::Clockwise);
                    // The cap must bulge AWAY from the slot body (past
                    // `center.x = 0` or `center.x = 10`, the slot's own
                    // endpoints), never back into it — the exact bug this
                    // regression test is for (see `add_slot`'s own doc
                    // comment on its `Clockwise` choice).
                    // The arithmetic mean of the two stored angles is
                    // direction-independent and cannot distinguish which
                    // of the two possible semicircles is meant — walk
                    // half the actual signed sweep instead (mirrors
                    // `cad_geometry_runtime::sketch_lowering::arc_mid_angle`'s
                    // own direction-aware calculation).
                    let tau = std::f64::consts::TAU;
                    let raw_delta = end_angle.magnitude - start_angle.magnitude;
                    let half_sweep = match direction {
                        RotationDirection::CounterClockwise => raw_delta.rem_euclid(tau) / 2.0,
                        RotationDirection::Clockwise => -(-raw_delta).rem_euclid(tau) / 2.0,
                    };
                    let mid_angle = start_angle.magnitude + half_sweep;
                    let bulge_x = center.x + radius.magnitude * mid_angle.cos();
                    let bulge_y = center.y + radius.magnitude * mid_angle.sin();
                    assert!(bulge_y.abs() < 1e-9, "cap must bulge along the centerline");
                    // The slot's centerline spans x in [0, 10]; each cap's
                    // own center sits at one end of it (x=0 or x=10) and
                    // must bulge further away from the other end, not
                    // back toward it.
                    if center.x > 5.0 {
                        assert!(
                            bulge_x > center.x,
                            "end cap (center.x={}) must bulge to x > {} (outward), got {bulge_x}",
                            center.x,
                            center.x
                        );
                    } else {
                        assert!(
                            bulge_x < center.x,
                            "start cap (center.x={}) must bulge to x < {} (outward), got {bulge_x}",
                            center.x,
                            center.x
                        );
                    }
                }
                other => panic!("slot must only produce Line/Arc entities, got {other:?}"),
            }
        }
        assert_eq!(line_count, 2);
        assert_eq!(arc_count, 2);
    }

    #[test]
    fn slot_rejects_a_degenerate_centerline() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let err = sketch
            .add_slot(Point2::ORIGIN, Point2::ORIGIN, length(1.0), span())
            .unwrap_err();
        assert!(matches!(err, SketchIrError::DegenerateSlot { .. }));
    }

    #[test]
    fn slot_rejects_a_non_positive_width() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let err = sketch
            .add_slot(Point2::ORIGIN, Point2::new(1.0, 0.0), length(0.0), span())
            .unwrap_err();
        assert!(matches!(err, SketchIrError::NonPositiveLength { .. }));
    }

    #[test]
    fn every_error_variant_converts_to_a_well_formed_diagnostic() {
        let sketch_id = SketchId::new(0);
        let errors = vec![
            SketchIrError::WrongSketch {
                expected: sketch_id,
                found: SketchEntityId {
                    sketch: SketchId::new(1),
                    index: 0,
                },
                span: span(),
            },
            SketchIrError::DimensionMismatch {
                context: "circle.radius",
                expected: Dimension::Length,
                found: OperandType::dimensional(Dimension::Angle, None),
                span: span(),
            },
            SketchIrError::NonPositiveLength {
                context: "circle.radius",
                found: -1.0,
                span: span(),
            },
            SketchIrError::EmptyProfile { span: span() },
            SketchIrError::TooFewPoints {
                minimum: 2,
                found: 1,
                span: span(),
            },
            SketchIrError::DegenerateSlot { span: span() },
        ];
        for err in errors {
            let code = err.code();
            assert!(code.starts_with("GEOM-E"));
            DiagnosticCode::parse(code).expect("every SketchIrError code must be well-formed");
            let diagnostic = err.to_diagnostic("test.aicad", "x");
            assert_eq!(diagnostic.category, "sketch-entity-ir");
            assert!(!diagnostic.message.is_empty());
            assert!(!err.to_string().is_empty());
        }
    }

    #[test]
    fn sketch_id_and_entity_id_display_are_stable() {
        assert_eq!(SketchId::new(2).to_string(), "sketch#2");
        let entity = SketchEntityId {
            sketch: SketchId::new(2),
            index: 5,
        };
        assert_eq!(entity.to_string(), "sketch#2/entity#5");
    }

    #[test]
    fn point2_and_vector2_basic_arithmetic() {
        let a = Point2::new(1.0, 2.0);
        let b = Point2::new(4.0, 6.0);
        let v = b - a;
        assert_eq!(v, Vector2::new(3.0, 4.0));
        assert_eq!(v.length(), 5.0);
        assert_eq!(a.translate(v), b);
        assert!(Vector2::ZERO.normalize().is_none());
        let dir = v.normalize().unwrap();
        assert!((dir.as_vector2().length() - 1.0).abs() < 1e-9);
        let perp = dir.perp();
        // Perpendicular to (3,4)/5 is (-4,3)/5.
        assert!((perp.as_vector2().x - (-4.0 / 5.0)).abs() < 1e-9);
        assert!((perp.as_vector2().y - (3.0 / 5.0)).abs() < 1e-9);
    }

    #[test]
    fn quantity_of_reports_its_dimension() {
        assert_eq!(length(3.0).dimension(), Some(Dimension::Length));
        assert_eq!(angle(3.0).dimension(), Some(Dimension::Angle));
    }
}
