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
  /* A pointer/argument the caller passed is null or out of range, OR
   * (owner-authorized context-lifetime-safety fix) the `context` pointer
   * passed is non-null but no longer live -- either it was never a
   * context this bridge created, or it addresses one that has since been
   * destroyed by aicad_occt_context_destroy. See that function's own doc
   * comment for the exact safety contract this covers. */
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

/* Opaque, context-scoped handle to one operation's own captured
 * Generated/Modified/IsDeleted lineage (AICAD-086) -- see
 * aicad_occt_boolean_union_lineage's own doc comment. Same field layout
 * and validation contract as aicad_shape_handle_t, but a distinct type so
 * a lineage handle can never be passed where a shape handle is expected
 * (or vice versa) without a compiler error. */
typedef struct aicad_lineage_handle {
  uint64_t context_id;
  uint32_t slot;
  uint32_t generation;
} aicad_lineage_handle_t;

#define AICAD_NULL_LINEAGE_HANDLE \
  { 0, 0, 0 }

/* Opaque kernel context. Owns a shape table and is conservatively
 * single-thread-affine (Stage-1 kernel policy #9): every function that
 * takes a context must be called from the thread that created it, or
 * fails with AICAD_OCCT_ERR_WRONG_THREAD. */
typedef struct aicad_occt_context aicad_occt_context_t;

/* Creates a new kernel context. `*out_context` is set on success only. */
aicad_occt_status_t aicad_occt_context_create(aicad_occt_context_t** out_context);

/* Destroys a context, releasing every shape it still owns. This
 * function's own memory-safety contract (owner-authorized fix,
 * see project/reports/reviews/STAGE1-INDEPENDENT-REVIEW.md's "Context
 * lifetime safety fix" section):
 *
 * - `context` itself is NEVER freed by this call or any other -- every
 *   context this bridge ever creates lives for the remainder of the
 *   process. Only the (potentially large) geometry it owns is released
 *   here.
 * - A second call to this function with the same (now-destroyed)
 *   `context` pointer is well-defined: it does not dereference freed
 *   memory (there is none) and deterministically returns
 *   AICAD_OCCT_ERR_INVALID_ARGUMENT, never a crash or a double-free.
 * - Any other function called afterward with this same `context`
 *   pointer likewise returns AICAD_OCCT_ERR_INVALID_ARGUMENT
 *   deterministically, rather than being undefined behavior.
 * - This safety property covers exactly one thing: a pointer value THIS
 *   BRIDGE ITSELF previously handed out via aicad_occt_context_create,
 *   used again after being destroyed. It does not, and no design built
 *   on an opaque C pointer can, make a wholly fabricated/foreign pointer
 *   value safe to pass here -- that remains the caller's responsibility,
 *   identical to any other opaque-handle C API (e.g. FILE*). */
aicad_occt_status_t aicad_occt_context_destroy(aicad_occt_context_t* context);

/* Releases one shape handle. After this call the handle is stale: any
 * further use of it (with this or any other still-valid handle that
 * happens to share its slot after reuse) is rejected with
 * AICAD_OCCT_ERR_STALE_HANDLE, never silently aliased onto a later
 * shape inserted into the same slot (Stage-1 kernel policy #5). */
aicad_occt_status_t aicad_occt_release_shape(aicad_occt_context_t* context,
                                              aicad_shape_handle_t handle);

/* Returns a second, independently-releasable handle onto the exact same
 * underlying shape as `handle` (a cheap map re-insertion, never a real
 * geometry copy). */
aicad_occt_status_t aicad_occt_shape_duplicate(aicad_occt_context_t* context,
                                                aicad_shape_handle_t handle,
                                                aicad_shape_handle_t* out_handle);

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

/* --- AICAD-077: mirror across a plane. A mirror is an IMPROPER isometry
 * (determinant -1) and therefore cannot be expressed as the rigid
 * `matrix` `aicad_occt_transform_shape` accepts and re-validates as
 * proper-rotation-only (`project/OWNER_DECISIONS.md`/`cad-kernel-api`'s
 * own "Rigidity (no reflection)" invariant) -- this is its own entry
 * point, taking a plane (origin + unit normal) directly rather than a
 * 12-element matrix, so no caller can construct a reflection the other
 * function would (correctly) reject.
 *
 * `origin`/`normal` are each a 3-element point/unit-vector; `normal` is
 * re-normalized defensively (the OCCT `gp_Ax2` constructor the
 * implementation uses already rejects a zero-length direction on its
 * own). Always produces a NEW shape handle, never mutates the input in
 * place (DL-2's functional/value-oriented semantics), matching
 * `aicad_occt_transform_shape`'s own contract. */
