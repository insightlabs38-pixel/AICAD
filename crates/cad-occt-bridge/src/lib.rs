//! `cad-occt-bridge` — safe Rust wrapper around
//! `native/occt_bridge`'s C ABI (AICAD-018).
//!
//! This is the **only** crate permitted to reference
//! `native/occt_bridge`/OCCT at all (RFC-0002 §3,
//! `project/DECISION_LOG.md#DL-5`). The [`ffi`] module holds the raw,
//! `unsafe`, one-to-one `extern "C"` declarations matching
//! `native/occt_bridge/include/aicad_occt_bridge.h` exactly; everything
//! else in this crate is a safe wrapper that never leaks an
//! `ffi`-module type, an OCCT type, or a raw pointer through its public
//! API. Handle identity above this module is expressed only in
//! `cad-kernel-api`'s kernel-neutral vocabulary
//! ([`cad_kernel_api::KernelShape`], [`cad_kernel_api::KernelId`]).

mod ffi;

use cad_kernel_api::topology::TopologyKind;
use cad_kernel_api::{
    Axis3, Direction3, KernelError, KernelId, KernelResult, KernelShape, Plane3, Point3, Transform,
};
use std::os::raw::c_int;

/// Converts one native `aicad_occt_status_t` value into a
/// [`KernelResult<()>`], per the one-to-one mapping documented on
/// [`cad_kernel_api::KernelError`] and `native/occt_bridge/include/aicad_occt_bridge.h`.
///
/// The native side is entirely under this workspace's control
/// (`native/occt_bridge/src/aicad_occt_bridge.cpp`), so every status
/// value it can ever produce is one of the eight enumerated here.
/// Nonetheless, any value this match does not recognize is treated as
/// [`KernelError::Internal`] rather than panicking or invoking undefined
/// behavior -- matching an unenumerated C `int` defensively is always
/// safe, whereas transmuting it into a Rust `#[repr(i32)]` enum would not
/// be.
fn status_result(raw: c_int) -> KernelResult<()> {
    match raw {
        0 => Ok(()),
        1 => Err(KernelError::InvalidArgument),
        2 => Err(KernelError::InvalidHandle),
        3 => Err(KernelError::StaleHandle),
        4 => Err(KernelError::ForeignContext),
        5 => Err(KernelError::WrongThread),
        6 => Err(KernelError::OperationFailed),
        _ => Err(KernelError::Internal), // 7 (ERR_INTERNAL), or anything unrecognized.
    }
}

fn id_to_handle(id: KernelId) -> ffi::aicad_shape_handle_t {
    ffi::aicad_shape_handle_t {
        context_id: id.context_id,
        slot: id.slot,
        generation: id.generation,
    }
}

fn handle_to_id(handle: ffi::aicad_shape_handle_t) -> KernelId {
    KernelId {
        context_id: handle.context_id,
        slot: handle.slot,
        generation: handle.generation,
    }
}

/// A safe, RAII-owning wrapper around one native OCCT kernel context.
///
/// Dropping an `OcctContext` destroys the underlying native context
/// (`aicad_occt_context_destroy`). Every [`Shape`] it produced borrows
/// this context for its own lifetime (`Shape<'ctx>`), so the borrow
/// checker rejects any attempt to drop the context while a `Shape` it
/// owns is still alive -- "released/stale handles must never accidentally
/// alias newly created geometry" and "kernel topology handles are
/// epoch/build-local" (Stage-1 kernel policies) hold at compile time
/// here, not only via the native bridge's own runtime checks
/// (`project/reports/AICAD-016.md`).
///
/// `OcctContext` is neither `Send` nor `Sync`: it holds a raw pointer,
/// so Rust does not implement either auto trait for it, matching the
/// native bridge's single-thread-affine contract (Stage-1 kernel policy
/// #9) at the type level, not only via the native bridge's own
/// `WRONG_THREAD` runtime check.
#[derive(Debug)]
pub struct OcctContext {
    raw: *mut ffi::aicad_occt_context_t,
}

impl OcctContext {
    /// Creates a new, independent kernel context.
    pub fn new() -> KernelResult<Self> {
        let mut raw: *mut ffi::aicad_occt_context_t = std::ptr::null_mut();
        // SAFETY: `&mut raw` is a valid, uniquely-owned `*mut *mut
        // aicad_occt_context_t` for the duration of this call, matching
        // the header's contract for `out_context`.
        let status = unsafe { ffi::aicad_occt_context_create(&mut raw) };
        status_result(status)?;
        debug_assert!(
            !raw.is_null(),
            "AICAD_OCCT_OK must set *out_context to a non-null value"
        );
        Ok(Self { raw })
    }

    /// Constructs an axis-aligned box of the given dimensions and returns
    /// a [`Shape`] owning it, borrowed from this context.
    pub fn create_box(&self, dx: f64, dy: f64, dz: f64) -> KernelResult<Shape<'_>> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `self.raw` is a valid context for `self`'s lifetime
        // (invariant maintained by this type); `&mut handle` is a valid
        // out-param per the header's contract.
        let status = unsafe { ffi::aicad_occt_create_box(self.raw, dx, dy, dz, &mut handle) };
        status_result(status)?;
        Ok(Shape {
            context: self,
            id: handle_to_id(handle),
        })
    }

    /// Constructs a capped cylindrical solid centered on the origin with
    /// its axis along +Z (AICAD-020). A future rigid-transform operation
    /// places it elsewhere, per RFC-0002 §3's capability-driven
    /// minimal-surface rule -- this constructor stays origin/+Z-only.
    pub fn create_cylinder(&self, radius: f64, height: f64) -> KernelResult<Shape<'_>> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: same argument as `create_box` above.
        let status =
            unsafe { ffi::aicad_occt_create_cylinder(self.raw, radius, height, &mut handle) };
        status_result(status)?;
        Ok(Shape {
            context: self,
            id: handle_to_id(handle),
        })
    }

    /// Imports a STEP file and returns the resulting shape (AICAD-035),
    /// via OCCT's own `STEPControl_Reader`. This is a narrow,
    /// kernel-adapter-scoped operation added specifically to support the
    /// Stage-1 proof's own export -> re-import verification pipeline --
    /// **not** the full public language-level `import_step()` described
    /// in `docs/plan/23_CROSS_SYSTEM_PARAMETER_CATALOG.md` §9 (no unit/
    /// heal/coordinate-policy/naming-policy options, no semantic-node
    /// wrapping or provenance capture). If the file describes more than
    /// one root shape, the returned shape is whichever single shape
    /// OCCT's own reader designates (typically a Compound containing all
    /// transferred roots) -- this does not impose or validate a
    /// single-root-shape contract on its caller's STEP files.
    ///
    /// Read this method's own verification-scope caveat before treating
    /// an export-then-import round trip through this bridge as
    /// independent evidence: both directions share one OCCT installation,
    /// so a round trip proves the pipeline is self-consistent, not that
    /// an independent (non-OCCT) implementation agrees with OCCT's own
    /// output. See `project/reports/AICAD-035.md` for the genuinely
    /// independent (non-OCCT, structural-only) verification path used
    /// alongside this.
    pub fn import_step(&self, path: &std::path::Path) -> KernelResult<Shape<'_>> {
        let path_str = path.to_str().ok_or(KernelError::InvalidArgument)?;
        let c_path = std::ffi::CString::new(path_str).map_err(|_| KernelError::InvalidArgument)?;
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `c_path` is a valid, live, null-terminated C string for
        // the duration of this call; `self.raw`/`&mut handle` as in
        // `create_box`.
        let status = unsafe { ffi::aicad_occt_import_step(self.raw, c_path.as_ptr(), &mut handle) };
        status_result(status)?;
        Ok(Shape {
            context: self,
            id: handle_to_id(handle),
        })
    }

    /// Constructs a straight edge between two points (AICAD-022).
    pub fn make_line_edge(&self, p0: Point3, p1: Point3) -> KernelResult<Shape<'_>> {
        let p0 = [p0.x, p0.y, p0.z];
        let p1 = [p1.x, p1.y, p1.z];
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `p0`/`p1` are valid, live `[f64; 3]` arrays for the
        // duration of this call; `self.raw`/`&mut handle` as in `create_box`.
        let status = unsafe {
            ffi::aicad_occt_make_line_edge(self.raw, p0.as_ptr(), p1.as_ptr(), &mut handle)
        };
        status_result(status)?;
        Ok(Shape {
            context: self,
            id: handle_to_id(handle),
        })
    }

    /// Constructs a closed circular wire (AICAD-022).
    pub fn make_circle_wire(
        &self,
        center: Point3,
        normal: Direction3,
        radius: f64,
    ) -> KernelResult<Shape<'_>> {
        let center = [center.x, center.y, center.z];
        let normal_v = normal.as_vector3();
        let normal = [normal_v.x, normal_v.y, normal_v.z];
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: see `make_line_edge` above; identical argument.
        let status = unsafe {
            ffi::aicad_occt_make_circle_wire(
                self.raw,
                center.as_ptr(),
                normal.as_ptr(),
                radius,
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self,
            id: handle_to_id(handle),
        })
    }

    /// Constructs a circular-arc edge passing through three points, in
    /// order `start -> mid -> end` (AICAD-075). `mid` must lie strictly
    /// between the other two along the intended arc -- see
    /// `aicad_occt_make_arc_edge`'s own doc comment for why this
    /// determines both which of the two possible arcs is built and its
    /// traversal direction, with no separate axis/sense parameter.
    pub fn make_arc_edge(
        &self,
        start: Point3,
        mid: Point3,
        end: Point3,
    ) -> KernelResult<Shape<'_>> {
        let start = [start.x, start.y, start.z];
        let mid = [mid.x, mid.y, mid.z];
        let end = [end.x, end.y, end.z];
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: see `make_line_edge` above; identical argument.
        let status = unsafe {
            ffi::aicad_occt_make_arc_edge(
                self.raw,
                start.as_ptr(),
                mid.as_ptr(),
                end.as_ptr(),
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self,
            id: handle_to_id(handle),
        })
    }

    /// Joins an ordered list of edges (each owned by this context) into
    /// one wire (AICAD-022).
    pub fn make_wire_from_edges<'ctx>(
        &'ctx self,
        edges: &[&Shape<'ctx>],
    ) -> KernelResult<Shape<'ctx>> {
        let handles: Vec<ffi::aicad_shape_handle_t> =
            edges.iter().map(|edge| id_to_handle(edge.id)).collect();
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `handles` is a valid, live, contiguous array of
        // `handles.len()` elements for the duration of this call (no STL
        // container crosses the boundary -- this is a plain pointer +
        // length, per the header's contract); `self.raw`/`&mut handle` as
        // in `create_box`.
        let status = unsafe {
            ffi::aicad_occt_make_wire_from_edges(
                self.raw,
                handles.as_ptr(),
                handles.len(),
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self,
            id: handle_to_id(handle),
        })
    }

    /// Lofts a solid through an ordered list of closed planar wire
    /// cross-sections (`sections.len() >= 2`), each owned by this
    /// context, using straight (ruled) generatrices between consecutive
    /// sections -- the minimal, analytically-predictable loft form
    /// (AICAD-025). All sections must share the same number of
    /// edges/vertices for OCCT to establish a correspondence between
    /// them.
    pub fn loft<'ctx>(&'ctx self, sections: &[&Shape<'ctx>]) -> KernelResult<Shape<'ctx>> {
        let handles: Vec<ffi::aicad_shape_handle_t> = sections
            .iter()
            .map(|section| id_to_handle(section.id))
            .collect();
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `handles` is a valid, live, contiguous array of
        // `handles.len()` elements for the duration of this call (no STL
        // container crosses the boundary), matching
        // `aicad_occt_make_wire_from_edges`' convention; `self.raw`/
        // `&mut handle` as in `create_box`.
        let status =
            unsafe { ffi::aicad_occt_loft(self.raw, handles.as_ptr(), handles.len(), &mut handle) };
        status_result(status)?;
        Ok(Shape {
            context: self,
            id: handle_to_id(handle),
        })
    }

    /// Constructs a single-point vertex (`AICAD-119`,
    /// `BRepBuilderAPI_MakeVertex`) -- the missing base case of the
    /// vertex->edge->wire->face->shell->solid pipeline `make_line_edge`/
    /// `make_circle_wire`/`make_arc_edge`/`make_wire_from_edges`/
    /// `Shape::make_face` already cover.
    pub fn make_vertex(&self, point: Point3) -> KernelResult<Shape<'_>> {
        let p = [point.x, point.y, point.z];
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `p` is a valid, live `[f64; 3]` for the duration of this
        // call; `self.raw`/`&mut handle` as in `create_box`.
        let status = unsafe { ffi::aicad_occt_make_vertex(self.raw, p.as_ptr(), &mut handle) };
        status_result(status)?;
        Ok(Shape {
            context: self,
            id: handle_to_id(handle),
        })
    }

    /// Assembles `faces` (each owned by this context) into one shell
    /// (`AICAD-119`, `BRep_Builder::MakeShell`/`Add`) -- a structural
    /// container only, no sewing/gap-closing (`AICAD-120`'s job): a shell
    /// built from faces that do not already share identical edges is
    /// open/non-manifold under [`Shape::validate`], not silently repaired.
    /// `faces` must be non-empty.
    pub fn make_shell<'ctx>(&'ctx self, faces: &[&Shape<'ctx>]) -> KernelResult<Shape<'ctx>> {
        if faces.is_empty() {
            return Err(KernelError::InvalidArgument);
        }
        let handles: Vec<ffi::aicad_shape_handle_t> =
            faces.iter().map(|face| id_to_handle(face.id)).collect();
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: see `make_wire_from_edges` above; identical argument.
        let status = unsafe {
            ffi::aicad_occt_make_shell(self.raw, handles.as_ptr(), handles.len(), &mut handle)
        };
        status_result(status)?;
        Ok(Shape {
            context: self,
            id: handle_to_id(handle),
        })
    }

    /// Groups `shapes` (each owned by this context, any kind, any mix of
    /// kinds) into one `TopoDS_Compound` (`AICAD-119`,
    /// `BRep_Builder::MakeCompound`/`Add`). `shapes` must be non-empty.
    pub fn make_compound<'ctx>(&'ctx self, shapes: &[&Shape<'ctx>]) -> KernelResult<Shape<'ctx>> {
        if shapes.is_empty() {
            return Err(KernelError::InvalidArgument);
        }
        let handles: Vec<ffi::aicad_shape_handle_t> =
            shapes.iter().map(|shape| id_to_handle(shape.id)).collect();
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: see `make_wire_from_edges` above; identical argument.
        let status = unsafe {
            ffi::aicad_occt_make_compound(self.raw, handles.as_ptr(), handles.len(), &mut handle)
        };
        status_result(status)?;
        Ok(Shape {
            context: self,
            id: handle_to_id(handle),
        })
    }

    /// Sews `shapes` together (`AICAD-120`, `BRepBuilderAPI_Sewing`) at
    /// `tolerance` (a modeling/construction-domain length, canonical
    /// metres, `> 0` -- `project/DECISION_LOG.md#DL-24` domain 2). Also
    /// captures Generated/Modified/IsDeleted lineage for every unique
    /// face/edge of each input against the sewing operation itself, in
    /// the exact same [`Lineage`] type/table `Shape::union_with_lineage`
    /// (`AICAD-086`) already uses -- consumable identically. Returns the
    /// sewed shape, its lineage, and a [`SewReport`] structured evidence
    /// summary; `shapes` must be non-empty. Never sews beyond what
    /// `BRepBuilderAPI_Sewing` itself reports -- `SewReport::is_valid`/
    /// `free_edge_count` are the required evidence, not this call's own
    /// success.
    pub fn sew<'ctx>(
        &'ctx self,
        shapes: &[&Shape<'ctx>],
        tolerance: f64,
    ) -> KernelResult<(Shape<'ctx>, Lineage<'ctx>, SewReport)> {
        if shapes.is_empty() {
            return Err(KernelError::InvalidArgument);
        }
        let handles: Vec<ffi::aicad_shape_handle_t> =
            shapes.iter().map(|shape| id_to_handle(shape.id)).collect();
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        let mut lineage_handle = ffi::aicad_lineage_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        let mut report = ffi::aicad_sew_report_t::default();
        // SAFETY: `handles` is a valid, live, contiguous array for the
        // duration of this call; `self.raw`/`&mut handle`/
        // `&mut lineage_handle` as in `boolean_with_lineage`; `&mut report`
        // is a valid out-param per the header's contract.
        let status = unsafe {
            ffi::aicad_occt_sew(
                self.raw,
                handles.as_ptr(),
                handles.len(),
                tolerance,
                &mut handle,
                &mut lineage_handle,
                &mut report,
            )
        };
        status_result(status)?;
        Ok((
            Shape {
                context: self,
                id: handle_to_id(handle),
            },
            Lineage {
                context: self,
                handle: lineage_handle,
            },
            SewReport {
                changed: report.changed != 0,
                is_valid: report.is_valid != 0,
                free_edge_count: report.free_edge_count,
                multiple_edge_count: report.multiple_edge_count,
                degenerated_shape_count: report.degenerated_shape_count,
            },
        ))
    }
}

impl Drop for OcctContext {
    fn drop(&mut self) {
        // SAFETY: `self.raw` was created by `aicad_occt_context_create`
        // and has not yet been destroyed (this is the only place that
        // destroys it, and it runs at most once per `OcctContext`). Every
        // `Shape` referencing this context has already been dropped by
        // the time this runs -- the borrow checker enforces that via
        // `Shape<'ctx>`'s lifetime.
        let status = unsafe { ffi::aicad_occt_context_destroy(self.raw) };
        debug_assert_eq!(
            status, 0,
            "aicad_occt_context_destroy failed during OcctContext::drop"
        );
    }
}

/// A shape owned by one [`OcctContext`], automatically released
/// (`aicad_occt_release_shape`) when dropped.
#[derive(Debug)]
pub struct Shape<'ctx> {
    context: &'ctx OcctContext,
    id: KernelId,
}

impl<'ctx> Shape<'ctx> {
    /// The backend-independent handle this shape can be exchanged as
    /// through `cad-kernel-api`'s kernel-neutral vocabulary. The returned
    /// [`KernelShape`] carries no lifetime and does not keep this
    /// `Shape`'s native resource alive by itself -- it is a value-only
    /// identity, matching the rest of `cad-kernel-api`.
    pub fn handle(&self) -> KernelShape {
        KernelShape::from_id(self.id)
    }

    /// Whether OCCT's own topology checker (`BRepCheck_Analyzer`)
    /// considers this shape valid.
    pub fn is_valid(&self) -> KernelResult<bool> {
        let mut is_valid: c_int = 0;
        // SAFETY: `self.context.raw` is valid for at least `'ctx`
        // (enforced by the borrow this `Shape` holds); `self.raw_handle()`
        // addresses a slot this `Shape` owns and has not yet released;
        // `&mut is_valid` is a valid out-param.
        let status = unsafe {
            ffi::aicad_occt_shape_is_valid(self.context.raw, self.raw_handle(), &mut is_valid)
        };
        status_result(status)?;
        Ok(is_valid != 0)
    }

