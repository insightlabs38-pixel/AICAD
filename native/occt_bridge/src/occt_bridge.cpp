// AICAD-016: native/occt_bridge C ABI boundary — implementation.
//
// This file is the only place in the repository allowed to mix OCCT
// types with the ABI's opaque handle/status types; nothing declared in
// aicad/occt_bridge.h ever names an OCCT type.

#include "aicad/occt_bridge.h"

#include <Standard_Failure.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepGProp.hxx>
#include <GProp_GProps.hxx>
#include <TopoDS_Shape.hxx>
#include <TopAbs_ShapeEnum.hxx>

#include <atomic>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <exception>
#include <new>
#include <vector>

namespace {

void SetStatus(AicadStatus* out_status, AicadStatusCode code, const char* message) {
  if (out_status == nullptr) {
    return;
  }
  out_status->code = code;
  // snprintf always null-terminates within the given capacity; never
  // lets a native/OCCT message overrun the fixed buffer that crosses
  // the ABI.
  std::snprintf(out_status->message, AICAD_STATUS_MESSAGE_CAPACITY, "%s", message);
}

void SetOk(AicadStatus* out_status) { SetStatus(out_status, AICAD_STATUS_OK, ""); }

uint64_t NextContextId() {
  // Starts at 1, never 0, so a zero-initialized/garbage AicadShapeHandle
  // (context_id == 0) can never spuriously match a real context and is
  // always rejected as AICAD_STATUS_FOREIGN_CONTEXT_HANDLE.
  static std::atomic<uint64_t> next_id{1};
  return next_id.fetch_add(1, std::memory_order_relaxed);
}

bool IsFiniteAndPositive(double value) { return std::isfinite(value) && value > 0.0; }

}  // namespace

// Definition of the opaque AicadOcctContext declared in the header.
// std::vector<TopoDS_Shape> is an OCCT-owned-object container and must
// never cross the ABI directly — it does not; it lives only inside this
// struct, which callers only ever hold as an opaque pointer.
struct AicadOcctContext {
  uint64_t id;
  std::vector<TopoDS_Shape> shapes;
};

extern "C" {

AicadOcctContext* aicad_occt_context_create(AicadStatus* out_status) {
  try {
    AicadOcctContext* ctx = new AicadOcctContext();
    ctx->id = NextContextId();
    SetOk(out_status);
    return ctx;
  } catch (const std::bad_alloc&) {
    SetStatus(out_status, AICAD_STATUS_INTERNAL_ERROR, "allocation failure creating context");
    return nullptr;
  } catch (...) {
    SetStatus(out_status, AICAD_STATUS_INTERNAL_ERROR,
              "uncaught non-standard exception creating context");
    return nullptr;
  }
}

void aicad_occt_context_destroy(AicadOcctContext* ctx) {
  // delete on a well-formed pointer from `new` cannot throw here (the
  // context holds no non-trivial destructors that OCCT documents as
  // throwing); still no exception may escape a destroy function, so
  // guard defensively.
  try {
    delete ctx;
  } catch (...) {
    // Nothing further can be reported: this function returns void and
    // the context is being torn down regardless. Swallowing here is
    // strictly better than letting a C++ exception cross into a Rust
    // caller.
  }
}

void aicad_occt_create_box(AicadOcctContext* ctx, double dx, double dy, double dz,
                            AicadShapeHandle* out_handle, AicadStatus* out_status) {
  if (ctx == nullptr) {
    SetStatus(out_status, AICAD_STATUS_INVALID_ARGUMENT, "ctx is null");
    return;
  }
  if (!IsFiniteAndPositive(dx) || !IsFiniteAndPositive(dy) || !IsFiniteAndPositive(dz)) {
    SetStatus(out_status, AICAD_STATUS_INVALID_ARGUMENT,
              "dx, dy, dz must each be finite and strictly positive");
    return;
  }

  try {
    BRepPrimAPI_MakeBox make_box(dx, dy, dz);
    make_box.Build();
    if (!make_box.IsDone()) {
      SetStatus(out_status, AICAD_STATUS_KERNEL_FAILURE, "BRepPrimAPI_MakeBox did not complete");
      return;
    }
    TopoDS_Shape shape = make_box.Shape();
    if (shape.IsNull() || shape.ShapeType() != TopAbs_SOLID) {
      SetStatus(out_status, AICAD_STATUS_KERNEL_FAILURE,
                "box builder produced a null or non-solid shape");
      return;
    }

    ctx->shapes.push_back(shape);
    if (out_handle != nullptr) {
      out_handle->context_id = ctx->id;
      out_handle->index = static_cast<uint32_t>(ctx->shapes.size() - 1);
    }
    SetOk(out_status);
  } catch (const Standard_Failure& e) {
    SetStatus(out_status, AICAD_STATUS_KERNEL_FAILURE,
              e.GetMessageString() != nullptr ? e.GetMessageString() : "OCCT Standard_Failure");
  } catch (const std::exception& e) {
    SetStatus(out_status, AICAD_STATUS_INTERNAL_ERROR, e.what());
  } catch (...) {
    SetStatus(out_status, AICAD_STATUS_INTERNAL_ERROR, "uncaught non-standard exception");
  }
}

void aicad_occt_shape_volume(AicadOcctContext* ctx, AicadShapeHandle handle, double* out_volume,
                              AicadStatus* out_status) {
  if (ctx == nullptr) {
    SetStatus(out_status, AICAD_STATUS_INVALID_ARGUMENT, "ctx is null");
    return;
  }
  if (handle.context_id != ctx->id) {
    SetStatus(out_status, AICAD_STATUS_FOREIGN_CONTEXT_HANDLE,
              "handle was not issued by this context");
    return;
  }
  if (handle.index >= ctx->shapes.size()) {
    SetStatus(out_status, AICAD_STATUS_INVALID_HANDLE, "handle index is out of range");
    return;
  }

  try {
    GProp_GProps props;
    BRepGProp::VolumeProperties(ctx->shapes[handle.index], props);
    if (out_volume != nullptr) {
      *out_volume = props.Mass();
    }
    SetOk(out_status);
  } catch (const Standard_Failure& e) {
    SetStatus(out_status, AICAD_STATUS_KERNEL_FAILURE,
              e.GetMessageString() != nullptr ? e.GetMessageString() : "OCCT Standard_Failure");
  } catch (const std::exception& e) {
    SetStatus(out_status, AICAD_STATUS_INTERNAL_ERROR, e.what());
  } catch (...) {
    SetStatus(out_status, AICAD_STATUS_INTERNAL_ERROR, "uncaught non-standard exception");
  }
}

}  // extern "C"