aicad_occt_status_t aicad_occt_mirror_shape(aicad_occt_context_t* context,
                                             aicad_shape_handle_t handle,
                                             const double origin[3],
                                             const double normal[3],
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

/* Constructs a circular-arc edge passing through three points, in the
 * order `p_start` -> `p_mid` -> `p_end` (AICAD-075: sketch `arc` entity
 * lowering needs an edge for a partial circle, which
 * `aicad_occt_make_circle_wire`'s always-closed-full-circle contract
 * cannot express). `p_mid` must lie strictly between the other two along
 * the intended arc -- it disambiguates both which of the two possible
 * circular arcs between `p_start`/`p_end` is built and which direction it
 * is traversed, with no separate axis/sense parameter needed (mirrors
 * `aicad_occt_make_line_edge`'s own "no separate handedness flag" style).
 * Rejects coincident or collinear points as AICAD_OCCT_ERR_INVALID_ARGUMENT
 * (OCCT's own GC_MakeArcOfCircle construction failure). */
aicad_occt_status_t aicad_occt_make_arc_edge(aicad_occt_context_t* context,
                                              const double p_start[3],
                                              const double p_mid[3],
                                              const double p_end[3],
                                              aicad_shape_handle_t* out_handle);

/* Joins an ordered list of edges into one wire. `edges`/`edge_count` is a
 * caller-owned array (no STL container crosses this boundary); edges must
 * form a single connected chain (open or closed) in the given order. */
aicad_occt_status_t aicad_occt_make_wire_from_edges(aicad_occt_context_t* context,
                                                     const aicad_shape_handle_t* edges,
                                                     size_t edge_count,
                                                     aicad_shape_handle_t* out_handle);

/* --- AICAD-131: freeform (Bezier/B-spline) curve/surface -> kernel
 * topology construction, closing the capability gap `project/benchmarks/
 * stage5_freeform_corpus/README.md` recorded (make_edge/
 * make_face_on_surface previously covered only the elementary Circle/
 * Arc/Line and Plane/Cylinder/Cone/Sphere/Torus families). --- */

/* Constructs an edge on a (possibly rational) Bezier curve of degree
 * `control_point_count - 1`. `control_points` is a flat, row-major
 * `[x0,y0,z0, x1,y1,z1, ...]` array of `control_point_count` points
 * (`control_point_count >= 2`); `weights` is `control_point_count`
 * positive finite values for a rational curve, or NULL for a plain
 * (non-rational) one. */
aicad_occt_status_t aicad_occt_make_bezier_edge(aicad_occt_context_t* context,
                                                 const double* control_points,
                                                 size_t control_point_count,
                                                 const double* weights,
                                                 aicad_shape_handle_t* out_handle);

/* Constructs an edge on a (possibly rational), non-periodic B-spline
 * curve of the given `degree` -- `aicad_occt_make_bezier_edge`'s
 * general-degree counterpart. `knots`/`multiplicities` is `knot_count`
 * DISTINCT knot values each repeated `multiplicities[i]` times (never
 * pre-expanded), mirroring `cad_geometry_api::curve::AnalyticCurve::
 * BSpline`'s own convention exactly -- the same shape OCCT's own
 * `Geom_BSplineCurve` constructor expects. */
aicad_occt_status_t aicad_occt_make_bspline_edge(aicad_occt_context_t* context,
                                                  size_t degree,
                                                  const double* control_points,
                                                  size_t control_point_count,
                                                  const double* knots,
                                                  const size_t* multiplicities,
                                                  size_t knot_count,
                                                  const double* weights,
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
 * without first being re-wrapped.
 *
 * AICAD-036 finding: like aicad_occt_export_step/aicad_occt_import_step,
 * this function is internally serialized process-wide (a single mutex,
 * not per-context) across ALL contexts. Unlike the STEP translator's
 * already-known issue, this was found for the underlying
 * BRepFilletAPI_MakeFillet/ChFi3d fillet-construction machinery: a
 * concave/reentrant edge on a multi-boolean shape was found to
 * intermittently produce an invalid B-rep when called concurrently from
 * independent contexts on independent threads, while the identical
 * sequential (non-concurrent) construction never failed. See
 * project/reports/AICAD-036.md for the full investigation. Callers do
 * not need their own external synchronization for this specific
 * function, but should expect concurrent aicad_occt_fillet calls from
 * different threads to block on each other rather than run in parallel
 * -- aicad_occt_chamfer was independently confirmed safe under the same
 * conditions and is NOT part of this serialization. */
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

/* --- AICAD-086: lineage-capturing operation variants, needed for
 * generated_by/modified_by feature-lineage evidence
 * (docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md §8). OCCT's own
 * Generated/Modified/IsDeleted history is only answerable while the
 * builder object that performed the operation is still alive; this
 * bridge therefore captures it at the moment of the operation itself
 * (one face/edge entry per unique face/edge of the operation's own input
 * shape(s)) and returns a second, independent handle a caller queries
 * afterward via aicad_occt_lineage_is_deleted, aicad_occt_lineage_
 * generated_count/_get, and aicad_occt_lineage_modified_count/_get.
 * Every other respect (arguments, failure modes, output shape) is
 * identical to the corresponding non-lineage function; only box/
 * cylinder/transform have no lineage variant (a primitive has no
 * consumed input shape for Generated/Modified/IsDeleted to describe
 * against). `out_lineage` must eventually be released via
 * aicad_occt_release_lineage, exactly like a shape handle. --- */

aicad_occt_status_t aicad_occt_boolean_union_lineage(aicad_occt_context_t* context,
                                                       aicad_shape_handle_t a,
                                                       aicad_shape_handle_t b,
                                                       aicad_shape_handle_t* out_handle,
                                                       aicad_lineage_handle_t* out_lineage);

aicad_occt_status_t aicad_occt_boolean_cut_lineage(aicad_occt_context_t* context,
                                                     aicad_shape_handle_t a,
                                                     aicad_shape_handle_t b,
                                                     aicad_shape_handle_t* out_handle,
                                                     aicad_lineage_handle_t* out_lineage);

aicad_occt_status_t aicad_occt_boolean_intersect_lineage(aicad_occt_context_t* context,
                                                           aicad_shape_handle_t a,
                                                           aicad_shape_handle_t b,
                                                           aicad_shape_handle_t* out_handle,
                                                           aicad_lineage_handle_t* out_lineage);

/* See aicad_occt_fillet for the edges/edge_count contract. */
aicad_occt_status_t aicad_occt_fillet_lineage(aicad_occt_context_t* context,
                                               aicad_shape_handle_t shape_handle,
                                               const aicad_shape_handle_t* edges,
                                               size_t edge_count,
                                               double radius,
                                               aicad_shape_handle_t* out_handle,
                                               aicad_lineage_handle_t* out_lineage);

/* See aicad_occt_chamfer for the edges/edge_count contract. */
aicad_occt_status_t aicad_occt_chamfer_lineage(aicad_occt_context_t* context,
                                                aicad_shape_handle_t shape_handle,
                                                const aicad_shape_handle_t* edges,
                                                size_t edge_count,
                                                double distance,
                                                aicad_shape_handle_t* out_handle,
                                                aicad_lineage_handle_t* out_lineage);

/* Releases a lineage handle -- mirrors aicad_occt_release_shape. */
aicad_occt_status_t aicad_occt_release_lineage(aicad_occt_context_t* context,
                                                aicad_lineage_handle_t handle);

/* Whether `input` (a face or edge handle belonging to one of the
 * lineage-capturing operation's own original input shapes, obtained from
 * a call made BEFORE that operation) has no surviving generated/modified
 * counterpart in the operation's result. AICAD_ERR_INVALID_ARGUMENT means
 * `input` was never one of the shapes this lineage was captured for
 * (e.g. it names an output shape instead) -- deliberately distinct from
 * "not deleted," which would silently conflate "no evidence" with a real
 * evidenced answer. */
aicad_occt_status_t aicad_occt_lineage_is_deleted(aicad_occt_context_t* context,
                                                   aicad_lineage_handle_t lineage,
                                                   aicad_shape_handle_t input,
                                                   int* out_is_deleted);

/* Number of shapes `input` was generated into by the operation this
 * `lineage` was captured from (OCCT's own Generated(input) -- e.g. a new
 * face created where a hole broke through an existing face). 0 if
 * `input` has no generated counterpart (including when it was deleted). */
aicad_occt_status_t aicad_occt_lineage_generated_count(aicad_occt_context_t* context,
                                                        aicad_lineage_handle_t lineage,
                                                        aicad_shape_handle_t input,
                                                        size_t* out_count);

/* Returns a fresh handle to the generated shape at `index` (0-based, <
 * aicad_occt_lineage_generated_count against the same lineage/input). */
aicad_occt_status_t aicad_occt_lineage_generated_get(aicad_occt_context_t* context,
                                                      aicad_lineage_handle_t lineage,
                                                      aicad_shape_handle_t input,
                                                      size_t index,
                                                      aicad_shape_handle_t* out_handle);

/* Same as generated_count/_get, for OCCT's own Modified(input) -- `input`
 * carried forward as a geometrically changed (but not newly created)
 * counterpart, e.g. a face re-trimmed by a boolean cut. */
aicad_occt_status_t aicad_occt_lineage_modified_count(aicad_occt_context_t* context,
                                                       aicad_lineage_handle_t lineage,
                                                       aicad_shape_handle_t input,
                                                       size_t* out_count);

aicad_occt_status_t aicad_occt_lineage_modified_get(aicad_occt_context_t* context,
                                                     aicad_lineage_handle_t lineage,
                                                     aicad_shape_handle_t input,
                                                     size_t index,
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

/* Number of unique shells in `handle`'s shape (any shape kind, matching
 * aicad_occt_shape_face_count's own "any shape kind" scope). */
aicad_occt_status_t aicad_occt_shape_shell_count(aicad_occt_context_t* context,
                                                  aicad_shape_handle_t handle,
                                                  size_t* out_count);

/* Returns a handle to the shell at `index` (0-based, `< shell_count`) in
 * `handle`'s shape, per its own current raw enumeration order --
 * ephemeral and epoch-bound, matching aicad_occt_shape_get_face's own
 * contract. */
aicad_occt_status_t aicad_occt_shape_get_shell(aicad_occt_context_t* context,
                                                aicad_shape_handle_t handle,
                                                size_t index,
                                                aicad_shape_handle_t* out_shell_handle);

/* Number of unique solids in `handle`'s shape (any shape kind, matching
 * aicad_occt_shape_face_count's own "any shape kind" scope). */
aicad_occt_status_t aicad_occt_shape_solid_count(aicad_occt_context_t* context,
                                                  aicad_shape_handle_t handle,
                                                  size_t* out_count);

/* Returns a handle to the solid at `index` (0-based, `< solid_count`) in
 * `handle`'s shape, per its own current raw enumeration order --
 * ephemeral and epoch-bound, matching aicad_occt_shape_get_face's own
 * contract. */
aicad_occt_status_t aicad_occt_shape_get_solid(aicad_occt_context_t* context,
                                                aicad_shape_handle_t handle,
                                                size_t index,
                                                aicad_shape_handle_t* out_solid_handle);

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

/* --- AICAD-029: topology exploration.
 *
 * `topology_faces`/`topology_edges` (docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md
 * §3) are already implemented by AICAD-027/028's
 * `aicad_occt_shape_edge_count`/`_get_edge` and
 * `aicad_occt_shape_face_count`/`_get_face` -- both accept any shape kind
 * (`LookupAnyKind`, not just Solid), so calling them with a Face handle
 * already enumerates that face's own boundary edges (`face_edges` in the
 * plan doc), and no separate function is added for it (native/occt_bridge's
 * own test suite, topology_test.cpp, is the evidence this actually holds).
 * This task adds the two primitives the plan doc's exploration surface
 * still lacked: vertex enumeration (`topology_vertices`) and edge-to-face
 * adjacency (`adjacent_faces`). Both reuse the same raw/indexed,
 * ephemeral, epoch-bound access pattern AICAD-027/028 established (ordinary
 * TopExp::MapShapes de-duplication, 0-based indices) -- not a new
 * architecture alternative. --- */

/* Number of unique vertices in `handle`'s shape (via TopExp::MapShapes,
 * matching aicad_occt_shape_edge_count/_face_count's own
 * de-duplication rationale). */
aicad_occt_status_t aicad_occt_shape_vertex_count(aicad_occt_context_t* context,
                                                   aicad_shape_handle_t handle,
                                                   size_t* out_count);

/* Returns a handle to the vertex at `index` (0-based, `< vertex_count`) in
 * `handle`'s shape, per its own current raw enumeration order --
 * ephemeral and epoch-bound, matching aicad_occt_shape_get_edge/_get_face's
 * own contract. */
aicad_occt_status_t aicad_occt_shape_get_vertex(aicad_occt_context_t* context,
                                                 aicad_shape_handle_t handle,
                                                 size_t index,
                                                 aicad_shape_handle_t* out_vertex_handle);

/* Returns `edge_handle`'s two endpoint vertices in the edge's own
 * orientation order (OCCT's TopExp::Vertices' "first"/"last" sense; for a
 * closed edge, e.g. a full circle, both are the same vertex -- callers
 * must not assume distinctness). `edge_handle` must address a shape of
 * exactly kind Edge (a caller-contract violation otherwise, matching
 * aicad_occt_fillet's own edge-typed-argument rejection). */
aicad_occt_status_t aicad_occt_edge_vertices(aicad_occt_context_t* context,
                                              aicad_shape_handle_t edge_handle,
                                              aicad_shape_handle_t* out_v0,
                                              aicad_shape_handle_t* out_v1);

/* Number of faces of `shape_handle` adjacent to (bounded by) the edge at
 * `edge_index` (0-based, `< aicad_occt_shape_edge_count(shape_handle)`,
 * per that same function's own enumeration order over `shape_handle`).
 * An edge shared by N faces (N=2 for an ordinary manifold solid edge, N=1
 * for a free/boundary edge, N>2 possible for a non-manifold compound)
 * reports N here. */
aicad_occt_status_t aicad_occt_shape_edge_adjacent_face_count(aicad_occt_context_t* context,
                                                               aicad_shape_handle_t shape_handle,
                                                               size_t edge_index,
                                                               size_t* out_count);

/* Returns a handle to the `adjacent_index`-th (0-based, `<
 * edge_adjacent_face_count`) face of `shape_handle` adjacent to the edge
 * at `edge_index`, in that adjacency query's own current raw enumeration
 * order -- ephemeral and epoch-bound, never a durable semantic reference. */
aicad_occt_status_t aicad_occt_shape_edge_adjacent_face_get(aicad_occt_context_t* context,
                                                             aicad_shape_handle_t shape_handle,
                                                             size_t edge_index,
                                                             size_t adjacent_index,
                                                             aicad_shape_handle_t* out_face_handle);

/* --- AICAD-030: length and center-of-mass queries (volume/area/
 * bounding_box already exist from AICAD-016..028's own construction/
 * validation evidence work). --- */

/* Total length of every unique edge in the shape (via
 * BRepGProp::LinearProperties with SkipShared=true -- without it, an edge
 * shared by 2 faces is counted twice, verified empirically: a box
 * reports 72 instead of the true 36 = 4*(dx+dy+dz) with SkipShared
 * false), matching aicad_occt_shape_edge_count's own "unique edges"
 * scope -- not restricted to Edge/Wire-kind handles, any shape's own
 * edges contribute (e.g. a solid's total edge length). */
aicad_occt_status_t aicad_occt_shape_length(aicad_occt_context_t* context,
                                             aicad_shape_handle_t handle,
                                             double* out_length);

/* Center of mass of the shape's own highest-dimensional content: a
 * volume-weighted centroid if the shape contains any Solid, else an
 * area-weighted centroid if it contains any Face, else a length-weighted
 * centroid over its Edges. Fails with AICAD_OCCT_ERR_OPERATION_FAILED if
 * the shape has none of these (e.g. a bare Vertex). `out_center` receives
 * 3 doubles (x, y, z). */
aicad_occt_status_t aicad_occt_shape_center_of_mass(aicad_occt_context_t* context,
                                                     aicad_shape_handle_t handle,
                                                     double out_center[3]);

/* --- AICAD-031: normalized B-rep validation report.
 *
 * docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md §3's `validate()` and
 * docs/plan/23_CROSS_SYSTEM_PARAMETER_CATALOG.md §7's fuller
 * `validate(target, level?, checks?, tolerance?, healing_allowed?)`
 * signature. Stage-1 implements the `target`-only, single-level subset:
 * no caller-selectable `level`/`checks`/`tolerance`, and
 * `healing_allowed` is not offered at all (Stage-1 kernel policy #14:
 * validation and repair/healing are distinct concepts, and no `heal`
 * operation exists yet for this to opt into). This normalizes
 * aicad_occt_shape_is_valid's single bool into a per-topological-kind
 * breakdown -- still not a full diagnostic (no per-subshape identity or
 * failure-reason enum crosses this ABI; see project/reports/AICAD-031.md
 * for why). --- */

/* Normalized validation report: overall validity plus a count of
 * invalid subshapes broken down by topological kind, via
 * BRepCheck_Analyzer::IsValid() queried per unique vertex/edge/wire/face/
 * shell/solid (TopExp::MapShapes-deduplicated, matching this bridge's own
 * established "unique subshapes" convention). `invalid_shell_count`/
 * `invalid_solid_count` were added by AICAD-119, alongside this batch's
 * own shell/solid construction paths -- a shape with no Shell/Solid
 * subshape (e.g. a bare face) always reports 0 for both, not an error. A
 * shape with `is_valid == 0` always has at least one nonzero count among
 * the six; a shape with `is_valid == 1` always has all six at zero. */
typedef struct aicad_validation_report {
  int is_valid;
  size_t invalid_vertex_count;
  size_t invalid_edge_count;
  size_t invalid_wire_count;
  size_t invalid_face_count;
  size_t invalid_shell_count;
  size_t invalid_solid_count;
} aicad_validation_report_t;

aicad_occt_status_t aicad_occt_shape_validate(aicad_occt_context_t* context,
                                               aicad_shape_handle_t handle,
                                               aicad_validation_report_t* out_report);

/* --- AICAD-032: display tessellation output.
 *
 * A two-call protocol, matching aicad_occt_shape_edge_count/_get_edge's
 * own count-then-fetch convention: aicad_occt_tessellate performs the
 * actual meshing and caches the result keyed to `handle`'s own slot (not
 * a single shared "last tessellation" -- multiple handles may each hold
 * their own cached result simultaneously); aicad_occt_tessellation_get
 * copies that cached result into caller-owned buffers. The cache is
 * cleared whenever `handle`'s slot is released or reused by a new shape
 * (Stage-1 kernel policy #10: epoch-bound, ephemeral) -- a stale
 * tessellation can never be returned for a different shape occupying the
 * same slot.
 *
 * Output is flat-shaded triangle-soup: each triangle owns 3 private
 * vertex positions and one flat geometric normal (not shared/averaged
 * with neighboring triangles), not a vertex-shared, smooth-normal mesh --
 * see project/reports/AICAD-032.md for why this simplification was made
 * for Stage-1's own scope. --- */

typedef struct aicad_tessellation_counts {
  size_t triangle_count;
} aicad_tessellation_counts_t;

/* Runs BRepMesh_IncrementalMesh on `handle`'s shape at the given
 * (absolute, not relative) linear/angular deflections and caches a
 * flat-shaded triangle-soup tessellation for it. `linear_deflection`/
 * `angular_deflection` must both be finite and > 0. Reports
 * `out_counts->triangle_count`; call aicad_occt_tessellation_get next
 * (with the SAME handle) to fetch the actual buffers. */
aicad_occt_status_t aicad_occt_tessellate(aicad_occt_context_t* context,
                                           aicad_shape_handle_t handle,
                                           double linear_deflection,
                                           double angular_deflection,
                                           aicad_tessellation_counts_t* out_counts);

/* Fills caller-owned buffers with `handle`'s own most recently cached
 * tessellation (from a prior aicad_occt_tessellate call against this
 * SAME handle). `out_vertices` and `out_normals` each receive
 * `9 * triangle_count` doubles (3 vertices per triangle * 3 coordinates;
 * `out_normals` holds each triangle's one flat normal, duplicated across
 * its 3 vertices, at the same offsets as `out_vertices`). Fails with
 * AICAD_OCCT_ERR_INVALID_ARGUMENT if no tessellation is cached for
 * `handle` (aicad_occt_tessellate was never called for it, or its cache
 * was invalidated by a slot release/reuse). */
aicad_occt_status_t aicad_occt_tessellation_get(aicad_occt_context_t* context,
                                                 aicad_shape_handle_t handle,
                                                 double* out_vertices,
                                                 double* out_normals);

/* --- AICAD-033: STEP export.
 *
 * docs/plan/23_CROSS_SYSTEM_PARAMETER_CATALOG.md §9's `export_step()`.
 * Writes `handle`'s shape to `file_path` as an AP214 STEP file via OCCT's
 * own `STEPControl_Writer`. See project/reports/AICAD-033.md for exactly
 * how this task's export was independently verified (a genuinely
 * OCCT-independent Python STEP-21 parser, not merely re-importing
 * through this same bridge). --- */

/* `file_path` is a caller-owned, null-terminated path (a raw C string,
 * not an STL std::string, per Stage-1 kernel policy #8); this bridge
 * neither retains nor frees it beyond the call. Fails with
 * AICAD_OCCT_ERR_OPERATION_FAILED if OCCT's own writer could not
 * transfer the shape or could not write the file (e.g. an unwritable
 * path).
 *
 * UNLIKE every other function in this bridge, this one is internally
 * serialized process-wide (a single mutex, not per-context) across ALL
 * contexts, shared with `aicad_occt_import_step` below: OCCT's own STEP
 * translator holds process-global, non-thread-safe state, and concurrent
 * calls from independent contexts on independent threads were
 * empirically found to segfault the process (see
 * project/reports/AICAD-033.md). Callers do not need to add their own
 * external synchronization for this specific function, but should expect
 * concurrent aicad_occt_export_step/aicad_occt_import_step calls from
 * different threads to block on each other rather than run in
 * parallel. */
aicad_occt_status_t aicad_occt_export_step(aicad_occt_context_t* context,
                                            aicad_shape_handle_t handle,
                                            const char* file_path);

/* --- AICAD-035: STEP import, added narrowly to support the Stage-1
 * proof's own export -> independent re-import/verification pipeline
 * (docs/plan/23_CROSS_SYSTEM_PARAMETER_CATALOG.md §9's `import_step()`
 * paired counterpart). This is deliberately NOT the full public
 * language-level `import_step()` described there (no unit/heal/
 * preserve_metadata/coordinate_policy/naming_policy options, no semantic
 * node wrapping, no provenance) -- that is later, higher-layer scope.
 * This is the same minimal, capability-driven kernel-adapter operation
 * every other Stage-1 bridge function already is: read a STEP file via
 * OCCT's own `STEPControl_Reader`, transfer its root shapes, and return
 * one resulting shape.
 *
 * Read the important verification-scope caveat before treating a
 * round-trip through this function as independent evidence: it uses the
 * SAME OCCT installation that performed the export, so
 * export-then-import-through-this-bridge proves the round-trip pipeline
 * itself is self-consistent (a real, useful check), not that an
 * independent, non-OCCT implementation agrees with OCCT's own output --
 * see project/reports/AICAD-035.md for the genuinely independent
 * (non-OCCT, structural-only) verification path used alongside this. */

/* `file_path` is a caller-owned, null-terminated path (as in
 * `aicad_occt_export_step`). Fails with AICAD_OCCT_ERR_INVALID_ARGUMENT
 * for a null/empty path or a null `out_handle`; fails with
 * AICAD_OCCT_ERR_OPERATION_FAILED if the file cannot be read, is not a
 * valid STEP file, or transfers zero shapes. If the file's DATA section
 * describes more than one root shape, the returned shape is whichever
 * single shape OCCT's own reader designates via `OneShape()` (typically
 * a Compound containing all transferred roots) -- this bridge does not
 * impose or validate a single-root-shape contract on its caller's STEP
 * files. Shares `aicad_occt_export_step`'s process-wide mutex (see
 * above). */
aicad_occt_status_t aicad_occt_import_step(aicad_occt_context_t* context,
                                            const char* file_path,
                                            aicad_shape_handle_t* out_handle);

/* --- AICAD-082: face/edge geometric classification.
 *
 * Stage-4 query geometry predicates (docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md
 * §6 "Geometry predicates": `planar`/`cylindrical`/`conical`/`spherical`/
 * `toroidal`/`bspline`/`radius`/`normal`/`axis`) need each face/edge's own
 * analytic surface/curve family, not just the aggregate area/length this
 * bridge already exposes. These functions stay kernel-neutral at the type
 * level (per this header's own top-of-file contract): `aicad_surface_kind_t`/
 * `aicad_curve_kind_t` are AICAD-owned enums, not re-exported OCCT
 * `GeomAbs_*` values -- their numeric order is this bridge's own and is
 * covered by native/occt_bridge's own test suite, not merely assumed
 * stable across an OCCT upgrade. --- */

/* A face's underlying surface family. `AICAD_SURFACE_OTHER` covers every
 * analytic/procedural surface kind this enum does not name (e.g. a swept,
 * offset, or surface-of-revolution/-extrusion face) -- never guessed into
 * one of the named kinds. */
typedef enum aicad_surface_kind {
  AICAD_SURFACE_PLANE = 0,
  AICAD_SURFACE_CYLINDER = 1,
  AICAD_SURFACE_CONE = 2,
  AICAD_SURFACE_SPHERE = 3,
  AICAD_SURFACE_TORUS = 4,
  AICAD_SURFACE_BEZIER = 5,
  AICAD_SURFACE_BSPLINE = 6,
  AICAD_SURFACE_OTHER = 7,
} aicad_surface_kind_t;

/* An edge's underlying curve family. `AICAD_CURVE_OTHER` covers every
 * curve kind this enum does not name (e.g. a hyperbola/parabola/offset
 * curve), matching `aicad_surface_kind_t::AICAD_SURFACE_OTHER`'s own
 * never-guess rule. */
typedef enum aicad_curve_kind {
  AICAD_CURVE_LINE = 0,
  AICAD_CURVE_CIRCLE = 1,
  AICAD_CURVE_ELLIPSE = 2,
  AICAD_CURVE_BEZIER = 3,
  AICAD_CURVE_BSPLINE = 4,
  AICAD_CURVE_OTHER = 5,
} aicad_curve_kind_t;

/* `face_handle` must address a shape of exactly kind Face (caller-contract
 * violation otherwise, matching aicad_occt_edge_vertices' own edge-typed
 * rejection), rejected with AICAD_OCCT_ERR_INVALID_ARGUMENT. */
aicad_occt_status_t aicad_occt_shape_surface_type(aicad_occt_context_t* context,
                                                   aicad_shape_handle_t face_handle,
                                                   int* out_kind);

/* The face's single characteristic radius. Defined only for
 * AICAD_SURFACE_CYLINDER/_SPHERE (their one radius) and AICAD_SURFACE_TORUS
 * (its major/tube-path radius -- the torus's own minor radius is not
 * returned by this function; a future task may add it if a predicate
 * needs it). Any other surface kind (including AICAD_SURFACE_CONE, whose
 * radius varies continuously along its axis with no single well-defined
 * value) fails with AICAD_OCCT_ERR_INVALID_ARGUMENT rather than guessing
 * which parameter to report. */
aicad_occt_status_t aicad_occt_shape_face_radius(aicad_occt_context_t* context,
                                                  aicad_shape_handle_t face_handle,
                                                  double* out_radius);

/* The face's rotational axis (origin + unit direction). Defined only for
 * AICAD_SURFACE_CYLINDER/_CONE/_TORUS; any other surface kind fails with
 * AICAD_OCCT_ERR_INVALID_ARGUMENT. */
aicad_occt_status_t aicad_occt_shape_face_axis(aicad_occt_context_t* context,
                                                aicad_shape_handle_t face_handle,
                                                double out_origin[3],
                                                double out_direction[3]);

/* A representative point on the face (its own parametric-domain midpoint,
 * NOT an area centroid) and the face's own outward unit normal there,
 * already corrected for the face's TopoDS orientation (a REVERSED face
 * reports the flipped-sign normal its actual material boundary has, not
 * its underlying surface's raw parametrization sense). Fails with
 * AICAD_OCCT_ERR_OPERATION_FAILED if the surface is singular at that exact
 * parameter (e.g. a cone apex) and no normal can be evaluated there. */
aicad_occt_status_t aicad_occt_shape_face_normal(aicad_occt_context_t* context,
                                                  aicad_shape_handle_t face_handle,
                                                  double out_point[3],
                                                  double out_normal[3]);

/* `edge_handle` must address a shape of exactly kind Edge, matching
 * aicad_occt_edge_vertices' own contract. */
aicad_occt_status_t aicad_occt_shape_curve_type(aicad_occt_context_t* context,
                                                 aicad_shape_handle_t edge_handle,
                                                 int* out_kind);

/* The edge's radius. Defined only for AICAD_CURVE_CIRCLE; an ellipse has
 * two distinct radii (major/minor) with no single "the" radius, so
 * AICAD_CURVE_ELLIPSE (and every other curve kind) fails with
 * AICAD_OCCT_ERR_INVALID_ARGUMENT rather than guessing which one to
 * report. */
aicad_occt_status_t aicad_occt_shape_edge_radius(aicad_occt_context_t* context,
                                                  aicad_shape_handle_t edge_handle,
                                                  double* out_radius);

/* The edge's axis (origin + unit direction), i.e. the normal to the
 * circle's own plane through its center. Defined only for
 * AICAD_CURVE_CIRCLE, matching aicad_occt_shape_edge_radius' own scope. */
aicad_occt_status_t aicad_occt_shape_edge_axis(aicad_occt_context_t* context,
                                                aicad_shape_handle_t edge_handle,
                                                double out_origin[3],
                                                double out_direction[3]);

/* --- AICAD-083: shape identity, needed to test topological adjacency
 * (e.g. "is this face, obtained via one enumeration path, the SAME face
 * as that one, obtained via another") without comparing raw handle slots,
 * which differ across independent aicad_occt_shape_get_face/
 * _edge_adjacent_face_get calls even when both name the same underlying
 * TopoDS_Shape. Uses OCCT's own TopoDS_Shape::IsSame (TShape + Location,
 * ignoring Orientation) -- deliberately not IsEqual (which also compares
 * Orientation): two differently-oriented handles onto the same underlying
 * face/edge are still "the same topological entity" for adjacency
 * purposes. `a`/`b` may address any shape kind and need not be the same
 * kind as each other (a mismatched kind simply reports not-same, not an
 * error). --- */
aicad_occt_status_t aicad_occt_shape_is_same(aicad_occt_context_t* context,
                                              aicad_shape_handle_t a,
                                              aicad_shape_handle_t b,
                                              int* out_is_same);

/* --- AICAD-083: wire enumeration/outer-boundary support, needed for the
 * `boundary(outer|inner)` topology predicate
 * (docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md §6). Mirrors
 * aicad_occt_shape_face_count/_get_face's own raw/indexed, ephemeral,
 * epoch-bound enumeration pattern applied to TopAbs_WIRE. --- */

/* Number of unique wires in `handle`'s shape (any shape kind, matching
 * aicad_occt_shape_face_count's own "any shape kind" scope). */
aicad_occt_status_t aicad_occt_shape_wire_count(aicad_occt_context_t* context,
                                                 aicad_shape_handle_t handle,
                                                 size_t* out_count);

/* Returns a handle to the wire at `index` (0-based, `< wire_count`) in
 * `handle`'s shape, per its own current raw enumeration order --
 * ephemeral and epoch-bound, matching aicad_occt_shape_get_face's own
 * contract. */
aicad_occt_status_t aicad_occt_shape_get_wire(aicad_occt_context_t* context,
                                               aicad_shape_handle_t handle,
                                               size_t index,
                                               aicad_shape_handle_t* out_wire_handle);

/* Whether `wire_handle` is `face_handle`'s own designated OUTER wire
 * (OCCT's own `BRepTools::OuterWire`, chosen by parametric area -- the
 * largest, in the face's own 2D parameter space) -- false for any of the
 * face's inner (hole) wires, and false if `wire_handle` does not bound
 * `face_handle` at all. `face_handle` must address a shape of exactly
 * kind Face; `wire_handle` must address a shape of exactly kind Wire. */
aicad_occt_status_t aicad_occt_shape_is_outer_wire(aicad_occt_context_t* context,
                                                    aicad_shape_handle_t face_handle,
                                                    aicad_shape_handle_t wire_handle,
                                                    int* out_is_outer);

/* --- AICAD-084: point extraction/classification support for baseline
 * spatial predicates. --- */

/* The vertex's own coordinate (`BRep_Tool::Pnt`), needed because
 * aicad_occt_shape_center_of_mass explicitly fails for a bare Vertex
 * shape (see that function's own doc comment) -- a vertex's "center of
 * mass" is just its own point, but this is its own function rather than
 * folding a 5th dispatch case into center_of_mass, since a point and a
 * mass-weighted centroid are conceptually different queries that happen
 * to coincide only for this one topological kind. `vertex_handle` must
 * address a shape of exactly kind Vertex. */
aicad_occt_status_t aicad_occt_shape_vertex_point(aicad_occt_context_t* context,
                                                   aicad_shape_handle_t vertex_handle,
                                                   double out_point[3]);

/* Exact point-vs-solid classification (`AICAD_CLASSIFY_OUT`/`_IN`/
 * `_ON_BOUNDARY`), via OCCT's own `BRepClass3d_SolidClassifier` -- an
 * exact B-rep test, never a mesh/bounding-box approximation (AGENTS.md's
 * "Exact B-rep is canonical compiled geometry" non-negotiable applies to
 * query predicates exactly as it does to modeling operations).
 * `solid_handle` must address a shape containing at least one Solid;
 * `tolerance` (> 0, finite) is the classifier's own boundary tolerance. */
typedef enum aicad_point_classification {
  AICAD_CLASSIFY_OUT = 0,
  AICAD_CLASSIFY_IN = 1,
  AICAD_CLASSIFY_ON_BOUNDARY = 2,
} aicad_point_classification_t;

aicad_occt_status_t aicad_occt_shape_classify_point(aicad_occt_context_t* context,
                                                     aicad_shape_handle_t solid_handle,
                                                     const double point[3],
                                                     double tolerance,
                                                     int* out_classification);

/* --- AICAD-119: general topology construction (vertex/face-on-surface/
 * shell/solid/compound), completing the vertex->edge->wire->face->shell->
 * solid pipeline docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md §3
 * specifies. aicad_occt_make_line_edge/_make_circle_wire/_make_arc_edge/
 * _make_wire_from_edges/_make_face_from_wire (AICAD-022/023) already cover
 * vertex-less edge/wire/planar-face construction; this batch adds the
 * missing vertex, non-planar (quadric-surface) face, shell, and solid
 * construction paths, plus compound grouping. None of these perform
 * sewing/gap-closing (AICAD-120's job): a shell/solid built here reflects
 * exactly the connectivity its input faces/shells already have byte-for-
 * byte, so a caller-supplied gap or orientation mismatch surfaces as an
 * open/invalid result under aicad_occt_shape_validate, never a silently
 * "fixed" one -- Stage-1 kernel policy #14 applies here exactly as it does
 * to aicad_occt_make_face_from_wire. --- */

/* Constructs a single-point vertex (`BRepBuilderAPI_MakeVertex`). Always
 * succeeds for a finite `point`. */
aicad_occt_status_t aicad_occt_make_vertex(aicad_occt_context_t* context,
                                            const double point[3],
                                            aicad_shape_handle_t* out_handle);

/* Builds a face bounded by `outer_wire` on the given elementary quadric
 * surface (`BRepBuilderAPI_MakeFace(surface, wire, Inside=true)`), then
 * adds each of `holes` (each must already be wound with the opposite
 * orientation from `outer_wire`, per OCCT's own inner-boundary convention
 * -- this bridge does not infer or correct hole orientation) as an inner
 * boundary. `outer_reversed != 0` builds the face on `outer_wire.Reversed()`
 * instead, the one explicit orientation control this batch exposes.
 * `outer_wire`/every element of `holes` must address a shape of exactly
 * kind Wire. As with aicad_occt_make_face_from_wire, construction success
 * is not evidence of validity -- call aicad_occt_shape_validate/
 * aicad_occt_shape_is_valid separately. */
aicad_occt_status_t aicad_occt_make_face_on_plane(aicad_occt_context_t* context,
                                                   aicad_shape_handle_t outer_wire,
                                                   const aicad_shape_handle_t* holes,
                                                   size_t hole_count,
                                                   const double origin[3],
                                                   const double normal[3],
                                                   int outer_reversed,
                                                   aicad_shape_handle_t* out_handle);

/* See aicad_occt_make_face_on_plane's own doc comment; the cylinder is
 * coaxial with (axis_origin, axis_direction). */
aicad_occt_status_t aicad_occt_make_face_on_cylinder(aicad_occt_context_t* context,
                                                      aicad_shape_handle_t outer_wire,
                                                      const aicad_shape_handle_t* holes,
                                                      size_t hole_count,
                                                      const double axis_origin[3],
                                                      const double axis_direction[3],
                                                      double radius,
                                                      int outer_reversed,
                                                      aicad_shape_handle_t* out_handle);

/* See aicad_occt_make_face_on_plane's own doc comment; the cone's apex is
 * at axis_origin, opening along axis_direction at half_angle_radians. */
aicad_occt_status_t aicad_occt_make_face_on_cone(aicad_occt_context_t* context,
                                                  aicad_shape_handle_t outer_wire,
                                                  const aicad_shape_handle_t* holes,
                                                  size_t hole_count,
                                                  const double axis_origin[3],
                                                  const double axis_direction[3],
                                                  double half_angle_radians,
                                                  int outer_reversed,
                                                  aicad_shape_handle_t* out_handle);

/* See aicad_occt_make_face_on_plane's own doc comment. */
aicad_occt_status_t aicad_occt_make_face_on_sphere(aicad_occt_context_t* context,
                                                    aicad_shape_handle_t outer_wire,
                                                    const aicad_shape_handle_t* holes,
                                                    size_t hole_count,
                                                    const double center[3],
                                                    double radius,
                                                    int outer_reversed,
                                                    aicad_shape_handle_t* out_handle);

/* See aicad_occt_make_face_on_plane's own doc comment; the torus is
 * coaxial with (axis_origin, axis_direction). */
aicad_occt_status_t aicad_occt_make_face_on_torus(aicad_occt_context_t* context,
                                                   aicad_shape_handle_t outer_wire,
                                                   const aicad_shape_handle_t* holes,
                                                   size_t hole_count,
                                                   const double axis_origin[3],
                                                   const double axis_direction[3],
                                                   double major_radius,
                                                   double minor_radius,
                                                   int outer_reversed,
                                                   aicad_shape_handle_t* out_handle);

/* AICAD-131: see aicad_occt_make_face_on_plane's own doc comment; the
 * surface is a (possibly rational) tensor-product Bezier surface of
 * bidegree `(rows - 1, cols - 1)`. `control_points` is a flat, row-major
 * (`u` outer index, `v` inner index) `rows * cols` point array;
 * `weights` is the same shape for a rational surface, or NULL. */
aicad_occt_status_t aicad_occt_make_face_on_bezier_surface(aicad_occt_context_t* context,
                                                             aicad_shape_handle_t outer_wire,
                                                             const aicad_shape_handle_t* holes,
                                                             size_t hole_count,
                                                             const double* control_points,
                                                             size_t rows,
                                                             size_t cols,
                                                             const double* weights,
                                                             int outer_reversed,
                                                             aicad_shape_handle_t* out_handle);

/* AICAD-131: see aicad_occt_make_face_on_bezier_surface's own doc
 * comment; the general-degree tensor-product B-spline counterpart,
 * non-periodic in both directions, mirroring
 * aicad_occt_make_bspline_edge's own knot/multiplicity convention
 * independently per parametric direction. */
aicad_occt_status_t aicad_occt_make_face_on_bspline_surface(aicad_occt_context_t* context,
                                                              aicad_shape_handle_t outer_wire,
                                                              const aicad_shape_handle_t* holes,
                                                              size_t hole_count,
                                                              size_t degree_u,
                                                              size_t degree_v,
                                                              const double* control_points,
                                                              size_t rows,
                                                              size_t cols,
                                                              const double* knots_u,
                                                              const size_t* multiplicities_u,
                                                              size_t knot_u_count,
                                                              const double* knots_v,
                                                              const size_t* multiplicities_v,
                                                              size_t knot_v_count,
                                                              const double* weights,
                                                              int outer_reversed,
                                                              aicad_shape_handle_t* out_handle);

/* Assembles `faces` into one shell (`BRep_Builder::MakeShell` + `Add` per
 * face, in order) -- a structural container only, exactly like a
 * `TopoDS_Compound` of faces except tagged Shell: no edge is matched,
 * merged, or moved, so a shell built from faces that do not already share
 * identical edges is open/non-manifold, not repaired. `face_count` must be
 * >= 1; every element of `faces` must address a shape of exactly kind
 * Face. */
aicad_occt_status_t aicad_occt_make_shell(aicad_occt_context_t* context,
                                           const aicad_shape_handle_t* faces,
                                           size_t face_count,
                                           aicad_shape_handle_t* out_handle);

/* Builds a solid from `outer_shell` (`BRepBuilderAPI_MakeSolid`), adding
 * each of `voids` as an additional (void/cavity) shell. `outer_shell`/
 * every element of `voids` must address a shape of exactly kind Shell;
 * `void_count` may be 0. Verified empirically, not assumed: OCCT's own
 * `BRepBuilderAPI_MakeSolid::IsDone()` does NOT require `outer_shell` to
 * be closed -- it reports done (and this function returns AICAD_OCCT_OK)
 * even for an open shell, producing a structurally-real but invalid
 * solid. This is Stage-1 kernel policy #14 in its starkest form: call
 * aicad_occt_shape_validate/aicad_occt_shape_is_valid separately, always
 * -- a non-`AICAD_OCCT_OK` status from this function means the C++ call
 * itself failed (a bad handle, a foreign-context handle, an unexpected
 * OCCT exception), never "the resulting solid is invalid." */
aicad_occt_status_t aicad_occt_make_solid(aicad_occt_context_t* context,
                                           aicad_shape_handle_t outer_shell,
                                           const aicad_shape_handle_t* voids,
                                           size_t void_count,
                                           aicad_shape_handle_t* out_handle);

/* Groups `shapes` (any kind, any mix of kinds) into one `TopoDS_Compound`
 * (`BRep_Builder::MakeCompound` + `Add` per shape, in order). `shape_count`
 * must be >= 1. Always succeeds once its arguments are valid handles --
 * a compound has no closure/connectivity requirement to fail. */
aicad_occt_status_t aicad_occt_make_compound(aicad_occt_context_t* context,
                                              const aicad_shape_handle_t* shapes,
                                              size_t shape_count,
                                              aicad_shape_handle_t* out_handle);

/* --- AICAD-120: sewing/healing with an explicit, bounded modeling/
 * construction tolerance policy (`project/DECISION_LOG.md#DL-24` domain
 * 2). Neither sews/heals silently beyond what `BRepBuilderAPI_Sewing`/
 * `ShapeFix_Shape` themselves report -- `aicad_sew_report_t`/
 * `aicad_heal_report_t` are the required structured evidence a caller
 * must consult before trusting the result; a non-`AICAD_OCCT_OK` status
 * means the C++ call itself failed, never "the result is invalid" (Stage-1
 * kernel policy #14, matching every other construction function in this
 * header, applied here in its starkest form: `aicad_occt_heal` can and
 * does return `AICAD_OCCT_OK` with `is_valid_after == 0`). --- */

typedef struct aicad_sew_report {
  int changed;
  int is_valid;
  size_t free_edge_count;
  size_t multiple_edge_count;
  size_t degenerated_shape_count;
} aicad_sew_report_t;

/* Sews `shapes` together (`BRepBuilderAPI_Sewing`) at `tolerance` (a
 * modeling/construction-domain Length, canonical metres, > 0). Also
 * captures Generated/Modified/IsDeleted lineage for every unique face/
 * edge of each input against the sewing operation itself, in the SAME
 * lineage table `aicad_occt_boolean_union_lineage` (`AICAD-086`) uses --
 * `out_lineage` is consumable by the same `aicad_occt_lineage_*` query
 * functions. Sewing never deletes an entity outright (it merges/relabels
 * coincident boundaries) and never generates one with no traceable
 * input, so every captured entry's own `deleted`/`generated` are always
 * false/empty; `modified` holds at most one entry
 * (`BRepBuilderAPI_Sewing::Modified`'s own single-result shape).
 * `shape_count` must be >= 1. */
aicad_occt_status_t aicad_occt_sew(aicad_occt_context_t* context,
                                    const aicad_shape_handle_t* shapes,
                                    size_t shape_count,
                                    double tolerance,
                                    aicad_shape_handle_t* out_handle,
                                    aicad_lineage_handle_t* out_lineage,
                                    aicad_sew_report_t* out_report);

typedef struct aicad_heal_report {
  int changed;
  int is_valid_before;
  int is_valid_after;
  /* Whether `ShapeFix_Shape` returned a shape of a DIFFERENT top-level
   * TopAbs kind than its input (e.g. an unclosable Solid silently
   * demoted to a bare Shell). Verified empirically, not assumed: a
   * Solid built from a single standalone Face (missing 5 of 6 faces, no
   * possible legitimate closure) reports `is_valid_after == 1` from
   * `BRepCheck_Analyzer` alone, because `ShapeFix_Shape` gave up closing
   * it and returned a Shell instead -- a bare Shell has no closure
   * requirement to fail, so it trivially validates. `is_valid_after`
   * must NEVER be read as "healing succeeded" without also checking
   * `kind_changed == 0`: healing never invents missing geometry to
   * close a shape, and this field is what stops that silent kind-
   * downgrade from being mistaken for success (the batch's own
   * "healing is never an invisible fallback" acceptance requirement, in
   * its sharpest concrete form). */
  int kind_changed;
} aicad_heal_report_t;

/* Repairs `handle`'s shape (`ShapeFix_Shape`) at `tolerance` (same domain
 * as `aicad_occt_sew`, > 0). Verified empirically, not assumed:
 * `ShapeFix_Shape`'s own history-tracking (`ShapeBuild_ReShape::
 * History()`) does not reliably populate for common fixes (an
 * orientation-only correction never populates it, confirmed empirically
 * against a hand-built inconsistently-oriented shell) -- this function
 * therefore reports only the coarse `changed`/before/after validity
 * evidence plus `kind_changed` (see that field's own doc comment)
 * `aicad_heal_report_t` holds, not per-entity lineage; see
 * `project/reports/AICAD-120.md` for the investigation and why finer-
 * grained heal lineage is deferred rather than faked. */
aicad_occt_status_t aicad_occt_heal(aicad_occt_context_t* context,
                                     aicad_shape_handle_t handle,
                                     double tolerance,
                                     aicad_shape_handle_t* out_handle,
                                     aicad_heal_report_t* out_report);

/* --- AICAD-121: safe topology inspection (entity-kind classification and
 * orientation) -- the two accessors no earlier task added: every other
 * enumeration/adjacency/point/classification primitive this batch's own
 * Rust wrapper needs already existed (AICAD-027..034, AICAD-082..084). --- */

/* This ABI's own stable, kernel-neutral encoding of the six concrete
 * topological entity kinds -- deliberately NOT TopAbs_ShapeEnum's own
 * numbering (Stage-1 kernel policy #2-3: no OCCT enum crosses this
 * header). */
typedef enum aicad_topology_kind {
  AICAD_TOPOLOGY_VERTEX = 0,
  AICAD_TOPOLOGY_EDGE = 1,
  AICAD_TOPOLOGY_WIRE = 2,
  AICAD_TOPOLOGY_FACE = 3,
  AICAD_TOPOLOGY_SHELL = 4,
  AICAD_TOPOLOGY_SOLID = 5,
} aicad_topology_kind_t;

/* Classifies `handle`'s own top-level topological kind
 * (`TopoDS_Shape::ShapeType()`), as one of `aicad_topology_kind_t`'s six
 * values. Fails with `AICAD_OCCT_ERR_OPERATION_FAILED` for a Compound/
 * CompSolid/generic-Shape top-level kind, which has no single
 * classifiable entity kind to report. */
aicad_occt_status_t aicad_occt_shape_kind(aicad_occt_context_t* context,
                                           aicad_shape_handle_t handle,
                                           int* out_kind);

/* Reports `handle`'s own top-level `TopAbs_Orientation`, collapsed to a
 * bool: true for FORWARD, false for REVERSED/INTERNAL/EXTERNAL. The
 * latter two are rare seam/degenerate-edge markers this ABI does not
 * distinguish further from REVERSED -- a deliberate, disclosed
 * simplification (see `project/reports/AICAD-121.md`), not an
 * unconsidered omission. */
aicad_occt_status_t aicad_occt_shape_is_forward_oriented(aicad_occt_context_t* context,
                                                          aicad_shape_handle_t handle,
                                                          int* out_is_forward);

/* --- AICAD-123: functional raw topology editing (`project/DECISION_LOG.md
 * #DL-24` (D22)). Every entity argument is a handle the caller already
 * resolved by raw index against a live shape (e.g. via
 * `aicad_occt_shape_get_face`) -- none of these functions resolves an
 * index itself, mirroring `aicad_occt_shell`'s own `faces_to_remove`
 * convention. As with every other construction function in this header, a
 * non-`AICAD_OCCT_OK` status means the C++ call itself failed; it never
 * means "the result is invalid" -- callers must separately consult
 * `aicad_occt_shape_is_valid`/`aicad_occt_shape_validate` (Stage-1 kernel
 * policy #14). Optional healing after an edit is the caller's own
 * separate `aicad_occt_heal` call, not built into these functions --
 * keeps each edit's own native surface minimal and reuses the existing,
 * already-tested healing primitive rather than duplicating it. --- */

/* Removes `faces_to_remove` (>= 1, each obtained from `shape_handle`
 * itself) from `shape_handle` via `BRepTools_ReShape::Remove`+`Apply` --
 * an explicit deletion, distinct from `aicad_occt_shell`'s thickening/
 * offsetting removal. The result is commonly an open shape (removing a
 * boundary face necessarily opens the shape there); validity is the
 * caller's own separate concern, per this section's own doc comment. */
aicad_occt_status_t aicad_occt_remove_face(aicad_occt_context_t* context,
                                            aicad_shape_handle_t shape_handle,
                                            const aicad_shape_handle_t* faces_to_remove,
                                            size_t face_count,
                                            aicad_shape_handle_t* out_handle);

/* Replaces `old_face_handle` (a subshape of `shape_handle`) with
 * `new_face_handle` throughout `shape_handle`
 * (`BRepTools_ReShape::Replace`+`Apply`). If `old_face_handle` does not
 * actually occur within `shape_handle`'s own subshape tree, `Apply`
 * silently returns `shape_handle` unchanged (an OCCT `ReShape` property,
 * not a bug in this wrapper) -- callers needing to detect that should
 * compare the result's own entity count/kind against the input, not
 * assume this call's `AICAD_OCCT_OK` status alone proves a real edit
 * happened. */
aicad_occt_status_t aicad_occt_replace_face(aicad_occt_context_t* context,
                                             aicad_shape_handle_t shape_handle,
                                             aicad_shape_handle_t old_face_handle,
                                             aicad_shape_handle_t new_face_handle,
                                             aicad_shape_handle_t* out_handle);

/* Splits `edge_handle`'s own underlying curve at `params` (strictly
 * increasing, each strictly interior to the edge's own parameter range --
 * a param at or beyond either end is rejected as
 * `AICAD_OCCT_ERR_INVALID_ARGUMENT` rather than producing a degenerate
 * zero-length segment), producing `param_count + 1` new edges
 * (`BRepBuilderAPI_MakeEdge` per segment) written into the caller-owned
 * `out_handles` buffer (which must hold at least `param_count + 1`
 * entries) in ascending-parameter order; `*out_handle_count` is always set
 * to `param_count + 1` on success. Fails with
 * `AICAD_OCCT_ERR_OPERATION_FAILED` for a degenerate edge with no
 * underlying 3D curve (`BRep_Tool::Curve` returns null). */
aicad_occt_status_t aicad_occt_split_edge(aicad_occt_context_t* context,
                                           aicad_shape_handle_t edge_handle,
                                           const double* params,
                                           size_t param_count,
                                           aicad_shape_handle_t* out_handles,
                                           size_t* out_handle_count);

/* Merges `faces` (>= 2, same-domain adjacent faces expected) into as few
 * faces as their shared underlying geometry allows
 * (`ShapeUpgrade_UnifySameDomain` over a compound of `faces`). The result
 * may collapse to a single Face (full merge), or remain a Compound/Shell
 * of more than one Face if not every input pair is actually same-domain
 * adjacent -- callers distinguish the two by classifying the result's own
 * top-level kind (`aicad_occt_shape_kind`), not by this call's status
 * alone. */
aicad_occt_status_t aicad_occt_merge_faces(aicad_occt_context_t* context,
                                            const aicad_shape_handle_t* faces,
                                            size_t face_count,
                                            aicad_shape_handle_t* out_handle);

#ifdef __cplusplus
}
#endif

#endif /* AICAD_OCCT_BRIDGE_H */
