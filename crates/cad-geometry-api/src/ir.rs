//! Geometry IR (`AICAD-059`) — the backend-independent representation
//! sitting between the (not-yet-built, Stage-3+) Feature IR/dependency DAG
//! and the Stage-1 kernel-neutral API (`cad-kernel-api`), per
//! `docs/plan/01_SYSTEM_ARCHITECTURE.md` §5's IR-layering diagram:
//!
//! ```text
//! ... Feature IR / dependency DAG
//!        |
//!     Geometry IR         <-- this module
//!        |  (curves/surfaces/topology operations)
//!     Kernel call graph
//!        |
//!      B-rep
//! ```
//!
//! # Backend independence (`AGENTS.md` non-negotiable)
//!
//! No OCCT type, enumeration assumption, or persistent OCCT topology
//! identity appears anywhere in this module. [`GeometryOp`]'s operand
//! types are exclusively: [`GeomId`] (this module's own SSA-style node
//! index, scoped to one [`GeometryGraph`] and entirely independent of any
//! kernel context or shape table), [`Quantity`] (a typed engineering
//! quantity, never a bare `f64`), and the pure-math primitives already
//! declared kernel-neutral by `cad-kernel-api` itself
//! ([`Point3`]/[`Vector3`]/[`Direction3`]/[`Axis3`]/[`Transform`] — see
//! that crate's own module doc comment: "Pure-Rust, backend-independent
//! value types... no OCCT... type or enum value appears anywhere in this
//! crate"). This module never imports `cad_kernel_api::KernelId` or any
//! `Kernel*` handle/error type: those are epoch-bound kernel-adapter
//! vocabulary (`project/DECISION_LOG.md#DL-5`), not Geometry IR vocabulary
//! — dispatching a [`GeometryGraph`] into actual `KernelShape` values via
//! `cad-kernel-api`/`cad-occt-bridge` is `AICAD-060`'s job, a distinct
//! architectural boundary this task deliberately does not cross (per the
//! active campaign brief: "Geometry IR must remain backend-independent...
//! Do not bypass Geometry IR or invoke the native bridge directly").
//!
//! # Functional/SSA shape (DL-2)
//!
//! A [`GeometryGraph`] is an append-only sequence of [`GeometryNode`]s.
//! Each node is assigned the next sequential [`GeomId`] when pushed and
//! may only reference **earlier** ids as operands — there is no way to
//! construct a cycle or a forward reference, matching `project/
//! DECISION_LOG.md#DL-2`'s functional/value-oriented core (an operation
//! consumes existing values and produces one new value; nothing is
//! mutated in place).
//!
//! # Typed quantities, not raw floats (`AGENTS.md` non-negotiable)
//!
//! Every dimensioned operation parameter (a box's `dx`, an extrusion
//! distance, a revolve angle, a fillet radius, ...) is a [`Quantity`]
//! (magnitude plus `cad_units::OperandType`), and [`GeometryGraph::push_op`]
//! rejects a quantity whose dimension does not match what the operation
//! slot requires ([`GeometryIrError::DimensionMismatch`]) rather than
//! silently accepting or coercing it.
//!
//! # Raw index-based topology selection (known, documented limitation)
//!
//! `Fillet`/`Chamfer`/`Shell` select the edges/faces they act on by
//! `usize` index into the target's own current topology
//! ([`EdgeIndex`]/[`FaceIndex`]), mirroring the *only* selection mechanism
//! `cad-occt-bridge`'s Stage-1 API actually exposes today
//! (`Shape::get_edge`/`Shape::get_face`, confirmed by
//! `crates/cad-occt-bridge/tests/stage1_bracket.rs`'s own `find_edge`
//! helper). This is raw, ephemeral, kernel-enumeration-order topology
//! selection — explicitly *not* a durable AICAD semantic reference
//! (`AGENTS.md`: "Raw topology is ephemeral/unsafe and epoch-bound";
//! `project/DECISION_LOG.md#DL-9`: "the semantic-reference layer owns
//! durable identity above the kernel"). The semantic-reference layer
//! (Stage 4, RFC-0003) is expected to supersede this with stable
//! identity-preserving selection; introducing that layer now would be
//! exactly the speculative future work `AGENTS.md`'s "No speculative
//! future work" rule forbids for a Stage-2 task.
//!
//! # Scope cut: no low-level topology-exploration ops
//!
//! This IR covers exactly the domain-level modeling/query operations
//! `cad-occt-bridge` exposes that `AICAD-063`'s end-to-end proof (parameters,
//! units, derived expressions, functions, control flow, geometry
//! operations, B-rep validity/bounds/dimensions/volume/center-of-mass/
//! solid-count/topology-sanity/STEP verification) actually needs: solid
//! primitives, curve/wire/face construction, the high-level feature
//! operations (extrude/revolve/sweep/loft), booleans, the dress-up
//! features (fillet/chamfer/shell/offset), rigid transform, STEP
//! import/export, and the property/validation queries. Low-level raw
//! topology *exploration* (`edge_vertices`, `edge_adjacent_face`,
//! `get_vertex`, ...) is `docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`
//! territory this task does not need and does not add, per "No
//! speculative future work".

