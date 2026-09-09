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

/* --- AICAD-026: boolean union/cut/intersect.
 *
 * Unlike extrude/revolve/sweep (which each require one specific
 * topological input kind, per RFC-0002 §3's minimal-surface rule), these
 * accept any non-null shape handle for both operands: OCCT's own
 * BRepAlgoAPI_Fuse/Cut/Common always produce a TopAbs_COMPOUND result
 * (verified empirically, not assumed -- even fusing two TopAbs_SOLID
 * boxes yields a COMPOUND, never a bare SOLID), so restricting these
 * operations' own inputs to TopAbs_SOLID would make boolean results
 * un-chainable into a second boolean operation. See
 * project/reports/AICAD-026.md. Both operands must already belong to
 * `context`; a stale, invalid, or foreign-context handle for either is
 * rejected exactly as every other operation in this bridge rejects one. --- */

/* Union (fuse) of `a` and `b`. */
aicad_occt_status_t aicad_occt_boolean_union(aicad_occt_context_t* context,
                                              aicad_shape_handle_t a,
                                              aicad_shape_handle_t b,
                                              aicad_shape_handle_t* out_handle);

/* Subtraction: `a` minus `b`. */
aicad_occt_status_t aicad_occt_boolean_cut(aicad_occt_context_t* context,
                                            aicad_shape_handle_t a,
                                            aicad_shape_handle_t b,
                                            aicad_shape_handle_t* out_handle);

/* Intersection (common material) of `a` and `b`. */
aicad_occt_status_t aicad_occt_boolean_intersect(aicad_occt_context_t* context,
                                                  aicad_shape_handle_t a,
                                                  aicad_shape_handle_t b,
                                                  aicad_shape_handle_t* out_handle);

/* --- AICAD-027: fillet and chamfer.
 *
 * Selecting WHICH edges to round/chamfer requires raw, index-based edge
 * access -- there is no persistent semantic edge-reference system yet
 * (that is Stage 4's job; `docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`
 * §5-6 already specifies exactly this raw/indexed access pattern --
 * `raw_edge(f, 2)` -- for cases like this one, so this is implementing an
 * already-approved design, not selecting a new architecture
 * alternative). `aicad_occt_shape_edge_count`/`_get_edge` expose the
 * minimum needed to select edges now: a strictly ephemeral, epoch-bound
 * enumeration of a shape's UNIQUE edges (via OCCT's `TopExp::MapShapes`,
 * NOT a raw `TopExp_Explorer` traversal, which revisits each edge once
 * per adjacent face -- verified empirically, e.g. 24 vs. the correct 12
 * for a box; see project/reports/AICAD-027.md). Callers must never treat
 * the resulting index or edge handle as a durable semantic reference
 * (Stage-1 kernel policies #10-12) -- it is valid only within the current
 * geometry epoch, like every other handle this bridge hands out. --- */

/* Number of unique edges in `handle`'s shape. */
aicad_occt_status_t aicad_occt_shape_edge_count(aicad_occt_context_t* context,
                                                 aicad_shape_handle_t handle,
                                                 size_t* out_count);

/* Returns a handle to the edge at `index` (0-based, `< edge_count` from
 * aicad_occt_shape_edge_count against the same handle) in `handle`'s
 * shape, per its own current raw enumeration order -- ephemeral and
 * epoch-bound, not a durable reference. */
aicad_occt_status_t aicad_occt_shape_get_edge(aicad_occt_context_t* context,
                                               aicad_shape_handle_t handle,
                                               size_t index,
                                               aicad_shape_handle_t* out_edge_handle);

/* Fillets (rounds) the given edges of `shape_handle` with a single
 * constant radius. `edges`/`edge_count` (>= 1) is a caller-owned array of
 * edge handles previously obtained from aicad_occt_shape_get_edge against
 * this same shape_handle. Accepts any non-null shape_handle (not
 * restricted to Solid) -- matching aicad_occt_boolean_union's own
 * rationale: a boolean result is a Compound and must remain fillet-able
 * without first being re-wrapped. */
