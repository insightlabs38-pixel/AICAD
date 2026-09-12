//! Geometry IR -> kernel dispatch (`AICAD-060`).
//!
//! [`dispatch_graph`] walks an already-validated
//! `cad_geometry_api::GeometryGraph` (every operand/dimension/non-empty-list
//! invariant already checked at graph-construction time by
//! `GeometryGraph::push_op`/`push_query` — this module never re-derives
//! those checks, it *trusts* them and defends with a structured error
//! rather than a panic if one is ever somehow violated, matching
//! `cad_runtime::interp`'s own "trusts, but verifies" precedent) and calls
//! the corresponding `cad_occt_bridge::OcctContext`/`Shape` operation for
//! each node, in order. Because a `GeometryGraph` is SSA/append-only and an
//! operand may only reference an earlier id (`cad_geometry_api::ir`'s own
//! module doc comment), a single forward pass over `graph.nodes()` always
//! dispatches every operand before the node that consumes it — no
//! topological sort is needed.
//!
//! # Quantity -> kernel `f64` convention
//!
//! Every `Quantity`'s magnitude is already stored in its dimension's
//! canonical unit (metres for `Length`, radians for `Angle` —
//! `cad_units::registry`). `cad-occt-bridge`'s own kernel operations take
//! plain `f64` parameters with no unit convention of their own (OCCT itself
//! is unit-agnostic; `crates/cad-occt-bridge/tests/stage1_bracket.rs` picks
//! whatever magnitudes its own test fixture wants). This dispatcher passes
//! a `Quantity`'s canonical magnitude straight through as the kernel's raw
//! `f64` — the simplest self-consistent convention, requiring no
//! additional lookup table, and invisible to public language semantics (an
//! AICAD program author who writes `5mm` never observes which internal
//! float convention the kernel received; the same `5mm` box always has the
//! same displayed/queried dimensions in the author's own chosen units).
//!
//! # Raw topology selectors are resolved, never cached
//!
//! `Fillet`/`Chamfer`/`Shell` carry `EdgeIndex`/`FaceIndex` selectors
//! (`cad_geometry_api::ir`'s own documented "raw index-based topology
//! selection" limitation). This dispatcher resolves each selector to a real
//! `Shape` via `Shape::get_edge`/`Shape::get_face` immediately before the
//! operation that consumes it, in the same dispatch pass, and never stores
//! a resolved edge/face `Shape` as a graph node's own result — matching
//! `AGENTS.md`'s "Raw topology is ephemeral/unsafe and epoch-bound" and
//! `cad-occt-bridge`'s own documentation that `get_edge`/`get_face` results
//! are "intended for immediate use as a selector, not for storage."

use cad_ast::Span;
use cad_diagnostics::{Diagnostic, DiagnosticCode, Position, Severity, SourceSpan};
use cad_geometry_api::{EdgeIndex, FaceIndex, GeomId, GeometryGraph, GeometryNodeKind};
use cad_geometry_api::{GeometryOp, GeometryQuery, Quantity};
use cad_kernel_api::KernelError;
use cad_kernel_api::Point3;
use cad_occt_bridge::{BoundingBox, OcctContext, Shape, TriangleMesh, ValidationReport};

/// One node's dispatched result. [`NodeResult::Shape`] is the only variant
/// usable as a later node's geometry operand (mirroring
/// `GeometryNodeKind::produces_geometry`); every other variant is a query's
/// terminal result.
#[derive(Debug)]
pub enum NodeResult<'ctx> {
    Shape(Shape<'ctx>),
    Bool(bool),
    Number(f64),
    Point(Point3),
    BoundingBox(BoundingBox),
    Validation(ValidationReport),
    Mesh(TriangleMesh),
    /// `ExportStep`'s result: the operation is a side effect (writing a
    /// file), not a value.
    Unit,
}

/// The full per-node result table for one dispatched graph, indexed by
/// `GeomId::index()`.
pub type GraphResults<'ctx> = Vec<NodeResult<'ctx>>;

/// Every way dispatching an already-validated `GeometryGraph` can fail.
#[derive(Debug, PartialEq)]
pub enum DispatchError {
    /// The underlying kernel operation itself failed (invalid geometry,
    /// degenerate input, stale handle, ...).
    Kernel {
        node: GeomId,
        span: Span,
        operation: &'static str,
        source: KernelError,
    },
    /// Defensive-only: `id` names a node this dispatcher has not yet
    /// computed a `NodeResult` for, or one that is not a
    /// [`NodeResult::Shape`]. `GeometryGraph::push_op`/`push_query` already
    /// guarantee every operand names an earlier geometry-producing node, so
    /// this should be unreachable for any graph actually built through that
    /// API — returned instead of panicking/unwrapping, in case a
    /// `GeometryGraph` is ever constructed by some future path that skips
    /// those checks.
    GraphInvariantViolated {
        node: GeomId,
        referenced: GeomId,
        span: Span,
    },
}

