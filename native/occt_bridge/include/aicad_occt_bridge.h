/* AICAD-016: native/occt_bridge C ABI boundary.
 *
 * This header is the ONLY contract crossing the native<->Rust boundary.
 * It is deliberately kernel-neutral at the type level: no OCCT class or
 * enum name appears here, only plain C types (per RFC-0002 §3 / Stage-1
 * kernel policy #2-3). `crates/cad-occt-bridge` is the only Rust crate
 * that may declare bindings against this header.
 *
 * Every function returns an `aicad_occt_status_t`. No C++ or OCCT
 * exception is ever allowed to propagate out of a function declared here
 * -- each is caught internally and converted into a status code (Stage-1
 * kernel policy #6-7). No STL container, std::string, or OCCT-owned
 * object crosses this boundary directly (policy #8); only plain-old-data
 * structs and opaque pointers do.
 */

#ifndef AICAD_OCCT_BRIDGE_H
#define AICAD_OCCT_BRIDGE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Status codes returned by every aicad_occt_* function. */
typedef enum aicad_occt_status {
  AICAD_OCCT_OK = 0,
  /* A pointer/argument the caller passed is null or out of range. */
  AICAD_OCCT_ERR_INVALID_ARGUMENT = 1,
  /* The handle's slot never existed in this context's shape table. */
  AICAD_OCCT_ERR_INVALID_HANDLE = 2,
  /* The handle's slot existed but has since been released, or its
   * generation no longer matches the slot's current occupant -- i.e. the
   * handle is from a prior geometry epoch (Stage-1 kernel policy #5/#10). */
  AICAD_OCCT_ERR_STALE_HANDLE = 3,
  /* The handle's context_id does not match the context it was passed to. */
  AICAD_OCCT_ERR_FOREIGN_CONTEXT = 4,
  /* The calling thread is not the thread that created this context
   * (Stage-1 kernel policy #9: contexts are single-thread-affine). */
  AICAD_OCCT_ERR_WRONG_THREAD = 5,
  /* OCCT reported the requested geometric operation could not be
   * completed (e.g. a degenerate input) -- a normal, expected outcome for
   * some inputs, not a bridge defect. */
  AICAD_OCCT_ERR_OPERATION_FAILED = 6,
  /* An unexpected C++/OCCT exception was caught at the ABI boundary.
   * Always a bridge or OCCT defect, never an expected outcome. */
  AICAD_OCCT_ERR_INTERNAL = 7,
} aicad_occt_status_t;

/* Opaque, context-scoped shape handle -- never a raw OCCT/C++ pointer
 * (Stage-1 kernel policy #3). `context_id` and `generation` let the
 * bridge reject stale and foreign-context handles by value comparison
 * alone, without ever dereferencing caller-supplied data as a pointer.
 * Kernel topology handles are build/epoch-local (Stage-1 kernel policy
 * #10) and must never be treated as persistent AICAD semantic references
 * (RFC-0002 §4) -- that is `cad-references`' job, not this bridge's. */
typedef struct aicad_shape_handle {
  uint64_t context_id;
  uint32_t slot;
  uint32_t generation;
} aicad_shape_handle_t;

/* An all-zero handle is never a handle this bridge ever hands out
 * (context_id 0 is never assigned to a real context); it is provided
 * only as a caller-side "no handle yet" sentinel and receives no special
 * treatment beyond ordinary validation. */
#define AICAD_NULL_SHAPE_HANDLE \
  { 0, 0, 0 }

/* Opaque kernel context. Owns a shape table and is conservatively
 * single-thread-affine (Stage-1 kernel policy #9): every function that
 * takes a context must be called from the thread that created it, or
 * fails with AICAD_OCCT_ERR_WRONG_THREAD. */
typedef struct aicad_occt_context aicad_occt_context_t;

/* Creates a new kernel context. `*out_context` is set on success only. */
aicad_occt_status_t aicad_occt_context_create(aicad_occt_context_t** out_context);

/* Destroys a context and every shape it still owns. Using any handle
 * from this context afterward is undefined at the process level (the
 * context itself, not just a slot, is gone) -- callers must not retain
 * handles past context destruction. */
aicad_occt_status_t aicad_occt_context_destroy(aicad_occt_context_t* context);

/* Releases one shape handle. After this call the handle is stale: any
 * further use of it (with this or any other still-valid handle that
 * happens to share its slot after reuse) is rejected with
 * AICAD_OCCT_ERR_STALE_HANDLE, never silently aliased onto a later
 * shape inserted into the same slot (Stage-1 kernel policy #5). */
