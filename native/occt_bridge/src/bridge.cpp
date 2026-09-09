// AICAD-016/AICAD-019: native/occt_bridge C ABI boundary — implementation.
//
// Every `extern "C"` function in this file follows the same shape:
//   1. validate `context` itself via check_context() — non-null and a
//      live, previously-created context (AICAD-019; see check_context's
//      own comment for why this is needed and how it stays safe even
//      against a stale/dangling pointer);
//   2. validate remaining arguments that can be checked without touching
//      OCCT (null out-params) and return a structured status immediately
//      if they fail;
//   3. do the real work inside a try/catch that catches Standard_Failure
//      (OCCT's own exception root) first, then std::exception, then
//      `...` (anything else, including a non-standard-derived throw from
//      inside OCCT) — nothing is allowed to propagate out of this file's
//      `extern "C"` functions.
//
// This is the discipline Stage-1 kernel policy #6/#7 require ("No
// C++/OCCT exception may cross the C ABI boundary"; "Normalize native
// failures into explicit structured status/error results").

#include "aicad_occt_bridge.h"

#include <atomic>
#include <cmath>
#include <mutex>
#include <string>
#include <unordered_map>
#include <unordered_set>

#include <Bnd_Box.hxx>
#include <BRepBndLib.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <Standard_Failure.hxx>
#include <TopoDS_Shape.hxx>
#include <gp_Pnt.hxx>

namespace {

// Process-wide, never-reused handle-id counter shared by every context.
// See the AicadShapeHandle comment in aicad_occt_bridge.h for why this
// must not be per-context.
std::atomic<uint64_t> g_next_handle_id{1};

// Process-wide registry of currently-live contexts (AICAD-019). Guarded
// by its own mutex, deliberately separate from any per-context state:
// contexts themselves stay single-thread-affine (Stage-1 kernel policy
// #9 — do not call two functions against the *same* context
// concurrently), but creating/destroying *different* contexts from
// different threads, or validating one context's liveness while another
// thread destroys a different context, must still be safe.
std::mutex g_live_contexts_mutex;
std::unordered_set<AicadKernelContext *> g_live_contexts;

bool is_live_context_locked_free(AicadKernelContext *context) {
  std::lock_guard<std::mutex> lock(g_live_contexts_mutex);
  return g_live_contexts.count(context) != 0;
}

// Validates `context` WITHOUT ever dereferencing it: only the pointer
// *value* is looked up in g_live_contexts, which is exactly what makes
// this safe to call even when `context` is a stale/dangling pointer left
// over from an already-destroyed context (the whole point of this
// function — turning that misuse from a use-after-free into a clean
// rejection). Comparing/hashing a dangling pointer's *value* (never
// reading through it) is the same well-established defensive-validation
// idiom used by allocators/object pools; it is not a guarantee against
// deliberately adversarial pointer forgery, only against ordinary
// programmer error (using a context after freeing it, a typo'd pointer,
// etc.), which is this ABI's actual threat model.
AicadStatus check_context(AicadKernelContext *context) {
  if (context == nullptr) {
    return AICAD_STATUS_NULL_CONTEXT;
  }
  if (!is_live_context_locked_free(context)) {
    return AICAD_STATUS_INVALID_CONTEXT;
  }
  return AICAD_STATUS_OK;
}

} // namespace

struct AicadKernelContext {
  std::unordered_map<uint64_t, TopoDS_Shape> shapes;
  std::string last_error;
};

namespace {

void set_last_error(AicadKernelContext *context, const std::string &message) {
  if (context != nullptr) {
    context->last_error = message;
  }
}

// Runs `body`, converting any exception that escapes it into a structured
// AicadStatus + last_error message on `context`. `body` must not itself
// touch `out`-parameters before success is certain (callers below follow
// this by construction: they only write to `*out_*` after `body`
// returns normally).
template <typename Fn>
AicadStatus guard(AicadKernelContext *context, const char *op_name, Fn &&body) {
  try {
    return body();
  } catch (const Standard_Failure &occt_exception) {
    std::string message = std::string(op_name) + ": OCCT exception (" +
                           occt_exception.DynamicType()->Name() +
                           "): " + occt_exception.GetMessageString();
    set_last_error(context, message);
    return AICAD_STATUS_KERNEL_INTERNAL_ERROR;
  } catch (const std::exception &generic_exception) {
    std::string message =
        std::string(op_name) + ": std::exception: " + generic_exception.what();
    set_last_error(context, message);
    return AICAD_STATUS_KERNEL_INTERNAL_ERROR;
  } catch (...) {
    set_last_error(context, std::string(op_name) +
                                 ": unknown non-standard exception escaped "
                                 "OCCT call");
    return AICAD_STATUS_UNKNOWN_ERROR;
  }
}

} // namespace

AicadKernelContext *aicad_kernel_context_create(void) {
  try {
    auto *context = new AicadKernelContext();
    // If registering `context` below somehow throws (e.g. bad_alloc from
    // the unordered_set itself — extremely unlikely for one pointer-sized
    // insert), this function falls through to `catch (...)` and returns
    // nullptr, leaking `context` rather than registering a half-created
    // entry: acceptable, since this is already a global out-of-memory
    // condition the caller cannot meaningfully recover from either way.
    std::lock_guard<std::mutex> lock(g_live_contexts_mutex);
    g_live_contexts.insert(context);
    return context;
  } catch (...) {
    return nullptr;
  }
}