impl DispatchError {
    fn code(&self) -> &'static str {
        match self {
            DispatchError::Kernel { .. } => "GEOM-E005",
            DispatchError::GraphInvariantViolated { .. } => "GEOM-E006",
        }
    }

    fn title(&self) -> &'static str {
        match self {
            DispatchError::Kernel { .. } => "GEOMETRY_KERNEL_OPERATION_FAILED",
            DispatchError::GraphInvariantViolated { .. } => "GEOMETRY_GRAPH_INVARIANT_VIOLATED",
        }
    }

    fn message(&self) -> String {
        match self {
            DispatchError::Kernel {
                node,
                operation,
                source,
                ..
            } => format!("{node} ({operation}) failed in the kernel: {source}"),
            DispatchError::GraphInvariantViolated {
                node, referenced, ..
            } => format!(
                "{node} references {referenced}, which this dispatcher has not computed a \
                 geometry result for -- the graph was not built through \
                 GeometryGraph::push_op/push_query's own validation"
            ),
        }
    }

    fn span(&self) -> Span {
        match self {
            DispatchError::Kernel { span, .. } => *span,
            DispatchError::GraphInvariantViolated { span, .. } => *span,
        }
    }

    /// Builds this error's `cad_diagnostics::Diagnostic`, mirroring
    /// `cad_geometry_api::ir::GeometryIrError::to_diagnostic`'s own
    /// pattern exactly.
    pub fn to_diagnostic(&self, file: &str, source: &str) -> Diagnostic {
        let span = self.span();
        let line_index = cad_ast::LineIndex::new(source);
        let start = line_index.line_column(source, span.start);
        let end = line_index.line_column(source, span.end);
        let code = DiagnosticCode::parse(self.code())
            .expect("DispatchError::code always returns a well-formed FAMILY-Exxx code");
        Diagnostic::new(
            code,
            Severity::Error,
            "geometry-dispatch",
            self.title(),
            self.message(),
        )
        .expect("every DispatchError code carries the 'E' severity letter")
        .with_source(SourceSpan {
            file: file.to_string(),
            start: Position::new(start.line, start.column),
            end: Position::new(end.line, end.column),
        })
    }
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for DispatchError {}

/// A `Quantity`'s canonical magnitude, passed straight through as the
/// kernel's raw `f64` parameter (see module doc comment).
fn mag(q: &Quantity) -> f64 {
    q.magnitude
}

fn shape_operand<'a, 'ctx>(
    results: &'a GraphResults<'ctx>,
    node: GeomId,
    operand: GeomId,
    span: Span,
) -> Result<&'a Shape<'ctx>, DispatchError> {
    match results.get(operand.index() as usize) {
        Some(NodeResult::Shape(shape)) => Ok(shape),
        _ => Err(DispatchError::GraphInvariantViolated {
            node,
            referenced: operand,
            span,
        }),
    }
}

fn shape_operands<'a, 'ctx>(
    results: &'a GraphResults<'ctx>,
    node: GeomId,
    operands: &[GeomId],
    span: Span,
) -> Result<Vec<&'a Shape<'ctx>>, DispatchError> {
    operands
        .iter()
        .map(|&id| shape_operand(results, node, id, span))
        .collect()
}

fn kernel_op<T>(
    node: GeomId,
    span: Span,
    operation: &'static str,
    result: cad_kernel_api::KernelResult<T>,
) -> Result<T, DispatchError> {
    result.map_err(|source| DispatchError::Kernel {
        node,
        span,
        operation,
        source,
    })
}