aicad_occt_status_t aicad_occt_fillet(aicad_occt_context_t* context,
                                       aicad_shape_handle_t shape_handle,
                                       const aicad_shape_handle_t* edges,
                                       size_t edge_count,
                                       double radius,
                                       aicad_shape_handle_t* out_handle);

/* Chamfers the given edges of `shape_handle` with a single constant
 * symmetric distance (equal setback on both faces adjacent to each
 * edge). See aicad_occt_fillet for the edges/edge_count contract. */
aicad_occt_status_t aicad_occt_chamfer(aicad_occt_context_t* context,
                                        aicad_shape_handle_t shape_handle,
                                        const aicad_shape_handle_t* edges,
                                        size_t edge_count,
                                        double distance,
                                        aicad_shape_handle_t* out_handle);

/* --- AICAD-028: shell and offset (spike).
 *
 * These are the most failure-prone operations in this bridge (OCCT's own
 * BRepOffsetAPI_MakeOffsetShape header documentation lists several
 * documented limitations: it may fail for vertices where more than 3
 * edges converge, the offset value must be small enough relative to
 * local curvature to avoid self-intersection, and BSpline surfaces with
 * C0 continuity are unsupported). Per AGENTS.md, this bridge does not
 * spend unbounded effort forcing universal success here -- honest
 * capability boundaries are recorded in project/reports/AICAD-028.md
 * rather than hidden or worked around. Face selection reuses the same
 * raw/index-based pattern aicad_occt_shape_edge_count/get_edge
 * established for AICAD-027 (docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md
 * §5-6), applied to TopAbs_FACE instead of TopAbs_EDGE. --- */

/* Number of unique faces in `handle`'s shape. */
aicad_occt_status_t aicad_occt_shape_face_count(aicad_occt_context_t* context,
                                                 aicad_shape_handle_t handle,
                                                 size_t* out_count);

/* Returns a handle to the face at `index` (0-based, `< face_count`) in
 * `handle`'s shape, per its own current raw enumeration order --
 * ephemeral and epoch-bound, not a durable reference. */
aicad_occt_status_t aicad_occt_shape_get_face(aicad_occt_context_t* context,
                                               aicad_shape_handle_t handle,
                                               size_t index,
                                               aicad_shape_handle_t* out_face_handle);

/* Hollows `shape_handle` into a shell of constant wall `thickness`,
 * removing (opening) the given `faces_to_remove` (>= 1, each obtained
 * from aicad_occt_shape_get_face against this same shape_handle).
 * `thickness`'s sign selects which side of the original surface the
 * hollow is built on (negative: hollow the interior out, leaving the
 * original outer boundary in place -- the common "shell" case; positive:
 * grow a shell wall outward). Accepts any non-null shape_handle (not
 * restricted to Solid), matching aicad_occt_boolean_union's own
 * rationale. */
aicad_occt_status_t aicad_occt_shell(aicad_occt_context_t* context,
                                      aicad_shape_handle_t shape_handle,
                                      const aicad_shape_handle_t* faces_to_remove,
                                      size_t face_count,
                                      double thickness,
                                      aicad_shape_handle_t* out_handle);

/* Constructs a shape parallel to `shape_handle`'s boundary, offset by
 * `distance` (positive: outside; negative: inside). Gaps at edges/
 * vertices are filled with pipes/spheres (OCCT's default GeomAbs_Arc join
 * mode) -- for a convex solid this makes a positive-distance offset
 * geometrically equivalent to filleting every edge with that same
 * distance as radius (see project/reports/AICAD-028.md for the analytic
 * evidence this equivalence enabled). */
aicad_occt_status_t aicad_occt_offset(aicad_occt_context_t* context,
                                       aicad_shape_handle_t shape_handle,
                                       double distance,
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