use cad_ast::Span;
use cad_diagnostics::{Diagnostic, DiagnosticCode, Position, Severity, SourceSpan};
use cad_kernel_api::{Axis3, Direction3, Point3, Transform};
use cad_types::Dimension;
use cad_units::OperandType;
use std::fmt;
use std::path::PathBuf;

/// SSA-style identity of one node within a single [`GeometryGraph`].
///
/// Deliberately *not* [`cad_kernel_api::KernelId`]: a `GeomId` names a
/// position in this backend-independent graph, minted by the graph itself
/// in strictly increasing order as nodes are pushed, with no kernel
/// context, shape-table slot, or generation involved at all. A `GeomId`
/// from one [`GeometryGraph`] is meaningless (and rejected, see
/// [`GeometryIrError::InvalidOperand`]) against a different graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GeomId(u32);

impl GeomId {
    /// Exposes the raw index, for callers (diagnostics, `AICAD-060`'s
    /// future dispatcher) that need to display or externally index by it.
    /// Not constructible from an arbitrary `u32` outside this crate —
    /// only [`GeometryGraph`] mints ids, by design (mirrors
    /// `cad_kernel_api`'s own "everything above the kernel adapter
    /// receives handles, it does not mint them" rule, applied here to the
    /// IR-graph layer instead of the kernel layer).
    pub fn index(self) -> u32 {
        self.0
    }
}

impl fmt::Display for GeomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.0)
    }
}

/// A typed engineering quantity used as a Geometry IR operation parameter
/// — never a bare `f64` (`AGENTS.md`: "Units are typed engineering
/// quantities, not untyped floats"). `magnitude` is expressed in
/// whatever canonical internal representation `cad-units` defines for
/// `ty`'s dimension (`project/DECISION_LOG.md#DL-3`); converting that into
/// whatever raw numeric convention the kernel adapter's own `f64`
/// parameters expect is `AICAD-060`'s dispatch-time concern, not this
/// module's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantity {
    pub magnitude: f64,
    pub ty: OperandType,
}

impl Quantity {
    pub fn new(magnitude: f64, ty: OperandType) -> Quantity {
        Quantity { magnitude, ty }
    }

    /// Convenience constructor for the common non-affine-dimension case
    /// (every dimension a geometry operation parameter uses today —
    /// `Length`, `Angle` — is non-affine; only `Temperature` is affine
    /// per `project/DECISION_LOG.md#DL-3`, and no geometry operation
    /// parameter is temperature-dimensioned).
    ///
    /// # Panics
    /// Panics (via `OperandType::dimensional`'s own assertion) if
    /// `dimension.is_affine()` — callers needing an affine dimension must
    /// build the `OperandType` themselves and use [`Quantity::new`].
    pub fn of(magnitude: f64, dimension: Dimension) -> Quantity {
        Quantity {
            magnitude,
            ty: OperandType::dimensional(dimension, None),
        }
    }

    /// This quantity's dimension, if it carries one (always `Some` for
    /// every constructor above; `None` is only reachable if a caller
    /// builds a `Quantity` directly from an `OperandType::Scalar`, which
    /// every Geometry IR operation slot rejects via
    /// [`GeometryIrError::DimensionMismatch`]).
    pub fn dimension(&self) -> Option<Dimension> {
        match self.ty {
            OperandType::Dimensional { dimension, .. } => Some(dimension),
            OperandType::Scalar(_) => None,
        }
    }
}

/// A raw, epoch-bound edge selector: the index `Shape::get_edge` would
/// need to retrieve this edge from its owning node's realized shape, in
/// whatever enumeration order the kernel backend currently produces. See
/// this module's doc comment, "Raw index-based topology selection".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeIndex(pub usize);

/// A raw, epoch-bound face selector, analogous to [`EdgeIndex`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FaceIndex(pub usize);