fn dispatch_op<'ctx>(
    id: GeomId,
    span: Span,
    op: &GeometryOp,
    ctx: &'ctx OcctContext,
    results: &GraphResults<'ctx>,
) -> Result<NodeResult<'ctx>, DispatchError> {
    let shape = match op {
        GeometryOp::Box { dx, dy, dz } => {
            kernel_op(id, span, "Box", ctx.create_box(mag(dx), mag(dy), mag(dz)))?
        }
        GeometryOp::Cylinder { radius, height } => kernel_op(
            id,
            span,
            "Cylinder",
            ctx.create_cylinder(mag(radius), mag(height)),
        )?,
        GeometryOp::ImportStep { path } => {
            kernel_op(id, span, "ImportStep", ctx.import_step(path))?
        }
        GeometryOp::LineEdge { start, end } => {
            kernel_op(id, span, "LineEdge", ctx.make_line_edge(*start, *end))?
        }
        GeometryOp::CircleWire {
            center,
            normal,
            radius,
        } => kernel_op(
            id,
            span,
            "CircleWire",
            ctx.make_circle_wire(*center, *normal, mag(radius)),
        )?,
        GeometryOp::ArcEdge { start, mid, end } => {
            kernel_op(id, span, "ArcEdge", ctx.make_arc_edge(*start, *mid, *end))?
        }
        GeometryOp::WireFromEdges { edges } => {
            let edge_shapes = shape_operands(results, id, edges, span)?;
            kernel_op(
                id,
                span,
                "WireFromEdges",
                ctx.make_wire_from_edges(&edge_shapes),
            )?
        }
        GeometryOp::MakeFace { wire } => {
            let wire_shape = shape_operand(results, id, *wire, span)?;
            kernel_op(id, span, "MakeFace", wire_shape.make_face())?
        }
        GeometryOp::GetFace { target, face } => {
            let target_shape = shape_operand(results, id, *target, span)?;
            kernel_op(id, span, "GetFace", target_shape.get_face(face.0))?
        }
        GeometryOp::Extrude {
            profile,
            direction,
            distance,
        } => {
            let profile_shape = shape_operand(results, id, *profile, span)?;
            kernel_op(
                id,
                span,
                "Extrude",
                profile_shape.extrude(*direction, mag(distance)),
            )?
        }
        GeometryOp::Revolve {
            profile,
            axis,
            angle,
        } => {
            let profile_shape = shape_operand(results, id, *profile, span)?;
            kernel_op(
                id,
                span,
                "Revolve",
                profile_shape.revolve(*axis, mag(angle)),
            )?
        }
        GeometryOp::Sweep { profile, spine } => {
            let profile_shape = shape_operand(results, id, *profile, span)?;
            let spine_shape = shape_operand(results, id, *spine, span)?;
            kernel_op(id, span, "Sweep", profile_shape.sweep(spine_shape))?
        }
        GeometryOp::Loft { sections } => {
            let section_shapes = shape_operands(results, id, sections, span)?;
            kernel_op(id, span, "Loft", ctx.loft(&section_shapes))?
        }
        GeometryOp::Union { lhs, rhs } => {
            let lhs_shape = shape_operand(results, id, *lhs, span)?;
            let rhs_shape = shape_operand(results, id, *rhs, span)?;
            kernel_op(id, span, "Union", lhs_shape.union(rhs_shape))?
        }
        GeometryOp::Cut { lhs, rhs } => {
            let lhs_shape = shape_operand(results, id, *lhs, span)?;
            let rhs_shape = shape_operand(results, id, *rhs, span)?;
            kernel_op(id, span, "Cut", lhs_shape.cut(rhs_shape))?
        }
        GeometryOp::Intersect { lhs, rhs } => {
            let lhs_shape = shape_operand(results, id, *lhs, span)?;
            let rhs_shape = shape_operand(results, id, *rhs, span)?;
            kernel_op(id, span, "Intersect", lhs_shape.intersect(rhs_shape))?
        }
        GeometryOp::Fillet {
            target,
            edges,
            radius,
        } => {
            let target_shape = shape_operand(results, id, *target, span)?;
            let edge_shapes = resolve_edges(id, span, target_shape, edges)?;
            let edge_refs: Vec<&Shape<'ctx>> = edge_shapes.iter().collect();
            kernel_op(
                id,
                span,
                "Fillet",
                target_shape.fillet(&edge_refs, mag(radius)),
            )?
        }
        GeometryOp::Chamfer {
            target,
            edges,
            distance,
        } => {
            let target_shape = shape_operand(results, id, *target, span)?;
            let edge_shapes = resolve_edges(id, span, target_shape, edges)?;
            let edge_refs: Vec<&Shape<'ctx>> = edge_shapes.iter().collect();
            kernel_op(
                id,
                span,
                "Chamfer",
                target_shape.chamfer(&edge_refs, mag(distance)),
            )?
        }
        GeometryOp::Shell {
            target,
            removed_faces,
            thickness,
        } => {
            let target_shape = shape_operand(results, id, *target, span)?;
            let face_shapes = resolve_faces(id, span, target_shape, removed_faces)?;
            let face_refs: Vec<&Shape<'ctx>> = face_shapes.iter().collect();
            kernel_op(
                id,
                span,
                "Shell",
                target_shape.shell(&face_refs, mag(thickness)),
            )?
        }
        GeometryOp::Offset { target, distance } => {
            let target_shape = shape_operand(results, id, *target, span)?;
            kernel_op(id, span, "Offset", target_shape.offset(mag(distance)))?
        }
        GeometryOp::Transform { target, transform } => {
            let target_shape = shape_operand(results, id, *target, span)?;
            kernel_op(id, span, "Transform", target_shape.transform(transform))?
        }
    };
    Ok(NodeResult::Shape(shape))
}

fn resolve_edges<'ctx>(
    node: GeomId,
    span: Span,
    target: &Shape<'ctx>,
    edges: &[EdgeIndex],
) -> Result<Vec<Shape<'ctx>>, DispatchError> {
    edges
        .iter()
        .map(|edge| kernel_op(node, span, "get_edge", target.get_edge(edge.0)))
        .collect()
}

fn resolve_faces<'ctx>(
    node: GeomId,
    span: Span,
    target: &Shape<'ctx>,
    faces: &[FaceIndex],
) -> Result<Vec<Shape<'ctx>>, DispatchError> {
    faces
        .iter()
        .map(|face| kernel_op(node, span, "get_face", target.get_face(face.0)))
        .collect()
}

