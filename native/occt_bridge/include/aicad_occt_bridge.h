/* AICAD-016: native/occt_bridge C ABI boundary.
 *
 * This header is the ONLY contract between the AICAD Rust workspace and
 * the native OCCT bridge. Per DL-5/DL-6 (project/DECISION_LOG.md) and the
 * Stage-1 kernel policies:
 *
 *   - No OCCT/C++ type may appear here (Standard_Failure, TopoDS_Shape,
 *     Handle(...), std::string, STL containers, etc.).
 *   - No raw OCCT/C++ object pointer is ever exposed as resource
 *     identity. AicadShapeHandle is an opaque, context-scoped,
 *     generation-checked handle (POD integers only), never a pointer
 *     into kernel-owned memory.
 *   - No C++ exception may cross this boundary: every function below is
 *     `noexcept` in the implementation and converts any native failure
 *     into an AicadStatus code.
 *   - AicadKernelContext is single-thread-affine (Stage-1 kernel policy
 *     #9): concurrent calls against the same context from multiple
 *     threads are undefined behavior; this header does not add locking.
 *
 * This is intentionally a narrow slice: context lifecycle plus exactly
 * one representative constructive operation (create_box) and one
 * representative query (shape_volume), enough to prove the ABI's
 * handle-safety and exception-containment properties end-to-end. The
 * full operation list in native/occt_bridge/README.md is added
 * incrementally by later Stage-1 batches (1B/1C), not by this task.
 */

#ifndef AICAD_OCCT_BRIDGE_H
#define AICAD_OCCT_BRIDGE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque kernel context. Only ever used through a pointer; the AICAD
 * side never inspects or copies its contents. */
typedef struct AicadKernelContext AicadKernelContext;

/* Structured result/status code. Every ABI function returns one of
 * these instead of throwing or aborting (Stage-1 kernel policy #7). */
typedef enum AicadStatus {
    AICAD_STATUS_OK = 0,
    AICAD_STATUS_INVALID_ARGUMENT = 1,
    AICAD_STATUS_INVALID_HANDLE = 2,
    AICAD_STATUS_STALE_HANDLE = 3,
    AICAD_STATUS_FOREIGN_CONTEXT = 4,
    AICAD_STATUS_NATIVE_EXCEPTION = 5,
    AICAD_STATUS_UNKNOWN_ERROR = 6
} AicadStatus;

/* Opaque, context-scoped handle to a kernel-owned shape. Identity is
 * (context_id, slot, generation) -- never a pointer. A handle whose
 * context_id does not match the context it is presented to is rejected
 * as AICAD_STATUS_FOREIGN_CONTEXT; a handle whose generation does not
 * match the current occupant of its slot is rejected as
 * AICAD_STATUS_STALE_HANDLE (Stage-1 kernel policy #5/#10). The
 * all-zero handle is never issued by a successful call and can be used
 * by a caller as a sentinel "no handle" value. */
typedef struct AicadShapeHandle {
    uint64_t context_id;
    uint32_t slot;
    uint32_t generation;
} AicadShapeHandle;

/* Create a new kernel context. On AICAD_STATUS_OK, *out_ctx is a valid
 * context that must eventually be passed to aicad_context_destroy.
 * On failure, *out_ctx is left unset. */
AicadStatus aicad_context_create(AicadKernelContext** out_ctx);

/* Destroy a kernel context and every shape it owns. ctx must not be
 * used again after this call, by this ABI or by the caller. Passing
 * NULL is a no-op that returns AICAD_STATUS_OK. */
AicadStatus aicad_context_destroy(AicadKernelContext* ctx);

/* Construct an axis-aligned box of the given extents (in the kernel's
 * canonical length unit; unit interpretation is the caller's
 * responsibility per cad-kernel-api/cad-units -- this ABI is
 * unit-unaware). dx, dy, dz must all be strictly positive. */
AicadStatus aicad_create_box(
    AicadKernelContext* ctx,
    double dx,
    double dy,
    double dz,
    AicadShapeHandle* out_handle
);

/* Compute the exact volume of the solid referenced by handle. Fails
 * with AICAD_STATUS_INVALID_HANDLE / AICAD_STATUS_STALE_HANDLE /
 * AICAD_STATUS_FOREIGN_CONTEXT if handle does not currently name a live
 * shape owned by ctx. */
AicadStatus aicad_shape_volume(
    AicadKernelContext* ctx,
    AicadShapeHandle handle,
    double* out_volume
);

/* Release a shape. After this call, handle (and any copy of it) is
 * stale: the slot may be reused by a later create, but the reused slot
 * is given a new generation, so the old handle is rejected rather than
 * silently aliasing the new shape (Stage-1 kernel policy #5). */
AicadStatus aicad_shape_destroy(
    AicadKernelContext* ctx,
    AicadShapeHandle handle
);

#ifdef __cplusplus
}
#endif

#endif /* AICAD_OCCT_BRIDGE_H */