aicad_occt_status_t aicad_occt_release_shape(aicad_occt_context_t* context,
                                              aicad_shape_handle_t handle);

/* --- Stage-1 starting operation set (native/occt_bridge/README.md) ---
 * Only create_box is implemented by AICAD-016; the remaining operations
 * in the README's list are added incrementally by later Stage-1 tasks,
 * per RFC-0002 §3's capability-driven minimal-surface rule -- this is not
 * a comprehensive up-front OCCT wrapper. */

/* Constructs an axis-aligned box of the given dimensions (in the
 * kernel's internal linear unit; unit semantics belong to `cad-units`
 * above this bridge, not here) and returns a handle to it. */
aicad_occt_status_t aicad_occt_create_box(aicad_occt_context_t* context,
                                           double dx,
                                           double dy,
                                           double dz,
                                           aicad_shape_handle_t* out_handle);

/* Constructs a capped cylindrical solid of the given radius/height,
 * centered on the origin with its axis along +Z (placement/orientation is
 * applied afterward via aicad_occt_transform_shape, per RFC-0002 §3's
 * capability-driven minimal-surface rule -- this bridge does not grow a
 * second, placement-aware constructor per primitive). */
aicad_occt_status_t aicad_occt_create_cylinder(aicad_occt_context_t* context,
                                                double radius,
                                                double height,
                                                aicad_shape_handle_t* out_handle);

/* --- AICAD-021: rigid/affine transforms (points/vectors/axes/frames are
 * pure Rust value types in `cad-kernel-api`; this is the one bridge
 * operation needed to actually move kernel-resident geometry, per
 * `docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s `transform` feature). ---
 *
 * `matrix` is a row-major 3x4 affine matrix:
 *   [ m[0]  m[1]  m[2]  m[3]  ]   [x]   [x']
 *   [ m[4]  m[5]  m[6]  m[7]  ] * [y] = [y']
 *   [ m[8]  m[9]  m[10] m[11] ]   [z]   [z']
 *                                 [1]
 * i.e. the leading 3x3 block is the linear (rotation/scale) part and the
 * last column is the translation. Always produces a NEW shape handle,
 * never mutates the input in place (DL-2's functional/value-oriented
 * semantics). */
aicad_occt_status_t aicad_occt_transform_shape(aicad_occt_context_t* context,
                                                aicad_shape_handle_t handle,
                                                const double matrix[12],
                                                aicad_shape_handle_t* out_handle);

/* --- AICAD-022: minimal curve/edge/wire construction. A raw
 * `Geom_Curve` is not exposed as its own handle kind yet (no Stage-1 task
 * needs curve evaluation independent of an edge) -- curves are
 * constructed directly into an edge, per the capability-driven
 * minimal-surface rule; a dedicated Curve handle can be added later
 * without breaking this surface. --- */

/* Constructs a straight edge between two points. */
aicad_occt_status_t aicad_occt_make_line_edge(aicad_occt_context_t* context,
                                               const double p0[3],
                                               const double p1[3],
                                               aicad_shape_handle_t* out_handle);

/* Constructs a closed circular wire (not just an edge -- a full circle is
 * always closed, and every Stage-1 consumer of a circle needs a Wire
 * directly usable by aicad_occt_make_face_from_wire). */
aicad_occt_status_t aicad_occt_make_circle_wire(aicad_occt_context_t* context,
                                                 const double center[3],
                                                 const double normal[3],
                                                 double radius,
                                                 aicad_shape_handle_t* out_handle);

/* Joins an ordered list of edges into one wire. `edges`/`edge_count` is a
 * caller-owned array (no STL container crosses this boundary); edges must
 * form a single connected chain (open or closed) in the given order. */
aicad_occt_status_t aicad_occt_make_wire_from_edges(aicad_occt_context_t* context,
                                                     const aicad_shape_handle_t* edges,
                                                     size_t edge_count,
                                                     aicad_shape_handle_t* out_handle);

/* --- AICAD-023: planar face from a closed wire. --- */

/* Builds a planar face bounded by `wire_handle`, which must address a
 * single closed, planar wire. The face's plane is inferred from the
 * wire's geometry (OCCT's own planarity deduction), not supplied
 * separately -- every Stage-1 profile is planar. */
aicad_occt_status_t aicad_occt_make_face_from_wire(aicad_occt_context_t* context,
                                                    aicad_shape_handle_t wire_handle,
                                                    aicad_shape_handle_t* out_handle);

/* --- AICAD-024: extrude and revolve, face -> solid. --- */