fn dispatch_query<'ctx>(
    id: GeomId,
    span: Span,
    query: &GeometryQuery,
    results: &GraphResults<'ctx>,
) -> Result<NodeResult<'ctx>, DispatchError> {
    let result = match query {
        GeometryQuery::IsValid(target) => {
            let shape = shape_operand(results, id, *target, span)?;
            NodeResult::Bool(kernel_op(id, span, "IsValid", shape.is_valid())?)
        }
        GeometryQuery::Volume(target) => {
            let shape = shape_operand(results, id, *target, span)?;
            NodeResult::Number(kernel_op(id, span, "Volume", shape.volume())?)
        }
        GeometryQuery::Area(target) => {
            let shape = shape_operand(results, id, *target, span)?;
            NodeResult::Number(kernel_op(id, span, "Area", shape.area())?)
        }
        GeometryQuery::BoundingBox(target) => {
            let shape = shape_operand(results, id, *target, span)?;
            NodeResult::BoundingBox(kernel_op(id, span, "BoundingBox", shape.bounding_box())?)
        }
        GeometryQuery::CenterOfMass(target) => {
            let shape = shape_operand(results, id, *target, span)?;
            NodeResult::Point(kernel_op(id, span, "CenterOfMass", shape.center_of_mass())?)
        }
        GeometryQuery::Validate(target) => {
            let shape = shape_operand(results, id, *target, span)?;
            NodeResult::Validation(kernel_op(id, span, "Validate", shape.validate())?)
        }
        GeometryQuery::Tessellate {
            target,
            linear_deflection,
            angular_deflection,
        } => {
            let shape = shape_operand(results, id, *target, span)?;
            NodeResult::Mesh(kernel_op(
                id,
                span,
                "Tessellate",
                shape.tessellate(mag(linear_deflection), mag(angular_deflection)),
            )?)
        }
        GeometryQuery::ExportStep { target, path } => {
            let shape = shape_operand(results, id, *target, span)?;
            kernel_op(id, span, "ExportStep", shape.export_step(path))?;
            NodeResult::Unit
        }
    };
    Ok(result)
}

