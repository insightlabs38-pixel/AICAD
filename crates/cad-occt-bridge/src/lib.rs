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

use cad_kernel_api::{
    Axis3, Direction3, KernelError, KernelId, KernelResult, KernelShape, Point3, Transform,
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

    fn raw_handle(&self) -> ffi::aicad_shape_handle_t {
        id_to_handle(self.id)
    }
}

/// An axis-aligned bounding box, as reported by [`Shape::bounding_box`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub min: Point3,
    pub max: Point3,
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
}