/* Linearly extrudes a planar face by `distance` along `direction`
 * (need not be unit-length; magnitude is ignored, only direction is
 * used) into a solid. */
aicad_occt_status_t aicad_occt_extrude(aicad_occt_context_t* context,
                                        aicad_shape_handle_t face_handle,
                                        const double direction[3],
                                        double distance,
                                        aicad_shape_handle_t* out_handle);

/* Revolves a planar face about an axis by `angle_radians` (0 < angle <=
 * 2*pi) into a solid. The face must not straddle the axis (OCCT rejects
 * a self-intersecting result with AICAD_OCCT_ERR_OPERATION_FAILED). */
aicad_occt_status_t aicad_occt_revolve(aicad_occt_context_t* context,
                                        aicad_shape_handle_t face_handle,
                                        const double axis_origin[3],
                                        const double axis_direction[3],
                                        double angle_radians,
                                        aicad_shape_handle_t* out_handle);

/* --- AICAD-025: sweep and loft, minimal supported forms.
 *
 * These are the first two Batch-1C "hard geometry operations": more
 * general than extrude/revolve's constant linear/rotational sweep, but
 * still deliberately narrow (RFC-0002 §3's capability-driven
 * minimal-surface rule) -- no variable-section sweep, no explicit
 * trihedron/up-vector control, and no ruled-vs-smoothed loft selection is
 * exposed yet. See `project/reports/AICAD-025.md` for exactly which
 * spine/section shapes are and are not supported -- per AGENTS.md, a
 * kernel-level limitation here is recorded honestly rather than forced to
 * appear universally successful. --- */

/* Sweeps a planar profile face along a path wire (the "spine"), producing
 * a solid. The spine may be open or closed and need not be planar or
 * straight, but OCCT requires it to be G1-continuous (no sharp tangent
 * discontinuity between consecutive edges) -- a polygonal spine with
 * sharp corners is rejected with AICAD_OCCT_ERR_OPERATION_FAILED rather
 * than silently healed or approximated. The profile is swept starting at
 * the spine's first vertex, oriented by OCCT's own corrected-Frenet
 * trihedron computation. */
aicad_occt_status_t aicad_occt_sweep(aicad_occt_context_t* context,
                                      aicad_shape_handle_t profile_face_handle,
                                      aicad_shape_handle_t spine_wire_handle,
                                      aicad_shape_handle_t* out_handle);

/* Lofts a solid through an ordered list of closed planar wire
 * cross-sections (`section_count` >= 2), producing a solid whose boundary
 * connects consecutive sections with ruled (straight-line generatrix)
 * surfaces -- the minimal, most geometrically predictable loft form, not
 * OCCT's smoothed/spline-fitted default. `sections`/`section_count` is a
 * caller-owned array (no STL container crosses this boundary), matching
 * aicad_occt_make_wire_from_edges' convention. All sections must share
 * the same number of edges/vertices for OCCT to establish a
 * correspondence between them; a mismatched section list is rejected
 * with AICAD_OCCT_ERR_OPERATION_FAILED. */
aicad_occt_status_t aicad_occt_loft(aicad_occt_context_t* context,
                                     const aicad_shape_handle_t* sections,
                                     size_t section_count,
                                     aicad_shape_handle_t* out_handle);

/* --- Query helpers used to prove these operations produced a real, valid
 * B-rep, per AGENTS.md's evidence rule -- not exposed as end-user
 * geometry API yet; `cad-geometry-api` owns that surface later. --- */

aicad_occt_status_t aicad_occt_shape_is_valid(aicad_occt_context_t* context,
                                               aicad_shape_handle_t handle,
                                               int* out_is_valid);

aicad_occt_status_t aicad_occt_shape_volume(aicad_occt_context_t* context,
                                             aicad_shape_handle_t handle,
                                             double* out_volume);

/* Total surface area of every face in the shape (for a solid: its full
 * boundary area; for a single face: that face's own area). */
aicad_occt_status_t aicad_occt_shape_area(aicad_occt_context_t* context,
                                           aicad_shape_handle_t handle,
                                           double* out_area);

/* Axis-aligned bounding box, in the kernel's internal linear unit.
 * `out_min`/`out_max` each receive 3 doubles (x, y, z). */
aicad_occt_status_t aicad_occt_shape_bounding_box(aicad_occt_context_t* context,
                                                   aicad_shape_handle_t handle,
                                                   double out_min[3],
                                                   double out_max[3]);

#ifdef __cplusplus
}
#endif

#endif /* AICAD_OCCT_BRIDGE_H */
