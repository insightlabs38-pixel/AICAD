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
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <exception>
#include <new>
#include <vector>

// AICAD-018: crates/cad-occt-bridge's FFI declarations represent
// AicadStatusCode as a fixed `#[repr(i32)]` Rust enum, which assumes
// this C++ compiler gives the C `enum AicadStatusCode` an `int`-sized
// underlying representation (true for GCC/Clang on this project's
// supported targets, since every enumerator is small and non-negative).
// This static_assert turns a violation of that assumption into a native
// build failure instead of undefined behavior at the ABI boundary.
static_assert(sizeof(AicadStatusCode) == sizeof(int32_t),
              "AicadStatusCode's underlying type must be 32 bits to match "
              "crates/cad-occt-bridge's #[repr(i32)] FFI declaration");

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

// AICAD-019: shape-handle table. One Slot per shape index; `live` and
// `generation` together implement the epoch/build-local contract
// AicadShapeHandle documents: a released slot is `!live` until reused,
// and its `generation` is bumped exactly once at release time so a
// handle issued before the release never matches again, even after the
// slot is reused for an unrelated shape.
struct Slot {
  TopoDS_Shape shape;
  uint32_t generation = 0;
  bool live = false;
};

// Definition of the opaque AicadOcctContext declared in the header.
// std::vector<Slot> (and, inside it, TopoDS_Shape) is an OCCT-owned-
// object container and must never cross the ABI directly — it does
// not; it lives only inside this struct, which callers only ever hold
// as an opaque pointer. `free_indices` is a simple free list: released
// slots are reused before growing the vector, bounding memory growth
// for a long-running context.
struct AicadOcctContext {
  uint64_t id;
  std::vector<Slot> slots;
  std::vector<uint32_t> free_indices;
};

namespace {

// Validates `handle` against `ctx` (context id, index range, liveness,
// and generation) and returns a pointer to its slot, or returns
// nullptr with `*out_status` set to the specific rejection reason.
// Shared by every operation that consumes an existing handle, so the
// stale/foreign/out-of-range rules are enforced identically everywhere
// rather than re-implemented per operation.
Slot* ValidateHandle(AicadOcctContext* ctx, const AicadShapeHandle& handle,
                      AicadStatus* out_status) {
  if (handle.context_id != ctx->id) {
    SetStatus(out_status, AICAD_STATUS_FOREIGN_CONTEXT_HANDLE,
              "handle was not issued by this context");
    return nullptr;
  }
  if (handle.index >= ctx->slots.size()) {
    SetStatus(out_status, AICAD_STATUS_INVALID_HANDLE, "handle index is out of range");
    return nullptr;
  }
  Slot& slot = ctx->slots[handle.index];
  if (!slot.live || slot.generation != handle.generation) {
    SetStatus(out_status, AICAD_STATUS_INVALID_HANDLE,
              "handle is stale: its slot was released and possibly reused");
    return nullptr;
  }
  return &slot;
}

}  // namespace

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

    uint32_t index;
    uint32_t generation;
    if (!ctx->free_indices.empty()) {
      index = ctx->free_indices.back();
      ctx->free_indices.pop_back();
      Slot& slot = ctx->slots[index];
      slot.shape = shape;
      slot.live = true;
      generation = slot.generation;  // already bumped by the release that freed it
    } else {
      index = static_cast<uint32_t>(ctx->slots.size());
      Slot slot;
      slot.shape = shape;
      slot.generation = 0;
      slot.live = true;
      ctx->slots.push_back(std::move(slot));
      generation = 0;
    }

    if (out_handle != nullptr) {
      out_handle->context_id = ctx->id;
      out_handle->index = index;
      out_handle->generation = generation;
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
  Slot* slot = ValidateHandle(ctx, handle, out_status);
  if (slot == nullptr) {
    return;  // *out_status already set by ValidateHandle.
  }

  try {
    GProp_GProps props;
    BRepGProp::VolumeProperties(slot->shape, props);
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

void aicad_occt_release_shape(AicadOcctContext* ctx, AicadShapeHandle handle,
                               AicadStatus* out_status) {
  if (ctx == nullptr) {
    SetStatus(out_status, AICAD_STATUS_INVALID_ARGUMENT, "ctx is null");
    return;
  }
  Slot* slot = ValidateHandle(ctx, handle, out_status);
  if (slot == nullptr) {
    return;  // *out_status already set by ValidateHandle; a double
             // release is rejected here, not treated as a no-op.
  }

  try {
    slot->shape = TopoDS_Shape();  // release this slot's OCCT-side reference
    slot->live = false;
    slot->generation += 1;  // permanently invalidates `handle` and any copy of it
    ctx->free_indices.push_back(handle.index);
    SetOk(out_status);
  } catch (const std::exception& e) {
    SetStatus(out_status, AICAD_STATUS_INTERNAL_ERROR, e.what());
  } catch (...) {
    SetStatus(out_status, AICAD_STATUS_INTERNAL_ERROR, "uncaught non-standard exception");
  }
}

}  // extern "C"
