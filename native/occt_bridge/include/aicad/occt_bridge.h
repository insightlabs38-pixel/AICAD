/* AICAD-016: native/occt_bridge C ABI boundary.
 *
 * This header is the ONLY contract between crates/cad-occt-bridge (Rust,
 * AICAD-018) and this native bridge. Per AGENTS.md's Stage-1 kernel
 * policies:
 *
 *   - no OCCT class/type may appear here or cross this boundary;
 *   - no C++/OCCT exception may cross this boundary (every function
 *     below normalizes native failures into AicadStatus instead);
 *   - resource identity crossing the boundary is an opaque, POD handle,
 *     never a raw OCCT/C++ object pointer;
 *   - STL containers, std::string, and OCCT-owned objects never cross
 *     this boundary directly (AicadStatus.message is a fixed-size
 *     buffer, not a std::string).
 *
 * AicadShapeHandle is epoch/build-local (RFC-0002 §5 "raw handles"):
 * `context_id` rejects a handle used against the wrong context;
 * `index` rejects an out-of-range handle. A `generation` field for
 * detecting a *stale* handle (one whose slot was released and reused)
 * is added by AICAD-019's shape-handle table, not by this task —
 * AICAD-016 has no operation that releases a shape yet, so there is
 * nothing for a generation counter to guard here.
 */

#ifndef AICAD_OCCT_BRIDGE_H
#define AICAD_OCCT_BRIDGE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque. Never dereferenced by callers; only ever passed back into
 * this ABI. Owns all shapes created against it. */
typedef struct AicadOcctContext AicadOcctContext;

typedef enum AicadStatusCode {
  AICAD_STATUS_OK = 0,
  AICAD_STATUS_INVALID_ARGUMENT = 1,
  AICAD_STATUS_INVALID_HANDLE = 2,
  AICAD_STATUS_FOREIGN_CONTEXT_HANDLE = 3,
  AICAD_STATUS_KERNEL_FAILURE = 4,
  AICAD_STATUS_INTERNAL_ERROR = 5
} AicadStatusCode;

#define AICAD_STATUS_MESSAGE_CAPACITY 256

typedef struct AicadStatus {
  AicadStatusCode code;
  char message[AICAD_STATUS_MESSAGE_CAPACITY];
} AicadStatus;

/* POD, safe to pass and copy by value. Fields are part of the public
 * contract (this is a raw/unsafe handle by design, per RFC-0002 §5), but
 * callers must treat them as opaque and never construct one from
 * scratch — always copy one previously returned by this ABI. */
typedef struct AicadShapeHandle {
  uint64_t context_id;
  uint32_t index;
} AicadShapeHandle;

/* Creates a new, independent kernel context. Returns NULL and sets
 * *out_status on failure (e.g. allocation failure); never throws. */
AicadOcctContext* aicad_occt_context_create(AicadStatus* out_status);

/* Destroys a context and every shape it owns. ctx may be NULL (no-op).
 * Every AicadShapeHandle previously returned for this context becomes
 * invalid; using one after this call is a caller error this ABI cannot
 * detect (the context itself is gone), which is why the Rust safe
 * wrapper (AICAD-018) must not allow it to happen. */
void aicad_occt_context_destroy(AicadOcctContext* ctx);

/* Constructs an exact-B-rep rectangular box solid of dimension
 * dx * dy * dz, anchored at the origin. dx, dy, dz must each be finite
 * and strictly positive; a zero, negative, non-finite, or NaN dimension
 * is rejected as AICAD_STATUS_INVALID_ARGUMENT before any kernel call is
 * made. Writes the new shape's handle to *out_handle and
 * AICAD_STATUS_OK to *out_status on success. */
void aicad_occt_create_box(AicadOcctContext* ctx, double dx, double dy,
                            double dz, AicadShapeHandle* out_handle,
                            AicadStatus* out_status);

/* Writes the exact volume of the solid referenced by handle to
 * *out_volume. Fails with AICAD_STATUS_INVALID_ARGUMENT if ctx is NULL,
 * AICAD_STATUS_FOREIGN_CONTEXT_HANDLE if handle was not issued by ctx,
 * or AICAD_STATUS_INVALID_HANDLE if handle.index is out of range for
 * ctx. */
void aicad_occt_shape_volume(AicadOcctContext* ctx, AicadShapeHandle handle,
                              double* out_volume, AicadStatus* out_status);

#ifdef __cplusplus
}
#endif

#endif /* AICAD_OCCT_BRIDGE_H */