void aicad_kernel_context_destroy(AicadKernelContext *context) {
  if (context == nullptr) {
    return;
  }
  bool was_live;
  {
    std::lock_guard<std::mutex> lock(g_live_contexts_mutex);
    was_live = g_live_contexts.erase(context) != 0;
  }
  if (was_live) {
    delete context;
  }
  // If `context` was not live (already destroyed, or a garbage/foreign
  // pointer), do nothing — in particular, never call `delete` on it. This
  // is what makes a second aicad_kernel_context_destroy call on the same
  // pointer a safe no-op instead of a double-free (see the header).
}

AicadStatus
aicad_kernel_context_live_shape_count(AicadKernelContext *context,
                                       uint64_t *out_count) {
  const AicadStatus context_status = check_context(context);
  if (context_status != AICAD_STATUS_OK) {
    return context_status;
  }
  if (out_count == nullptr) {
    set_last_error(context,
                    "aicad_kernel_context_live_shape_count: out_count is null");
    return AICAD_STATUS_INVALID_ARGUMENT;
  }
  *out_count = context->shapes.size();
  return AICAD_STATUS_OK;
}

const char *aicad_kernel_context_last_error(AicadKernelContext *context) {
  static const char empty[] = "";
  if (check_context(context) != AICAD_STATUS_OK) {
    return empty;
  }
  return context->last_error.c_str();
}

AicadStatus aicad_create_box(AicadKernelContext *context, double dx, double dy,
                              double dz, AicadShapeHandle *out_handle) {
  const AicadStatus context_status = check_context(context);
  if (context_status != AICAD_STATUS_OK) {
    return context_status;
  }
  if (out_handle == nullptr) {
    set_last_error(context, "aicad_create_box: out_handle is null");
    return AICAD_STATUS_INVALID_ARGUMENT;
  }

  return guard(context, "aicad_create_box", [&]() -> AicadStatus {
    // BRepPrimAPI_MakeBox is documented (and independently verified
    // during this task) to throw Standard_DomainError for a degenerate
    // (zero) dimension rather than merely reporting IsDone() == false;
    // that is exactly the kind of native failure this guard() wrapper
    // exists to catch and normalize.
    BRepPrimAPI_MakeBox box_builder(gp_Pnt(0.0, 0.0, 0.0), dx, dy, dz);

    // BRepBuilderAPI_MakeShape::Build() runs lazily inside Shape(), so
    // IsDone() is only meaningful after Shape() has been called (see
    // project/reports/AICAD-015.md's debugging notes for how this was
    // first discovered).
    TopoDS_Shape shape = box_builder.Shape();
    if (!box_builder.IsDone() || shape.IsNull()) {
      set_last_error(context,
                      "aicad_create_box: OCCT reported the box as not done");
      return AICAD_STATUS_KERNEL_INTERNAL_ERROR;
    }

    const uint64_t handle_id = g_next_handle_id.fetch_add(1);
    context->shapes.emplace(handle_id, std::move(shape));
    out_handle->id = handle_id;
    return AICAD_STATUS_OK;
  });
}

AicadStatus aicad_shape_bbox_diagonal(AicadKernelContext *context,
                                       AicadShapeHandle handle,
                                       double *out_diagonal) {
  const AicadStatus context_status = check_context(context);
  if (context_status != AICAD_STATUS_OK) {
    return context_status;
  }
  if (out_diagonal == nullptr) {
    set_last_error(context, "aicad_shape_bbox_diagonal: out_diagonal is null");
    return AICAD_STATUS_INVALID_ARGUMENT;
  }

  auto found = context->shapes.find(handle.id);
  if (handle.id == 0 || found == context->shapes.end()) {
    set_last_error(context, "aicad_shape_bbox_diagonal: unknown handle");
    return AICAD_STATUS_INVALID_HANDLE;
  }

  return guard(context, "aicad_shape_bbox_diagonal", [&]() -> AicadStatus {
    Bnd_Box bounds;
    BRepBndLib::Add(found->second, bounds);
    if (bounds.IsVoid()) {
      set_last_error(context,
                      "aicad_shape_bbox_diagonal: bounding box is void");
      return AICAD_STATUS_KERNEL_INTERNAL_ERROR;
    }
    // BRepBndLib::Add leaves a small default gap (~1e-7, independently
    // measured during this task) baked into the box, added symmetrically
    // on every side when Get() is queried — it is not itself part of the
    // shape's true extent. Bnd_Box::SetGap only takes effect for Get()
    // calls made after it, so it must be called here, after Add and
    // before Get, not before Add (verified empirically: setting it
    // before Add has no effect, since Add sets its own gap).
    bounds.SetGap(0.0);
    double xmin, ymin, zmin, xmax, ymax, zmax;
    bounds.Get(xmin, ymin, zmin, xmax, ymax, zmax);
    const double dx = xmax - xmin;
    const double dy = ymax - ymin;
    const double dz = zmax - zmin;
    *out_diagonal = std::sqrt(dx * dx + dy * dy + dz * dz);
    return AICAD_STATUS_OK;
  });
}

AicadStatus aicad_shape_release(AicadKernelContext *context,
                                 AicadShapeHandle handle) {
  const AicadStatus context_status = check_context(context);
  if (context_status != AICAD_STATUS_OK) {
    return context_status;
  }
  const auto erased = context->shapes.erase(handle.id);
  if (handle.id == 0 || erased == 0) {
    set_last_error(context, "aicad_shape_release: unknown handle");
    return AICAD_STATUS_INVALID_HANDLE;
  }
  return AICAD_STATUS_OK;
}
