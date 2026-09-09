// AICAD-016: native/occt_bridge C ABI boundary implementation.
//
// This is the only translation unit permitted to include OCCT headers
// and the only place OCCT types (TopoDS_Shape, Standard_Failure, ...)
// exist in this bridge's own state -- they never appear in
// aicad_occt_bridge.h and never cross the extern "C" functions below.

#include "aicad_occt_bridge.h"

#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <GProp_GProps.hxx>
#include <Standard_Failure.hxx>
#include <TopoDS_Shape.hxx>

#include <atomic>
#include <exception>
#include <thread>
#include <vector>

namespace {

// One slot in a context's shape table. `generation` is bumped every time
// the slot is released, so a handle minted before release never matches
// a shape later inserted into the same slot (Stage-1 kernel policy #5).
struct ShapeSlot {
  TopoDS_Shape shape;
  uint32_t generation = 0;
  bool occupied = false;
};

// Context-owned table of shapes, addressed by (slot, generation). Not
// thread-safe by itself; the owning context enforces single-thread
// affinity before ever touching this table (Stage-1 kernel policy #9).
class ShapeTable {
 public:
  aicad_shape_handle_t Insert(uint64_t context_id, TopoDS_Shape shape) {
    uint32_t slot_index;
    if (!free_slots_.empty()) {
      slot_index = free_slots_.back();
      free_slots_.pop_back();
    } else {
      slot_index = static_cast<uint32_t>(slots_.size());
      slots_.emplace_back();
    }
    ShapeSlot& slot = slots_[slot_index];
    slot.shape = std::move(shape);
    slot.occupied = true;
    // generation starts at 1 for a slot's first-ever occupant (not 0, so
    // a value-initialized/zeroed handle can never validate).
    if (slot.generation == 0) {
      slot.generation = 1;
    }
    return aicad_shape_handle_t{context_id, slot_index, slot.generation};
  }

  aicad_occt_status_t Lookup(aicad_shape_handle_t handle, const TopoDS_Shape** out) const {
    if (handle.slot >= slots_.size()) {
      return AICAD_OCCT_ERR_INVALID_HANDLE;
    }
    const ShapeSlot& slot = slots_[handle.slot];
    if (!slot.occupied || slot.generation != handle.generation) {
      return AICAD_OCCT_ERR_STALE_HANDLE;
    }
    *out = &slot.shape;
    return AICAD_OCCT_OK;
  }

  aicad_occt_status_t Release(aicad_shape_handle_t handle) {
    if (handle.slot >= slots_.size()) {
      return AICAD_OCCT_ERR_INVALID_HANDLE;
    }
    ShapeSlot& slot = slots_[handle.slot];
    if (!slot.occupied || slot.generation != handle.generation) {
      return AICAD_OCCT_ERR_STALE_HANDLE;
    }
    slot.shape = TopoDS_Shape();
    slot.occupied = false;
    slot.generation += 1;
    free_slots_.push_back(handle.slot);
    return AICAD_OCCT_OK;
  }

 private:
  std::vector<ShapeSlot> slots_;
  std::vector<uint32_t> free_slots_;
};

uint64_t NextContextId() {
  static std::atomic<uint64_t> counter{1};  // 0 is reserved (never valid).
  return counter.fetch_add(1, std::memory_order_relaxed);
}

}  // namespace

struct aicad_occt_context {
  uint64_t id = NextContextId();
  std::thread::id owning_thread = std::this_thread::get_id();
  ShapeTable shapes;
};

namespace {

aicad_occt_status_t CheckContext(aicad_occt_context_t* context) {
  if (context == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  if (std::this_thread::get_id() != context->owning_thread) {
    return AICAD_OCCT_ERR_WRONG_THREAD;
  }
  return AICAD_OCCT_OK;
}

aicad_occt_status_t CheckHandleContext(aicad_occt_context_t* context, aicad_shape_handle_t handle) {
  if (handle.context_id != context->id) {
    return AICAD_OCCT_ERR_FOREIGN_CONTEXT;
  }
  return AICAD_OCCT_OK;
}

}  // namespace

extern "C" {

aicad_occt_status_t aicad_occt_context_create(aicad_occt_context_t** out_context) {
  if (out_context == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  try {
    *out_context = new aicad_occt_context();
    return AICAD_OCCT_OK;
  } catch (...) {
    *out_context = nullptr;
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_context_destroy(aicad_occt_context_t* context) {
  if (context == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    delete context;
    return AICAD_OCCT_OK;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_release_shape(aicad_occt_context_t* context, aicad_shape_handle_t handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  status = CheckHandleContext(context, handle);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    return context->shapes.Release(handle);
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_create_box(aicad_occt_context_t* context,
                                           double dx,
                                           double dy,
                                           double dz,
                                           aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  if (!(dx > 0.0) || !(dy > 0.0) || !(dz > 0.0)) {
    // Also rejects NaN (every comparison with NaN is false), not just
    // non-positive values.
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  try {
    BRepPrimAPI_MakeBox make_box(dx, dy, dz);
    make_box.Build();
    if (!make_box.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, make_box.Shape());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_is_valid(aicad_occt_context_t* context,
                                               aicad_shape_handle_t handle,
                                               int* out_is_valid) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  status = CheckHandleContext(context, handle);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_is_valid == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = context->shapes.Lookup(handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepCheck_Analyzer analyzer(*shape);
    *out_is_valid = analyzer.IsValid() ? 1 : 0;
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_volume(aicad_occt_context_t* context,
                                             aicad_shape_handle_t handle,
                                             double* out_volume) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  status = CheckHandleContext(context, handle);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_volume == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = context->shapes.Lookup(handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    GProp_GProps props;
    BRepGProp::VolumeProperties(*shape, props);
    *out_volume = props.Mass();
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

}  // extern "C"