/// One Geometry IR construction operation: consumes zero or more prior
/// [`GeomId`]s (functional/SSA, `DL-2`) plus typed parameters, and
/// produces one new geometry value. Mirrors exactly the domain-level
/// capability set `cad-occt-bridge::OcctContext`/`Shape` already expose
/// (`project/DECISION_LOG.md#DL-5`'s capability-driven minimal surface) —
/// this IR adds no new capability the Stage-1 kernel adapter cannot
/// perform, only a backend-independent way to describe *which* sequence
/// of those capabilities a program's geometry expressions request.
#[derive(Debug, Clone, PartialEq)]
pub enum GeometryOp {
    /// An axis-aligned box (`OcctContext::create_box`).
    Box {
        dx: Quantity,
        dy: Quantity,
        dz: Quantity,
    },
    /// A capped cylinder centered on the origin, axis along +Z
    /// (`OcctContext::create_cylinder`).
    Cylinder { radius: Quantity, height: Quantity },
    /// Imports a shape from a STEP file (`OcctContext::import_step`).
    ImportStep { path: PathBuf },
    /// A single straight edge between two points
    /// (`OcctContext::make_line_edge`).
    LineEdge { start: Point3, end: Point3 },
    /// A closed circular wire (`OcctContext::make_circle_wire`).
    CircleWire {
        center: Point3,
        normal: Direction3,
        radius: Quantity,
    },
    /// A circular-arc edge passing through three points, in order `start
    /// -> mid -> end` (`OcctContext::make_arc_edge`, `AICAD-075`). `mid`
    /// must lie strictly between the other two along the intended arc —
    /// this determines both which of the two possible arcs between
    /// `start`/`end` is built and its traversal direction, with no
    /// separate axis/sense parameter (mirrors `LineEdge`'s own "no
    /// separate handedness flag" style, and fills the gap `CircleWire`'s
    /// always-closed-full-circle contract cannot: a sketch `arc` entity
    /// lowers to a *partial* circle, not a full one).
    ArcEdge {
        start: Point3,
        mid: Point3,
        end: Point3,
    },
    /// Assembles a wire from an ordered list of edges
    /// (`OcctContext::make_wire_from_edges`). `edges` must be non-empty.
    WireFromEdges { edges: Vec<GeomId> },
    /// Builds a planar face bounded by a wire (`Shape::make_face`).
    MakeFace { wire: GeomId },
    /// Selects one face of `target` by raw, epoch-bound
    /// kernel-enumeration-order index (`Shape::get_face`), producing it
    /// as its own new geometry value (`AICAD-076`) -- the only
    /// source-visible way to obtain a profile for `Extrude`/`Revolve`
    /// before source-level sketch/profile construction exists (`cad_hir::
    /// sketch` has no grammar/lowering integration yet), mirroring
    /// `Fillet`/`Chamfer`'s own already-established raw-index selection
    /// precedent. Like every other `FaceIndex`/`EdgeIndex` use in this
    /// module, this is raw and epoch-bound, never a durable semantic
    /// reference (`AGENTS.md`: "Raw topology is ephemeral/unsafe").
    GetFace { target: GeomId, face: FaceIndex },
    /// Extrudes a profile along a direction by a `Length` distance
    /// (`Shape::extrude`).
    Extrude {
        profile: GeomId,
        direction: Direction3,
        distance: Quantity,
    },
    /// Revolves a profile about an axis by an `Angle` (`Shape::revolve`).
    Revolve {
        profile: GeomId,
        axis: Axis3,
        angle: Quantity,
    },
    /// Sweeps a profile along a spine wire (`Shape::sweep`).
    Sweep { profile: GeomId, spine: GeomId },
    /// Lofts a solid/shell through an ordered list of sections
    /// (`OcctContext::loft`). `sections` must be non-empty.
    Loft { sections: Vec<GeomId> },
    /// Boolean union (`Shape::union`).
    Union { lhs: GeomId, rhs: GeomId },
    /// Boolean cut, `lhs - rhs` (`Shape::cut`).
    Cut { lhs: GeomId, rhs: GeomId },
    /// Boolean intersection (`Shape::intersect`).
    Intersect { lhs: GeomId, rhs: GeomId },
    /// Rounds the given edges of `target` by a `Length` radius
    /// (`Shape::fillet`). `edges` must be non-empty.
    Fillet {
        target: GeomId,
        edges: Vec<EdgeIndex>,
        radius: Quantity,
    },
    /// Chamfers the given edges of `target` by a `Length` distance
    /// (`Shape::chamfer`). `edges` must be non-empty.
    Chamfer {
        target: GeomId,
        edges: Vec<EdgeIndex>,
        distance: Quantity,
    },
    /// Hollows `target`, removing the given faces, to a `Length`
    /// thickness (`Shape::shell`). `removed_faces` may be empty (a fully
    /// closed shell) — unlike `Fillet`/`Chamfer`/`WireFromEdges`/`Loft`,
    /// an empty selector list is a legitimate request here, so this
    /// variant is deliberately not subject to
    /// [`GeometryIrError::EmptyOperandList`].
    Shell {
        target: GeomId,
        removed_faces: Vec<FaceIndex>,
        thickness: Quantity,
    },
    /// Offsets every face of `target` by a `Length` distance
    /// (`Shape::offset`).
    Offset { target: GeomId, distance: Quantity },
    /// Applies a rigid transform to `target` (`Shape::transform`).
    Transform {
        target: GeomId,
        transform: Transform,
    },
}

/// A property/validation query against an already-constructed geometry
/// value. Unlike [`GeometryOp`], a query never produces a new geometry
/// value usable as another operation's geometry operand (enforced by
/// [`GeometryGraph::push_op`]/[`GeometryGraph::push_query`] — see
/// [`GeometryIrError::OperandIsNotGeometry`]).
#[derive(Debug, Clone, PartialEq)]
pub enum GeometryQuery {
    /// `Shape::is_valid`.
    IsValid(GeomId),
    /// `Shape::volume`.
    Volume(GeomId),
    /// `Shape::area`.
    Area(GeomId),
    /// `Shape::bounding_box`.
    BoundingBox(GeomId),
    /// `Shape::center_of_mass`.
    CenterOfMass(GeomId),
    /// `Shape::validate` (the full structured validation report, distinct
    /// from the boolean `IsValid`).
    Validate(GeomId),
    /// `Shape::tessellate`, given a `Length` linear deflection tolerance
    /// and an `Angle` angular deflection tolerance (`AICAD-060` finding:
    /// `Shape::tessellate(&self, linear_deflection: f64, angular_deflection:
    /// f64)` takes both; this variant originally carried only the linear
    /// one, an incomplete-parameter gap fixed here rather than carried
    /// forward, per `AGENTS.md`'s "when a bug is found... fix the root
    /// cause").
    Tessellate {
        target: GeomId,
        linear_deflection: Quantity,
        angular_deflection: Quantity,
    },
    /// `Shape::export_step`.
    ExportStep { target: GeomId, path: PathBuf },
}

