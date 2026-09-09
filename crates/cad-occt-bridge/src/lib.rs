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
    Direction3, KernelError, KernelId, KernelResult, KernelShape, Point3, Transform,
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
}