/// Dispatches every node of `graph`, in order, against `ctx`, returning one
/// [`NodeResult`] per node (indexed by `GeomId::index()`) or the first
/// [`DispatchError`] encountered.
pub fn dispatch_graph<'ctx>(
    graph: &GeometryGraph,
    ctx: &'ctx OcctContext,
) -> Result<GraphResults<'ctx>, DispatchError> {
    let mut results: GraphResults<'ctx> = Vec::with_capacity(graph.nodes().len());
    for node in graph.nodes() {
        let result = match &node.kind {
            GeometryNodeKind::Construct(op) => dispatch_op(node.id, node.span, op, ctx, &results)?,
            GeometryNodeKind::Query(query) => dispatch_query(node.id, node.span, query, &results)?,
        };
        results.push(result);
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_kernel_api::Transform;
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

    /// Box/Cut/Fillet/Transform, then IsValid/Volume/BoundingBox/
    /// CenterOfMass/Validate/ExportStep -> reimport -> IsValid, all
    /// dispatched from one `GeometryGraph` against a real `OcctContext`.
    /// Never a render-only check (`AGENTS.md`'s evidence rule): every
    /// assertion is exact-B-rep-validity or a closed-form/analytic
    /// numeric comparison, mirroring `stage1_bracket.rs`'s own evidence
    /// style.
    #[test]
    fn full_pipeline_dispatches_a_notched_filleted_box_through_the_kernel() {
        const BASE_DX: f64 = 0.05;
        const BASE_DY: f64 = 0.03;
        const BASE_DZ: f64 = 0.02;
        // Both `Box`es are corner-based at the origin (`create_box_is_
        // valid_and_has_the_expected_volume`'s own convention), so a
        // strictly-smaller second box is guaranteed entirely contained in
        // the first -- a convention-agnostic way to build a real interior
        // notch without needing to know the cylinder-placement convention
        // too.
        const NOTCH_DX: f64 = 0.01;
        const NOTCH_DY: f64 = 0.01;
        const NOTCH_DZ: f64 = 0.015;
        const FILLET_RADIUS: f64 = 0.001;

        let mut graph = GeometryGraph::new();
        let base = graph
            .push_op(
                GeometryOp::Box {
                    dx: length(BASE_DX),
                    dy: length(BASE_DY),
                    dz: length(BASE_DZ),
                },
                span(),
            )
            .unwrap();
        let notch = graph
            .push_op(
                GeometryOp::Box {
                    dx: length(NOTCH_DX),
                    dy: length(NOTCH_DY),
                    dz: length(NOTCH_DZ),
                },
                span(),
            )
            .unwrap();
        let notched = graph
            .push_op(
                GeometryOp::Cut {
                    lhs: base,
                    rhs: notch,
                },
                span(),
            )
            .unwrap();
        let filleted = graph
            .push_op(
                GeometryOp::Fillet {
                    target: notched,
                    edges: vec![EdgeIndex(0)],
                    radius: length(FILLET_RADIUS),
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
        let valid_q = graph
            .push_query(GeometryQuery::IsValid(placed), span())
            .unwrap();
        let validate_q = graph
            .push_query(GeometryQuery::Validate(placed), span())
            .unwrap();
        let volume_q = graph
            .push_query(GeometryQuery::Volume(placed), span())
            .unwrap();
        let bbox_q = graph
            .push_query(GeometryQuery::BoundingBox(placed), span())
            .unwrap();
        let com_q = graph
            .push_query(GeometryQuery::CenterOfMass(placed), span())
            .unwrap();
        let export_path =
            std::env::temp_dir().join("aicad_geometry_runtime_dispatch_pipeline.step");
        let export_q = graph
            .push_query(
                GeometryQuery::ExportStep {
                    target: placed,
                    path: export_path.clone(),
                },
                span(),
            )
            .unwrap();

        let ctx = OcctContext::new().expect("context creation should succeed");
        let results = dispatch_graph(&graph, &ctx).expect("dispatch should succeed");

        match &results[valid_q.index() as usize] {
            NodeResult::Bool(valid) => {
                assert!(*valid, "notched/filleted box must be a valid B-rep")
            }
            other => panic!("expected Bool, got {other:?}"),
        }
        match &results[validate_q.index() as usize] {
            NodeResult::Validation(report) => assert!(report.is_valid),
            other => panic!("expected Validation, got {other:?}"),
        }

        let expected_volume = BASE_DX * BASE_DY * BASE_DZ - NOTCH_DX * NOTCH_DY * NOTCH_DZ;
        match &results[volume_q.index() as usize] {
            // 5% relative tolerance covers the (unclaimed, un-analytically-
            // derived here) small volume change the fillet itself
            // introduces on whichever edge index 0 happens to name --
            // this test proves the pipeline dispatches correctly end to
            // end, not the fillet's own exact analytic volume delta
            // (`stage1_bracket.rs` derives that in closed form for its own
            // fixed, known edges; this test intentionally does not pick a
            // specific edge).
            NodeResult::Number(volume) => assert!(
                (*volume - expected_volume).abs() < expected_volume * 0.05,
                "volume {volume} far from expected {expected_volume}"
            ),
            other => panic!("expected Number, got {other:?}"),
        }
        match &results[bbox_q.index() as usize] {
            NodeResult::BoundingBox(bbox) => {
                // Loose tolerance (bigger than FILLET_RADIUS) since the
                // fillet may perturb an outer-boundary edge slightly.
                const TOL: f64 = 0.002;
                assert!((bbox.min.x - 0.0).abs() < TOL);
                assert!((bbox.max.x - BASE_DX).abs() < TOL);
                assert!((bbox.min.y - 0.0).abs() < TOL);
                assert!((bbox.max.y - BASE_DY).abs() < TOL);
                assert!((bbox.min.z - 0.0).abs() < TOL);
                assert!((bbox.max.z - BASE_DZ).abs() < TOL);
            }
            other => panic!("expected BoundingBox, got {other:?}"),
        }
        match &results[com_q.index() as usize] {
            NodeResult::Point(com) => {
                assert!(com.x > 0.0 && com.x < BASE_DX);
                assert!(com.y > 0.0 && com.y < BASE_DY);
                assert!(com.z > 0.0 && com.z < BASE_DZ);
            }
            other => panic!("expected Point, got {other:?}"),
        }

        // The dispatcher's own `ExportStep` query path (not a direct
        // `Shape::export_step` call) wrote `export_path`; assert it ran as
        // a side effect (`NodeResult::Unit`), then reimport that same file
        // through a fresh `GeometryGraph`'s `ImportStep` op (dispatched
        // against a second, independent `OcctContext`) and confirm it
        // reads back as a valid B-rep -- proving the dispatcher's
        // `ExportStep`/`ImportStep` handling agrees with a real file on
        // disk, not only with itself in memory.
        assert!(matches!(
            results[export_q.index() as usize],
            NodeResult::Unit
        ));

        let mut reimport_graph = GeometryGraph::new();
        let reimported = reimport_graph
            .push_op(
                GeometryOp::ImportStep {
                    path: export_path.clone(),
                },
                span(),
            )
            .unwrap();
        let reimported_valid_q = reimport_graph
            .push_query(GeometryQuery::IsValid(reimported), span())
            .unwrap();
        let reimport_ctx = OcctContext::new().expect("context creation should succeed");
        let reimport_results = dispatch_graph(&reimport_graph, &reimport_ctx)
            .expect("reimport dispatch should succeed");
        match &reimport_results[reimported_valid_q.index() as usize] {
            NodeResult::Bool(valid) => assert!(*valid, "reimported STEP shape must be valid"),
            other => panic!("expected Bool, got {other:?}"),
        }
        let _ = std::fs::remove_file(&export_path);
    }

    #[test]
    fn cylinder_volume_matches_the_closed_form_formula() {
        const RADIUS: f64 = 0.005;
        const HEIGHT: f64 = 0.02;

        let mut graph = GeometryGraph::new();
        let cyl = graph
            .push_op(
                GeometryOp::Cylinder {
                    radius: length(RADIUS),
                    height: length(HEIGHT),
                },
                span(),
            )
            .unwrap();
        let volume_q = graph
            .push_query(GeometryQuery::Volume(cyl), span())
            .unwrap();

        let ctx = OcctContext::new().expect("context creation should succeed");
        let results = dispatch_graph(&graph, &ctx).expect("dispatch should succeed");

        let expected = std::f64::consts::PI * RADIUS * RADIUS * HEIGHT;
        match &results[volume_q.index() as usize] {
            NodeResult::Number(volume) => {
                assert!(
                    (*volume - expected).abs() < expected * 1e-6,
                    "volume {volume} did not match closed-form {expected}"
                )
            }
            other => panic!("expected Number, got {other:?}"),
        }
    }

    #[test]
    fn extruded_circle_wire_matches_the_closed_form_volume() {
        const RADIUS: f64 = 0.004;
        const HEIGHT: f64 = 0.012;

        let mut graph = GeometryGraph::new();
        let wire = graph
            .push_op(
                GeometryOp::CircleWire {
                    center: Point3::ORIGIN,
                    normal: cad_kernel_api::Direction3::Z,
                    radius: length(RADIUS),
                },
                span(),
            )
            .unwrap();
        let face = graph
            .push_op(GeometryOp::MakeFace { wire }, span())
            .unwrap();
        let solid = graph
            .push_op(
                GeometryOp::Extrude {
                    profile: face,
                    direction: cad_kernel_api::Direction3::Z,
                    distance: length(HEIGHT),
                },
                span(),
            )
            .unwrap();
        let valid_q = graph
            .push_query(GeometryQuery::IsValid(solid), span())
            .unwrap();
        let volume_q = graph
            .push_query(GeometryQuery::Volume(solid), span())
            .unwrap();

        let ctx = OcctContext::new().expect("context creation should succeed");
        let results = dispatch_graph(&graph, &ctx).expect("dispatch should succeed");

        match &results[valid_q.index() as usize] {
            NodeResult::Bool(valid) => assert!(*valid),
            other => panic!("expected Bool, got {other:?}"),
        }
        let expected = std::f64::consts::PI * RADIUS * RADIUS * HEIGHT;
        match &results[volume_q.index() as usize] {
            NodeResult::Number(volume) => assert!(
                (*volume - expected).abs() < expected * 1e-3,
                "volume {volume} did not match closed-form {expected}"
            ),
            other => panic!("expected Number, got {other:?}"),
        }
    }

    #[test]
    fn wire_from_edges_builds_a_square_face_with_the_expected_area() {
        const SIDE: f64 = 0.01;
        let p00 = Point3::new(0.0, 0.0, 0.0);
        let p10 = Point3::new(SIDE, 0.0, 0.0);
        let p11 = Point3::new(SIDE, SIDE, 0.0);
        let p01 = Point3::new(0.0, SIDE, 0.0);

        let mut graph = GeometryGraph::new();
        let e0 = graph
            .push_op(
                GeometryOp::LineEdge {
                    start: p00,
                    end: p10,
                },
                span(),
            )
            .unwrap();
        let e1 = graph
            .push_op(
                GeometryOp::LineEdge {
                    start: p10,
                    end: p11,
                },
                span(),
            )
            .unwrap();
        let e2 = graph
            .push_op(
                GeometryOp::LineEdge {
                    start: p11,
                    end: p01,
                },
                span(),
            )
            .unwrap();
        let e3 = graph
            .push_op(
                GeometryOp::LineEdge {
                    start: p01,
                    end: p00,
                },
                span(),
            )
            .unwrap();
        let wire = graph
            .push_op(
                GeometryOp::WireFromEdges {
                    edges: vec![e0, e1, e2, e3],
                },
                span(),
            )
            .unwrap();
        let face = graph
            .push_op(GeometryOp::MakeFace { wire }, span())
            .unwrap();
        let valid_q = graph
            .push_query(GeometryQuery::IsValid(face), span())
            .unwrap();
        let area_q = graph.push_query(GeometryQuery::Area(face), span()).unwrap();

        let ctx = OcctContext::new().expect("context creation should succeed");
        let results = dispatch_graph(&graph, &ctx).expect("dispatch should succeed");

        match &results[valid_q.index() as usize] {
            NodeResult::Bool(valid) => assert!(*valid),
            other => panic!("expected Bool, got {other:?}"),
        }
        match &results[area_q.index() as usize] {
            NodeResult::Number(area) => {
                let expected = SIDE * SIDE;
                assert!(
                    (*area - expected).abs() < expected * 1e-6,
                    "area {area} did not match expected {expected}"
                )
            }
            other => panic!("expected Number, got {other:?}"),
        }
    }

    #[test]
    fn arc_edge_and_line_edge_build_a_semicircular_face_with_the_expected_area() {
        // A "D" shape: a straight diameter plus a semicircular arc edge,
        // proving `GeometryOp::ArcEdge` dispatches through the same
        // `WireFromEdges`/`MakeFace` path `LineEdge` already does.
        const RADIUS: f64 = 0.01;
        let p_top = Point3::new(0.0, RADIUS, 0.0);
        let p_bottom = Point3::new(0.0, -RADIUS, 0.0);
        let p_right = Point3::new(RADIUS, 0.0, 0.0);

        let mut graph = GeometryGraph::new();
        let diameter = graph
            .push_op(
                GeometryOp::LineEdge {
                    start: p_bottom,
                    end: p_top,
                },
                span(),
            )
            .unwrap();
        let arc = graph
            .push_op(
                GeometryOp::ArcEdge {
                    start: p_top,
                    mid: p_right,
                    end: p_bottom,
                },
                span(),
            )
            .unwrap();
        let wire = graph
            .push_op(
                GeometryOp::WireFromEdges {
                    edges: vec![diameter, arc],
                },
                span(),
            )
            .unwrap();
        let face = graph
            .push_op(GeometryOp::MakeFace { wire }, span())
            .unwrap();
        let valid_q = graph
            .push_query(GeometryQuery::IsValid(face), span())
            .unwrap();
        let area_q = graph.push_query(GeometryQuery::Area(face), span()).unwrap();

        let ctx = OcctContext::new().expect("context creation should succeed");
        let results = dispatch_graph(&graph, &ctx).expect("dispatch should succeed");

        match &results[valid_q.index() as usize] {
            NodeResult::Bool(valid) => assert!(*valid),
            other => panic!("expected Bool, got {other:?}"),
        }
        match &results[area_q.index() as usize] {
            NodeResult::Number(area) => {
                let expected = std::f64::consts::PI * RADIUS * RADIUS / 2.0;
                assert!(
                    (*area - expected).abs() < expected * 1e-6,
                    "area {area} did not match expected {expected}"
                )
            }
            other => panic!("expected Number, got {other:?}"),
        }
    }

    #[test]
    fn tessellate_produces_a_nonempty_mesh() {
        let mut graph = GeometryGraph::new();
        let solid = graph
            .push_op(
                GeometryOp::Box {
                    dx: length(0.01),
                    dy: length(0.01),
                    dz: length(0.01),
                },
                span(),
            )
            .unwrap();
        let mesh_q = graph
            .push_query(
                GeometryQuery::Tessellate {
                    target: solid,
                    linear_deflection: length(0.001),
                    angular_deflection: angle(0.5),
                },
                span(),
            )
            .unwrap();

        let ctx = OcctContext::new().expect("context creation should succeed");
        let results = dispatch_graph(&graph, &ctx).expect("dispatch should succeed");

        match &results[mesh_q.index() as usize] {
            NodeResult::Mesh(mesh) => assert!(mesh.triangle_count() > 0),
            other => panic!("expected Mesh, got {other:?}"),
        }
    }

    #[test]
    fn kernel_error_surfaces_as_a_well_formed_dispatch_error_diagnostic() {
        let mut graph = GeometryGraph::new();
        graph
            .push_op(
                GeometryOp::Box {
                    dx: length(-1.0),
                    dy: length(1.0),
                    dz: length(1.0),
                },
                span(),
            )
            .unwrap();

        let ctx = OcctContext::new().expect("context creation should succeed");
        let err = dispatch_graph(&graph, &ctx).unwrap_err();
        assert!(matches!(
            err,
            DispatchError::Kernel {
                operation: "Box",
                ..
            }
        ));
        let diagnostic = err.to_diagnostic("test.aicad", "x");
        assert_eq!(diagnostic.category, "geometry-dispatch");
        assert!(!diagnostic.message.is_empty());
        assert!(err.to_string().starts_with("GEOM-E005"));
    }

    #[test]
    fn graph_invariant_violation_is_reported_rather_than_panicking() {
        // `shape_operand` is only ever called by this module's own
        // dispatch functions with a `results` table it has itself been
        // building up in lockstep with an already-`push_op`-validated
        // graph, so this defensive path is unreachable through the public
        // `dispatch_graph` API. Exercise it directly instead, with an
        // empty results table standing in for "no node computed yet" --
        // proving it reports a structured error rather than panicking on
        // the out-of-bounds index.
        let mut graph = GeometryGraph::new();
        let id = graph
            .push_op(
                GeometryOp::Box {
                    dx: length(1.0),
                    dy: length(1.0),
                    dz: length(1.0),
                },
                span(),
            )
            .unwrap();
        let empty_results: GraphResults<'_> = Vec::new();
        let err = shape_operand(&empty_results, id, id, span()).unwrap_err();
        assert!(matches!(err, DispatchError::GraphInvariantViolated { .. }));
    }

    /// `project/DECISION_LOG.md#DL-15`'s own required-tests list, item 10:
    /// "runtime execution reaches the already-implemented real OCCT
    /// dispatcher and produces valid exact B-rep evidence." This is the
    /// full pipeline `DL-15`'s own "IMPLEMENTATION FLOW TO PROVE" names:
    /// `.aicad` source -> parser -> binding -> type checker -> typed HIR
    /// ordinary call -> `RuntimeBuiltin` dispatch -> `GeometryGraph` node
    /// -> this crate's own dispatcher -> the Stage-1 kernel-neutral API ->
    /// OCCT — every layer exercised for real, never bypassed. Not a
    /// substitute for `AICAD-063`'s own full end-to-end gate (a much
    /// larger fixture with parameters/control flow/STEP verification),
    /// only this task's own proof that the `D18` mechanism it adds
    /// actually reaches a real kernel call.
    #[test]
    fn full_source_to_kernel_pipeline_produces_a_valid_exact_brep() {
        let source = "\
            fn f() -> Geometry { \
                let hole = transform(cylinder(2mm, 20mm), 5mm, 5mm, -5mm); \
                return cut(box(10mm, 10mm, 10mm), hole); \
            }";
        let (program, parse_diagnostics) = cad_parser::parse_program(source, "test.aicad");
        assert!(parse_diagnostics.is_empty(), "{parse_diagnostics:?}");
        let lowered = cad_hir::lower::lower_program(&program, "test.aicad", source);
        assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
        let checked = cad_hir::typeck::check_program(
            &lowered.program,
            &lowered.bindings,
            "test.aicad",
            source,
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);

        let mut interp = cad_runtime::interp::Interpreter::new(
            &lowered.program,
            &lowered.bindings,
            "test.aicad",
            source,
        );
        let result = interp
            .call_by_name("f", vec![])
            .expect("execution should succeed");
        let cad_runtime::value::Value::Geometry(id) = result else {
            panic!("expected Value::Geometry, got {result:?}");
        };

        let ctx = OcctContext::new().expect("context creation should succeed");
        let results =
            dispatch_graph(interp.geometry_graph(), &ctx).expect("dispatch should succeed");
        match &results[id.index() as usize] {
            NodeResult::Shape(shape) => {
                assert!(
                    shape.is_valid().unwrap(),
                    "cut result must be a valid B-rep"
                );
                let volume = shape.volume().unwrap();
                let box_volume = 0.01 * 0.01 * 0.01;
                let hole_volume = std::f64::consts::PI * 0.002 * 0.002 * 0.01;
                let expected = box_volume - hole_volume;
                assert!(
                    (volume - expected).abs() < expected * 0.01,
                    "volume {volume} far from expected {expected}"
                );
            }
            other => panic!("expected Shape, got {other:?}"),
        }
    }

    /// The `hole` Safe CAD builtin (`AICAD-076`) end to end from real
    /// `.aicad` source through a real kernel: a through-hole (with a
    /// deliberate 1mm overshoot on the entry side, `AICAD-034`'s own
    /// "clean through-cut" precedent) bored along +Z through a box,
    /// verified against the closed-form removed volume. Needs
    /// `cad_hir::geometry_types` prepended (unlike `full_source_to_
    /// kernel_pipeline_produces_a_valid_exact_brep` above) because `hole`
    /// takes a `Vector3<Float>` direction argument.
    #[test]
    fn hole_builtin_end_to_end_bores_a_clean_through_hole() {
        let source = "\
            fn f() -> Geometry { \
                return hole(box(20mm, 20mm, 10mm), 5mm, 5mm, -1mm, \
                    Vector3(x = 0.0, y = 0.0, z = 1.0), 4mm, 12mm); \
            }";
        let (program, parse_diagnostics) = cad_parser::parse_program(source, "test.aicad");
        assert!(parse_diagnostics.is_empty(), "{parse_diagnostics:?}");
        let program = cad_hir::geometry_types::with_geometry_types(&program);
        let lowered = cad_hir::lower::lower_program(&program, "test.aicad", source);
        assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
        let checked = cad_hir::typeck::check_program(
            &lowered.program,
            &lowered.bindings,
            "test.aicad",
            source,
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);

        let mut interp = cad_runtime::interp::Interpreter::new(
            &lowered.program,
            &lowered.bindings,
            "test.aicad",
            source,
        );
        let result = interp
            .call_by_name("f", vec![])
            .expect("execution should succeed");
        let cad_runtime::value::Value::Geometry(id) = result else {
            panic!("expected Value::Geometry, got {result:?}");
        };

        let ctx = OcctContext::new().expect("context creation should succeed");
        let results =
            dispatch_graph(interp.geometry_graph(), &ctx).expect("dispatch should succeed");
        match &results[id.index() as usize] {
            NodeResult::Shape(shape) => {
                assert!(shape.is_valid().unwrap(), "bored box must be a valid B-rep");
                let volume = shape.volume().unwrap();
                let box_volume = 0.02 * 0.02 * 0.01;
                // The hole overshoots the box's own 10mm thickness on both
                // ends (entry at -1mm, 12mm tall), so the box's own full
                // 10mm thickness is bored through cleanly -- the removed
                // volume is bounded by the box's own material extent, not
                // the cylinder's nominal 12mm height.
                let hole_removed_volume = std::f64::consts::PI * 0.002 * 0.002 * 0.01;
                let expected = box_volume - hole_removed_volume;
                assert!(
                    (volume - expected).abs() < expected * 1e-6,
                    "volume {volume} far from expected {expected}"
                );
            }
            other => panic!("expected Shape, got {other:?}"),
        }
    }

    /// `GetFace` (`AICAD-076`) selects a real, valid, non-degenerate face
    /// out of an already-built box -- proving the new op actually
    /// resolves to a real kernel face `Extrude`/`Revolve` can consume as
    /// their own `profile` operand, not just that it constructs a
    /// well-formed graph node (`ir::tests::get_face_accepts_a_valid_
    /// target_and_assigns_the_next_sequential_id` already covers that).
    /// A non-cubic box's face area must be exactly one of the three
    /// possible face areas (kernel face-enumeration order is not part of
    /// this dispatcher's own contract, so this test does not assume which
    /// face index 0 happens to be).
    #[test]
    fn get_face_selects_a_real_valid_face_with_one_of_the_expected_areas() {
        const DX: f64 = 0.03;
        const DY: f64 = 0.05;
        const DZ: f64 = 0.07;

        let mut graph = GeometryGraph::new();
        let target = graph
            .push_op(
                GeometryOp::Box {
                    dx: length(DX),
                    dy: length(DY),
                    dz: length(DZ),
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
        let valid_q = graph
            .push_query(GeometryQuery::IsValid(face), span())
            .unwrap();
        let area_q = graph.push_query(GeometryQuery::Area(face), span()).unwrap();

        let ctx = OcctContext::new().expect("context creation should succeed");
        let results = dispatch_graph(&graph, &ctx).expect("dispatch should succeed");

        match &results[valid_q.index() as usize] {
            NodeResult::Bool(valid) => assert!(*valid, "selected face must be a valid B-rep"),
            other => panic!("expected Bool, got {other:?}"),
        }
        let possible_areas = [DX * DY, DY * DZ, DX * DZ];
        match &results[area_q.index() as usize] {
            NodeResult::Number(area) => assert!(
                possible_areas
                    .iter()
                    .any(|expected| (*area - expected).abs() < expected * 1e-6),
                "face area {area} did not match any expected face area {possible_areas:?}"
            ),
            other => panic!("expected Number, got {other:?}"),
        }
    }
}