/// Distinguishes a node that produces a new geometry value (usable as a
/// later node's operand) from one that only reports a property of an
/// existing value.
#[derive(Debug, Clone, PartialEq)]
pub enum GeometryNodeKind {
    Construct(GeometryOp),
    Query(GeometryQuery),
}

impl GeometryNodeKind {
    /// Whether a node of this kind may be referenced as another node's
    /// geometry operand. Only [`GeometryNodeKind::Construct`] nodes can —
    /// a query's result (a bool/number/report/tessellation/side-effecting
    /// export) is never itself a geometry value.
    pub fn produces_geometry(&self) -> bool {
        matches!(self, GeometryNodeKind::Construct(_))
    }
}

/// One node of a [`GeometryGraph`]: an id, the source span it was built
/// from (threaded through for diagnostics, matching every other Stage-2
/// IR's own span-fidelity requirement), and its operation.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryNode {
    pub id: GeomId,
    pub span: Span,
    pub kind: GeometryNodeKind,
}

/// Every way constructing a [`GeometryGraph`] node can fail. All are
/// static, structural, backend-independent checks — no kernel call is
/// ever made while building the IR (that is `AICAD-060`'s dispatch-time
/// concern).
#[derive(Debug, Clone, PartialEq)]
pub enum GeometryIrError {
    /// `referenced` does not name any node already present in this graph
    /// (an out-of-range/not-yet-created index, or an id that actually
    /// belongs to a different `GeometryGraph`).
    InvalidOperand { referenced: GeomId, span: Span },
    /// `referenced` names a real node in this graph, but it is a
    /// [`GeometryNodeKind::Query`], which never produces a geometry
    /// value.
    OperandIsNotGeometry { referenced: GeomId, span: Span },
    /// A [`Quantity`] operand's dimension did not match what the
    /// operation slot named by `context` requires.
    DimensionMismatch {
        context: &'static str,
        expected: Dimension,
        found: OperandType,
        span: Span,
    },
    /// An operand list that must be non-empty (`WireFromEdges`'s `edges`,
    /// `Loft`'s `sections`, `Fillet`/`Chamfer`'s `edges`) was empty.
    EmptyOperandList { context: &'static str, span: Span },
}

impl GeometryIrError {
    pub fn code(&self) -> &'static str {
        match self {
            GeometryIrError::InvalidOperand { .. } => "GEOM-E001",
            GeometryIrError::OperandIsNotGeometry { .. } => "GEOM-E002",
            GeometryIrError::DimensionMismatch { .. } => "GEOM-E003",
            GeometryIrError::EmptyOperandList { .. } => "GEOM-E004",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            GeometryIrError::InvalidOperand { .. } => "INVALID_GEOMETRY_OPERAND",
            GeometryIrError::OperandIsNotGeometry { .. } => "OPERAND_IS_NOT_GEOMETRY",
            GeometryIrError::DimensionMismatch { .. } => "GEOMETRY_DIMENSION_MISMATCH",
            GeometryIrError::EmptyOperandList { .. } => "EMPTY_GEOMETRY_OPERAND_LIST",
        }
    }

    pub fn message(&self) -> String {
        match self {
            GeometryIrError::InvalidOperand { referenced, .. } => {
                format!("{referenced} does not name any geometry node already built in this graph")
            }
            GeometryIrError::OperandIsNotGeometry { referenced, .. } => format!(
                "{referenced} is a property query, not a geometry value, and cannot be used as \
                 a geometry operand"
            ),
            GeometryIrError::DimensionMismatch {
                context,
                expected,
                found,
                ..
            } => format!("{context} requires a {expected} quantity, found {found}"),
            GeometryIrError::EmptyOperandList { context, .. } => {
                format!("{context} requires at least one element, found none")
            }
        }
    }

    pub fn span(&self) -> Span {
        match self {
            GeometryIrError::InvalidOperand { span, .. }
            | GeometryIrError::OperandIsNotGeometry { span, .. }
            | GeometryIrError::DimensionMismatch { span, .. }
            | GeometryIrError::EmptyOperandList { span, .. } => *span,
        }
    }

    /// Builds this error's `cad_diagnostics::Diagnostic`, mirroring
    /// `cad_runtime::RuntimeError::to_diagnostic`'s own pattern exactly
    /// (dedicated family, category, always `Error` severity — every
    /// variant here means a `GeometryGraph` could not be built at all).
    pub fn to_diagnostic(&self, file: &str, source: &str) -> Diagnostic {
        let span = self.span();
        let line_index = cad_ast::LineIndex::new(source);
        let start = line_index.line_column(source, span.start);
        let end = line_index.line_column(source, span.end);
        let code = DiagnosticCode::parse(self.code())
            .expect("GeometryIrError::code always returns a well-formed FAMILY-Exxx code");
        Diagnostic::new(
            code,
            Severity::Error,
            "geometry-ir",
            self.title(),
            self.message(),
        )
        .expect("every GeometryIrError code carries the 'E' severity letter")
        .with_source(SourceSpan {
            file: file.to_string(),
            start: Position::new(start.line, start.column),
            end: Position::new(end.line, end.column),
        })
    }
}

impl fmt::Display for GeometryIrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for GeometryIrError {}

