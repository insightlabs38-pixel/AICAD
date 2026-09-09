// AICAD-016: native/occt_bridge C ABI boundary — implementation.
//
// Every `extern "C"` function in this file follows the same shape:
//   1. validate arguments that can be checked without touching OCCT
//      (null context, null out-params) and return a structured status
//      immediately if they fail;
//   2. do the real work inside a try/catch that catches
//      Standard_Failure (OCCT's own exception root) first, then
//      std::exception, then `...` (anything else, including a
//      non-standard-derived throw from inside OCCT) — nothing is
//      allowed to propagate out of this file's `extern "C"` functions.
//
// This is the discipline Stage-1 kernel policy #6/#7 require ("No
// C++/OCCT exception may cross the C ABI boundary"; "Normalize native
// failures into explicit structured status/error results").

#include "aicad_occt_bridge.h"

#include <atomic>
#include <cmath>
#include <string>
#include <unordered_map>

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
    return new AicadKernelContext();
  } catch (...) {
    // Allocation failure (or a throwing default constructor, which this
    // type does not have) is the one case this function itself can hit;
    // there is no context to attach an error message to yet.
    return nullptr;
  }
}

void aicad_kernel_context_destroy(AicadKernelContext *context) {
  delete context; // no-op if context == nullptr, per C++ `delete` semantics.
}

const char *aicad_kernel_context_last_error(AicadKernelContext *context) {
  static const char empty[] = "";
  if (context == nullptr) {
    return empty;
  }
  return context->last_error.c_str();
}

AicadStatus aicad_create_box(AicadKernelContext *context, double dx, double dy,
                              double dz, AicadShapeHandle *out_handle) {
  if (context == nullptr) {
    return AICAD_STATUS_NULL_CONTEXT;
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
  if (context == nullptr) {
    return AICAD_STATUS_NULL_CONTEXT;
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
  if (context == nullptr) {
    return AICAD_STATUS_NULL_CONTEXT;
  }
  const auto erased = context->shapes.erase(handle.id);
  if (handle.id == 0 || erased == 0) {
    set_last_error(context, "aicad_shape_release: unknown handle");
    return AICAD_STATUS_INVALID_HANDLE;
  }
  return AICAD_STATUS_OK;
}
