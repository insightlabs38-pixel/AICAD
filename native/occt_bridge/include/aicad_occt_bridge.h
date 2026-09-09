/* AICAD-016: native/occt_bridge C ABI boundary.
 *
 * This is the ONLY interface `crates/cad-occt-bridge` (AICAD-018) may call
 * into. Per docs/plan/01_SYSTEM_ARCHITECTURE.md §2.6 and
 * docs/plan/22_REPOSITORY_WORK_PACKAGES.md WP-01, no OCCT type/class may
 * cross this boundary in either direction:
 *
 *   - handles are opaque (AicadKernelContext* is an incomplete type;
 *     AicadShapeHandle is a plain integer with no OCCT-derived meaning
 *     visible to the caller);
 *   - no STL container, std::string, or OCCT-owned object crosses this
 *     boundary directly (only POD structs, integers, and a
 *     context-owned, null-terminated `const char*` for error text);
 *   - no C++/OCCT exception may cross this boundary — every function
 *     below is implemented to catch every exception internally and
 *     report it as an AicadStatus instead (see src/bridge.cpp).
 *
 * Handle-table safety (invalid/stale/foreign-context handle rejection)
 * is provisional in this task and will be substantially hardened by
 * AICAD-019 ("Implement kernel context lifecycle and shape-handle
 * table"), which owns the real epoch/generation semantics. This task
 * establishes the ABI mechanics (opaque handles, structured status, no
 * exceptions escaping) and a minimal handle registry sufficient to prove
 * those mechanics end-to-end with one real geometry round-trip
 * (create_box -> query -> release). Do not treat the current handle
 * registry as the final safety contract.
 */

#ifndef AICAD_OCCT_BRIDGE_H
#define AICAD_OCCT_BRIDGE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque context type. A context owns a set of shape handles and its own
 * last-error text. Per Stage-1 kernel policy #9, treat a context as
 * single-thread-affine: do not call two `aicad_*` functions against the
 * same context concurrently from different threads. */
typedef struct AicadKernelContext AicadKernelContext;

/* A shape handle is a plain 64-bit id with no pointer/pointer-arithmetic
 * meaning to the caller. Ids are minted from a single process-wide
 * counter (never reused, never restarted per-context) specifically so
 * that:
 *   - a released handle can never accidentally alias a newly created
 *     shape (Stage-1 kernel policy #5);
 *   - a handle minted by one context is guaranteed to never coincide
 *     with any id known to a different context, so passing one
 *     context's handle to another context's functions is rejected as
 *     unknown rather than risking an aliasing collision.
 * `0` is never a valid id and denotes "no handle". */
typedef struct AicadShapeHandle {
  uint64_t id;
} AicadShapeHandle;

typedef enum AicadStatus {
  AICAD_STATUS_OK = 0,
  AICAD_STATUS_NULL_CONTEXT = 1,
  AICAD_STATUS_INVALID_ARGUMENT = 2,
  AICAD_STATUS_INVALID_HANDLE = 3,
  AICAD_STATUS_KERNEL_INTERNAL_ERROR = 4,
  AICAD_STATUS_UNKNOWN_ERROR = 5
} AicadStatus;

/* Create a new kernel context. Returns NULL on allocation failure only
 * (never throws/aborts). The caller owns the returned context and must
 * release it with aicad_kernel_context_destroy. */
AicadKernelContext *aicad_kernel_context_create(void);

/* Destroy a context and every shape it still owns. `context` may be
 * NULL (no-op). Do not use `context` or any AicadShapeHandle obtained
 * from it after this call: aicad_kernel_context_destroy invalidates the
 * whole handle namespace that context minted. */
void aicad_kernel_context_destroy(AicadKernelContext *context);

/* Returns a null-terminated description of the most recent non-OK
 * status returned by a call against this context, or an empty string if
 * none has occurred yet. The returned pointer is owned by `context`: it
 * remains valid only until the next `aicad_*` call made against the
 * same context, and must never be freed by the caller. Returns an empty
 * static string (not NULL) if `context` is NULL. */
const char *aicad_kernel_context_last_error(AicadKernelContext *context);

/* Minimal geometry round-trip. The full bridge catalog
 * (docs/plan/01_SYSTEM_ARCHITECTURE.md §4's create_box .. export_step
 * list) is added incrementally by later Stage-1 tasks (AICAD-020
 * onward) as the safe Rust wrapper (AICAD-018) needs it — this function
 * exists in AICAD-016 only to prove the ABI mechanics with one real
 * operation, not to be a complete catalog entry point. */

/* Construct an axis-aligned box of size dx*dy*dz at the origin.
 * On success, writes a fresh handle to *out_handle and returns
 * AICAD_STATUS_OK. On failure (including an OCCT-side exception, e.g. a
 * degenerate zero-dimension box), *out_handle is left unmodified and a
 * non-OK status is returned; aicad_kernel_context_last_error(context)
 * describes why. */
AicadStatus aicad_create_box(AicadKernelContext *context, double dx,
                              double dy, double dz,
                              AicadShapeHandle *out_handle);

/* Compute the diagonal length of the shape's axis-aligned bounding box.
 * Returns AICAD_STATUS_INVALID_HANDLE if `handle` is not a live handle
 * owned by `context` (never created, already released, or minted by a
 * different context). */
AicadStatus aicad_shape_bbox_diagonal(AicadKernelContext *context,
                                       AicadShapeHandle handle,
                                       double *out_diagonal);

/* Release a shape handle. Returns AICAD_STATUS_INVALID_HANDLE if
 * `handle` is not a live handle owned by `context`. After a successful
 * release, `handle`'s id is never reissued by any context (see the
 * AicadShapeHandle comment above) — using it again always yields
 * AICAD_STATUS_INVALID_HANDLE rather than aliasing a new shape. */
AicadStatus aicad_shape_release(AicadKernelContext *context,
                                 AicadShapeHandle handle);

#ifdef __cplusplus
}
#endif

#endif /* AICAD_OCCT_BRIDGE_H */