/// A backend-independent, functional/SSA geometry program: an append-only
/// sequence of [`GeometryNode`]s. See module doc comment for the full
/// design rationale.
#[derive(Debug, Clone, Default)]
pub struct GeometryGraph {
    nodes: Vec<GeometryNode>,
}

impl GeometryGraph {
    pub fn new() -> GeometryGraph {
        GeometryGraph::default()
    }

    pub fn nodes(&self) -> &[GeometryNode] {
        &self.nodes
    }

    pub fn get(&self, id: GeomId) -> Option<&GeometryNode> {
        self.nodes.get(id.0 as usize)
    }

    fn next_id(&self) -> GeomId {
        GeomId(self.nodes.len() as u32)
    }

    /// Validates that `id` names an earlier node in *this* graph whose
    /// kind produces a geometry value.
    fn check_geometry_operand(&self, id: GeomId, span: Span) -> Result<(), GeometryIrError> {
        match self.get(id) {
            None => Err(GeometryIrError::InvalidOperand {
                referenced: id,
                span,
            }),
            Some(node) if !node.kind.produces_geometry() => {
                Err(GeometryIrError::OperandIsNotGeometry {
                    referenced: id,
                    span,
                })
            }
            Some(_) => Ok(()),
        }
    }

    fn check_geometry_operands<'a>(
        &self,
        ids: impl IntoIterator<Item = &'a GeomId>,
        span: Span,
    ) -> Result<(), GeometryIrError> {
        for &id in ids {
            self.check_geometry_operand(id, span)?;
        }
        Ok(())
    }

    fn check_dimension(
        quantity: &Quantity,
        expected: Dimension,
        context: &'static str,
        span: Span,
    ) -> Result<(), GeometryIrError> {
        match quantity.ty {
            OperandType::Dimensional { dimension, .. } if dimension == expected => Ok(()),
            other => Err(GeometryIrError::DimensionMismatch {
                context,
                expected,
                found: other,
                span,
            }),
        }
    }

    fn check_non_empty<T>(
        items: &[T],
        context: &'static str,
        span: Span,
    ) -> Result<(), GeometryIrError> {
        if items.is_empty() {
            Err(GeometryIrError::EmptyOperandList { context, span })
        } else {
            Ok(())
        }
    }

    /// Validates and appends one [`GeometryOp`], returning the new node's
    /// id. Every operand [`GeomId`] must already exist in this graph and
    /// produce geometry; every [`Quantity`] must carry the dimension its
    /// slot requires; every operand list documented as non-empty must be.
    pub fn push_op(&mut self, op: GeometryOp, span: Span) -> Result<GeomId, GeometryIrError> {
        match &op {
            GeometryOp::Box { dx, dy, dz } => {
                Self::check_dimension(dx, Dimension::Length, "Box.dx", span)?;
                Self::check_dimension(dy, Dimension::Length, "Box.dy", span)?;
                Self::check_dimension(dz, Dimension::Length, "Box.dz", span)?;
            }
            GeometryOp::Cylinder { radius, height } => {
                Self::check_dimension(radius, Dimension::Length, "Cylinder.radius", span)?;
                Self::check_dimension(height, Dimension::Length, "Cylinder.height", span)?;
            }
            GeometryOp::ImportStep { .. } => {}
            GeometryOp::LineEdge { .. } => {}
            GeometryOp::CircleWire { radius, .. } => {
                Self::check_dimension(radius, Dimension::Length, "CircleWire.radius", span)?;
            }
            GeometryOp::ArcEdge { .. } => {}
            GeometryOp::WireFromEdges { edges } => {
                Self::check_non_empty(edges, "WireFromEdges.edges", span)?;
                self.check_geometry_operands(edges, span)?;
            }
            GeometryOp::MakeFace { wire } => {
                self.check_geometry_operand(*wire, span)?;
            }
            GeometryOp::GetFace { target, .. } => {
                self.check_geometry_operand(*target, span)?;
            }
            GeometryOp::Extrude {
                profile, distance, ..
            } => {
                self.check_geometry_operand(*profile, span)?;
                Self::check_dimension(distance, Dimension::Length, "Extrude.distance", span)?;
            }
            GeometryOp::Revolve { profile, angle, .. } => {
                self.check_geometry_operand(*profile, span)?;
                Self::check_dimension(angle, Dimension::Angle, "Revolve.angle", span)?;
            }
            GeometryOp::Sweep { profile, spine } => {
                self.check_geometry_operand(*profile, span)?;
                self.check_geometry_operand(*spine, span)?;
            }
            GeometryOp::Loft { sections } => {
                Self::check_non_empty(sections, "Loft.sections", span)?;
                self.check_geometry_operands(sections, span)?;
            }
            GeometryOp::Union { lhs, rhs }
            | GeometryOp::Cut { lhs, rhs }
            | GeometryOp::Intersect { lhs, rhs } => {
                self.check_geometry_operand(*lhs, span)?;
                self.check_geometry_operand(*rhs, span)?;
            }
            GeometryOp::Fillet {
                target,
                edges,
                radius,
            } => {
                self.check_geometry_operand(*target, span)?;
                Self::check_non_empty(edges, "Fillet.edges", span)?;
                Self::check_dimension(radius, Dimension::Length, "Fillet.radius", span)?;
            }
            GeometryOp::Chamfer {
                target,
                edges,
                distance,
            } => {
                self.check_geometry_operand(*target, span)?;
                Self::check_non_empty(edges, "Chamfer.edges", span)?;
                Self::check_dimension(distance, Dimension::Length, "Chamfer.distance", span)?;
            }
            GeometryOp::Shell {
                target, thickness, ..
            } => {
                self.check_geometry_operand(*target, span)?;
                Self::check_dimension(thickness, Dimension::Length, "Shell.thickness", span)?;
            }
            GeometryOp::Offset { target, distance } => {
                self.check_geometry_operand(*target, span)?;
                Self::check_dimension(distance, Dimension::Length, "Offset.distance", span)?;
            }
            GeometryOp::Transform { target, .. } => {
                self.check_geometry_operand(*target, span)?;
            }
        }
        let id = self.next_id();
        self.nodes.push(GeometryNode {
            id,
            span,
            kind: GeometryNodeKind::Construct(op),
        });
        Ok(id)
    }

    /// Validates and appends one [`GeometryQuery`], returning the new
    /// node's id (queries are still addressable/orderable nodes, even
    /// though nothing may use one as a geometry operand).
    pub fn push_query(
        &mut self,
        query: GeometryQuery,
        span: Span,
    ) -> Result<GeomId, GeometryIrError> {
        match &query {
            GeometryQuery::IsValid(target)
            | GeometryQuery::Volume(target)
            | GeometryQuery::Area(target)
            | GeometryQuery::BoundingBox(target)
            | GeometryQuery::CenterOfMass(target)
            | GeometryQuery::Validate(target) => {
                self.check_geometry_operand(*target, span)?;
            }
            GeometryQuery::Tessellate {
                target,
                linear_deflection,
                angular_deflection,
            } => {
                self.check_geometry_operand(*target, span)?;
                Self::check_dimension(
                    linear_deflection,
                    Dimension::Length,
                    "Tessellate.linear_deflection",
                    span,
                )?;
                Self::check_dimension(
                    angular_deflection,
                    Dimension::Angle,
                    "Tessellate.angular_deflection",
                    span,
                )?;
            }
            GeometryQuery::ExportStep { target, .. } => {
                self.check_geometry_operand(*target, span)?;
            }
        }
        let id = self.next_id();
        self.nodes.push(GeometryNode {
            id,
            span,
            kind: GeometryNodeKind::Query(query),
        });
        Ok(id)
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
    fn push_op_assigns_sequential_ids() {
        let mut graph = GeometryGraph::new();
        let a = graph
            .push_op(
                GeometryOp::Box {
                    dx: length(1.0),
                    dy: length(1.0),
                    dz: length(1.0),
                },
                span(),
            )
            .unwrap();
        let b = graph
            .push_op(
                GeometryOp::Cylinder {
                    radius: length(0.5),
                    height: length(2.0),
                },
                span(),
            )
            .unwrap();
        assert_eq!(a.index(), 0);
        assert_eq!(b.index(), 1);
        assert_eq!(graph.nodes().len(), 2);
    }

    #[test]
    fn boolean_op_references_prior_geometry_nodes() {
        let mut graph = GeometryGraph::new();
        let a = graph
            .push_op(
                GeometryOp::Box {
                    dx: length(1.0),
                    dy: length(1.0),
                    dz: length(1.0),
                },
                span(),
            )
            .unwrap();
        let b = graph
            .push_op(
                GeometryOp::Cylinder {
                    radius: length(0.25),
                    height: length(1.0),
                },
                span(),
            )
            .unwrap();
        let cut = graph
            .push_op(GeometryOp::Cut { lhs: a, rhs: b }, span())
            .unwrap();
        assert_eq!(cut.index(), 2);
    }

    #[test]
    fn unknown_operand_is_rejected() {
        let mut graph = GeometryGraph::new();
        let bogus = GeomId(7);
        let err = graph
            .push_op(GeometryOp::MakeFace { wire: bogus }, span())
            .unwrap_err();
        assert_eq!(
            err,
            GeometryIrError::InvalidOperand {
                referenced: bogus,
                span: span()
            }
        );
    }

    #[test]
    fn forward_reference_is_rejected() {
        // GeomId(0) does not exist yet -- nothing has been pushed.
        let mut graph = GeometryGraph::new();
        let not_yet_built = GeomId(0);
        let err = graph
            .push_op(
                GeometryOp::MakeFace {
                    wire: not_yet_built,
                },
                span(),
            )
            .unwrap_err();
        assert!(matches!(err, GeometryIrError::InvalidOperand { .. }));
    }

    #[test]
    fn id_from_a_different_graph_is_rejected() {
        let mut graph_a = GeometryGraph::new();
        let a_box = graph_a
            .push_op(
                GeometryOp::Box {
                    dx: length(1.0),
                    dy: length(1.0),
                    dz: length(1.0),
                },
                span(),
            )
            .unwrap();

        let mut graph_b = GeometryGraph::new();
        // graph_b has zero nodes, so a_box (index 0) looks structurally
        // plausible but names nothing real in graph_b.
        let err = graph_b
            .push_op(GeometryOp::MakeFace { wire: a_box }, span())
            .unwrap_err();
        assert!(matches!(err, GeometryIrError::InvalidOperand { .. }));
    }

    #[test]
    fn get_face_accepts_a_valid_target_and_assigns_the_next_sequential_id() {
        let mut graph = GeometryGraph::new();
        let target = graph
            .push_op(
                GeometryOp::Box {
                    dx: length(1.0),
                    dy: length(1.0),
                    dz: length(1.0),
                },
                span(),
            )
            .unwrap();
        let face = graph
            .push_op(
                GeometryOp::GetFace {
                    target,
                    face: FaceIndex(0),
                },
                span(),
            )
            .unwrap();
        assert_eq!(face.index(), 1);
    }

    #[test]
    fn get_face_rejects_an_invalid_target_operand() {
        let mut graph = GeometryGraph::new();
        let bogus = GeomId(9);
        let err = graph
            .push_op(
                GeometryOp::GetFace {
                    target: bogus,
                    face: FaceIndex(0),
                },
                span(),
            )
            .unwrap_err();
        assert_eq!(
            err,
            GeometryIrError::InvalidOperand {
                referenced: bogus,
                span: span()
            }
        );
    }

    #[test]
    fn query_result_cannot_be_used_as_a_geometry_operand() {
        let mut graph = GeometryGraph::new();
        let solid = graph
            .push_op(
                GeometryOp::Box {
                    dx: length(1.0),
                    dy: length(1.0),
                    dz: length(1.0),
                },
                span(),
            )
            .unwrap();
        let volume = graph
            .push_query(GeometryQuery::Volume(solid), span())
            .unwrap();
        let err = graph
            .push_op(GeometryOp::MakeFace { wire: volume }, span())
            .unwrap_err();
        assert_eq!(
            err,
            GeometryIrError::OperandIsNotGeometry {
                referenced: volume,
                span: span()
            }
        );
    }

    #[test]
    fn wire_from_edges_rejects_empty_edge_list() {
        let mut graph = GeometryGraph::new();
        let err = graph
            .push_op(GeometryOp::WireFromEdges { edges: vec![] }, span())
            .unwrap_err();
        assert!(matches!(
            err,
            GeometryIrError::EmptyOperandList {
                context: "WireFromEdges.edges",
                ..
            }
        ));
    }

    #[test]
    fn loft_rejects_empty_section_list() {
        let mut graph = GeometryGraph::new();
        let err = graph
            .push_op(GeometryOp::Loft { sections: vec![] }, span())
            .unwrap_err();
        assert!(matches!(
            err,
            GeometryIrError::EmptyOperandList {
                context: "Loft.sections",
                ..
            }
        ));
    }

    #[test]
    fn fillet_rejects_empty_edge_list() {
        let mut graph = GeometryGraph::new();
        let solid = graph
            .push_op(
                GeometryOp::Box {
                    dx: length(1.0),
                    dy: length(1.0),
                    dz: length(1.0),
                },
                span(),
            )
            .unwrap();
        let err = graph
            .push_op(
                GeometryOp::Fillet {
                    target: solid,
                    edges: vec![],
                    radius: length(0.1),
                },
                span(),
            )
            .unwrap_err();
        assert!(matches!(
            err,
            GeometryIrError::EmptyOperandList {
                context: "Fillet.edges",
                ..
            }
        ));
    }

    #[test]
    fn shell_permits_an_empty_removed_face_list() {
        let mut graph = GeometryGraph::new();
        let solid = graph
            .push_op(
                GeometryOp::Box {
                    dx: length(1.0),
                    dy: length(1.0),
                    dz: length(1.0),
                },
                span(),
            )
            .unwrap();
        let shelled = graph.push_op(
            GeometryOp::Shell {
                target: solid,
                removed_faces: vec![],
                thickness: length(0.05),
            },
            span(),
        );
        assert!(shelled.is_ok());
    }

    #[test]
    fn extrude_rejects_a_wrong_dimension_distance() {
        let mut graph = GeometryGraph::new();
        let wire = graph
            .push_op(
                GeometryOp::CircleWire {
                    center: Point3::ORIGIN,
                    normal: Direction3::Z,
                    radius: length(1.0),
                },
                span(),
            )
            .unwrap();
        let err = graph
            .push_op(
                GeometryOp::Extrude {
                    profile: wire,
                    direction: Direction3::Z,
                    // an Angle where a Length is required
                    distance: angle(1.0),
                },
                span(),
            )
            .unwrap_err();
        match err {
            GeometryIrError::DimensionMismatch {
                context, expected, ..
            } => {
                assert_eq!(context, "Extrude.distance");
                assert_eq!(expected, Dimension::Length);
            }
            other => panic!("expected DimensionMismatch, got {other:?}"),
        }
    }

    #[test]
    fn arc_edge_op_pushes_with_no_dimension_checks() {
        let mut graph = GeometryGraph::new();
        let id = graph
            .push_op(
                GeometryOp::ArcEdge {
                    start: Point3::new(1.0, 0.0, 0.0),
                    mid: Point3::new(0.0, 1.0, 0.0),
                    end: Point3::new(-1.0, 0.0, 0.0),
                },
                span(),
            )
            .unwrap();
        assert!(graph.get(id).unwrap().kind.produces_geometry());
    }

    #[test]
    fn revolve_requires_an_angle_not_a_length() {
        let mut graph = GeometryGraph::new();
        let wire = graph
            .push_op(
                GeometryOp::CircleWire {
                    center: Point3::ORIGIN,
                    normal: Direction3::Z,
                    radius: length(1.0),
                },
                span(),
            )
            .unwrap();
        let err = graph
            .push_op(
                GeometryOp::Revolve {
                    profile: wire,
                    axis: Axis3::new(Point3::ORIGIN, Direction3::Z),
                    angle: length(1.0),
                },
                span(),
            )
            .unwrap_err();
        assert!(matches!(
            err,
            GeometryIrError::DimensionMismatch {
                expected: Dimension::Angle,
                ..
            }
        ));
    }

    #[test]
    fn full_pipeline_builds_a_well_formed_graph() {
        let mut graph = GeometryGraph::new();
        let base = graph
            .push_op(
                GeometryOp::Box {
                    dx: length(10.0),
                    dy: length(10.0),
                    dz: length(5.0),
                },
                span(),
            )
            .unwrap();
        let hole = graph
            .push_op(
                GeometryOp::Cylinder {
                    radius: length(1.0),
                    height: length(5.0),
                },
                span(),
            )
            .unwrap();
        let drilled = graph
            .push_op(
                GeometryOp::Cut {
                    lhs: base,
                    rhs: hole,
                },
                span(),
            )
            .unwrap();
        let filleted = graph
            .push_op(
                GeometryOp::Fillet {
                    target: drilled,
                    edges: vec![EdgeIndex(0)],
                    radius: length(0.2),
                },
                span(),
            )
            .unwrap();
        let placed = graph
            .push_op(
                GeometryOp::Transform {
                    target: filleted,
                    transform: Transform::identity(),
                },
                span(),
            )
            .unwrap();
        let _validity = graph
            .push_query(GeometryQuery::IsValid(placed), span())
            .unwrap();
        let _volume = graph
            .push_query(GeometryQuery::Volume(placed), span())
            .unwrap();
        let _exported = graph
            .push_query(
                GeometryQuery::ExportStep {
                    target: placed,
                    path: PathBuf::from("out.step"),
                },
                span(),
            )
            .unwrap();

        assert_eq!(graph.nodes().len(), 8);
        assert!(graph.nodes()[0].kind.produces_geometry());
        assert!(!graph.nodes()[5].kind.produces_geometry()); // IsValid query
        assert!(!graph.nodes()[6].kind.produces_geometry()); // Volume query
        assert!(!graph.nodes()[7].kind.produces_geometry()); // ExportStep query
    }

    #[test]
    fn quantity_of_reports_its_dimension() {
        let q = length(42.0);
        assert_eq!(q.dimension(), Some(Dimension::Length));
    }

    #[test]
    fn every_error_variant_converts_to_a_well_formed_diagnostic() {
        let errors = vec![
            GeometryIrError::InvalidOperand {
                referenced: GeomId(3),
                span: span(),
            },
            GeometryIrError::OperandIsNotGeometry {
                referenced: GeomId(1),
                span: span(),
            },
            GeometryIrError::DimensionMismatch {
                context: "Extrude.distance",
                expected: Dimension::Length,
                found: OperandType::dimensional(Dimension::Angle, None),
                span: span(),
            },
            GeometryIrError::EmptyOperandList {
                context: "Loft.sections",
                span: span(),
            },
        ];
        for err in errors {
            let code = err.code();
            assert!(code.starts_with("GEOM-E"));
            DiagnosticCode::parse(code).expect("every GeometryIrError code must be well-formed");
            let diagnostic = err.to_diagnostic("test.aicad", "x");
            assert_eq!(diagnostic.category, "geometry-ir");
            assert!(!diagnostic.message.is_empty());
            assert!(!err.to_string().is_empty());
        }
    }

    #[test]
    fn geom_id_display_is_stable() {
        assert_eq!(GeomId(5).to_string(), "%5");
    }

    #[test]
    fn tessellate_requires_a_length_linear_and_angle_angular_deflection() {
        let mut graph = GeometryGraph::new();
        let solid = graph
            .push_op(
                GeometryOp::Box {
                    dx: length(1.0),
                    dy: length(1.0),
                    dz: length(1.0),
                },
                span(),
            )
            .unwrap();
        let ok = graph.push_query(
            GeometryQuery::Tessellate {
                target: solid,
                linear_deflection: length(0.1),
                angular_deflection: angle(0.5),
            },
            span(),
        );
        assert!(ok.is_ok());

        let wrong_linear = graph
            .push_query(
                GeometryQuery::Tessellate {
                    target: solid,
                    // an Angle where a Length is required
                    linear_deflection: angle(0.1),
                    angular_deflection: angle(0.5),
                },
                span(),
            )
            .unwrap_err();
        assert!(matches!(
            wrong_linear,
            GeometryIrError::DimensionMismatch {
                context: "Tessellate.linear_deflection",
                expected: Dimension::Length,
                ..
            }
        ));

        let wrong_angular = graph
            .push_query(
                GeometryQuery::Tessellate {
                    target: solid,
                    linear_deflection: length(0.1),
                    // a Length where an Angle is required
                    angular_deflection: length(0.5),
                },
                span(),
            )
            .unwrap_err();
        assert!(matches!(
            wrong_angular,
            GeometryIrError::DimensionMismatch {
                context: "Tessellate.angular_deflection",
                expected: Dimension::Angle,
                ..
            }
        ));
    }
}