    /// A second, independently-releasable handle onto the exact same
    /// underlying shape (`AICAD-100A`) — a cheap map re-insertion on the
    /// native side, never a real geometry copy (`Shape` itself has no
    /// `Clone` impl, matching every other `RAII`-owned-resource wrapper in
    /// this crate — this is the one deliberate, explicit exception, needed
    /// by `cad_query::Candidate::with_root` to give a candidate its own
    /// independently-owned handle onto the whole shape it was enumerated
    /// from, without borrowing the original).
    pub fn duplicate(&self) -> KernelResult<Shape<'ctx>> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `self.context.raw`/`self.raw_handle()`/`&mut handle` as
        // in `is_valid`/`create_box`.
        let status = unsafe {
            ffi::aicad_occt_shape_duplicate(self.context.raw, self.raw_handle(), &mut handle)
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// This shape's volume, per OCCT's `BRepGProp::VolumeProperties`, in
    /// the kernel's internal linear unit (unit semantics belong to
    /// `cad-units` above this crate, not here).
    pub fn volume(&self) -> KernelResult<f64> {
        let mut volume: f64 = 0.0;
        // SAFETY: see `is_valid` above; identical argument.
        let status = unsafe {
            ffi::aicad_occt_shape_volume(self.context.raw, self.raw_handle(), &mut volume)
        };
        status_result(status)?;
        Ok(volume)
    }

    /// Total surface area of every face in this shape.
    pub fn area(&self) -> KernelResult<f64> {
        let mut area: f64 = 0.0;
        // SAFETY: see `is_valid`'s SAFETY comment; identical argument.
        let status =
            unsafe { ffi::aicad_occt_shape_area(self.context.raw, self.raw_handle(), &mut area) };
        status_result(status)?;
        Ok(area)
    }

    /// This shape's axis-aligned bounding box.
    pub fn bounding_box(&self) -> KernelResult<BoundingBox> {
        let mut min = [0.0; 3];
        let mut max = [0.0; 3];
        // SAFETY: see `is_valid`'s SAFETY comment; `&mut min`/`&mut max`
        // are valid 3-element out-params per the header's contract.
        let status = unsafe {
            ffi::aicad_occt_shape_bounding_box(
                self.context.raw,
                self.raw_handle(),
                min.as_mut_ptr(),
                max.as_mut_ptr(),
            )
        };
        status_result(status)?;
        Ok(BoundingBox {
            min: Point3::new(min[0], min[1], min[2]),
            max: Point3::new(max[0], max[1], max[2]),
        })
    }

    /// Applies a rigid transform, producing a new [`Shape`] in the same
    /// context (AICAD-021). Never mutates `self` -- functional/
    /// value-oriented semantics, `project/DECISION_LOG.md#DL-2`.
    pub fn transform(&self, t: &Transform) -> KernelResult<Shape<'ctx>> {
        let matrix = t.to_row_major_3x4();
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `matrix` is a valid, live `[f64; 12]` for the duration
        // of this call; `self.context.raw`/`self.raw_handle()` as in
        // `is_valid`; `&mut handle` as in `create_box`.
        let status = unsafe {
            ffi::aicad_occt_transform_shape(
                self.context.raw,
                self.raw_handle(),
                matrix.as_ptr(),
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Mirrors this shape across `plane`, producing a new [`Shape`] in the
    /// same context (`AICAD-077`). A mirror is an *improper* isometry
    /// (determinant -1), so this is its own native entry point
    /// (`aicad_occt_mirror_shape`) rather than going through
    /// [`Shape::transform`]'s `matrix`, which `aicad_occt_transform_shape`
    /// correctly rejects for not being a proper rigid displacement (see
    /// `cad_kernel_api::geometry`'s own "Rigidity (no reflection)" module
    /// doc comment). Never mutates `self` -- functional/value-oriented
    /// semantics, `project/DECISION_LOG.md#DL-2`, matching `transform`'s
    /// own precedent exactly.
    pub fn mirror(&self, plane: Plane3) -> KernelResult<Shape<'ctx>> {
        let origin = [plane.origin.x, plane.origin.y, plane.origin.z];
        let n = plane.normal.as_vector3();
        let normal = [n.x, n.y, n.z];
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `origin`/`normal` are valid, live `[f64; 3]` arrays for
        // the duration of this call; other arguments as in
        // `is_valid`/`create_box`.
        let status = unsafe {
            ffi::aicad_occt_mirror_shape(
                self.context.raw,
                self.raw_handle(),
                origin.as_ptr(),
                normal.as_ptr(),
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Builds a planar face bounded by this wire (AICAD-023). `self` must
    /// address a single closed, planar wire; the resulting face's own
    /// validity should be checked with [`Shape::is_valid`] rather than
    /// assumed from this call succeeding -- construction success and
    /// topological validity are distinct concepts here (Stage-1 kernel
    /// policy #14), matching `aicad_occt_make_face_from_wire`'s
    /// documented contract.
    pub fn make_face(&self) -> KernelResult<Shape<'ctx>> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: see `is_valid`'s SAFETY comment; identical argument.
        let status = unsafe {
            ffi::aicad_occt_make_face_from_wire(self.context.raw, self.raw_handle(), &mut handle)
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Builds a face bounded by this wire (`self`) on an explicit plane
    /// through `origin` normal to `normal` (`AICAD-119`,
    /// `BRepBuilderAPI_MakeFace(gp_Pln, wire, Inside)`) -- the
    /// holes-and-orientation-capable counterpart of [`Shape::make_face`],
    /// which infers its plane from a planar wire and supports neither.
    /// `reversed` builds the face on `self.Reversed()` instead of `self`;
    /// each of `holes` (each owned by this context) is added exactly as
    /// given, with no orientation inferred or corrected -- see
    /// `aicad_occt_make_face_on_plane`'s own doc comment. As with
    /// `make_face`, construction success is not evidence of validity.
    pub fn make_face_on_plane(
        &self,
        holes: &[&Shape<'ctx>],
        origin: Point3,
        normal: Direction3,
        reversed: bool,
    ) -> KernelResult<Shape<'ctx>> {
        let origin = [origin.x, origin.y, origin.z];
        let n = normal.as_vector3();
        let normal = [n.x, n.y, n.z];
        let hole_handles: Vec<ffi::aicad_shape_handle_t> =
            holes.iter().map(|hole| id_to_handle(hole.id)).collect();
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `origin`/`normal` are valid, live `[f64; 3]` arrays and
        // `hole_handles` a valid, live, contiguous array for the duration
        // of this call; `self.context.raw`/`self.raw_handle()`/`&mut
        // handle` as in `is_valid`/`create_box`.
        let status = unsafe {
            ffi::aicad_occt_make_face_on_plane(
                self.context.raw,
                self.raw_handle(),
                hole_handles.as_ptr(),
                hole_handles.len(),
                origin.as_ptr(),
                normal.as_ptr(),
                c_int::from(reversed),
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Same as [`Shape::make_face_on_plane`], on a cylinder of `radius`
    /// coaxial with `axis` (`AICAD-119`).
    pub fn make_face_on_cylinder(
        &self,
        holes: &[&Shape<'ctx>],
        axis: Axis3,
        radius: f64,
        reversed: bool,
    ) -> KernelResult<Shape<'ctx>> {
        let origin = [axis.origin.x, axis.origin.y, axis.origin.z];
        let d = axis.direction.as_vector3();
        let direction = [d.x, d.y, d.z];
        let hole_handles: Vec<ffi::aicad_shape_handle_t> =
            holes.iter().map(|hole| id_to_handle(hole.id)).collect();
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: see `make_face_on_plane` above; identical argument shapes.
        let status = unsafe {
            ffi::aicad_occt_make_face_on_cylinder(
                self.context.raw,
                self.raw_handle(),
                hole_handles.as_ptr(),
                hole_handles.len(),
                origin.as_ptr(),
                direction.as_ptr(),
                radius,
                c_int::from(reversed),
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Same as [`Shape::make_face_on_plane`], on a cone with apex at
    /// `axis.origin`, opening along `axis.direction` at
    /// `half_angle_radians` (`AICAD-119`).
    pub fn make_face_on_cone(
        &self,
        holes: &[&Shape<'ctx>],
        axis: Axis3,
        half_angle_radians: f64,
        reversed: bool,
    ) -> KernelResult<Shape<'ctx>> {
        let origin = [axis.origin.x, axis.origin.y, axis.origin.z];
        let d = axis.direction.as_vector3();
        let direction = [d.x, d.y, d.z];
        let hole_handles: Vec<ffi::aicad_shape_handle_t> =
            holes.iter().map(|hole| id_to_handle(hole.id)).collect();
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: see `make_face_on_plane` above; identical argument shapes.
        let status = unsafe {
            ffi::aicad_occt_make_face_on_cone(
                self.context.raw,
                self.raw_handle(),
                hole_handles.as_ptr(),
                hole_handles.len(),
                origin.as_ptr(),
                direction.as_ptr(),
                half_angle_radians,
                c_int::from(reversed),
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Same as [`Shape::make_face_on_plane`], on a sphere of `radius`
    /// centered at `center` (`AICAD-119`).
    pub fn make_face_on_sphere(
        &self,
        holes: &[&Shape<'ctx>],
        center: Point3,
        radius: f64,
        reversed: bool,
    ) -> KernelResult<Shape<'ctx>> {
        let center = [center.x, center.y, center.z];
        let hole_handles: Vec<ffi::aicad_shape_handle_t> =
            holes.iter().map(|hole| id_to_handle(hole.id)).collect();
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: see `make_face_on_plane` above; identical argument shapes.
        let status = unsafe {
            ffi::aicad_occt_make_face_on_sphere(
                self.context.raw,
                self.raw_handle(),
                hole_handles.as_ptr(),
                hole_handles.len(),
                center.as_ptr(),
                radius,
                c_int::from(reversed),
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Same as [`Shape::make_face_on_plane`], on a torus coaxial with
    /// `axis` (`major_radius` from the axis to the tube's own center
    /// circle, `minor_radius` the tube's own cross-section radius)
    /// (`AICAD-119`).
    pub fn make_face_on_torus(
        &self,
        holes: &[&Shape<'ctx>],
        axis: Axis3,
        major_radius: f64,
        minor_radius: f64,
        reversed: bool,
    ) -> KernelResult<Shape<'ctx>> {
        let origin = [axis.origin.x, axis.origin.y, axis.origin.z];
        let d = axis.direction.as_vector3();
        let direction = [d.x, d.y, d.z];
        let hole_handles: Vec<ffi::aicad_shape_handle_t> =
            holes.iter().map(|hole| id_to_handle(hole.id)).collect();
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: see `make_face_on_plane` above; identical argument shapes.
        let status = unsafe {
            ffi::aicad_occt_make_face_on_torus(
                self.context.raw,
                self.raw_handle(),
                hole_handles.as_ptr(),
                hole_handles.len(),
                origin.as_ptr(),
                direction.as_ptr(),
                major_radius,
                minor_radius,
                c_int::from(reversed),
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Builds a solid from this shell (`self`, `AICAD-119`,
    /// `BRepBuilderAPI_MakeSolid`), adding each of `voids` (each owned by
    /// this context) as an additional void/cavity shell. Verified
    /// empirically, not assumed: OCCT's own `BRepBuilderAPI_MakeSolid`
    /// does not require `self` to be closed -- it happily reports done for
    /// an open shell, producing a structurally-real but invalid solid.
    /// This call succeeding is therefore even less evidence of validity
    /// than [`Shape::make_face`]'s own already-non-evidence success:
    /// always call [`Shape::validate`]/[`Shape::is_valid`] separately.
    pub fn make_solid(&self, voids: &[&Shape<'ctx>]) -> KernelResult<Shape<'ctx>> {
        let void_handles: Vec<ffi::aicad_shape_handle_t> =
            voids.iter().map(|void| id_to_handle(void.id)).collect();
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `void_handles` is a valid, live, contiguous array for
        // the duration of this call; `self.context.raw`/`self.raw_handle()`/
        // `&mut handle` as in `is_valid`/`create_box`.
        let status = unsafe {
            ffi::aicad_occt_make_solid(
                self.context.raw,
                self.raw_handle(),
                void_handles.as_ptr(),
                void_handles.len(),
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Repairs this shape (`AICAD-120`, `ShapeFix_Shape`) at `tolerance`
    /// (same modeling/construction tolerance domain as
    /// [`OcctContext::sew`], `> 0`). Returns the repaired shape and a
    /// [`HealReport`]. Verified empirically, not assumed: this can and
    /// does return `HealReport::is_valid_after == true` for a shape that
    /// was NOT genuinely repaired -- an unclosable Solid (e.g. built from
    /// far fewer faces than it needs) is silently demoted to a bare Shell
    /// by `ShapeFix_Shape` (a Shell has no closure requirement, so it
    /// trivially validates). [`HealReport::kind_changed`] is what
    /// distinguishes this from a genuine repair; `is_valid_after` must
    /// never be read as "healing succeeded" on its own. Healing never
    /// invents missing geometry, and this call has no per-entity lineage
    /// (see [`HealReport`]'s own doc comment for why).
    pub fn heal(&self, tolerance: f64) -> KernelResult<(Shape<'ctx>, HealReport)> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        let mut report = ffi::aicad_heal_report_t::default();
        // SAFETY: `self.context.raw`/`self.raw_handle()`/`&mut handle` as
        // in `is_valid`/`create_box`; `&mut report` is a valid out-param
        // per the header's contract.
        let status = unsafe {
            ffi::aicad_occt_heal(
                self.context.raw,
                self.raw_handle(),
                tolerance,
                &mut handle,
                &mut report,
            )
        };
        status_result(status)?;
        Ok((
            Shape {
                context: self.context,
                id: handle_to_id(handle),
            },
            HealReport {
                changed: report.changed != 0,
                is_valid_before: report.is_valid_before != 0,
                is_valid_after: report.is_valid_after != 0,
                kind_changed: report.kind_changed != 0,
            },
        ))
    }

    /// Linearly extrudes this planar face by `distance` along `direction`
    /// into a solid (AICAD-024).
    pub fn extrude(&self, direction: Direction3, distance: f64) -> KernelResult<Shape<'ctx>> {
        let d = direction.as_vector3();
        let direction = [d.x, d.y, d.z];
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `direction` is a valid, live `[f64; 3]` for the
        // duration of this call; other arguments as in `is_valid`/`create_box`.
        let status = unsafe {
            ffi::aicad_occt_extrude(
                self.context.raw,
                self.raw_handle(),
                direction.as_ptr(),
                distance,
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Revolves this planar face about `axis` by `angle_radians` (0 <
    /// angle <= 2*pi) into a solid (AICAD-024).
    pub fn revolve(&self, axis: Axis3, angle_radians: f64) -> KernelResult<Shape<'ctx>> {
        let origin = [axis.origin.x, axis.origin.y, axis.origin.z];
        let d = axis.direction.as_vector3();
        let direction = [d.x, d.y, d.z];
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `origin`/`direction` are valid, live `[f64; 3]` arrays
        // for the duration of this call; other arguments as in
        // `is_valid`/`create_box`.
        let status = unsafe {
            ffi::aicad_occt_revolve(
                self.context.raw,
                self.raw_handle(),
                origin.as_ptr(),
                direction.as_ptr(),
                angle_radians,
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Sweeps this planar profile face along `spine` (a path wire owned by
    /// the same context), producing a solid (AICAD-025). The spine may be
    /// open or closed but must be G1-continuous; a sharp-cornered spine
    /// may construct a shape that reports invalid rather than being
    /// rejected outright -- construction success and topological validity
    /// are distinct here, matching [`Shape::make_face`]'s own documented
    /// contract (Stage-1 kernel policy #14). See
    /// `project/reports/AICAD-025.md` for exactly which spines are
    /// supported.
    pub fn sweep(&self, spine: &Shape<'ctx>) -> KernelResult<Shape<'ctx>> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `self.context.raw`/`self.raw_handle()`/`&mut handle` as
        // in `is_valid`/`create_box`; `spine.raw_handle()` addresses a
        // slot `spine` owns in the same context (enforced by `'ctx`).
        let status = unsafe {
            ffi::aicad_occt_sweep(
                self.context.raw,
                self.raw_handle(),
                spine.raw_handle(),
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Union (fuse) of `self` and `other` (AICAD-026). Neither operand is
    /// mutated; the result is a new [`Shape`] in the same context (DL-2).
    /// Unlike `extrude`/`revolve`/`sweep`, this does not require a
    /// specific topological input kind: OCCT's own boolean algorithms
    /// always produce a Compound result (verified empirically, not
    /// assumed -- see `project/reports/AICAD-026.md`), so restricting
    /// operands to Solid would make a boolean result un-chainable into a
    /// further boolean operation.
    pub fn union(&self, other: &Shape<'ctx>) -> KernelResult<Shape<'ctx>> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `self.context.raw`/`self.raw_handle()`/`&mut handle` as
        // in `is_valid`/`create_box`; `other.raw_handle()` addresses a
        // slot `other` owns in the same context (enforced by `'ctx`).
        let status = unsafe {
            ffi::aicad_occt_boolean_union(
                self.context.raw,
                self.raw_handle(),
                other.raw_handle(),
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Subtraction: `self` minus `other` (AICAD-026). See [`Shape::union`]
    /// for the operand-kind rationale.
    pub fn cut(&self, other: &Shape<'ctx>) -> KernelResult<Shape<'ctx>> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: see `union` above; identical argument.
        let status = unsafe {
            ffi::aicad_occt_boolean_cut(
                self.context.raw,
                self.raw_handle(),
                other.raw_handle(),
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Intersection (common material) of `self` and `other` (AICAD-026).
    /// See [`Shape::union`] for the operand-kind rationale.
    pub fn intersect(&self, other: &Shape<'ctx>) -> KernelResult<Shape<'ctx>> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: see `union` above; identical argument.
        let status = unsafe {
            ffi::aicad_occt_boolean_intersect(
                self.context.raw,
                self.raw_handle(),
                other.raw_handle(),
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Like [`Shape::union`], but also captures Generated/Modified/
    /// IsDeleted lineage for every unique face/edge of `self`/`other`
    /// (`AICAD-086`) -- needed to give `generated_by`/`modified_by`
    /// (`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §6/§8) real
    /// evidence. The returned [`Lineage`] answers "what became of this
    /// specific face/edge of one of my two original operands?" -- see
    /// that type's own doc comment.
    pub fn union_with_lineage(
        &self,
        other: &Shape<'ctx>,
    ) -> KernelResult<(Shape<'ctx>, Lineage<'ctx>)> {
        self.boolean_with_lineage(other, ffi::aicad_occt_boolean_union_lineage)
    }

    /// Like [`Shape::cut`], but also captures lineage -- see
    /// [`Shape::union_with_lineage`].
    pub fn cut_with_lineage(
        &self,
        other: &Shape<'ctx>,
    ) -> KernelResult<(Shape<'ctx>, Lineage<'ctx>)> {
        self.boolean_with_lineage(other, ffi::aicad_occt_boolean_cut_lineage)
    }

    /// Like [`Shape::intersect`], but also captures lineage -- see
    /// [`Shape::union_with_lineage`].
    pub fn intersect_with_lineage(
        &self,
        other: &Shape<'ctx>,
    ) -> KernelResult<(Shape<'ctx>, Lineage<'ctx>)> {
        self.boolean_with_lineage(other, ffi::aicad_occt_boolean_intersect_lineage)
    }

    /// Shared implementation for `union_with_lineage`/`cut_with_lineage`/
    /// `intersect_with_lineage`: each ABI function has an identical
    /// signature (`context, a, b, out_handle, out_lineage`), differing
    /// only in which underlying Boolean operation runs.
    fn boolean_with_lineage(
        &self,
        other: &Shape<'ctx>,
        raw_fn: unsafe extern "C" fn(
            *mut ffi::aicad_occt_context_t,
            ffi::aicad_shape_handle_t,
            ffi::aicad_shape_handle_t,
            *mut ffi::aicad_shape_handle_t,
            *mut ffi::aicad_lineage_handle_t,
        ) -> c_int,
    ) -> KernelResult<(Shape<'ctx>, Lineage<'ctx>)> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        let mut lineage_handle = ffi::aicad_lineage_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `self.context.raw`/`self.raw_handle()`/`other.raw_handle()`
        // as in `union`; `&mut handle`/`&mut lineage_handle` are valid
        // out-params per the header's contract for every `*_lineage`
        // variant (identical shape to the non-lineage function, plus one
        // more out-param).
        let status = unsafe {
            raw_fn(
                self.context.raw,
                self.raw_handle(),
                other.raw_handle(),
                &mut handle,
                &mut lineage_handle,
            )
        };
        status_result(status)?;
        Ok((
            Shape {
                context: self.context,
                id: handle_to_id(handle),
            },
            Lineage {
                context: self.context,
                handle: lineage_handle,
            },
        ))
    }

    /// The number of unique edges in this shape (AICAD-027), via OCCT's
    /// own de-duplicated `TopExp::MapShapes` (a raw `TopExp_Explorer`
    /// traversal instead revisits each edge once per adjacent face,
    /// verified empirically -- see `project/reports/AICAD-027.md`).
    pub fn edge_count(&self) -> KernelResult<usize> {
        let mut count: usize = 0;
        // SAFETY: `self.context.raw`/`self.raw_handle()` as in `is_valid`;
        // `&mut count` is a valid out-param per the header's contract.
        let status = unsafe {
            ffi::aicad_occt_shape_edge_count(self.context.raw, self.raw_handle(), &mut count)
        };
        status_result(status)?;
        Ok(count)
    }

    /// Returns the edge at `index` (0-based, `< self.edge_count()`) in
    /// this shape's own current raw enumeration order (AICAD-027) --
    /// ephemeral and epoch-bound, never a durable semantic reference
    /// (Stage-1 kernel policies #10-12). Intended for immediate use as a
    /// [`Shape::fillet`]/[`Shape::chamfer`] edge selector, not for
    /// storage.
    pub fn get_edge(&self, index: usize) -> KernelResult<Shape<'ctx>> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `self.context.raw`/`self.raw_handle()`/`&mut handle` as
        // in `is_valid`/`create_box`.
        let status = unsafe {
            ffi::aicad_occt_shape_get_edge(self.context.raw, self.raw_handle(), index, &mut handle)
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Fillets (rounds) `edges` (each obtained from this shape's own
    /// [`Shape::get_edge`]) with a single constant `radius` (AICAD-027).
    /// Accepts any shape kind (not restricted to Solid) -- see
    /// [`Shape::union`] for the same rationale applied here.
    pub fn fillet(&self, edges: &[&Shape<'ctx>], radius: f64) -> KernelResult<Shape<'ctx>> {
        let handles: Vec<ffi::aicad_shape_handle_t> =
            edges.iter().map(|edge| id_to_handle(edge.id)).collect();
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `handles` is a valid, live, contiguous array of
        // `handles.len()` elements for the duration of this call (no STL
        // container crosses the boundary), matching
        // `aicad_occt_make_wire_from_edges`' convention; `self.context.raw`/
        // `self.raw_handle()`/`&mut handle` as in `is_valid`/`create_box`.
        let status = unsafe {
            ffi::aicad_occt_fillet(
                self.context.raw,
                self.raw_handle(),
                handles.as_ptr(),
                handles.len(),
                radius,
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Chamfers `edges` (each obtained from this shape's own
    /// [`Shape::get_edge`]) with a single constant symmetric `distance`
    /// (AICAD-027). See [`Shape::fillet`] for the operand-kind rationale.
    pub fn chamfer(&self, edges: &[&Shape<'ctx>], distance: f64) -> KernelResult<Shape<'ctx>> {
        let handles: Vec<ffi::aicad_shape_handle_t> =
            edges.iter().map(|edge| id_to_handle(edge.id)).collect();
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: see `fillet` above; identical argument.
        let status = unsafe {
            ffi::aicad_occt_chamfer(
                self.context.raw,
                self.raw_handle(),
                handles.as_ptr(),
                handles.len(),
                distance,
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Like [`Shape::fillet`], but also captures Generated/Modified/
    /// IsDeleted lineage for every unique face/edge of `self`
    /// (`AICAD-086`) -- see [`Shape::union_with_lineage`].
    pub fn fillet_with_lineage(
        &self,
        edges: &[&Shape<'ctx>],
        radius: f64,
    ) -> KernelResult<(Shape<'ctx>, Lineage<'ctx>)> {
        let handles: Vec<ffi::aicad_shape_handle_t> =
            edges.iter().map(|edge| id_to_handle(edge.id)).collect();
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        let mut lineage_handle = ffi::aicad_lineage_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: see `fillet`'s own SAFETY comment; `&mut lineage_handle`
        // is a valid out-param, matching `aicad_occt_fillet_lineage`'s
        // own contract.
        let status = unsafe {
            ffi::aicad_occt_fillet_lineage(
                self.context.raw,
                self.raw_handle(),
                handles.as_ptr(),
                handles.len(),
                radius,
                &mut handle,
                &mut lineage_handle,
            )
        };
        status_result(status)?;
        Ok((
            Shape {
                context: self.context,
                id: handle_to_id(handle),
            },
            Lineage {
                context: self.context,
                handle: lineage_handle,
            },
        ))
    }

    /// Like [`Shape::chamfer`], but also captures lineage -- see
    /// [`Shape::fillet_with_lineage`].
    pub fn chamfer_with_lineage(
        &self,
        edges: &[&Shape<'ctx>],
        distance: f64,
    ) -> KernelResult<(Shape<'ctx>, Lineage<'ctx>)> {
        let handles: Vec<ffi::aicad_shape_handle_t> =
            edges.iter().map(|edge| id_to_handle(edge.id)).collect();
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        let mut lineage_handle = ffi::aicad_lineage_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: see `fillet_with_lineage` above; identical argument.
        let status = unsafe {
            ffi::aicad_occt_chamfer_lineage(
                self.context.raw,
                self.raw_handle(),
                handles.as_ptr(),
                handles.len(),
                distance,
                &mut handle,
                &mut lineage_handle,
            )
        };
        status_result(status)?;
        Ok((
            Shape {
                context: self.context,
                id: handle_to_id(handle),
            },
            Lineage {
                context: self.context,
                handle: lineage_handle,
            },
        ))
    }

    /// The number of unique faces in this shape (AICAD-028), via OCCT's
    /// own de-duplicated `TopExp::MapShapes`.
    pub fn face_count(&self) -> KernelResult<usize> {
        let mut count: usize = 0;
        // SAFETY: `self.context.raw`/`self.raw_handle()` as in `is_valid`;
        // `&mut count` is a valid out-param per the header's contract.
        let status = unsafe {
            ffi::aicad_occt_shape_face_count(self.context.raw, self.raw_handle(), &mut count)
        };
        status_result(status)?;
        Ok(count)
    }

    /// Returns the face at `index` (0-based, `< self.face_count()`) in
    /// this shape's own current raw enumeration order (AICAD-028) --
    /// ephemeral and epoch-bound, never a durable semantic reference, per
    /// the same contract as [`Shape::get_edge`]. Intended for immediate
    /// use as a [`Shape::shell`] face-to-remove selector, not for
    /// storage.
    pub fn get_face(&self, index: usize) -> KernelResult<Shape<'ctx>> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `self.context.raw`/`self.raw_handle()`/`&mut handle` as
        // in `is_valid`/`create_box`.
        let status = unsafe {
            ffi::aicad_occt_shape_get_face(self.context.raw, self.raw_handle(), index, &mut handle)
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// The number of unique shells in this shape (`AICAD-100A`), via
    /// OCCT's own de-duplicated `TopExp::MapShapes` over `TopAbs_SHELL` --
    /// same pattern as [`Shape::face_count`], applied to the next topology
    /// kind up. Closes the Stage-4 gap `crate::reference_replay::
    /// candidates_of_kind`'s own prior doc comment named: no bridge
    /// accessor existed to enumerate a shape's own sub-shells, so a
    /// `ShellRef` query against real geometry could never find a
    /// candidate at all.
    pub fn shell_count(&self) -> KernelResult<usize> {
        let mut count: usize = 0;
        // SAFETY: `self.context.raw`/`self.raw_handle()` as in `is_valid`;
        // `&mut count` is a valid out-param per the header's contract.
        let status = unsafe {
            ffi::aicad_occt_shape_shell_count(self.context.raw, self.raw_handle(), &mut count)
        };
        status_result(status)?;
        Ok(count)
    }

    /// Returns the shell at `index` (0-based, `< self.shell_count()`) in
    /// this shape's own current raw enumeration order (`AICAD-100A`) --
    /// ephemeral and epoch-bound, never a durable semantic reference, per
    /// the same contract as [`Shape::get_face`].
    pub fn get_shell(&self, index: usize) -> KernelResult<Shape<'ctx>> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `self.context.raw`/`self.raw_handle()`/`&mut handle` as
        // in `is_valid`/`create_box`.
        let status = unsafe {
            ffi::aicad_occt_shape_get_shell(self.context.raw, self.raw_handle(), index, &mut handle)
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// The number of unique solids in this shape (`AICAD-100A`), via
    /// OCCT's own de-duplicated `TopExp::MapShapes` over `TopAbs_SOLID` --
    /// same pattern as [`Shape::shell_count`].
    pub fn solid_count(&self) -> KernelResult<usize> {
        let mut count: usize = 0;
        // SAFETY: `self.context.raw`/`self.raw_handle()` as in `is_valid`;
        // `&mut count` is a valid out-param per the header's contract.
        let status = unsafe {
            ffi::aicad_occt_shape_solid_count(self.context.raw, self.raw_handle(), &mut count)
        };
        status_result(status)?;
        Ok(count)
    }

    /// Returns the solid at `index` (0-based, `< self.solid_count()`) in
    /// this shape's own current raw enumeration order (`AICAD-100A`) --
    /// ephemeral and epoch-bound, never a durable semantic reference, per
    /// the same contract as [`Shape::get_face`].
    pub fn get_solid(&self, index: usize) -> KernelResult<Shape<'ctx>> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `self.context.raw`/`self.raw_handle()`/`&mut handle` as
        // in `is_valid`/`create_box`.
        let status = unsafe {
            ffi::aicad_occt_shape_get_solid(self.context.raw, self.raw_handle(), index, &mut handle)
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Hollows this shape into a shell of constant wall `thickness`,
    /// removing (opening) `faces_to_remove` (each obtained from this
    /// shape's own [`Shape::get_face`]) (AICAD-028). `thickness`'s sign
    /// selects which side of the original surface the hollow is built on
    /// (negative: hollow the interior out, the common "shell" case).
    /// Accepts any shape kind, matching [`Shape::union`]'s rationale.
    pub fn shell(
        &self,
        faces_to_remove: &[&Shape<'ctx>],
        thickness: f64,
    ) -> KernelResult<Shape<'ctx>> {
        let handles: Vec<ffi::aicad_shape_handle_t> = faces_to_remove
            .iter()
            .map(|face| id_to_handle(face.id))
            .collect();
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `handles` is a valid, live, contiguous array of
        // `handles.len()` elements for the duration of this call (no STL
        // container crosses the boundary); `self.context.raw`/
        // `self.raw_handle()`/`&mut handle` as in `is_valid`/`create_box`.
        let status = unsafe {
            ffi::aicad_occt_shell(
                self.context.raw,
                self.raw_handle(),
                handles.as_ptr(),
                handles.len(),
                thickness,
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Constructs a shape parallel to this shape's boundary, offset by
    /// `distance` (positive: outside; negative: inside) (AICAD-028). For
    /// a convex solid, a positive-distance offset is geometrically
    /// equivalent to filleting every edge with that distance as radius
    /// (both are the Minkowski sum of the solid with a ball of that
    /// radius) -- see `project/reports/AICAD-028.md`.
    pub fn offset(&self, distance: f64) -> KernelResult<Shape<'ctx>> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `self.context.raw`/`self.raw_handle()`/`&mut handle` as
        // in `is_valid`/`create_box`.
        let status = unsafe {
            ffi::aicad_occt_offset(self.context.raw, self.raw_handle(), distance, &mut handle)
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// The number of unique vertices in this shape (AICAD-029), via OCCT's
    /// own de-duplicated `TopExp::MapShapes` -- matching
    /// [`Shape::edge_count`]/[`Shape::face_count`]'s own rationale.
    pub fn vertex_count(&self) -> KernelResult<usize> {
        let mut count: usize = 0;
        // SAFETY: `self.context.raw`/`self.raw_handle()` as in `is_valid`;
        // `&mut count` is a valid out-param per the header's contract.
        let status = unsafe {
            ffi::aicad_occt_shape_vertex_count(self.context.raw, self.raw_handle(), &mut count)
        };
        status_result(status)?;
        Ok(count)
    }

    /// Returns the vertex at `index` (0-based, `< self.vertex_count()`) in
    /// this shape's own current raw enumeration order (AICAD-029) --
    /// ephemeral and epoch-bound, never a durable semantic reference, per
    /// the same contract as [`Shape::get_edge`]/[`Shape::get_face`].
    pub fn get_vertex(&self, index: usize) -> KernelResult<Shape<'ctx>> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `self.context.raw`/`self.raw_handle()`/`&mut handle` as
        // in `is_valid`/`create_box`.
        let status = unsafe {
            ffi::aicad_occt_shape_get_vertex(
                self.context.raw,
                self.raw_handle(),
                index,
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// This edge's two endpoint vertices, in the edge's own orientation
    /// order (AICAD-029). `self` must address a shape of exactly kind
    /// Edge. For a closed edge (e.g. a full circle) both returned vertices
    /// coincide -- callers must not assume distinctness.
    pub fn edge_vertices(&self) -> KernelResult<(Shape<'ctx>, Shape<'ctx>)> {
        let mut v0 = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        let mut v1 = v0;
        // SAFETY: `self.context.raw`/`self.raw_handle()` as in `is_valid`;
        // `&mut v0`/`&mut v1` are valid out-params per the header's
        // contract.
        let status = unsafe {
            ffi::aicad_occt_edge_vertices(self.context.raw, self.raw_handle(), &mut v0, &mut v1)
        };
        status_result(status)?;
        Ok((
            Shape {
                context: self.context,
                id: handle_to_id(v0),
            },
            Shape {
                context: self.context,
                id: handle_to_id(v1),
            },
        ))
    }

    /// The number of faces of this shape adjacent to (bounded by) the edge
    /// at `edge_index` (0-based, `< self.edge_count()`, per that same
    /// function's own enumeration order) (AICAD-029).
    pub fn edge_adjacent_face_count(&self, edge_index: usize) -> KernelResult<usize> {
        let mut count: usize = 0;
        // SAFETY: `self.context.raw`/`self.raw_handle()` as in `is_valid`;
        // `&mut count` is a valid out-param per the header's contract.
        let status = unsafe {
            ffi::aicad_occt_shape_edge_adjacent_face_count(
                self.context.raw,
                self.raw_handle(),
                edge_index,
                &mut count,
            )
        };
        status_result(status)?;
        Ok(count)
    }

    /// Returns the `adjacent_index`-th (0-based, `<
    /// self.edge_adjacent_face_count(edge_index)`) face adjacent to the
    /// edge at `edge_index` (AICAD-029) -- ephemeral and epoch-bound,
    /// never a durable semantic reference.
    pub fn edge_adjacent_face(
        &self,
        edge_index: usize,
        adjacent_index: usize,
    ) -> KernelResult<Shape<'ctx>> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: `self.context.raw`/`self.raw_handle()`/`&mut handle` as
        // in `is_valid`/`create_box`.
        let status = unsafe {
            ffi::aicad_occt_shape_edge_adjacent_face_get(
                self.context.raw,
                self.raw_handle(),
                edge_index,
                adjacent_index,
                &mut handle,
            )
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Total length of every unique edge in this shape (AICAD-030), via
    /// OCCT's own `BRepGProp::LinearProperties` with `SkipShared=true` --
    /// matching [`Shape::edge_count`]'s own "unique edges" scope (without
    /// it, an edge shared by 2 faces is counted twice).
    pub fn length(&self) -> KernelResult<f64> {
        let mut length: f64 = 0.0;
        // SAFETY: `self.context.raw`/`self.raw_handle()` as in `is_valid`;
        // `&mut length` is a valid out-param per the header's contract.
        let status = unsafe {
            ffi::aicad_occt_shape_length(self.context.raw, self.raw_handle(), &mut length)
        };
        status_result(status)?;
        Ok(length)
    }

    /// Center of mass of this shape's own highest-dimensional content
    /// (AICAD-030): volume-weighted if it contains any Solid, else
    /// area-weighted if it contains any Face, else length-weighted over
    /// its Edges. Fails with [`KernelError::OperationFailed`] if the
    /// shape has none of these (e.g. a bare Vertex).
    pub fn center_of_mass(&self) -> KernelResult<Point3> {
        let mut centre = [0.0; 3];
        // SAFETY: `self.context.raw`/`self.raw_handle()` as in `is_valid`;
        // `&mut centre` is a valid 3-element out-param per the header's
        // contract.
        let status = unsafe {
            ffi::aicad_occt_shape_center_of_mass(
                self.context.raw,
                self.raw_handle(),
                centre.as_mut_ptr(),
            )
        };
        status_result(status)?;
        Ok(Point3::new(centre[0], centre[1], centre[2]))
    }

    /// A normalized B-rep validation report (AICAD-031): overall validity
    /// plus a per-topological-kind breakdown of invalid subshapes, via
    /// OCCT's own `BRepCheck_Analyzer` queried per unique vertex/edge/
    /// wire/face. Unlike [`Shape::is_valid`], this never itself indicates
    /// failure for an invalid shape -- validity is data in the returned
    /// report, not a rejected call (only a genuine bridge/argument error
    /// produces `Err`).
    pub fn validate(&self) -> KernelResult<ValidationReport> {
        let mut report = ffi::aicad_validation_report_t::default();
        // SAFETY: `self.context.raw`/`self.raw_handle()` as in `is_valid`;
        // `&mut report` is a valid out-param per the header's contract.
        let status = unsafe {
            ffi::aicad_occt_shape_validate(self.context.raw, self.raw_handle(), &mut report)
        };
        status_result(status)?;
        Ok(ValidationReport {
            is_valid: report.is_valid != 0,
            invalid_vertex_count: report.invalid_vertex_count,
            invalid_edge_count: report.invalid_edge_count,
            invalid_wire_count: report.invalid_wire_count,
            invalid_face_count: report.invalid_face_count,
            invalid_shell_count: report.invalid_shell_count,
            invalid_solid_count: report.invalid_solid_count,
        })
    }

    /// Tessellates this shape into a flat-shaded triangle-soup mesh
    /// (AICAD-032), via `BRepMesh_IncrementalMesh` at the given
    /// (absolute) linear/angular deflections. Hides the native two-call
    /// count-then-fetch protocol behind one Rust call; each call
    /// re-tessellates from scratch (no caching is exposed at this level).
    pub fn tessellate(
        &self,
        linear_deflection: f64,
        angular_deflection: f64,
    ) -> KernelResult<TriangleMesh> {
        let mut counts = ffi::aicad_tessellation_counts_t::default();
        // SAFETY: `self.context.raw`/`self.raw_handle()` as in `is_valid`;
        // `&mut counts` is a valid out-param per the header's contract.
        let status = unsafe {
            ffi::aicad_occt_tessellate(
                self.context.raw,
                self.raw_handle(),
                linear_deflection,
                angular_deflection,
                &mut counts,
            )
        };
        status_result(status)?;
        let triangle_count = counts.triangle_count;
        let mut raw_vertices = vec![0.0_f64; 9 * triangle_count];
        let mut raw_normals = vec![0.0_f64; 9 * triangle_count];
        // SAFETY: `raw_vertices`/`raw_normals` are each a valid, live,
        // `9 * triangle_count`-element `f64` buffer for the duration of
        // this call, matching aicad_occt_tessellation_get's documented
        // buffer-size contract for the SAME handle's just-cached
        // tessellation; `self.context.raw`/`self.raw_handle()` as above.
        let status = unsafe {
            ffi::aicad_occt_tessellation_get(
                self.context.raw,
                self.raw_handle(),
                raw_vertices.as_mut_ptr(),
                raw_normals.as_mut_ptr(),
            )
        };
        status_result(status)?;
        let vertices = raw_vertices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|c| Point3::new(c[0], c[1], c[2]))
            .collect();
        let normals = raw_normals
            .as_chunks::<3>()
            .0
            .iter()
            .map(|c| cad_kernel_api::Vector3::new(c[0], c[1], c[2]))
            .collect();
        Ok(TriangleMesh { vertices, normals })
    }

    /// Exports this shape to `path` as an AP214 STEP file via OCCT's own
    /// `STEPControl_Writer` (AICAD-033). Returns
    /// [`KernelError::InvalidArgument`] if `path` cannot be represented
    /// as a null-terminated C string (e.g. it contains an embedded NUL
    /// byte, or is not valid UTF-8/OS-string-representable as such).
    pub fn export_step(&self, path: &std::path::Path) -> KernelResult<()> {
        let path_str = path.to_str().ok_or(KernelError::InvalidArgument)?;
        let c_path = std::ffi::CString::new(path_str).map_err(|_| KernelError::InvalidArgument)?;
        // SAFETY: `c_path` is a valid, live, null-terminated C string for
        // the duration of this call; `self.context.raw`/`self.raw_handle()`
        // as in `is_valid`.
        let status = unsafe {
            ffi::aicad_occt_export_step(self.context.raw, self.raw_handle(), c_path.as_ptr())
        };
        status_result(status)
    }

    /// This face's underlying surface family (`AICAD-082`), via OCCT's own
    /// `BRepAdaptor_Surface::GetType()`. `self` must address a shape of
    /// exactly kind Face -- [`KernelError::InvalidArgument`] otherwise.
    pub fn surface_type(&self) -> KernelResult<SurfaceKind> {
        let mut kind: c_int = 0;
        // SAFETY: see `is_valid`'s SAFETY comment; identical argument;
        // `&mut kind` is a valid out-param per the header's contract.
        let status = unsafe {
            ffi::aicad_occt_shape_surface_type(self.context.raw, self.raw_handle(), &mut kind)
        };
        status_result(status)?;
        Ok(SurfaceKind::from_raw(kind))
    }

    /// This face's single characteristic radius (`AICAD-082`) -- defined
    /// only for [`SurfaceKind::Cylinder`]/[`SurfaceKind::Sphere`] (their
    /// one radius) and [`SurfaceKind::Torus`] (its major/tube-path
    /// radius; the torus's own minor radius is not exposed here). Any
    /// other surface kind (including [`SurfaceKind::Cone`], whose radius
    /// varies continuously along its axis) fails with
    /// [`KernelError::InvalidArgument`] rather than guessing which value
    /// to report.
    pub fn face_radius(&self) -> KernelResult<f64> {
        let mut radius: f64 = 0.0;
        // SAFETY: see `is_valid`'s SAFETY comment; identical argument.
        let status = unsafe {
            ffi::aicad_occt_shape_face_radius(self.context.raw, self.raw_handle(), &mut radius)
        };
        status_result(status)?;
        Ok(radius)
    }

    /// This face's rotational axis (`AICAD-082`) -- defined only for
    /// [`SurfaceKind::Cylinder`]/[`SurfaceKind::Cone`]/[`SurfaceKind::Torus`];
    /// any other surface kind fails with [`KernelError::InvalidArgument`].
    pub fn face_axis(&self) -> KernelResult<Axis3> {
        let mut origin = [0.0; 3];
        let mut direction = [0.0; 3];
        // SAFETY: see `is_valid`'s SAFETY comment; `&mut origin`/`&mut
        // direction` are each a valid 3-element out-param per the
        // header's contract.
        let status = unsafe {
            ffi::aicad_occt_shape_face_axis(
                self.context.raw,
                self.raw_handle(),
                origin.as_mut_ptr(),
                direction.as_mut_ptr(),
            )
        };
        status_result(status)?;
        let direction = cad_kernel_api::Vector3::new(direction[0], direction[1], direction[2])
            .normalize()
            .ok_or(KernelError::Internal)?;
        Ok(Axis3::new(
            Point3::new(origin[0], origin[1], origin[2]),
            direction,
        ))
    }

    /// A representative point on this face (its own parametric-domain
    /// midpoint, not an area centroid) and the face's own outward unit
    /// normal there, already corrected for this face's orientation
    /// (`AICAD-082`). Fails with [`KernelError::OperationFailed`] if the
    /// surface is singular at that exact parameter (e.g. a cone apex).
    pub fn face_normal(&self) -> KernelResult<(Point3, Direction3)> {
        let mut point = [0.0; 3];
        let mut normal = [0.0; 3];
        // SAFETY: see `face_axis` above; identical argument shape.
        let status = unsafe {
            ffi::aicad_occt_shape_face_normal(
                self.context.raw,
                self.raw_handle(),
                point.as_mut_ptr(),
                normal.as_mut_ptr(),
            )
        };
        status_result(status)?;
        let normal = cad_kernel_api::Vector3::new(normal[0], normal[1], normal[2])
            .normalize()
            .ok_or(KernelError::Internal)?;
        Ok((Point3::new(point[0], point[1], point[2]), normal))
    }

    /// This edge's underlying curve family (`AICAD-082`), via OCCT's own
    /// `BRepAdaptor_Curve::GetType()`. `self` must address a shape of
    /// exactly kind Edge -- [`KernelError::InvalidArgument`] otherwise.
    pub fn curve_type(&self) -> KernelResult<CurveKind> {
        let mut kind: c_int = 0;
        // SAFETY: see `is_valid`'s SAFETY comment; identical argument.
        let status = unsafe {
            ffi::aicad_occt_shape_curve_type(self.context.raw, self.raw_handle(), &mut kind)
        };
        status_result(status)?;
        Ok(CurveKind::from_raw(kind))
    }

    /// This edge's radius (`AICAD-082`) -- defined only for
    /// [`CurveKind::Circle`]; an ellipse has two distinct radii with no
    /// single "the" radius, so every other curve kind fails with
    /// [`KernelError::InvalidArgument`].
    pub fn edge_radius(&self) -> KernelResult<f64> {
        let mut radius: f64 = 0.0;
        // SAFETY: see `is_valid`'s SAFETY comment; identical argument.
        let status = unsafe {
            ffi::aicad_occt_shape_edge_radius(self.context.raw, self.raw_handle(), &mut radius)
        };
        status_result(status)?;
        Ok(radius)
    }

    /// This edge's axis (`AICAD-082`) -- the normal to the circle's own
    /// plane through its center. Defined only for [`CurveKind::Circle`].
    pub fn edge_axis(&self) -> KernelResult<Axis3> {
        let mut origin = [0.0; 3];
        let mut direction = [0.0; 3];
        // SAFETY: see `face_axis` above; identical argument shape.
        let status = unsafe {
            ffi::aicad_occt_shape_edge_axis(
                self.context.raw,
                self.raw_handle(),
                origin.as_mut_ptr(),
                direction.as_mut_ptr(),
            )
        };
        status_result(status)?;
        let direction = cad_kernel_api::Vector3::new(direction[0], direction[1], direction[2])
            .normalize()
            .ok_or(KernelError::Internal)?;
        Ok(Axis3::new(
            Point3::new(origin[0], origin[1], origin[2]),
            direction,
        ))
    }

    /// Whether `self` and `other` denote the same underlying topological
    /// entity (`AICAD-083`), via OCCT's own `TopoDS_Shape::IsSame`
    /// (TShape + Location, ignoring Orientation) -- needed because two
    /// independently obtained handles (e.g. from [`Shape::get_face`] vs.
    /// [`Shape::edge_adjacent_face`]) can address the same face with
    /// different raw slots. `self`/`other` need not share a context to
    /// call this safely, but always report `false` when they don't (an
    /// entity from a different context/build can never be "the same"
    /// entity, matching Stage-1 kernel policy #10's epoch-bound handles).
    pub fn is_same(&self, other: &Shape<'_>) -> KernelResult<bool> {
        // SAFETY: `self.context.raw`/`self.raw_handle()` as in `is_valid`;
        // `other.raw_handle()` addresses a slot `other` owns in its own
        // (possibly different) context -- `aicad_occt_shape_is_same`
        // looks each handle up against `context` independently and
        // reports INVALID_HANDLE/FOREIGN_CONTEXT rather than crossing
        // contexts unsafely, so passing a foreign-context handle here is
        // memory-safe even though it is rejected below.
        let mut is_same: c_int = 0;
        let status = unsafe {
            ffi::aicad_occt_shape_is_same(
                self.context.raw,
                self.raw_handle(),
                other.raw_handle(),
                &mut is_same,
            )
        };
        match status_result(status) {
            Ok(()) => Ok(is_same != 0),
            // A handle from a different context is never "the same"
            // entity -- reported as `Ok(false)`, not an error, matching
            // this method's own doc comment.
            Err(KernelError::ForeignContext) => Ok(false),
            Err(other) => Err(other),
        }
    }

    /// Number of unique wires in this shape (`AICAD-083`), matching
    /// [`Shape::face_count`]'s own "any shape kind" scope.
    pub fn wire_count(&self) -> KernelResult<usize> {
        let mut count: usize = 0;
        // SAFETY: see `is_valid`'s SAFETY comment; identical argument.
        let status = unsafe {
            ffi::aicad_occt_shape_wire_count(self.context.raw, self.raw_handle(), &mut count)
        };
        status_result(status)?;
        Ok(count)
    }

    /// Returns the wire at `index` (0-based, `< self.wire_count()`) in
    /// this shape's own current raw enumeration order (`AICAD-083`) --
    /// ephemeral and epoch-bound, matching [`Shape::get_face`]'s own
    /// contract.
    pub fn get_wire(&self, index: usize) -> KernelResult<Shape<'ctx>> {
        let mut handle = ffi::aicad_shape_handle_t {
            context_id: 0,
            slot: 0,
            generation: 0,
        };
        // SAFETY: see `is_valid`'s SAFETY comment; `&mut handle` as in
        // `create_box`.
        let status = unsafe {
            ffi::aicad_occt_shape_get_wire(self.context.raw, self.raw_handle(), index, &mut handle)
        };
        status_result(status)?;
        Ok(Shape {
            context: self.context,
            id: handle_to_id(handle),
        })
    }

    /// Whether `wire` is `self`'s own designated OUTER wire (`AICAD-083`)
    /// -- false for any inner (hole) wire, and false if `wire` does not
    /// bound `self` at all. `self` must address a shape of exactly kind
    /// Face; `wire` must address a shape of exactly kind Wire.
    pub fn is_outer_wire(&self, wire: &Shape<'ctx>) -> KernelResult<bool> {
        let mut is_outer: c_int = 0;
        // SAFETY: see `is_valid`'s SAFETY comment; `wire.raw_handle()`
        // addresses a slot `wire` owns in the same context (enforced by
        // `'ctx`).
        let status = unsafe {
            ffi::aicad_occt_shape_is_outer_wire(
                self.context.raw,
                self.raw_handle(),
                wire.raw_handle(),
                &mut is_outer,
            )
        };
        status_result(status)?;
        Ok(is_outer != 0)
    }

    /// This vertex's own coordinate (`AICAD-084`) -- unlike
    /// [`Shape::center_of_mass`] (which fails for a bare Vertex), this is
    /// defined exactly because a vertex's "center" is just its own point.
    /// `self` must address a shape of exactly kind Vertex.
    pub fn vertex_point(&self) -> KernelResult<Point3> {
        let mut point = [0.0; 3];
        // SAFETY: see `is_valid`'s SAFETY comment; `&mut point` is a
        // valid 3-element out-param per the header's contract.
        let status = unsafe {
            ffi::aicad_occt_shape_vertex_point(
                self.context.raw,
                self.raw_handle(),
                point.as_mut_ptr(),
            )
        };
        status_result(status)?;
        Ok(Point3::new(point[0], point[1], point[2]))
    }

    /// Exact point-vs-solid classification (`AICAD-084`), via OCCT's own
    /// `BRepClass3d_SolidClassifier` -- never a mesh/bounding-box
    /// approximation. `self` must address a shape containing at least one
    /// Solid; `tolerance` must be finite and `> 0.0`.
    pub fn classify_point(
        &self,
        point: Point3,
        tolerance: f64,
    ) -> KernelResult<PointClassification> {
        let point = [point.x, point.y, point.z];
        let mut classification: c_int = 0;
        // SAFETY: `point` is a valid, live `[f64; 3]` for the duration of
        // this call; `self.context.raw`/`self.raw_handle()` as in
        // `is_valid`; `&mut classification` is a valid out-param.
        let status = unsafe {
            ffi::aicad_occt_shape_classify_point(
                self.context.raw,
                self.raw_handle(),
                point.as_ptr(),
                tolerance,
                &mut classification,
            )
        };
        status_result(status)?;
        Ok(PointClassification::from_raw(classification))
    }

    /// This shape's own top-level topological kind (`AICAD-121`) —
    /// kernel-neutral (`cad_kernel_api::topology::TopologyKind`), never an
    /// OCCT `TopAbs_ShapeEnum` value. Fails with
    /// [`KernelError::OperationFailed`] for a Compound/CompSolid/generic-
    /// Shape top-level kind, which has no single classifiable entity kind.
    pub fn topology_kind(&self) -> KernelResult<TopologyKind> {
        let mut kind: c_int = 0;
        // SAFETY: see `is_valid`'s SAFETY comment; `&mut kind` is a valid
        // out-param per the header's contract.
        let status =
            unsafe { ffi::aicad_occt_shape_kind(self.context.raw, self.raw_handle(), &mut kind) };
        status_result(status)?;
        match kind {
            0 => Ok(TopologyKind::Vertex),
            1 => Ok(TopologyKind::Edge),
            2 => Ok(TopologyKind::Wire),
            3 => Ok(TopologyKind::Face),
            4 => Ok(TopologyKind::Shell),
            5 => Ok(TopologyKind::Solid),
            // The native side only ever returns AICAD_OCCT_OK alongside
            // one of the six values above (anything else is
            // AICAD_OCCT_ERR_OPERATION_FAILED, already mapped by
            // `status_result` above) -- an unrecognized value here would
            // be a bridge defect, not an expected outcome.
            _ => Err(KernelError::Internal),
        }
    }

    /// Whether this shape's own top-level `TopAbs_Orientation` is FORWARD
    /// (`AICAD-121`) -- REVERSED, INTERNAL, and EXTERNAL (the latter two
    /// are rare seam/degenerate-edge markers) all report `false`. A
    /// deliberate, disclosed simplification -- see
    /// `aicad_occt_shape_is_forward_oriented`'s own doc comment.
    pub fn is_forward_oriented(&self) -> KernelResult<bool> {
        let mut is_forward: c_int = 0;
        // SAFETY: see `is_valid`'s SAFETY comment; `&mut is_forward` is a
        // valid out-param per the header's contract.
        let status = unsafe {
            ffi::aicad_occt_shape_is_forward_oriented(
                self.context.raw,
                self.raw_handle(),
                &mut is_forward,
            )
        };
        status_result(status)?;
        Ok(is_forward != 0)
    }

    fn raw_handle(&self) -> ffi::aicad_shape_handle_t {
        id_to_handle(self.id)
    }
}

/// One operation's own captured Generated/Modified/IsDeleted lineage
/// (`AICAD-086`), returned alongside the result [`Shape`] by
/// [`Shape::union_with_lineage`]/`cut_with_lineage`/`intersect_with_lineage`/
/// `fillet_with_lineage`/`chamfer_with_lineage`. Answers, for a specific
/// face/edge of one of that operation's own *original input* shapes
/// (never the result shape): was it deleted, what did it generate, and
/// what did it get modified into?
///
/// This is deliberately a snapshot, not a live query against the OCCT
/// builder that performed the operation -- that builder is a C++ local
/// variable inside the native function that produced this `Lineage` and
/// no longer exists by the time this type's methods run (see `native/
/// occt_bridge/src/aicad_occt_bridge.cpp`'s own `LineageEntry` doc
/// comment). Automatically released (`aicad_occt_release_lineage`) when
/// dropped, exactly like [`Shape`].
#[derive(Debug)]
pub struct Lineage<'ctx> {
    context: &'ctx OcctContext,
    handle: ffi::aicad_lineage_handle_t,
}

impl<'ctx> Lineage<'ctx> {
    /// Whether `input` (a face/edge [`Shape`] obtained from one of this
    /// lineage's own two original operands, via a handle obtained
    /// *before* the operation ran) has no surviving generated/modified
    /// counterpart in the operation's result -- e.g. a face entirely
    /// consumed by a boolean cut. `input` must belong to the same
    /// operation this lineage was captured from; a face/edge from an
    /// unrelated shape returns [`KernelError::InvalidArgument`], never a
    /// guessed `false` -- see `ResolveLineageEntry`'s own native doc
    /// comment for why "no evidence" and "evidenced not-deleted" are kept
    /// distinct.
    pub fn is_deleted(&self, input: &Shape<'ctx>) -> KernelResult<bool> {
        let mut is_deleted: c_int = 0;
        // SAFETY: `self.context.raw` is valid for `'ctx`; `self.handle`
        // addresses a lineage slot this `Lineage` owns and has not yet
        // released; `input.raw_handle()` addresses a slot `input` owns in
        // the same context; `&mut is_deleted` is a valid out-param.
        let status = unsafe {
            ffi::aicad_occt_lineage_is_deleted(
                self.context.raw,
                self.handle,
                input.raw_handle(),
                &mut is_deleted,
            )
        };
        status_result(status)?;
        Ok(is_deleted != 0)
    }

    /// Every shape `input` was generated into by this operation (OCCT's
    /// own `Generated(input)`) -- e.g. a new face created where a hole
    /// broke through an existing face. Empty (not an error) if `input`
    /// has no generated counterpart, including when it was deleted. See
    /// [`Lineage::is_deleted`] for `input`'s own membership requirement.
    pub fn generated(&self, input: &Shape<'ctx>) -> KernelResult<Vec<Shape<'ctx>>> {
        self.shape_list(
            input,
            ffi::aicad_occt_lineage_generated_count,
            ffi::aicad_occt_lineage_generated_get,
        )
    }

    /// Every shape `input` was modified into by this operation (OCCT's
    /// own `Modified(input)`) -- `input` carried forward as a
    /// geometrically changed (but not newly created) counterpart, e.g. a
    /// face re-trimmed by a boolean cut. See [`Lineage::generated`].
    pub fn modified(&self, input: &Shape<'ctx>) -> KernelResult<Vec<Shape<'ctx>>> {
        self.shape_list(
            input,
            ffi::aicad_occt_lineage_modified_count,
            ffi::aicad_occt_lineage_modified_get,
        )
    }

    /// Shared count-then-index-each implementation for
    /// [`Lineage::generated`]/[`Lineage::modified`] -- both native query
    /// pairs share an identical count/get shape.
    fn shape_list(
        &self,
        input: &Shape<'ctx>,
        count_fn: unsafe extern "C" fn(
            *mut ffi::aicad_occt_context_t,
            ffi::aicad_lineage_handle_t,
            ffi::aicad_shape_handle_t,
            *mut usize,
        ) -> c_int,
        get_fn: unsafe extern "C" fn(
            *mut ffi::aicad_occt_context_t,
            ffi::aicad_lineage_handle_t,
            ffi::aicad_shape_handle_t,
            usize,
            *mut ffi::aicad_shape_handle_t,
        ) -> c_int,
    ) -> KernelResult<Vec<Shape<'ctx>>> {
        let mut count: usize = 0;
        // SAFETY: see `is_deleted`'s SAFETY comment; `&mut count` is a
        // valid out-param.
        let status = unsafe {
            count_fn(
                self.context.raw,
                self.handle,
                input.raw_handle(),
                &mut count,
            )
        };
        status_result(status)?;
        let mut out = Vec::with_capacity(count);
        for index in 0..count {
            let mut handle = ffi::aicad_shape_handle_t {
                context_id: 0,
                slot: 0,
                generation: 0,
            };
            // SAFETY: see `is_deleted`'s SAFETY comment; `index < count`
            // from the successful `count_fn` call above; `&mut handle` is
            // a valid out-param.
            let status = unsafe {
                get_fn(
                    self.context.raw,
                    self.handle,
                    input.raw_handle(),
                    index,
                    &mut handle,
                )
            };
            status_result(status)?;
            out.push(Shape {
                context: self.context,
                id: handle_to_id(handle),
            });
        }
        Ok(out)
    }
}

impl<'ctx> Drop for Lineage<'ctx> {
    fn drop(&mut self) {
        // SAFETY: see `Shape`'s own `Drop` impl -- identical argument, a
        // different table on the same context.
        let _ = unsafe { ffi::aicad_occt_release_lineage(self.context.raw, self.handle) };
    }
}

/// Exact point-vs-solid classification (`AICAD-084`), as reported by
/// [`Shape::classify_point`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointClassification {
    Outside,
    Inside,
    OnBoundary,
}

impl PointClassification {
    fn from_raw(raw: c_int) -> Self {
        match raw {
            1 => PointClassification::Inside,
            2 => PointClassification::OnBoundary,
            // 0 (AICAD_CLASSIFY_OUT), or anything unrecognized -- matching
            // `SurfaceKind::from_raw`'s own defensive-default rationale.
            _ => PointClassification::Outside,
        }
    }
}

/// A face's underlying surface family (`AICAD-082`), via
/// [`Shape::surface_type`]. Kernel-neutral: never an OCCT `GeomAbs_*`
/// value re-exported directly (`native/occt_bridge/include/
/// aicad_occt_bridge.h`'s own kernel-neutral-at-the-ABI contract, which
/// this Rust-level type preserves one layer up).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    Plane,
    Cylinder,
    Cone,
    Sphere,
    Torus,
    Bezier,
    Bspline,
    /// Any analytic/procedural surface family this enum does not name
    /// (e.g. a swept, offset, or surface-of-revolution/-extrusion face)
    /// -- never guessed into one of the named kinds.
    Other,
}

impl SurfaceKind {
    fn from_raw(raw: c_int) -> Self {
        match raw {
            0 => SurfaceKind::Plane,
            1 => SurfaceKind::Cylinder,
            2 => SurfaceKind::Cone,
            3 => SurfaceKind::Sphere,
            4 => SurfaceKind::Torus,
            5 => SurfaceKind::Bezier,
            6 => SurfaceKind::Bspline,
            // The native side is entirely under this workspace's control
            // (matching `status_result`'s own rationale for its `_ =>`
            // arm): any value this match does not recognize is treated
            // as `Other` rather than panicking.
            _ => SurfaceKind::Other,
        }
    }
}

/// An edge's underlying curve family (`AICAD-082`), via
/// [`Shape::curve_type`]. Kernel-neutral, matching [`SurfaceKind`]'s own
/// rationale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveKind {
    Line,
    Circle,
    Ellipse,
    Bezier,
    Bspline,
    /// Any curve family this enum does not name (e.g. a hyperbola,
    /// parabola, or offset curve) -- never guessed into one of the named
    /// kinds, matching [`SurfaceKind::Other`]'s own rationale.
    Other,
}

impl CurveKind {
    fn from_raw(raw: c_int) -> Self {
        match raw {
            0 => CurveKind::Line,
            1 => CurveKind::Circle,
            2 => CurveKind::Ellipse,
            3 => CurveKind::Bezier,
            4 => CurveKind::Bspline,
            _ => CurveKind::Other,
        }
    }
}

/// An axis-aligned bounding box, as reported by [`Shape::bounding_box`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub min: Point3,
    pub max: Point3,
}

/// A normalized B-rep validation report, as reported by
/// [`Shape::validate`] (AICAD-031). `is_valid` mirrors
/// [`Shape::is_valid`]'s own bool for the same shape; the six
/// `invalid_*_count` fields break that down by topological kind, each
/// counted over the shape's own *unique* subshapes (matching
/// [`Shape::edge_count`]/[`Shape::face_count`]'s own de-duplication
/// convention). `invalid_shell_count`/`invalid_solid_count` were added by
/// `AICAD-119` alongside this batch's own [`OcctContext::make_shell`]/
/// [`Shape::make_solid`] construction paths -- this is the primary
/// validity-evidence mechanism those "construction success is not
/// validity" doc comments point to (e.g. building a solid from a
/// non-closed shell succeeds structurally, then reports here as
/// `is_valid: false, invalid_solid_count: 1`). A shape with `is_valid ==
/// false` always has at least one nonzero count; a shape with `is_valid ==
/// true` always has all six at zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationReport {
    pub is_valid: bool,
    pub invalid_vertex_count: usize,
    pub invalid_edge_count: usize,
    pub invalid_wire_count: usize,
    pub invalid_face_count: usize,
    pub invalid_shell_count: usize,
    pub invalid_solid_count: usize,
}

/// Structured sewing evidence, as returned by [`OcctContext::sew`]
/// (`AICAD-120`). `free_edge_count`/`multiple_edge_count`/
/// `degenerated_shape_count` are `BRepBuilderAPI_Sewing`'s own residual-
/// defect counts on the *result* (a free edge remains unshared by any
/// second face; a multiple edge is shared by more than 2, itself a
/// non-manifold defect neither this nor `AICAD-119`'s bare `make_shell`
/// silently repairs). `is_valid` mirrors a full [`Shape::validate`] call
/// against the sewed result exactly (`BRepCheck_Analyzer`); `changed` is
/// true iff at least one input face/edge was actually merged/relabeled
/// (an already-fully-connected input, e.g. a real solid's own faces,
/// reports `changed: false`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SewReport {
    pub changed: bool,
    pub is_valid: bool,
    pub free_edge_count: usize,
    pub multiple_edge_count: usize,
    pub degenerated_shape_count: usize,
}

/// Structured healing evidence, as returned by [`Shape::heal`]
/// (`AICAD-120`). **`is_valid_after` alone is never sufficient evidence
/// that healing succeeded** -- see `kind_changed`'s own doc comment,
/// which is exactly why this task added it. `changed` mirrors
/// `ShapeFix_Shape::Perform`'s own return value (whether it did anything
/// at all, coarse and not per-entity -- see [`Shape::heal`]'s own doc
/// comment for why no finer-grained lineage is offered here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealReport {
    pub changed: bool,
    pub is_valid_before: bool,
    pub is_valid_after: bool,
    /// Whether `ShapeFix_Shape` returned a shape of a different top-level
    /// topological kind than its input -- verified empirically (not
    /// assumed) to happen for an unclosable Solid, which gets silently
    /// demoted to a bare Shell (trivially valid, since a Shell has no
    /// closure requirement). `is_valid_after == true` together with
    /// `kind_changed == true` means healing gave up on the shape it was
    /// asked to fix and returned a different, lesser kind instead -- not
    /// a genuine repair, and must not be presented as one.
    pub kind_changed: bool,
}

/// A flat-shaded triangle-soup mesh, as returned by [`Shape::tessellate`]
/// (AICAD-032). `vertices.len() == normals.len() == 3 *
/// triangle_count()`; vertices are **not** shared/deduplicated across
/// triangles -- triangle `i` owns `vertices[3*i..3*i+3]`, each paired 1:1
/// with `normals[3*i..3*i+3]`: that triangle's own flat geometric normal,
/// duplicated across its 3 vertices (not an averaged/smooth per-vertex
/// normal). See `project/reports/AICAD-032.md` for why this
/// simplification was made for Stage-1's own scope.
#[derive(Debug, Clone, PartialEq)]
pub struct TriangleMesh {
    pub vertices: Vec<Point3>,
    pub normals: Vec<cad_kernel_api::Vector3>,
}

impl TriangleMesh {
    /// The number of triangles in this mesh (`vertices.len() / 3`).
    pub fn triangle_count(&self) -> usize {
        self.vertices.len() / 3
    }
}

impl<'ctx> Drop for Shape<'ctx> {
    fn drop(&mut self) {
        // SAFETY: see `is_valid`'s SAFETY comment; identical argument.
        // The result is intentionally ignored: `Drop::drop` cannot
        // propagate an error, and a release failing here would indicate a
        // defect this crate's own tests are responsible for catching
        // directly (e.g. a double-release bug), not something to panic
        // over during unwind-sensitive drop code.
        let _ = unsafe { ffi::aicad_occt_release_shape(self.context.raw, self.raw_handle()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_create_and_destroy_succeeds() {
        let context = OcctContext::new().expect("context creation should succeed");
        drop(context);
    }

    #[test]
    fn create_box_is_valid_and_has_the_expected_volume() {
        let context = OcctContext::new().unwrap();
        let shape = context
            .create_box(1.0, 2.0, 3.0)
            .expect("create_box should succeed");
        assert!(
            shape.is_valid().unwrap(),
            "a freshly created box must be a valid B-rep"
        );
        let volume = shape.volume().unwrap();
        assert!(
            (volume - 6.0).abs() < 1e-9,
            "expected volume 6.0, got {volume}"
        );
    }

    #[test]
    fn create_box_rejects_invalid_dimensions() {
        let context = OcctContext::new().unwrap();
        assert_eq!(
            context.create_box(0.0, 1.0, 1.0).unwrap_err(),
            KernelError::InvalidArgument
        );
        assert_eq!(
            context.create_box(-1.0, 1.0, 1.0).unwrap_err(),
            KernelError::InvalidArgument
        );
        assert_eq!(
            context.create_box(f64::NAN, 1.0, 1.0).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn multiple_shapes_in_one_context_have_independent_lifecycles() {
        let context = OcctContext::new().unwrap();
        let a = context.create_box(1.0, 1.0, 1.0).unwrap(); // volume 1
        let b = context.create_box(2.0, 2.0, 2.0).unwrap(); // volume 8

        assert!((a.volume().unwrap() - 1.0).abs() < 1e-9);
        assert!((b.volume().unwrap() - 8.0).abs() < 1e-9);

        // Dropping `a` (which releases its native handle) must not affect
        // `b`, which occupies a different slot in the same context's
        // shape table.
        drop(a);
        assert!(
            (b.volume().unwrap() - 8.0).abs() < 1e-9,
            "releasing one shape must not disturb an unrelated shape in the same context"
        );
    }

    #[test]
    fn dropping_and_recreating_reuses_the_slot_without_aliasing() {
        let context = OcctContext::new().unwrap();
        let first = context.create_box(1.0, 1.0, 1.0).unwrap(); // volume 1
        let first_handle = first.handle();
        drop(first); // releases the native handle

        let second = context.create_box(5.0, 5.0, 5.0).unwrap(); // volume 125, likely reuses the freed slot
        let second_handle = second.handle();

        assert_ne!(
            first_handle, second_handle,
            "a released handle's identity must never equal a new shape's handle, even if the slot was reused"
        );
        assert!((second.volume().unwrap() - 125.0).abs() < 1e-9);
    }

    #[test]
    fn two_contexts_are_fully_independent() {
        let context_a = OcctContext::new().unwrap();
        let context_b = OcctContext::new().unwrap();
        let shape_a = context_a.create_box(3.0, 3.0, 3.0).unwrap(); // volume 27
        let shape_b = context_b.create_box(4.0, 4.0, 4.0).unwrap(); // volume 64
        assert!((shape_a.volume().unwrap() - 27.0).abs() < 1e-9);
        assert!((shape_b.volume().unwrap() - 64.0).abs() < 1e-9);
    }

    /// AICAD-019: proves `OcctContext` is safe to use concurrently across
    /// real OS threads as long as each thread owns its own context (the
    /// pattern the single-thread-affine design is meant to support).
    /// Each `OcctContext` is created *inside* its owning thread's closure
    /// -- `OcctContext` is neither `Send` nor `Sync`, so the type system
    /// would reject any attempt to create one thread-side and move or
    /// share it into another, which is exactly the property under test.
    #[test]
    fn many_contexts_are_safe_across_real_threads() {
        const THREAD_COUNT: usize = 8;
        const SHAPES_PER_THREAD: usize = 50;

        let results: Vec<bool> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..THREAD_COUNT)
                .map(|t| {
                    scope.spawn(move || {
                        let context = OcctContext::new()
                            .expect("context creation should succeed on a worker thread");
                        for i in 0..SHAPES_PER_THREAD {
                            let side = 1.0 + (t as f64) * 0.01 + (i as f64) * 0.0001;
                            let shape = context
                                .create_box(side, side, side)
                                .expect("create_box should succeed on a worker thread");
                            let expected = side * side * side;
                            let volume = shape
                                .volume()
                                .expect("volume should succeed on a worker thread");
                            if (volume - expected).abs() >= 1e-6 {
                                return false;
                            }
                        }
                        true
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

        assert!(
            results.iter().all(|&ok| ok),
            "every worker thread's independently-owned context must produce correct, non-aliased results"
        );
    }

    // --- AICAD-020: cylinder ---

    #[test]
    fn create_cylinder_is_valid_and_has_the_expected_volume() {
        let context = OcctContext::new().unwrap();
        let shape = context
            .create_cylinder(2.0, 5.0)
            .expect("create_cylinder should succeed");
        assert!(shape.is_valid().unwrap());
        let expected = std::f64::consts::PI * 2.0 * 2.0 * 5.0;
        assert!((shape.volume().unwrap() - expected).abs() < expected * 1e-6);
    }

    #[test]
    fn create_cylinder_rejects_invalid_dimensions() {
        let context = OcctContext::new().unwrap();
        assert_eq!(
            context.create_cylinder(0.0, 5.0).unwrap_err(),
            KernelError::InvalidArgument
        );
        assert_eq!(
            context.create_cylinder(2.0, -1.0).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    // --- AICAD-021: transform ---

    #[test]
    fn transform_identity_is_a_no_op_and_produces_a_new_handle() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(2.0, 3.0, 4.0).unwrap();
        let moved = box_shape.transform(&Transform::identity()).unwrap();
        assert!((moved.volume().unwrap() - 24.0).abs() < 1e-9);
        assert_ne!(box_shape.handle(), moved.handle());
    }

    #[test]
    fn transform_translation_shifts_the_bounding_box() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(2.0, 2.0, 2.0).unwrap();
        let t = Transform::translation(cad_kernel_api::Vector3::new(10.0, 0.0, 0.0));
        let moved = box_shape.transform(&t).unwrap();
        let bbox = moved.bounding_box().unwrap();
        assert!((bbox.min.x - 10.0).abs() < 1e-6);
        assert!((bbox.max.x - 12.0).abs() < 1e-6);
    }

    #[test]
    fn transform_does_not_mutate_the_source_shape() {
        // The stale/foreign/invalid-handle rejection paths themselves are
        // exhaustively covered natively
        // (native/occt_bridge/tests/transform_test.cpp) and cannot even be
        // expressed against this crate's safe API in the first place: a
        // `Shape`'s borrow checker-enforced lifetime means there is no way
        // to call `.transform()` on an already-released shape without a
        // compile error, which is the point of the RAII wrapper
        // (AICAD-018). This test instead proves the *value* semantics
        // (DL-2): transforming a shape must leave the original untouched.
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(2.0, 2.0, 2.0).unwrap();
        let original_bbox = box_shape.bounding_box().unwrap();
        let _moved = box_shape.transform(&Transform::translation(cad_kernel_api::Vector3::new(
            100.0, 0.0, 0.0,
        )));
        let bbox_after = box_shape.bounding_box().unwrap();
        assert_eq!(
            original_bbox, bbox_after,
            "transform must not mutate the source shape"
        );
    }

    // --- AICAD-077: mirror ---

    #[test]
    fn mirror_across_a_world_plane_flips_the_bounding_box_and_produces_a_new_handle() {
        let context = OcctContext::new().unwrap();
        // A box with one corner at the origin, extending into +x/+y/+z.
        let box_shape = context.create_box(2.0, 3.0, 4.0).unwrap();
        let plane = Plane3::new(Point3::ORIGIN, Direction3::X);
        let mirrored = box_shape.mirror(plane).unwrap();
        assert_ne!(box_shape.handle(), mirrored.handle());
        // Volume is preserved by a reflection.
        assert!((mirrored.volume().unwrap() - 24.0).abs() < 1e-9);
        let bbox = mirrored.bounding_box().unwrap();
        // Reflected across x=0: the box's own [0, 2] x-extent becomes
        // [-2, 0].
        assert!((bbox.min.x - -2.0).abs() < 1e-6);
        assert!((bbox.max.x - 0.0).abs() < 1e-6);
        // y/z extents are unaffected (the mirror plane's normal is +X).
        assert!((bbox.min.y - 0.0).abs() < 1e-6);
        assert!((bbox.max.y - 3.0).abs() < 1e-6);
    }

    #[test]
    fn mirror_across_an_offset_plane_reflects_about_that_plane_not_the_origin() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(2.0, 1.0, 1.0).unwrap();
        // Mirror plane at x = 5, normal +X.
        let plane = Plane3::new(Point3::new(5.0, 0.0, 0.0), Direction3::X);
        let mirrored = box_shape.mirror(plane).unwrap();
        let bbox = mirrored.bounding_box().unwrap();
        // [0, 2] reflected about x=5 becomes [8, 10].
        assert!((bbox.min.x - 8.0).abs() < 1e-6);
        assert!((bbox.max.x - 10.0).abs() < 1e-6);
    }

    #[test]
    fn mirror_does_not_mutate_the_source_shape() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(2.0, 2.0, 2.0).unwrap();
        let original_bbox = box_shape.bounding_box().unwrap();
        let _mirrored = box_shape.mirror(Plane3::new(Point3::ORIGIN, Direction3::X));
        let bbox_after = box_shape.bounding_box().unwrap();
        assert_eq!(
            original_bbox, bbox_after,
            "mirror must not mutate the source shape"
        );
    }

    #[test]
    fn mirroring_twice_across_the_same_plane_returns_to_the_original_bounding_box() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(2.0, 3.0, 1.0).unwrap();
        let plane = Plane3::new(Point3::new(5.0, 0.0, 0.0), Direction3::X);
        let once = box_shape.mirror(plane).unwrap();
        let twice = once.mirror(plane).unwrap();
        let original_bbox = box_shape.bounding_box().unwrap();
        let twice_bbox = twice.bounding_box().unwrap();
        assert!((original_bbox.min.x - twice_bbox.min.x).abs() < 1e-6);
        assert!((original_bbox.max.x - twice_bbox.max.x).abs() < 1e-6);
    }

    // --- AICAD-022: curves/edges/wires ---

    #[test]
    fn make_line_edge_is_valid_with_the_expected_bounding_box() {
        let context = OcctContext::new().unwrap();
        let edge = context
            .make_line_edge(Point3::new(0.0, 0.0, 0.0), Point3::new(3.0, 4.0, 0.0))
            .unwrap();
        assert!(edge.is_valid().unwrap());
        let bbox = edge.bounding_box().unwrap();
        assert!((bbox.max.x - bbox.min.x - 3.0).abs() < 1e-6);
        assert!((bbox.max.y - bbox.min.y - 4.0).abs() < 1e-6);
    }

    #[test]
    fn make_line_edge_rejects_coincident_points() {
        let context = OcctContext::new().unwrap();
        let p = Point3::new(1.0, 1.0, 1.0);
        assert_eq!(
            context.make_line_edge(p, p).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn make_arc_edge_is_valid_with_the_expected_endpoints_and_bounding_box() {
        let context = OcctContext::new().unwrap();
        // A quarter circle of radius 2 in the XY plane, centered at the
        // origin, from angle 0 to angle pi/2 (start=(2,0,0), mid at pi/4,
        // end=(0,2,0)).
        let start = Point3::new(2.0, 0.0, 0.0);
        let mid = Point3::new(
            2.0 * std::f64::consts::FRAC_1_SQRT_2,
            2.0 * std::f64::consts::FRAC_1_SQRT_2,
            0.0,
        );
        let end = Point3::new(0.0, 2.0, 0.0);
        let edge = context.make_arc_edge(start, mid, end).unwrap();
        assert!(edge.is_valid().unwrap());
        let bbox = edge.bounding_box().unwrap();
        // The arc bulges out to x=2 (at start) and y=2 (at end); its
        // bounding box must match the quarter-circle's own bounds, within
        // `bounding_box`'s own tessellation-based tolerance (matching
        // `make_circle_wire_is_valid_with_the_expected_bounding_box`'s own
        // `1e-6` tolerance below, not this task's own numeric policy).
        assert!((bbox.max.x - 2.0).abs() < 1e-6);
        assert!((bbox.max.y - 2.0).abs() < 1e-6);
        assert!(bbox.min.x.abs() < 1e-6);
        assert!(bbox.min.y.abs() < 1e-6);
    }

    #[test]
    fn make_arc_edge_rejects_coincident_points() {
        let context = OcctContext::new().unwrap();
        let p = Point3::new(1.0, 0.0, 0.0);
        let other = Point3::new(0.0, 1.0, 0.0);
        assert_eq!(
            context.make_arc_edge(p, p, other).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn make_arc_edge_rejects_collinear_points() {
        let context = OcctContext::new().unwrap();
        let p0 = Point3::new(0.0, 0.0, 0.0);
        let p1 = Point3::new(1.0, 0.0, 0.0);
        let p2 = Point3::new(2.0, 0.0, 0.0);
        assert_eq!(
            context.make_arc_edge(p0, p1, p2).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn make_arc_edge_can_be_joined_with_line_edges_into_a_closed_wire() {
        // A "D" shape: a straight diameter plus a semicircular arc,
        // proving an arc edge composes with `make_wire_from_edges` and
        // `make_face` exactly like a line edge does.
        let context = OcctContext::new().unwrap();
        let p_top = Point3::new(0.0, 1.0, 0.0);
        let p_bottom = Point3::new(0.0, -1.0, 0.0);
        let p_right = Point3::new(1.0, 0.0, 0.0);
        let diameter = context.make_line_edge(p_bottom, p_top).unwrap();
        let arc = context.make_arc_edge(p_top, p_right, p_bottom).unwrap();
        let wire = context
            .make_wire_from_edges(&[&diameter, &arc])
            .expect("a diameter edge plus a semicircular arc edge must close");
        assert!(wire.is_valid().unwrap());
        let face = wire.make_face().unwrap();
        assert!(face.is_valid().unwrap());
        let expected_area = std::f64::consts::PI * 1.0 * 1.0 / 2.0;
        assert!((face.area().unwrap() - expected_area).abs() < expected_area * 1e-6);
    }

    #[test]
    fn make_circle_wire_is_valid_with_the_expected_bounding_box() {
        let context = OcctContext::new().unwrap();
        let wire = context
            .make_circle_wire(Point3::ORIGIN, Direction3::Z, 3.0)
            .unwrap();
        assert!(wire.is_valid().unwrap());
        let bbox = wire.bounding_box().unwrap();
        assert!((bbox.max.x - bbox.min.x - 6.0).abs() < 1e-6);
        assert!((bbox.max.y - bbox.min.y - 6.0).abs() < 1e-6);
    }

    #[test]
    fn make_wire_from_edges_joins_a_square_and_is_valid() {
        let context = OcctContext::new().unwrap();
        let p = |x: f64, y: f64| Point3::new(x, y, 0.0);
        let e0 = context.make_line_edge(p(0.0, 0.0), p(1.0, 0.0)).unwrap();
        let e1 = context.make_line_edge(p(1.0, 0.0), p(1.0, 1.0)).unwrap();
        let e2 = context.make_line_edge(p(1.0, 1.0), p(0.0, 1.0)).unwrap();
        let e3 = context.make_line_edge(p(0.0, 1.0), p(0.0, 0.0)).unwrap();
        let wire = context.make_wire_from_edges(&[&e0, &e1, &e2, &e3]).unwrap();
        assert!(wire.is_valid().unwrap());
        let bbox = wire.bounding_box().unwrap();
        assert!((bbox.max.x - bbox.min.x - 1.0).abs() < 1e-6);
        assert!((bbox.max.y - bbox.min.y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn make_wire_from_edges_rejects_an_empty_list() {
        let context = OcctContext::new().unwrap();
        let edges: [&Shape<'_>; 0] = [];
        assert_eq!(
            context.make_wire_from_edges(&edges).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    // --- AICAD-023: planar face from a closed wire ---

    #[test]
    fn make_circle_wire_and_face_matches_analytic_area() {
        let context = OcctContext::new().unwrap();
        let wire = context
            .make_circle_wire(Point3::ORIGIN, Direction3::Z, 3.0)
            .expect("make_circle_wire should succeed");
        let face = wire
            .make_face()
            .expect("make_face should succeed for a closed circle wire");
        assert!(face.is_valid().unwrap());
        let expected_area = std::f64::consts::PI * 3.0 * 3.0;
        assert!((face.area().unwrap() - expected_area).abs() < expected_area * 1e-6);
    }

    #[test]
    fn make_face_from_square_wire_matches_unit_area() {
        let context = OcctContext::new().unwrap();
        let p = |x: f64, y: f64| Point3::new(x, y, 0.0);
        let e0 = context.make_line_edge(p(0.0, 0.0), p(1.0, 0.0)).unwrap();
        let e1 = context.make_line_edge(p(1.0, 0.0), p(1.0, 1.0)).unwrap();
        let e2 = context.make_line_edge(p(1.0, 1.0), p(0.0, 1.0)).unwrap();
        let e3 = context.make_line_edge(p(0.0, 1.0), p(0.0, 0.0)).unwrap();
        let wire = context.make_wire_from_edges(&[&e0, &e1, &e2, &e3]).unwrap();
        let face = wire.make_face().unwrap();
        assert!((face.area().unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn open_wire_face_is_constructible_but_invalid() {
        // Stage-1 kernel policy #14 ("validation and repair/healing are
        // distinct semantic concepts") through the Rust wrapper: an open
        // wire's face still builds (IsDone) but must report invalid.
        let context = OcctContext::new().unwrap();
        let p = |x: f64, y: f64| Point3::new(x, y, 0.0);
        let e0 = context.make_line_edge(p(0.0, 0.0), p(1.0, 0.0)).unwrap();
        let e1 = context.make_line_edge(p(1.0, 0.0), p(1.0, 1.0)).unwrap();
        let open_wire = context.make_wire_from_edges(&[&e0, &e1]).unwrap();
        let open_face = open_wire.make_face().expect("construction itself succeeds");
        assert!(
            !open_face.is_valid().unwrap(),
            "an open wire's face must report as invalid"
        );
    }

    #[test]
    fn make_face_rejects_a_handle_that_is_not_a_wire() {
        let context = OcctContext::new().unwrap();
        let edge = context
            .make_line_edge(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0))
            .unwrap();
        assert_eq!(edge.make_face().unwrap_err(), KernelError::InvalidArgument);
    }

    // --- AICAD-024: extrude and revolve ---

    #[test]
    fn build_square_profile_and_extrude_to_the_expected_volume() {
        let context = OcctContext::new().unwrap();
        let p = |x: f64, y: f64| Point3::new(x, y, 0.0);
        let e0 = context.make_line_edge(p(0.0, 0.0), p(1.0, 0.0)).unwrap();
        let e1 = context.make_line_edge(p(1.0, 0.0), p(1.0, 1.0)).unwrap();
        let e2 = context.make_line_edge(p(1.0, 1.0), p(0.0, 1.0)).unwrap();
        let e3 = context.make_line_edge(p(0.0, 1.0), p(0.0, 0.0)).unwrap();
        let wire = context.make_wire_from_edges(&[&e0, &e1, &e2, &e3]).unwrap();
        assert!(wire.is_valid().unwrap());
        let face = wire.make_face().unwrap();
        assert!(face.is_valid().unwrap());
        assert!((face.area().unwrap() - 1.0).abs() < 1e-9);

        let solid = face
            .extrude(Direction3::Z, 5.0)
            .expect("extrude should succeed");
        assert!(solid.is_valid().unwrap());
        assert!((solid.volume().unwrap() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn build_circle_profile_and_revolve_to_the_expected_volume() {
        let context = OcctContext::new().unwrap();
        // A rectangle offset from the Z axis in the Y=0 half-plane,
        // revolved a full turn about Z, produces a tube of known volume
        // pi*(R^2-r^2)*h -- mirrors native/occt_bridge/tests/extrude_revolve_test.cpp.
        let p = |x: f64, z: f64| Point3::new(x, 0.0, z);
        let (inner, outer, height) = (1.0, 3.0, 5.0);
        let e0 = context
            .make_line_edge(p(inner, 0.0), p(outer, 0.0))
            .unwrap();
        let e1 = context
            .make_line_edge(p(outer, 0.0), p(outer, height))
            .unwrap();
        let e2 = context
            .make_line_edge(p(outer, height), p(inner, height))
            .unwrap();
        let e3 = context
            .make_line_edge(p(inner, height), p(inner, 0.0))
            .unwrap();
        let wire = context.make_wire_from_edges(&[&e0, &e1, &e2, &e3]).unwrap();
        let face = wire.make_face().unwrap();

        let axis = Axis3::new(Point3::ORIGIN, Direction3::Z);
        let tube = face
            .revolve(axis, std::f64::consts::TAU)
            .expect("revolve should succeed");
        assert!(tube.is_valid().unwrap());
        let expected = std::f64::consts::PI * (outer * outer - inner * inner) * height;
        assert!((tube.volume().unwrap() - expected).abs() < expected * 1e-6);
    }

    #[test]
    fn extrude_rejects_a_handle_that_is_not_a_face() {
        let context = OcctContext::new().unwrap();
        let edge = context
            .make_line_edge(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0))
            .unwrap();
        assert_eq!(
            edge.extrude(Direction3::Z, 1.0).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn extrude_rejects_a_non_positive_distance() {
        let context = OcctContext::new().unwrap();
        let p = |x: f64, y: f64| Point3::new(x, y, 0.0);
        let e0 = context.make_line_edge(p(0.0, 0.0), p(1.0, 0.0)).unwrap();
        let e1 = context.make_line_edge(p(1.0, 0.0), p(1.0, 1.0)).unwrap();
        let e2 = context.make_line_edge(p(1.0, 1.0), p(0.0, 1.0)).unwrap();
        let e3 = context.make_line_edge(p(0.0, 1.0), p(0.0, 0.0)).unwrap();
        let wire = context.make_wire_from_edges(&[&e0, &e1, &e2, &e3]).unwrap();
        let face = wire.make_face().unwrap();
        assert_eq!(
            face.extrude(Direction3::Z, 0.0).unwrap_err(),
            KernelError::InvalidArgument
        );
        assert_eq!(
            face.extrude(Direction3::Z, -1.0).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    // --- AICAD-025: sweep and loft ---

    fn square_wire<'ctx>(context: &'ctx OcctContext, side: f64, z: f64) -> Shape<'ctx> {
        let p = |x: f64, y: f64| Point3::new(x, y, z);
        let e0 = context.make_line_edge(p(0.0, 0.0), p(side, 0.0)).unwrap();
        let e1 = context.make_line_edge(p(side, 0.0), p(side, side)).unwrap();
        let e2 = context.make_line_edge(p(side, side), p(0.0, side)).unwrap();
        let e3 = context.make_line_edge(p(0.0, side), p(0.0, 0.0)).unwrap();
        context.make_wire_from_edges(&[&e0, &e1, &e2, &e3]).unwrap()
    }

    fn centered_square_wire<'ctx>(context: &'ctx OcctContext, side: f64, z: f64) -> Shape<'ctx> {
        let h = side / 2.0;
        let p = |x: f64, y: f64| Point3::new(x, y, z);
        let e0 = context.make_line_edge(p(-h, -h), p(h, -h)).unwrap();
        let e1 = context.make_line_edge(p(h, -h), p(h, h)).unwrap();
        let e2 = context.make_line_edge(p(h, h), p(-h, h)).unwrap();
        let e3 = context.make_line_edge(p(-h, h), p(-h, -h)).unwrap();
        context.make_wire_from_edges(&[&e0, &e1, &e2, &e3]).unwrap()
    }

    fn straight_spine<'ctx>(context: &'ctx OcctContext, p0: Point3, p1: Point3) -> Shape<'ctx> {
        let edge = context.make_line_edge(p0, p1).unwrap();
        context.make_wire_from_edges(&[&edge]).unwrap()
    }

    #[test]
    fn sweep_square_profile_along_straight_spine_matches_extrude() {
        let context = OcctContext::new().unwrap();
        let profile = square_wire(&context, 1.0, 0.0).make_face().unwrap();
        let spine = straight_spine(&context, Point3::ORIGIN, Point3::new(0.0, 0.0, 5.0));
        let swept = profile.sweep(&spine).expect("sweep should succeed");
        assert!(swept.is_valid().unwrap());
        assert!(
            (swept.volume().unwrap() - 5.0).abs() < 1e-6,
            "straight-spine sweep of a unit square must match extrude's volume 1*1*5=5.0"
        );
    }

    #[test]
    fn sweep_circle_profile_along_straight_spine_matches_cylinder_volume() {
        let context = OcctContext::new().unwrap();
        let profile = context
            .make_circle_wire(Point3::ORIGIN, Direction3::Z, 2.0)
            .unwrap()
            .make_face()
            .unwrap();
        let spine = straight_spine(&context, Point3::ORIGIN, Point3::new(0.0, 0.0, 5.0));
        let swept = profile.sweep(&spine).expect("sweep should succeed");
        let expected = std::f64::consts::PI * 2.0 * 2.0 * 5.0;
        assert!((swept.volume().unwrap() - expected).abs() < expected * 1e-6);
    }

    #[test]
    fn sweep_rejects_a_profile_that_is_not_a_face() {
        let context = OcctContext::new().unwrap();
        let wire_profile = square_wire(&context, 1.0, 0.0);
        let spine = straight_spine(&context, Point3::ORIGIN, Point3::new(0.0, 0.0, 5.0));
        assert_eq!(
            wire_profile.sweep(&spine).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn sweep_rejects_a_spine_that_is_not_a_wire() {
        let context = OcctContext::new().unwrap();
        let profile = square_wire(&context, 1.0, 0.0).make_face().unwrap();
        let not_a_wire = context
            .make_line_edge(Point3::ORIGIN, Point3::new(0.0, 0.0, 5.0))
            .unwrap();
        assert_eq!(
            profile.sweep(&not_a_wire).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn loft_between_two_identical_squares_matches_prism_volume() {
        let context = OcctContext::new().unwrap();
        let a = centered_square_wire(&context, 1.0, 0.0);
        let b = centered_square_wire(&context, 1.0, 5.0);
        let lofted = context.loft(&[&a, &b]).expect("loft should succeed");
        assert!(lofted.is_valid().unwrap());
        assert!(
            (lofted.volume().unwrap() - 5.0).abs() < 1e-6,
            "loft between two identical square sections must match prism volume 1*1*5=5.0"
        );
    }

    #[test]
    fn loft_square_frustum_matches_analytic_volume() {
        // side 2 at z=0, side 4 at z=3: an exact frustum of a pyramid
        // (ruled ThruSections between two concentric, axis-aligned
        // squares produces planar trapezoid side faces), analytic volume
        // h/3*(A1+A2+sqrt(A1*A2)).
        let context = OcctContext::new().unwrap();
        let a = centered_square_wire(&context, 2.0, 0.0);
        let b = centered_square_wire(&context, 4.0, 3.0);
        let lofted = context.loft(&[&a, &b]).expect("loft should succeed");
        assert!(lofted.is_valid().unwrap());
        let (a1, a2, h): (f64, f64, f64) = (2.0 * 2.0, 4.0 * 4.0, 3.0);
        let expected = (h / 3.0) * (a1 + a2 + (a1 * a2).sqrt());
        assert!((lofted.volume().unwrap() - expected).abs() < expected * 1e-6);
    }

    #[test]
    fn loft_rejects_fewer_than_two_sections() {
        let context = OcctContext::new().unwrap();
        let a = centered_square_wire(&context, 1.0, 0.0);
        assert_eq!(
            context.loft(&[&a]).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn loft_rejects_a_section_that_is_not_a_wire() {
        let context = OcctContext::new().unwrap();
        let not_a_wire = context
            .make_line_edge(Point3::ORIGIN, Point3::new(0.0, 0.0, 5.0))
            .unwrap();
        let b = centered_square_wire(&context, 1.0, 5.0);
        assert_eq!(
            context.loft(&[&not_a_wire, &b]).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    // --- AICAD-026: boolean union/cut/intersect ---

    fn overlapping_boxes(context: &OcctContext) -> (Shape<'_>, Shape<'_>) {
        let a = context.create_box(2.0, 2.0, 2.0).unwrap();
        let b_raw = context.create_box(2.0, 2.0, 2.0).unwrap();
        let b = b_raw
            .transform(&Transform::translation(cad_kernel_api::Vector3::new(
                1.0, 1.0, 1.0,
            )))
            .unwrap();
        (a, b)
    }

    #[test]
    fn boolean_union_matches_inclusion_exclusion_volume() {
        let context = OcctContext::new().unwrap();
        let (a, b) = overlapping_boxes(&context);
        let fused = a.union(&b).expect("union should succeed");
        assert!(fused.is_valid().unwrap());
        assert!((fused.volume().unwrap() - 15.0).abs() < 1e-6);
    }

    #[test]
    fn boolean_cut_matches_inclusion_exclusion_volume() {
        let context = OcctContext::new().unwrap();
        let (a, b) = overlapping_boxes(&context);
        let cut = a.cut(&b).expect("cut should succeed");
        assert!(cut.is_valid().unwrap());
        assert!((cut.volume().unwrap() - 7.0).abs() < 1e-6);
    }

    #[test]
    fn boolean_intersect_matches_inclusion_exclusion_volume() {
        let context = OcctContext::new().unwrap();
        let (a, b) = overlapping_boxes(&context);
        let common = a.intersect(&b).expect("intersect should succeed");
        assert!(common.is_valid().unwrap());
        assert!((common.volume().unwrap() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn boolean_result_is_chainable_into_a_further_boolean() {
        // Boolean results are Compounds, not Solids (verified empirically
        // -- see project/reports/AICAD-026.md); this proves the Rust
        // wrapper's operand-kind-unrestricted contract actually allows
        // chaining, not just that the native ABI does.
        let context = OcctContext::new().unwrap();
        let (a, b) = overlapping_boxes(&context);
        let fused = a.union(&b).unwrap(); // volume 15
        let third = context.create_box(3.0, 3.0, 3.0).unwrap(); // volume 27
        let third_far = third
            .transform(&Transform::translation(cad_kernel_api::Vector3::new(
                100.0, 100.0, 100.0,
            )))
            .unwrap();
        let chained = fused
            .union(&third_far)
            .expect("chained union should succeed");
        assert!((chained.volume().unwrap() - 42.0).abs() < 1e-6);
    }

    /// AICAD-026/SESSION_HANDOFF: the Rust-level counterpart of
    /// `native/occt_bridge/tests/boolean_test.cpp`'s "epoch bump on
    /// mutation" case, but expressed through the borrow checker instead
    /// of the raw ABI: dropping one boolean-union input must release
    /// exactly that shape's native handle without disturbing the other
    /// input or the union result, both of which remain independently
    /// usable `Shape<'ctx>` values for the rest of the context's
    /// lifetime.
    #[test]
    fn dropping_one_boolean_input_does_not_disturb_the_other_input_or_the_result() {
        let context = OcctContext::new().unwrap();
        let (a, b) = overlapping_boxes(&context);
        let fused = a.union(&b).expect("union should succeed");
        drop(a); // releases only `a`'s native handle
        assert!(
            (b.volume().unwrap() - 8.0).abs() < 1e-9,
            "the other, undropped boolean-union input must remain valid after its sibling is dropped"
        );
        assert!(
            (fused.volume().unwrap() - 15.0).abs() < 1e-6,
            "the union result must remain valid and unchanged after one of its two original inputs is dropped"
        );
    }

    #[test]
    fn boolean_disjoint_intersect_is_empty() {
        let context = OcctContext::new().unwrap();
        let a = context.create_box(1.0, 1.0, 1.0).unwrap();
        let b_raw = context.create_box(1.0, 1.0, 1.0).unwrap();
        let b = b_raw
            .transform(&Transform::translation(cad_kernel_api::Vector3::new(
                10.0, 10.0, 10.0,
            )))
            .unwrap();
        let common = a
            .intersect(&b)
            .expect("intersect should succeed (construct an empty result)");
        assert!((common.volume().unwrap() - 0.0).abs() < 1e-9);
    }

    #[test]
    fn boolean_union_rejects_a_handle_from_a_foreign_context() {
        let context_a = OcctContext::new().unwrap();
        let context_b = OcctContext::new().unwrap();
        let a = context_a.create_box(1.0, 1.0, 1.0).unwrap();
        let foreign = context_b.create_box(1.0, 1.0, 1.0).unwrap();
        assert_eq!(a.union(&foreign).unwrap_err(), KernelError::ForeignContext);
    }

    // --- AICAD-027: fillet and chamfer ---

    #[test]
    fn a_box_has_exactly_twelve_unique_edges() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(4.0, 5.0, 6.0).unwrap();
        assert_eq!(box_shape.edge_count().unwrap(), 12);
    }

    #[test]
    fn get_edge_rejects_an_out_of_range_index() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        assert_eq!(
            box_shape.get_edge(12).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn fillet_all_edges_matches_rounded_box_analytic_volume() {
        // Filleting every edge of a box with radius r produces a "rounded
        // box" whose volume is the closed-form Minkowski-sum-with-a-ball
        // formula: V = Lx*Ly*Lz + 2r(Lx*Ly+Ly*Lz+Lz*Lx) + pi*r^2*(Lx+Ly+Lz)
        // + (4/3)*pi*r^3, where Lx/Ly/Lz are the box dimensions inset by r
        // on every side.
        let context = OcctContext::new().unwrap();
        let (dx, dy, dz, r) = (4.0, 5.0, 6.0, 1.0);
        let box_shape = context.create_box(dx, dy, dz).unwrap();
        let count = box_shape.edge_count().unwrap();
        let edges: Vec<Shape<'_>> = (0..count).map(|i| box_shape.get_edge(i).unwrap()).collect();
        let edge_refs: Vec<&Shape<'_>> = edges.iter().collect();
        let rounded = box_shape
            .fillet(&edge_refs, r)
            .expect("fillet should succeed for all 12 edges");
        assert!(rounded.is_valid().unwrap());
        let (lx, ly, lz) = (dx - 2.0 * r, dy - 2.0 * r, dz - 2.0 * r);
        let pi = std::f64::consts::PI;
        let expected = lx * ly * lz
            + 2.0 * r * (lx * ly + ly * lz + lz * lx)
            + pi * r * r * (lx + ly + lz)
            + (4.0 / 3.0) * pi * r * r * r;
        let volume = rounded.volume().unwrap();
        assert!(
            (volume - expected).abs() < expected * 1e-4,
            "rounded-box volume {volume} did not match analytic {expected}"
        );
    }

    #[test]
    fn chamfer_single_edge_matches_analytic_volume() {
        // Chamfering exactly one edge (and only one -- neither adjacent
        // edge is also modified) removes a clean triangular prism of
        // cross-section legs (d,d) along the full edge length, so
        // volume = box_volume - (d^2/2)*edge_length.
        let context = OcctContext::new().unwrap();
        let (dx, dy, dz, d) = (4.0, 5.0, 6.0, 0.5);
        let box_shape = context.create_box(dx, dy, dz).unwrap();
        let count = box_shape.edge_count().unwrap();
        // Find the edge from (0,0,dz) to (dx,0,dz): the intersection of
        // the y=0 face and the z=dz (top) face, length dx.
        let target = (0..count)
            .map(|i| box_shape.get_edge(i).unwrap())
            .find(|edge| {
                let bbox = edge.bounding_box().unwrap();
                (bbox.min.x - 0.0).abs() < 1e-6
                    && (bbox.max.x - dx).abs() < 1e-6
                    && (bbox.min.y - 0.0).abs() < 1e-6
                    && (bbox.max.y - 0.0).abs() < 1e-6
                    && (bbox.min.z - dz).abs() < 1e-6
                    && (bbox.max.z - dz).abs() < 1e-6
            })
            .expect("the specific top-front edge must be found among the box's 12 edges");
        let chamfered = box_shape
            .chamfer(&[&target], d)
            .expect("chamfer should succeed for a single identified edge");
        assert!(chamfered.is_valid().unwrap());
        let expected = dx * dy * dz - (d * d / 2.0) * dx;
        let volume = chamfered.volume().unwrap();
        assert!((volume - expected).abs() < expected * 1e-6);
    }

    #[test]
    fn fillet_and_chamfer_reject_a_handle_that_is_not_an_edge() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        assert_eq!(
            box_shape.fillet(&[&box_shape], 1.0).unwrap_err(),
            KernelError::InvalidArgument
        );
        assert_eq!(
            box_shape.chamfer(&[&box_shape], 0.1).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn fillet_rejects_a_non_positive_radius() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        let edge = box_shape.get_edge(0).unwrap();
        assert_eq!(
            box_shape.fillet(&[&edge], 0.0).unwrap_err(),
            KernelError::InvalidArgument
        );
        assert_eq!(
            box_shape.fillet(&[&edge], -1.0).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    // --- AICAD-028: shell and offset ---

    #[test]
    fn a_box_has_exactly_six_unique_faces() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(2.0, 3.0, 4.0).unwrap();
        assert_eq!(box_shape.face_count().unwrap(), 6);
    }

    #[test]
    fn get_face_rejects_an_out_of_range_index() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        assert_eq!(
            box_shape.get_face(6).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn duplicate_is_independently_releasable_and_addresses_the_same_shape() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(2.0, 3.0, 4.0).unwrap();
        let dup = box_shape.duplicate().unwrap();
        assert!(box_shape.is_same(&dup).unwrap());
        assert_eq!(dup.face_count().unwrap(), box_shape.face_count().unwrap());
        // Releasing one handle must not invalidate the other -- confirmed
        // by both still answering queries cleanly afterward.
        drop(dup);
        assert!(box_shape.is_valid().unwrap());
    }

    // --- AICAD-100A: shell/solid candidate enumeration ---

    #[test]
    fn a_box_solid_has_exactly_one_unique_solid_and_one_unique_shell() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(2.0, 3.0, 4.0).unwrap();
        assert_eq!(box_shape.solid_count().unwrap(), 1);
        assert_eq!(box_shape.shell_count().unwrap(), 1);
    }

    #[test]
    fn get_solid_and_get_shell_return_a_shape_with_the_same_face_count_as_the_box() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(2.0, 3.0, 4.0).unwrap();
        let solid = box_shape.get_solid(0).unwrap();
        assert_eq!(solid.face_count().unwrap(), box_shape.face_count().unwrap());
        let shell = box_shape.get_shell(0).unwrap();
        assert_eq!(shell.face_count().unwrap(), box_shape.face_count().unwrap());
    }

    #[test]
    fn get_solid_rejects_an_out_of_range_index() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        assert_eq!(
            box_shape.get_solid(1).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn get_shell_rejects_an_out_of_range_index() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        assert_eq!(
            box_shape.get_shell(1).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn a_bare_face_has_zero_shells_and_zero_solids() {
        // A Face-kind shape contains no TopAbs_SHELL/TopAbs_SOLID
        // sub-shapes at all -- matching `face_radius`'s own "not
        // applicable to this kind -> Ok(0)/Ok(false)" precedent rather
        // than an error, so a `ShellRef`/`SolidRef` query against a Face
        // candidate correctly finds zero candidates instead of failing.
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face = box_shape.get_face(0).unwrap();
        assert_eq!(face.solid_count().unwrap(), 0);
        assert_eq!(face.shell_count().unwrap(), 0);
    }

    #[test]
    fn shell_hollowed_box_matches_analytic_volume() {
        // Hollowing a box with its top face removed and wall thickness t
        // (built inward) leaves a cavity spanning x in [t,dx-t], y in
        // [t,dy-t], z in [t,dz] (open at the top) -> volume =
        // dx*dy*dz - (dx-2t)*(dy-2t)*(dz-t).
        let context = OcctContext::new().unwrap();
        let (dx, dy, dz, t) = (2.0, 3.0, 4.0, 0.2);
        let box_shape = context.create_box(dx, dy, dz).unwrap();
        let count = box_shape.face_count().unwrap();
        let top_face = (0..count)
            .map(|i| box_shape.get_face(i).unwrap())
            .find(|face| {
                let bbox = face.bounding_box().unwrap();
                (bbox.min.z - dz).abs() < 1e-6 && (bbox.max.z - dz).abs() < 1e-6
            })
            .expect("the top face must be found among the box's 6 faces");
        let shelled = box_shape
            .shell(&[&top_face], -t)
            .expect("shell should succeed");
        assert!(shelled.is_valid().unwrap());
        let expected = dx * dy * dz - (dx - 2.0 * t) * (dy - 2.0 * t) * (dz - t);
        let volume = shelled.volume().unwrap();
        assert!((volume - expected).abs() < expected * 1e-4);
    }

    #[test]
    fn offset_box_matches_minkowski_sum_analytic_volume() {
        // A positive-distance offset with OCCT's default arc join is the
        // Minkowski sum of the box with a ball of that radius -- same
        // closed form as fillet-all-edges, without the inset term.
        let context = OcctContext::new().unwrap();
        let (dx, dy, dz, delta) = (2.0, 3.0, 4.0, 0.3);
        let box_shape = context.create_box(dx, dy, dz).unwrap();
        let offset = box_shape.offset(delta).expect("offset should succeed");
        assert!(offset.is_valid().unwrap());
        let pi = std::f64::consts::PI;
        let expected = dx * dy * dz
            + 2.0 * delta * (dx * dy + dy * dz + dz * dx)
            + pi * delta * delta * (dx + dy + dz)
            + (4.0 / 3.0) * pi * delta * delta * delta;
        let volume = offset.volume().unwrap();
        assert!((volume - expected).abs() < expected * 1e-4);
    }

    #[test]
    fn shell_rejects_a_handle_that_is_not_a_face() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        assert_eq!(
            box_shape.shell(&[&box_shape], -0.1).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn offset_rejects_a_zero_distance() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        assert_eq!(
            box_shape.offset(0.0).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn fillet_and_chamfer_accept_a_compound_shape() {
        // Matches the boolean-result-is-chainable rationale: fillet a
        // Compound (a boolean-union result of two disjoint boxes) to
        // prove operand-kind is not restricted to Solid here either.
        let context = OcctContext::new().unwrap();
        let a = context.create_box(2.0, 2.0, 2.0).unwrap();
        let b_raw = context.create_box(2.0, 2.0, 2.0).unwrap();
        let b = b_raw
            .transform(&Transform::translation(cad_kernel_api::Vector3::new(
                20.0, 0.0, 0.0,
            )))
            .unwrap();
        let compound = a.union(&b).unwrap();
        assert_eq!(
            compound.edge_count().unwrap(),
            24,
            "two disjoint boxes' union has 12+12=24 edges"
        );
    }

    // --- AICAD-029: topology exploration ---

    #[test]
    fn a_box_has_exactly_eight_unique_vertices() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(2.0, 3.0, 4.0).unwrap();
        assert_eq!(box_shape.vertex_count().unwrap(), 8);
    }

    #[test]
    fn get_vertex_rejects_an_out_of_range_index() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        assert_eq!(
            box_shape.get_vertex(8).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn every_box_edge_is_adjacent_to_exactly_two_faces() {
        // A closed manifold solid's every edge is shared by exactly 2
        // faces -- Euler-formula-consistent for a box (V=8, E=12, F=6).
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(2.0, 3.0, 4.0).unwrap();
        let edge_count = box_shape.edge_count().unwrap();
        for i in 0..edge_count {
            assert_eq!(
                box_shape.edge_adjacent_face_count(i).unwrap(),
                2,
                "edge {i} should be adjacent to exactly 2 faces"
            );
        }
    }

    #[test]
    fn edge_adjacent_face_get_rejects_an_out_of_range_adjacent_index() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        assert_eq!(
            box_shape.edge_adjacent_face(0, 2).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn edge_adjacent_face_count_rejects_an_out_of_range_edge_index() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        let edge_count = box_shape.edge_count().unwrap();
        assert_eq!(
            box_shape.edge_adjacent_face_count(edge_count).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn face_edges_is_covered_by_edge_count_applied_to_a_face_handle() {
        // docs/plan/05 §3's face_edges: no new function was added for it --
        // shape_edge_count/_get_edge (AICAD-027) already accept any shape
        // kind, so applying them to a Face handle enumerates that face's
        // own boundary edges. This test is the evidence that holds, not
        // just an assumption.
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(2.0, 3.0, 4.0).unwrap();
        let face = box_shape.get_face(0).unwrap();
        assert_eq!(
            face.edge_count().unwrap(),
            4,
            "a box face is a quad: 4 edges"
        );
    }

    #[test]
    fn edge_vertices_of_an_open_line_edge_match_its_endpoints() {
        let context = OcctContext::new().unwrap();
        let p0 = Point3::new(1.0, 2.0, 3.0);
        let p1 = Point3::new(4.0, 5.0, 6.0);
        let edge = context.make_line_edge(p0, p1).unwrap();
        let (v0, v1) = edge.edge_vertices().unwrap();
        // Bnd_Box enlarges by the vertex's own confusion tolerance (~1e-7
        // per side) even for a single-point shape; 1e-5 is comfortably
        // above that gap.
        let v0_box = v0.bounding_box().unwrap();
        let v1_box = v1.bounding_box().unwrap();
        assert!((v0_box.min.x - p0.x).abs() < 1e-5 && (v0_box.min.y - p0.y).abs() < 1e-5);
        assert!((v1_box.min.x - p1.x).abs() < 1e-5 && (v1_box.min.y - p1.y).abs() < 1e-5);
    }

    #[test]
    fn edge_vertices_of_a_closed_circle_edge_coincide() {
        let context = OcctContext::new().unwrap();
        let circle_wire = context
            .make_circle_wire(Point3::ORIGIN, Direction3::Z, 2.0)
            .unwrap();
        assert_eq!(circle_wire.edge_count().unwrap(), 1);
        let edge = circle_wire.get_edge(0).unwrap();
        let (v0, v1) = edge.edge_vertices().unwrap();
        let v0_box = v0.bounding_box().unwrap();
        let v1_box = v1.bounding_box().unwrap();
        assert!((v0_box.min.x - v1_box.min.x).abs() < 1e-5);
        assert!((v0_box.min.y - v1_box.min.y).abs() < 1e-5);
        assert!((v0_box.min.z - v1_box.min.z).abs() < 1e-5);
    }

    #[test]
    fn edge_vertices_rejects_a_handle_that_is_not_an_edge() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        assert_eq!(
            box_shape.edge_vertices().unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn a_standalone_edge_has_zero_adjacent_faces() {
        let context = OcctContext::new().unwrap();
        let edge = context
            .make_line_edge(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0))
            .unwrap();
        assert_eq!(edge.edge_adjacent_face_count(0).unwrap(), 0);
    }

    // --- AICAD-030: length and center-of-mass queries ---

    #[test]
    fn a_3_4_5_line_edge_has_length_exactly_5() {
        let context = OcctContext::new().unwrap();
        let edge = context
            .make_line_edge(Point3::new(0.0, 0.0, 0.0), Point3::new(3.0, 4.0, 0.0))
            .unwrap();
        assert!((edge.length().unwrap() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn a_box_total_edge_length_matches_4_times_dx_plus_dy_plus_dz() {
        // SkipShared=true is required in the native implementation:
        // without it every edge (shared by 2 faces) is counted twice.
        let context = OcctContext::new().unwrap();
        let (dx, dy, dz) = (2.0, 3.0, 4.0);
        let box_shape = context.create_box(dx, dy, dz).unwrap();
        let expected = 4.0 * (dx + dy + dz);
        assert!((box_shape.length().unwrap() - expected).abs() < 1e-9);
    }

    #[test]
    fn create_box_center_of_mass_is_its_geometric_center() {
        let context = OcctContext::new().unwrap();
        let (dx, dy, dz) = (2.0, 3.0, 4.0);
        let box_shape = context.create_box(dx, dy, dz).unwrap();
        let centre = box_shape.center_of_mass().unwrap();
        assert!((centre.x - dx / 2.0).abs() < 1e-9);
        assert!((centre.y - dy / 2.0).abs() < 1e-9);
        assert!((centre.z - dz / 2.0).abs() < 1e-9);
    }

    #[test]
    fn a_standalone_line_edge_center_of_mass_is_its_midpoint() {
        let context = OcctContext::new().unwrap();
        let edge = context
            .make_line_edge(Point3::new(0.0, 0.0, 0.0), Point3::new(10.0, 0.0, 0.0))
            .unwrap();
        let centre = edge.center_of_mass().unwrap();
        assert!((centre.x - 5.0).abs() < 1e-9);
        assert!(centre.y.abs() < 1e-9);
        assert!(centre.z.abs() < 1e-9);
    }

    #[test]
    fn center_of_mass_on_a_bare_vertex_fails_cleanly() {
        // A bare Vertex (obtained via AICAD-029's get_vertex) has no
        // edge/face/solid content for any of the three GProp dispatch
        // branches to measure.
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        let vertex = box_shape.get_vertex(0).unwrap();
        assert_eq!(
            vertex.center_of_mass().unwrap_err(),
            KernelError::OperationFailed
        );
    }

    #[test]
    fn length_of_a_bare_vertex_is_zero_not_an_error() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        let vertex = box_shape.get_vertex(0).unwrap();
        assert!((vertex.length().unwrap() - 0.0).abs() < 1e-12);
    }

    // --- AICAD-031: normalized B-rep validation report ---

    #[test]
    fn validate_of_a_valid_box_is_fully_clean() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 2.0, 3.0).unwrap();
        let report = box_shape.validate().unwrap();
        assert_eq!(
            report,
            ValidationReport {
                is_valid: true,
                invalid_vertex_count: 0,
                invalid_edge_count: 0,
                invalid_wire_count: 0,
                invalid_face_count: 0,
                invalid_shell_count: 0,
                invalid_solid_count: 0,
            }
        );
        assert_eq!(report.is_valid, box_shape.is_valid().unwrap());
    }

    #[test]
    fn validate_of_an_open_wire_face_attributes_exactly_one_invalid_face() {
        let context = OcctContext::new().unwrap();
        let e0 = context
            .make_line_edge(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0))
            .unwrap();
        let e1 = context
            .make_line_edge(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0))
            .unwrap();
        let e2 = context
            .make_line_edge(Point3::new(1.0, 1.0, 0.0), Point3::new(0.0, 1.0, 0.0))
            .unwrap();
        let open_wire = context.make_wire_from_edges(&[&e0, &e1, &e2]).unwrap();
        let open_face = open_wire.make_face().unwrap();
        let report = open_face.validate().unwrap();
        assert!(!report.is_valid);
        assert_eq!(report.invalid_face_count, 1);
        assert_eq!(report.invalid_vertex_count, 0);
        assert_eq!(report.invalid_edge_count, 0);
        assert_eq!(report.invalid_wire_count, 0);
    }

    // --- AICAD-119: general topology construction ---

    fn unit_square_wire<'ctx>(context: &'ctx OcctContext) -> Shape<'ctx> {
        let e0 = context
            .make_line_edge(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0))
            .unwrap();
        let e1 = context
            .make_line_edge(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0))
            .unwrap();
        let e2 = context
            .make_line_edge(Point3::new(1.0, 1.0, 0.0), Point3::new(0.0, 1.0, 0.0))
            .unwrap();
        let e3 = context
            .make_line_edge(Point3::new(0.0, 1.0, 0.0), Point3::new(0.0, 0.0, 0.0))
            .unwrap();
        context.make_wire_from_edges(&[&e0, &e1, &e2, &e3]).unwrap()
    }

    #[test]
    fn make_vertex_reports_exactly_its_own_point() {
        let context = OcctContext::new().unwrap();
        let vertex = context.make_vertex(Point3::new(1.0, 2.0, 3.0)).unwrap();
        let point = vertex.vertex_point().unwrap();
        assert_eq!(point, Point3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn make_face_on_plane_of_a_unit_square_wire_has_unit_area() {
        let context = OcctContext::new().unwrap();
        let wire = unit_square_wire(&context);
        let face = wire
            .make_face_on_plane(&[], Point3::ORIGIN, Direction3::Z, false)
            .unwrap();
        let report = face.validate().unwrap();
        assert!(report.is_valid);
        assert!((face.area().unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn make_shell_then_make_solid_from_a_box_faces_matches_the_box_volume() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 2.0, 3.0).unwrap();
        let faces: Vec<Shape> = (0..box_shape.face_count().unwrap())
            .map(|i| box_shape.get_face(i).unwrap())
            .collect();
        let face_refs: Vec<&Shape> = faces.iter().collect();
        let shell = context.make_shell(&face_refs).unwrap();
        let shell_report = shell.validate().unwrap();
        assert!(shell_report.is_valid);
        assert_eq!(shell_report.invalid_shell_count, 0);

        let solid = shell.make_solid(&[]).unwrap();
        let solid_report = solid.validate().unwrap();
        assert!(solid_report.is_valid);
        assert_eq!(solid_report.invalid_solid_count, 0);
        assert!((solid.volume().unwrap() - 6.0).abs() < 1e-9);
    }

    #[test]
    fn make_solid_from_a_non_closed_shell_succeeds_structurally_but_is_reported_invalid() {
        // AICAD-119: verified empirically that BRepBuilderAPI_MakeSolid does
        // not require its input shell to be closed -- this is Stage-1
        // kernel policy #14 ("construction success is not evidence of
        // validity") in its starkest form, and is exactly the behavior
        // `Shape::make_solid`'s own doc comment describes.
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        let single_face = box_shape.get_face(0).unwrap();
        let open_shell = context.make_shell(&[&single_face]).unwrap();
        let solid = open_shell
            .make_solid(&[])
            .expect("construction itself succeeds even though the shell is open");
        let report = solid.validate().unwrap();
        assert!(!report.is_valid);
        assert_eq!(report.invalid_solid_count, 1);
    }

    #[test]
    fn make_shell_rejects_an_empty_face_list() {
        let context = OcctContext::new().unwrap();
        assert_eq!(
            context.make_shell(&[]).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn make_compound_rejects_an_empty_shape_list() {
        let context = OcctContext::new().unwrap();
        assert_eq!(
            context.make_compound(&[]).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn make_compound_groups_mixed_kind_shapes_without_error() {
        let context = OcctContext::new().unwrap();
        let vertex = context.make_vertex(Point3::ORIGIN).unwrap();
        let wire = unit_square_wire(&context);
        let compound = context.make_compound(&[&vertex, &wire]).unwrap();
        assert!(compound.is_valid().unwrap());
    }

    #[test]
    fn make_face_on_cylinder_of_a_planar_wire_is_structurally_real_but_not_required_to_be_valid() {
        // A unit-square wire does not lie on the cylinder's own surface, so
        // this documents the honest outcome (a structurally-built face that
        // validate() may reject) rather than asserting a specific result --
        // exactly [`Shape::make_face_on_plane`]'s own "construction success
        // is not validity" contract, exercised on a non-planar surface.
        let context = OcctContext::new().unwrap();
        let wire = unit_square_wire(&context);
        let axis = Axis3::new(Point3::ORIGIN, Direction3::Z);
        let face = wire.make_face_on_cylinder(&[], axis, 0.5, false).unwrap();
        let _ = face.validate().unwrap();
    }

    #[test]
    fn make_face_on_sphere_torus_cone_construct_without_error() {
        let context = OcctContext::new().unwrap();
        let axis = Axis3::new(Point3::ORIGIN, Direction3::Z);
        assert!(
            unit_square_wire(&context)
                .make_face_on_sphere(&[], Point3::ORIGIN, 2.0, false)
                .is_ok()
        );
        assert!(
            unit_square_wire(&context)
                .make_face_on_torus(&[], axis, 2.0, 0.5, false)
                .is_ok()
        );
        assert!(
            unit_square_wire(&context)
                .make_face_on_cone(&[], axis, std::f64::consts::FRAC_PI_4, false)
                .is_ok()
        );
    }

    // --- AICAD-120: sewing/healing ---

    #[test]
    fn sewing_an_already_connected_solids_own_faces_is_a_deterministic_no_op() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        let faces: Vec<Shape> = (0..6).map(|i| box_shape.get_face(i).unwrap()).collect();
        let face_refs: Vec<&Shape> = faces.iter().collect();
        let (sewed, _lineage, report) = context.sew(&face_refs, 1e-6).unwrap();
        assert!(!report.changed);
        assert!(report.is_valid);
        assert_eq!(report.free_edge_count, 0);
        assert_eq!(report.multiple_edge_count, 0);
        assert!((sewed.volume().unwrap() - 1.0).abs() < 1e-9);

        // Deterministic repeated behavior: sewing the same faces again
        // produces the identical report.
        let (_sewed2, _lineage2, report2) = context.sew(&face_refs, 1e-6).unwrap();
        assert_eq!(report, report2);
    }

    #[test]
    fn sewing_two_edge_adjacent_squares_merges_them_and_reports_the_perimeter() {
        let context = OcctContext::new().unwrap();
        let e0 = context
            .make_line_edge(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0))
            .unwrap();
        let e1 = context
            .make_line_edge(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0))
            .unwrap();
        let e2 = context
            .make_line_edge(Point3::new(1.0, 1.0, 0.0), Point3::new(0.0, 1.0, 0.0))
            .unwrap();
        let e3 = context
            .make_line_edge(Point3::new(0.0, 1.0, 0.0), Point3::new(0.0, 0.0, 0.0))
            .unwrap();
        let wire1 = context.make_wire_from_edges(&[&e0, &e1, &e2, &e3]).unwrap();
        let face1 = wire1
            .make_face_on_plane(&[], Point3::ORIGIN, Direction3::Z, false)
            .unwrap();

        let f0 = context
            .make_line_edge(Point3::new(1.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0))
            .unwrap();
        let f1 = context
            .make_line_edge(Point3::new(2.0, 0.0, 0.0), Point3::new(2.0, 1.0, 0.0))
            .unwrap();
        let f2 = context
            .make_line_edge(Point3::new(2.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0))
            .unwrap();
        let f3 = context
            .make_line_edge(Point3::new(1.0, 1.0, 0.0), Point3::new(1.0, 0.0, 0.0))
            .unwrap();
        let wire2 = context.make_wire_from_edges(&[&f0, &f1, &f2, &f3]).unwrap();
        let face2 = wire2
            .make_face_on_plane(&[], Point3::ORIGIN, Direction3::Z, false)
            .unwrap();

        let (sewed, _lineage, report) = context.sew(&[&face1, &face2], 1e-6).unwrap();
        assert!(report.changed);
        assert!(report.is_valid);
        // Two unit squares glued along one shared edge -> a 1x2 rectangle,
        // whose own perimeter is 6 unique boundary edges.
        assert_eq!(report.free_edge_count, 6);
        assert!((sewed.area().unwrap() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn sew_rejects_an_empty_shape_list() {
        let context = OcctContext::new().unwrap();
        assert_eq!(
            context.sew(&[], 1e-6).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn healing_an_already_valid_box_reports_unchanged() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        let (healed, report) = box_shape.heal(1e-6).unwrap();
        assert!(!report.changed);
        assert!(report.is_valid_before);
        assert!(report.is_valid_after);
        assert!(!report.kind_changed);
        assert!((healed.volume().unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn healing_an_unclosable_solid_reports_valid_after_only_alongside_kind_changed() {
        // AICAD-120's own central discovery: ShapeFix_Shape cannot invent
        // the 5 missing faces a solid built from a single standalone face
        // would need to actually close, so it silently demotes the Solid
        // to a bare Shell -- which trivially validates (no closure
        // requirement). `is_valid_after` alone would misrepresent this as
        // a successful repair; `kind_changed` is the required evidence
        // that stops that misreading.
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        let single_face = box_shape.get_face(0).unwrap();
        let open_shell = context.make_shell(&[&single_face]).unwrap();
        let open_solid = open_shell.make_solid(&[]).unwrap();
        assert!(!open_solid.validate().unwrap().is_valid);

        let (_healed, report) = open_solid.heal(1e-6).unwrap();
        assert!(report.changed);
        assert!(!report.is_valid_before);
        assert!(report.is_valid_after);
        assert!(
            report.kind_changed,
            "a bare 'is_valid_after: true' here would silently misrepresent an unclosable \
             solid demoted to a Shell as a genuine repair"
        );
    }

    // --- AICAD-121: safe topology inspection ---

    #[test]
    fn topology_kind_classifies_every_concrete_entity_kind() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        assert_eq!(box_shape.topology_kind().unwrap(), TopologyKind::Solid);

        let face = box_shape.get_face(0).unwrap();
        assert_eq!(face.topology_kind().unwrap(), TopologyKind::Face);

        let wire = face.get_wire(0).unwrap();
        assert_eq!(wire.topology_kind().unwrap(), TopologyKind::Wire);

        let edge = face.get_edge(0).unwrap();
        assert_eq!(edge.topology_kind().unwrap(), TopologyKind::Edge);

        let vertex = face.get_vertex(0).unwrap();
        assert_eq!(vertex.topology_kind().unwrap(), TopologyKind::Vertex);

        let shell = context.make_shell(&[&face]).unwrap();
        assert_eq!(shell.topology_kind().unwrap(), TopologyKind::Shell);
    }

    #[test]
    fn topology_kind_rejects_a_compound_as_unclassifiable() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face = box_shape.get_face(0).unwrap();
        let compound = context.make_compound(&[&face]).unwrap();
        assert_eq!(
            compound.topology_kind().unwrap_err(),
            KernelError::OperationFailed
        );
    }

    #[test]
    fn is_forward_oriented_is_deterministic_across_repeated_calls() {
        // Not every face of a real solid is FORWARD-oriented relative to
        // the solid's own topology -- verified empirically (box face 0 is
        // REVERSED) -- so this asserts determinism/repeatability, not a
        // specific expected orientation (matching this task's own
        // acceptance line: tests must not rely on a specific kernel
        // enumeration/orientation outcome, only that it is stable).
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face = box_shape.get_face(0).unwrap();
        let first = face.is_forward_oriented().unwrap();
        let second = face.is_forward_oriented().unwrap();
        assert_eq!(first, second);
    }

    // --- AICAD-032: display tessellation output ---

    #[test]
    fn a_box_tessellates_into_exactly_twelve_triangles() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(2.0, 3.0, 4.0).unwrap();
        let mesh = box_shape.tessellate(0.1, 0.5).unwrap();
        assert_eq!(mesh.triangle_count(), 12);
        assert_eq!(mesh.vertices.len(), 36);
        assert_eq!(mesh.normals.len(), 36);
    }

    #[test]
    fn tessellated_box_vertices_span_its_analytic_bounding_box() {
        let context = OcctContext::new().unwrap();
        let (dx, dy, dz) = (2.0, 3.0, 4.0);
        let box_shape = context.create_box(dx, dy, dz).unwrap();
        let mesh = box_shape.tessellate(0.1, 0.5).unwrap();
        let (mut min, mut max) = (
            Point3::new(1e300, 1e300, 1e300),
            Point3::new(-1e300, -1e300, -1e300),
        );
        for v in &mesh.vertices {
            min = Point3::new(min.x.min(v.x), min.y.min(v.y), min.z.min(v.z));
            max = Point3::new(max.x.max(v.x), max.y.max(v.y), max.z.max(v.z));
        }
        assert!((min.x - 0.0).abs() < 1e-9 && (max.x - dx).abs() < 1e-9);
        assert!((min.y - 0.0).abs() < 1e-9 && (max.y - dy).abs() < 1e-9);
        assert!((min.z - 0.0).abs() < 1e-9 && (max.z - dz).abs() < 1e-9);
    }

    #[test]
    fn every_tessellated_box_normal_is_a_unit_axis_aligned_vector() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        let mesh = box_shape.tessellate(0.1, 0.5).unwrap();
        for n in &mesh.normals {
            let len = (n.x * n.x + n.y * n.y + n.z * n.z).sqrt();
            assert!((len - 1.0).abs() < 1e-6);
            let axis_aligned_count = [n.x, n.y, n.z]
                .iter()
                .filter(|c| (c.abs() - 1.0).abs() < 1e-6)
                .count();
            assert_eq!(axis_aligned_count, 1);
        }
    }

    #[test]
    fn a_cylinder_tessellates_into_more_than_a_trivial_handful_of_triangles() {
        let context = OcctContext::new().unwrap();
        let cylinder = context.create_cylinder(1.0, 2.0).unwrap();
        let mesh = cylinder.tessellate(0.05, 0.2).unwrap();
        assert!(mesh.triangle_count() > 8);
    }

    #[test]
    fn tessellate_rejects_non_positive_deflections() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        assert_eq!(
            box_shape.tessellate(0.0, 0.5).unwrap_err(),
            KernelError::InvalidArgument
        );
        assert_eq!(
            box_shape.tessellate(0.1, 0.0).unwrap_err(),
            KernelError::InvalidArgument
        );
        assert_eq!(
            box_shape.tessellate(-0.1, 0.5).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn two_shapes_tessellate_independently() {
        let context = OcctContext::new().unwrap();
        let box_a = context.create_box(1.0, 1.0, 1.0).unwrap();
        let box_b = context.create_box(5.0, 5.0, 5.0).unwrap();
        let mesh_a = box_a.tessellate(0.1, 0.5).unwrap();
        let _mesh_b = box_b.tessellate(0.1, 0.5).unwrap();
        let max_x = mesh_a.vertices.iter().fold(f64::MIN, |acc, v| acc.max(v.x));
        assert!(
            (max_x - 1.0).abs() < 1e-9,
            "box_a's mesh must still reflect its own dimensions"
        );
    }

    // --- AICAD-033: STEP export ---

    #[test]
    fn export_step_writes_a_syntactically_valid_step_file() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(2.0, 3.0, 4.0).unwrap();
        let path = std::env::temp_dir().join("aicad_rust_step_export_test.step");
        box_shape.export_step(&path).unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.starts_with("ISO-10303-21;"));
        assert!(contents.contains("FILE_SCHEMA("));
        assert!(contents.contains("MANIFOLD_SOLID_BREP("));
        assert_eq!(
            contents.matches("ADVANCED_FACE(").count(),
            6,
            "a box has 6 unique faces (AICAD-028)"
        );
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn export_step_fails_cleanly_for_an_unwritable_path() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(1.0, 1.0, 1.0).unwrap();
        let path = std::path::Path::new("/nonexistent_directory_aicad/x.step");
        assert_eq!(
            box_shape.export_step(path).unwrap_err(),
            KernelError::OperationFailed
        );
    }

    #[test]
    fn concurrent_export_step_from_independent_contexts_does_not_crash() {
        // Regression test for a genuine native defect found while writing
        // this task's own tests: OCCT's STEP translator holds
        // process-global, non-thread-safe state, and calling
        // aicad_occt_export_step concurrently from independent contexts
        // on independent threads intermittently segfaulted the process
        // before native/occt_bridge/src/aicad_occt_bridge.cpp's
        // StepExportMutex fix (project/reports/AICAD-033.md). This test
        // reproduces the exact concurrency pattern that crashed and
        // confirms it no longer does, matching AGENTS.md's native
        // crash/hang policy ("add a permanent regression case").
        const THREAD_COUNT: usize = 8;
        const EXPORTS_PER_THREAD: usize = 20;

        let results: Vec<bool> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..THREAD_COUNT)
                .map(|t| {
                    scope.spawn(move || {
                        let context = OcctContext::new()
                            .expect("context creation should succeed on a worker thread");
                        let path = std::env::temp_dir()
                            .join(format!("aicad_step_concurrency_test_{t}.step"));
                        for i in 0..EXPORTS_PER_THREAD {
                            let side = 1.0 + (i as f64) * 0.01;
                            let shape = context
                                .create_box(side, side, side)
                                .expect("create_box should succeed on a worker thread");
                            if shape.export_step(&path).is_err() {
                                return false;
                            }
                        }
                        std::fs::remove_file(&path).ok();
                        true
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

        assert!(
            results.iter().all(|&ok| ok),
            "every thread's every export_step call should succeed without crashing the process"
        );
    }

    // --- AICAD-036: fillet concurrency regression (found while building
    // the Stage-1 proof bracket, AICAD-034) ---

    #[test]
    fn concurrent_fillet_of_a_concave_edge_from_independent_contexts_does_not_corrupt_the_result() {
        // Regression test for a genuine native defect found while building
        // the Stage-1 proof bracket (project/reports/AICAD-034.md /
        // AICAD-036.md): BRepFilletAPI_MakeFillet (and the ChFi3d
        // fillet-construction machinery it drives) intermittently produced
        // an invalid B-rep (1-2 invalid faces per `Shape::validate`) when
        // filleting a concave/reentrant edge on a multi-boolean shape
        // concurrently from independent contexts on independent threads --
        // reproduced at a ~47% failure rate over 30 repetitions with just 3
        // concurrent threads, while the byte-for-byte identical
        // construction run with NO concurrency never failed once across
        // 300 repetitions. `union`/`cut`/`chamfer` were independently
        // stress-tested under the same concurrent conditions and never
        // failed, isolating the defect specifically to
        // `BRepFilletAPI_MakeFillet`. Fixed the same way as AICAD-033's
        // STEP-translator finding: a process-wide mutex serializing every
        // `aicad_occt_fillet` call (`native/occt_bridge/src/aicad_occt_bridge.cpp`'s
        // `FilletMutex`). This test reproduces the exact concurrent
        // pattern that corrupted results and confirms it no longer does,
        // per AGENTS.md's native crash/hang policy ("add a permanent
        // regression case").
        const THREADS: usize = 3;
        const ROUNDS: usize = 15;

        fn concave_l_shape(context: &OcctContext) -> Shape<'_> {
            // An L-shaped bracket cross-section (base + perpendicular
            // wall, genuine 3D overlap, not a coincident interface) with
            // one concave/reentrant interior root edge -- the exact
            // geometry category the original defect was found on.
            let base = context.create_box(20.0, 15.0, 5.0).unwrap();
            let wall = context.create_box(20.0, 5.0, 15.0).unwrap();
            base.union(&wall).unwrap()
        }

        fn find_root_edge<'ctx>(shape: &Shape<'ctx>) -> Shape<'ctx> {
            let count = shape.edge_count().unwrap();
            (0..count)
                .map(|i| shape.get_edge(i).unwrap())
                .find(|edge| {
                    let b = edge.bounding_box().unwrap();
                    let close = |a: f64, b: f64| (a - b).abs() < 1e-6;
                    close(b.min.y, 5.0)
                        && close(b.max.y, 5.0)
                        && close(b.min.z, 5.0)
                        && close(b.max.z, 5.0)
                        && close(b.min.x, 0.0)
                        && close(b.max.x, 20.0)
                })
                .expect("the concave root edge must be found")
        }

        for _ in 0..ROUNDS {
            let all_valid: Vec<bool> = std::thread::scope(|scope| {
                let handles: Vec<_> = (0..THREADS)
                    .map(|_| {
                        scope.spawn(|| {
                            let context = OcctContext::new()
                                .expect("context creation should succeed on a worker thread");
                            let shape = concave_l_shape(&context);
                            let root_edge = find_root_edge(&shape);
                            let filleted = shape
                                .fillet(&[&root_edge], 2.0)
                                .expect("fillet should succeed for the concave root edge");
                            filleted
                                .is_valid()
                                .expect("is_valid should not itself fail")
                        })
                    })
                    .collect();
                handles.into_iter().map(|h| h.join().unwrap()).collect()
            });
            assert!(
                all_valid.iter().all(|&ok| ok),
                "every thread's concurrent fillet of the concave root edge must produce a valid B-rep"
            );
        }
    }

    // --- AICAD-035: STEP import (round-trip verification harness) ---

    #[test]
    fn import_step_round_trip_preserves_volume_bbox_and_topology_counts() {
        // NOT independent verification of the exporter (both directions
        // share one OCCT installation) -- proves the export/import
        // pipeline is internally self-consistent for a shape whose exact
        // properties this bridge already independently established.
        // See project/reports/AICAD-035.md for the genuinely independent
        // (non-OCCT) verification path used alongside this.
        let context = OcctContext::new().unwrap();
        let (dx, dy, dz) = (3.0, 4.0, 5.0);
        let original = context.create_box(dx, dy, dz).unwrap();
        let path = std::env::temp_dir().join("aicad_rust_step_import_round_trip.step");
        original.export_step(&path).unwrap();

        let reimported = context
            .import_step(&path)
            .expect("import_step should succeed for the file this bridge just exported");
        assert!(reimported.is_valid().unwrap());

        assert!((reimported.volume().unwrap() - original.volume().unwrap()).abs() < 1e-9);
        assert!((original.volume().unwrap() - dx * dy * dz).abs() < 1e-9);

        let original_bbox = original.bounding_box().unwrap();
        let reimported_bbox = reimported.bounding_box().unwrap();
        assert!((original_bbox.min.x - reimported_bbox.min.x).abs() < 1e-9);
        assert!((original_bbox.max.x - reimported_bbox.max.x).abs() < 1e-9);
        assert!((original_bbox.min.y - reimported_bbox.min.y).abs() < 1e-9);
        assert!((original_bbox.max.y - reimported_bbox.max.y).abs() < 1e-9);
        assert!((original_bbox.min.z - reimported_bbox.min.z).abs() < 1e-9);
        assert!((original_bbox.max.z - reimported_bbox.max.z).abs() < 1e-9);

        assert_eq!(
            original.face_count().unwrap(),
            reimported.face_count().unwrap()
        );
        assert_eq!(
            original.edge_count().unwrap(),
            reimported.edge_count().unwrap()
        );
        assert_eq!(original.face_count().unwrap(), 6);
        assert_eq!(original.edge_count().unwrap(), 12);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn import_step_rejects_a_nonexistent_file() {
        let context = OcctContext::new().unwrap();
        let path = std::path::Path::new("/nonexistent_directory_aicad/x.step");
        assert_eq!(
            context.import_step(path).unwrap_err(),
            KernelError::OperationFailed
        );
    }

    #[test]
    fn import_step_rejects_a_syntactically_invalid_file() {
        let context = OcctContext::new().unwrap();
        let path = std::env::temp_dir().join("aicad_rust_step_import_garbage.step");
        std::fs::write(&path, b"this is not a STEP file\n").unwrap();
        assert_eq!(
            context.import_step(&path).unwrap_err(),
            KernelError::OperationFailed
        );
        std::fs::remove_file(&path).ok();
    }

    // --- AICAD-082: face/edge geometric classification ---

    fn faces<'ctx>(shape: &Shape<'ctx>) -> Vec<Shape<'ctx>> {
        (0..shape.face_count().unwrap())
            .map(|i| shape.get_face(i).unwrap())
            .collect()
    }

    fn edges<'ctx>(shape: &Shape<'ctx>) -> Vec<Shape<'ctx>> {
        (0..shape.edge_count().unwrap())
            .map(|i| shape.get_edge(i).unwrap())
            .collect()
    }

    #[test]
    fn surface_type_reports_plane_for_every_box_face() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 3.0, 4.0).unwrap();
        for face in faces(&cube) {
            assert_eq!(face.surface_type().unwrap(), SurfaceKind::Plane);
        }
    }

    #[test]
    fn surface_type_reports_cylinder_for_exactly_one_capped_cylinder_face() {
        let context = OcctContext::new().unwrap();
        let cylinder = context.create_cylinder(2.0, 5.0).unwrap();
        let kinds: Vec<SurfaceKind> = faces(&cylinder)
            .iter()
            .map(|f| f.surface_type().unwrap())
            .collect();
        assert_eq!(
            kinds
                .iter()
                .filter(|k| **k == SurfaceKind::Cylinder)
                .count(),
            1
        );
        assert!(
            kinds
                .iter()
                .all(|k| matches!(k, SurfaceKind::Plane | SurfaceKind::Cylinder))
        );
    }

    #[test]
    fn surface_type_rejects_a_non_face_handle() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let edge = cube.get_edge(0).unwrap();
        assert_eq!(
            edge.surface_type().unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    fn cylindrical_face<'ctx>(shape: &Shape<'ctx>) -> Shape<'ctx> {
        faces(shape)
            .into_iter()
            .find(|f| f.surface_type().unwrap() == SurfaceKind::Cylinder)
            .expect("shape must have a cylindrical face")
    }

    #[test]
    fn face_radius_matches_the_constructed_cylinder_radius() {
        let context = OcctContext::new().unwrap();
        let cylinder = context.create_cylinder(2.5, 5.0).unwrap();
        let lateral = cylindrical_face(&cylinder);
        assert!((lateral.face_radius().unwrap() - 2.5).abs() < 1e-9);
    }

    #[test]
    fn face_radius_rejects_a_planar_face() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face = cube.get_face(0).unwrap();
        assert_eq!(face.surface_type().unwrap(), SurfaceKind::Plane);
        assert_eq!(
            face.face_radius().unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn face_axis_is_parallel_to_plus_z_for_an_origin_cylinder() {
        let context = OcctContext::new().unwrap();
        let cylinder = context.create_cylinder(2.0, 5.0).unwrap();
        let lateral = cylindrical_face(&cylinder);
        let axis = lateral.face_axis().unwrap();
        assert!((axis.direction.dot(Direction3::Z).abs() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn face_axis_rejects_a_planar_face() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face = cube.get_face(0).unwrap();
        assert_eq!(face.face_axis().unwrap_err(), KernelError::InvalidArgument);
    }

    #[test]
    fn face_normal_is_a_unit_vector_and_lies_on_the_bounding_box() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 3.0, 4.0).unwrap();
        let bbox = cube.bounding_box().unwrap();
        for face in faces(&cube) {
            let (point, normal) = face.face_normal().unwrap();
            let length = (normal.as_vector3().x.powi(2)
                + normal.as_vector3().y.powi(2)
                + normal.as_vector3().z.powi(2))
            .sqrt();
            assert!((length - 1.0).abs() < 1e-9);
            let on_boundary = (point.x - bbox.min.x).abs() < 1e-6
                || (point.x - bbox.max.x).abs() < 1e-6
                || (point.y - bbox.min.y).abs() < 1e-6
                || (point.y - bbox.max.y).abs() < 1e-6
                || (point.z - bbox.min.z).abs() < 1e-6
                || (point.z - bbox.max.z).abs() < 1e-6;
            assert!(on_boundary, "face midpoint {point:?} not on box boundary");
        }
    }

    #[test]
    fn face_normal_points_outward_from_the_box_center() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let center = cube.center_of_mass().unwrap();
        for face in faces(&cube) {
            let (point, normal) = face.face_normal().unwrap();
            let outward = cad_kernel_api::Vector3::new(
                point.x - center.x,
                point.y - center.y,
                point.z - center.z,
            );
            assert!(
                outward.dot(normal.as_vector3()) > 0.0,
                "normal at {point:?} should point away from the box center"
            );
        }
    }

    #[test]
    fn curve_type_reports_line_for_every_box_edge() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        for edge in edges(&cube) {
            assert_eq!(edge.curve_type().unwrap(), CurveKind::Line);
        }
    }

    #[test]
    fn curve_type_reports_circle_for_cylinder_rim_edges() {
        let context = OcctContext::new().unwrap();
        let cylinder = context.create_cylinder(2.0, 5.0).unwrap();
        let circle_count = edges(&cylinder)
            .iter()
            .filter(|e| e.curve_type().unwrap() == CurveKind::Circle)
            .count();
        assert_eq!(
            circle_count, 2,
            "a capped cylinder has exactly 2 circular rim edges"
        );
    }

    #[test]
    fn curve_type_rejects_a_non_edge_handle() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face = cube.get_face(0).unwrap();
        assert_eq!(face.curve_type().unwrap_err(), KernelError::InvalidArgument);
    }

    fn a_circular_edge<'ctx>(shape: &Shape<'ctx>) -> Shape<'ctx> {
        edges(shape)
            .into_iter()
            .find(|e| e.curve_type().unwrap() == CurveKind::Circle)
            .expect("shape must have a circular edge")
    }

    #[test]
    fn edge_radius_matches_the_constructed_cylinder_radius() {
        let context = OcctContext::new().unwrap();
        let cylinder = context.create_cylinder(2.5, 5.0).unwrap();
        let rim = a_circular_edge(&cylinder);
        assert!((rim.edge_radius().unwrap() - 2.5).abs() < 1e-9);
    }

    #[test]
    fn edge_radius_rejects_a_line_edge() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let edge = cube.get_edge(0).unwrap();
        assert_eq!(
            edge.edge_radius().unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn edge_axis_is_parallel_to_the_cylinder_axis() {
        let context = OcctContext::new().unwrap();
        let cylinder = context.create_cylinder(2.0, 5.0).unwrap();
        let rim = a_circular_edge(&cylinder);
        let axis = rim.edge_axis().unwrap();
        assert!((axis.direction.dot(Direction3::Z).abs() - 1.0).abs() < 1e-9);
    }

    // --- AICAD-083: shape identity ---

    #[test]
    fn is_same_reports_true_for_two_handles_of_the_same_face() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let a = cube.get_face(0).unwrap();
        let b = cube.get_face(0).unwrap();
        assert!(a.is_same(&b).unwrap());
    }

    #[test]
    fn is_same_reports_false_for_two_different_faces() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let a = cube.get_face(0).unwrap();
        let b = cube.get_face(1).unwrap();
        assert!(!a.is_same(&b).unwrap());
    }

    #[test]
    fn is_same_reports_true_across_two_independent_adjacency_lookups() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        // Every unique edge of a manifold solid box is adjacent to
        // exactly 2 faces; fetching the same adjacent face a second time
        // via a fresh lookup must report `is_same` against the first.
        let first = cube.edge_adjacent_face(0, 0).unwrap();
        let first_again = cube.edge_adjacent_face(0, 0).unwrap();
        assert!(first.is_same(&first_again).unwrap());
    }

    #[test]
    fn is_same_reports_false_across_independent_contexts() {
        let context_a = OcctContext::new().unwrap();
        let context_b = OcctContext::new().unwrap();
        let cube_a = context_a.create_box(1.0, 1.0, 1.0).unwrap();
        let cube_b = context_b.create_box(1.0, 1.0, 1.0).unwrap();
        let face_a = cube_a.get_face(0).unwrap();
        let face_b = cube_b.get_face(0).unwrap();
        assert!(!face_a.is_same(&face_b).unwrap());
    }

    // --- AICAD-083: wire enumeration / outer-boundary ---

    #[test]
    fn a_box_face_has_exactly_one_wire_and_it_is_the_outer_wire() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face = cube.get_face(0).unwrap();
        assert_eq!(face.wire_count().unwrap(), 1);
        let wire = face.get_wire(0).unwrap();
        assert!(face.is_outer_wire(&wire).unwrap());
    }

    #[test]
    fn is_outer_wire_rejects_a_wire_from_an_unrelated_face() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face0 = cube.get_face(0).unwrap();
        let face1 = cube.get_face(1).unwrap();
        let unrelated_wire = face1.get_wire(0).unwrap();
        // A face's own outer wire is never `is_same` as a wire that
        // actually bounds an entirely different face.
        assert!(!face0.is_outer_wire(&unrelated_wire).unwrap());
    }

    #[test]
    fn get_wire_rejects_an_out_of_range_index() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let face = cube.get_face(0).unwrap();
        assert_eq!(
            face.get_wire(face.wire_count().unwrap()).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    // --- AICAD-084: vertex point / point-solid classification ---

    #[test]
    fn vertex_point_matches_a_box_corner() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 3.0, 4.0).unwrap();
        let bbox = cube.bounding_box().unwrap();
        // `Bnd_Box` enlarges by a small internal gap tolerance, so
        // `bounding_box()`'s own min/max are not bit-exact with any
        // vertex's true coordinate -- compare within a tolerance well
        // above that gap instead of exact equality.
        let close = |a: f64, b: f64| (a - b).abs() < 1e-6;
        for i in 0..cube.vertex_count().unwrap() {
            let vertex = cube.get_vertex(i).unwrap();
            let point = vertex.vertex_point().unwrap();
            assert!(close(point.x, bbox.min.x) || close(point.x, bbox.max.x));
            assert!(close(point.y, bbox.min.y) || close(point.y, bbox.max.y));
            assert!(close(point.z, bbox.min.z) || close(point.z, bbox.max.z));
        }
    }

    #[test]
    fn vertex_point_rejects_a_non_vertex_handle() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let edge = cube.get_edge(0).unwrap();
        assert_eq!(
            edge.vertex_point().unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    #[test]
    fn classify_point_reports_inside_for_the_box_center() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let center = cube.center_of_mass().unwrap();
        assert_eq!(
            cube.classify_point(center, 1e-7).unwrap(),
            PointClassification::Inside
        );
    }

    #[test]
    fn classify_point_reports_outside_for_a_far_point() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let far = Point3::new(1000.0, 1000.0, 1000.0);
        assert_eq!(
            cube.classify_point(far, 1e-7).unwrap(),
            PointClassification::Outside
        );
    }

    #[test]
    fn classify_point_reports_on_boundary_for_a_face_point() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        // An exact vertex coordinate (unlike `bounding_box()`'s own
        // min/max, which `Bnd_Box` enlarges by a small internal gap) lies
        // exactly on the solid's own boundary.
        let corner = cube.get_vertex(0).unwrap().vertex_point().unwrap();
        assert_eq!(
            cube.classify_point(corner, 1e-7).unwrap(),
            PointClassification::OnBoundary
        );
    }

    #[test]
    fn classify_point_rejects_a_non_finite_point() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(1.0, 1.0, 1.0).unwrap();
        let bad = Point3::new(f64::NAN, 0.0, 0.0);
        assert_eq!(
            cube.classify_point(bad, 1e-7).unwrap_err(),
            KernelError::InvalidArgument
        );
    }

    /// Classifies one of a 10x10x10 origin box's own axis-aligned faces by
    /// which coordinate plane it lies flat against, for the `_lineage`
    /// tests below (a plain, geometry-based classification independent of
    /// `get_face`'s own raw enumeration order, matching this crate's own
    /// "never assume kernel enumeration order" convention).
    fn classify_axis_aligned_box_face(bbox: &BoundingBox) -> &'static str {
        let flat_x = (bbox.max.x - bbox.min.x).abs() < 1e-6;
        let flat_y = (bbox.max.y - bbox.min.y).abs() < 1e-6;
        let flat_z = (bbox.max.z - bbox.min.z).abs() < 1e-6;
        if flat_z && bbox.min.z < 1.0 {
            "bottom"
        } else if flat_z {
            "top"
        } else if flat_x || flat_y {
            "side"
        } else {
            panic!("face is not axis-aligned-flat: {bbox:?}")
        }
    }

    /// `AICAD-086`: a straight-through cylindrical hole only ever touches
    /// the two faces it actually pierces (top/bottom); the four side
    /// faces it never reaches must report no lineage evidence at all
    /// (`deleted == false`, `generated`/`modified` both empty) -- not a
    /// guessed `false`, a real evidenced absence.
    #[test]
    fn cut_with_lineage_marks_pierced_faces_and_leaves_untouched_faces_evidence_free() {
        let context = OcctContext::new().unwrap();
        let a = context.create_box(10.0, 10.0, 10.0).unwrap();
        let cyl_raw = context.create_cylinder(2.0, 20.0).unwrap();
        // A radius-2 cylinder centered at box XY-center (5, 5), spanning
        // z -5..15 -- comfortably inside the box's own x/y=0..10 footprint
        // (never touching the four side faces) while fully piercing
        // through the box's own z=0..10 extent (touching top and bottom).
        let cyl = cyl_raw
            .transform(&Transform::translation(cad_kernel_api::Vector3::new(
                5.0, 5.0, -5.0,
            )))
            .unwrap();
        let faces: Vec<Shape> = (0..a.face_count().unwrap())
            .map(|i| a.get_face(i).unwrap())
            .collect();
        let (result, lineage) = a.cut_with_lineage(&cyl).expect("cut should succeed");
        assert!(result.is_valid().unwrap());
        for f in &faces {
            let bbox = f.bounding_box().unwrap();
            let kind = classify_axis_aligned_box_face(&bbox);
            let deleted = lineage.is_deleted(f).unwrap();
            let generated = lineage.generated(f).unwrap();
            let modified = lineage.modified(f).unwrap();
            assert!(
                !deleted,
                "a pierced-but-not-fully-removed face is never deleted"
            );
            match kind {
                "top" | "bottom" => assert!(
                    !generated.is_empty() || !modified.is_empty(),
                    "a face the hole actually pierces must carry generated/modified evidence"
                ),
                "side" => assert!(
                    generated.is_empty() && modified.is_empty(),
                    "a face the hole never reaches must carry no lineage evidence at all"
                ),
                other => panic!("unexpected face classification {other}"),
            }
        }
    }

    /// `AICAD-086`: a cutting tool that entirely swallows one whole face
    /// (rather than merely trimming it) must report that face `deleted`
    /// -- with no generated/modified counterpart of its own, since it
    /// does not survive into the result at all. The opposite (untouched)
    /// face and the four trimmed side faces must each report their own,
    /// different, correctly evidenced state.
    #[test]
    fn cut_with_lineage_marks_a_wholly_removed_face_as_deleted() {
        let context = OcctContext::new().unwrap();
        let a = context.create_box(10.0, 10.0, 10.0).unwrap();
        let tool_raw = context.create_box(20.0, 20.0, 10.0).unwrap();
        // A 20x20x10 tool centered over the box's own top half (z 5..15,
        // x/y -5..15) entirely contains the box's own top face (z=10,
        // x/y 0..10) while leaving the bottom face (z=0) untouched.
        let tool = tool_raw
            .transform(&Transform::translation(cad_kernel_api::Vector3::new(
                -5.0, -5.0, 5.0,
            )))
            .unwrap();
        let faces: Vec<Shape> = (0..a.face_count().unwrap())
            .map(|i| a.get_face(i).unwrap())
            .collect();
        let (result, lineage) = a.cut_with_lineage(&tool).expect("cut should succeed");
        assert!(result.is_valid().unwrap());
        for f in &faces {
            let bbox = f.bounding_box().unwrap();
            let kind = classify_axis_aligned_box_face(&bbox);
            let deleted = lineage.is_deleted(f).unwrap();
            let generated = lineage.generated(f).unwrap();
            let modified = lineage.modified(f).unwrap();
            match kind {
                "top" => {
                    assert!(
                        deleted,
                        "the wholly-swallowed top face must be reported deleted"
                    );
                    assert!(generated.is_empty() && modified.is_empty());
                }
                "bottom" => {
                    assert!(!deleted);
                    assert!(
                        generated.is_empty() && modified.is_empty(),
                        "the untouched bottom face must carry no lineage evidence"
                    );
                }
                "side" => {
                    assert!(!deleted, "a merely-trimmed side face is not deleted");
                    assert!(
                        !generated.is_empty() || !modified.is_empty(),
                        "a side face trimmed by the tool must carry evidence"
                    );
                }
                other => panic!("unexpected face classification {other}"),
            }
        }
    }

    /// `AICAD-086`: filleting one edge of a box must leave real,
    /// differentiated lineage evidence -- some faces touched (adjacent to
    /// the filleted edge or its endpoints), some left with no evidence at
    /// all, and none ever wholly deleted by a single-edge fillet. Exactly
    /// which faces land in which group is real OCCT behavior this test
    /// observes rather than assumes (the box's own face-enumeration order
    /// is not itself semantic, matching this crate's other tests), but
    /// the differentiation itself -- not "every face identically
    /// touched," not "every face identically untouched" -- is the
    /// property this task actually needs.
    #[test]
    fn fillet_with_lineage_differentiates_touched_from_untouched_faces() {
        let context = OcctContext::new().unwrap();
        let a = context.create_box(10.0, 10.0, 10.0).unwrap();
        let faces: Vec<Shape> = (0..a.face_count().unwrap())
            .map(|i| a.get_face(i).unwrap())
            .collect();
        let edge0 = a.get_edge(0).unwrap();
        let (result, lineage) = a
            .fillet_with_lineage(&[&edge0], 1.0)
            .expect("fillet should succeed");
        assert!(result.is_valid().unwrap());
        assert!(
            result.volume().unwrap() < a.volume().unwrap(),
            "rounding an edge must remove material"
        );
        let mut touched = 0;
        let mut untouched = 0;
        for f in &faces {
            assert!(
                !lineage.is_deleted(f).unwrap(),
                "a single-edge fillet never wholly deletes one of the box's own six faces"
            );
            let generated = lineage.generated(f).unwrap();
            let modified = lineage.modified(f).unwrap();
            if generated.is_empty() && modified.is_empty() {
                untouched += 1;
            } else {
                touched += 1;
            }
        }
        assert!(
            touched >= 2,
            "at least the two faces adjacent to the filleted edge must carry evidence"
        );
        assert!(
            untouched >= 1,
            "a face far from the filleted edge must carry no evidence"
        );
    }

    /// `AICAD-086`: `union_with_lineage`/`intersect_with_lineage` produce
    /// the same result geometry as their non-lineage counterparts
    /// (`Shape::union`/`Shape::intersect`) -- capturing lineage must never
    /// change the operation's own outcome.
    #[test]
    fn union_and_intersect_with_lineage_match_their_plain_counterparts() {
        let context = OcctContext::new().unwrap();
        let (a, b) = overlapping_boxes(&context);
        let (fused, _) = a.union_with_lineage(&b).expect("union should succeed");
        assert!((fused.volume().unwrap() - 15.0).abs() < 1e-6);
        let (a2, b2) = overlapping_boxes(&context);
        let (common, _) = a2
            .intersect_with_lineage(&b2)
            .expect("intersect should succeed");
        assert!((common.volume().unwrap() - 1.0).abs() < 1e-6);
    }

    /// `AICAD-086`: querying lineage with a face from a shape that was
    /// never one of the operation's own two original operands at all is a
    /// distinct, explicit error -- never silently answered as
    /// "unchanged," which would be indistinguishable from real evidence.
    /// (A face of the operation's own *result* is deliberately not used
    /// here: cut/union/intersect can carry an untouched or even
    /// unmodified operand face through into the result unchanged, so it
    /// would not reliably exercise the "truly unrelated" case this test
    /// targets -- an entirely separate, never-passed-in shape does.)
    #[test]
    fn lineage_query_for_an_unrelated_shape_is_an_explicit_error_not_a_silent_unchanged() {
        let context = OcctContext::new().unwrap();
        let a = context.create_box(10.0, 10.0, 10.0).unwrap();
        let cyl_raw = context.create_cylinder(2.0, 20.0).unwrap();
        let cyl = cyl_raw
            .transform(&Transform::translation(cad_kernel_api::Vector3::new(
                5.0, 5.0, -5.0,
            )))
            .unwrap();
        let (_result, lineage) = a.cut_with_lineage(&cyl).expect("cut should succeed");
        let unrelated = context.create_box(1.0, 1.0, 1.0).unwrap();
        let unrelated_face = unrelated.get_face(0).unwrap();
        assert_eq!(
            lineage.is_deleted(&unrelated_face).unwrap_err(),
            KernelError::InvalidArgument
        );
    }
}
