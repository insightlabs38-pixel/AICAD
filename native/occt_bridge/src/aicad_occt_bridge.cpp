// AICAD-016: native/occt_bridge C ABI boundary implementation.
//
// This is the only translation unit permitted to include OCCT headers
// and the only place OCCT types (TopoDS_Shape, Standard_Failure, ...)
// exist in this bridge's own state -- they never appear in
// aicad_occt_bridge.h and never cross the extern "C" functions below.

#include "aicad_occt_bridge.h"

#include <BRepBndLib.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <Bnd_Box.hxx>
#include <GProp_GProps.hxx>
#include <Standard_Failure.hxx>
#include <TopoDS_Shape.hxx>
#include <gp_Trsf.hxx>

#include <atomic>
#include <cmath>
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

// --- AICAD-021 helper ---

double Norm3(const double v[3]) {
  return std::sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
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
  if (!(dx > 0.0) || !(dy > 0.0) || !(dz > 0.0) || !std::isfinite(dx) || !std::isfinite(dy) ||
      !std::isfinite(dz)) {
    // The `> 0.0` comparisons also reject NaN (every comparison with NaN
    // is false); the explicit `isfinite` checks additionally reject
    // +infinity, which passes `> 0.0` but is not a valid dimension.
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

aicad_occt_status_t aicad_occt_create_cylinder(aicad_occt_context_t* context,
                                                double radius,
                                                double height,
                                                aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  if (!(radius > 0.0) || !(height > 0.0) || !std::isfinite(radius) || !std::isfinite(height)) {
    // The `> 0.0` comparisons also reject NaN; `isfinite` additionally
    // rejects +infinity, which passes `> 0.0` but is not a valid dimension.
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  try {
    BRepPrimAPI_MakeCylinder make_cylinder(radius, height);
    make_cylinder.Build();
    if (!make_cylinder.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, make_cylinder.Shape());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_transform_shape(aicad_occt_context_t* context,
                                                aicad_shape_handle_t handle,
                                                const double matrix[12],
                                                aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  status = CheckHandleContext(context, handle);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (matrix == nullptr || out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  for (int i = 0; i < 12; ++i) {
    if (!std::isfinite(matrix[i])) {
      return AICAD_OCCT_ERR_INVALID_ARGUMENT;
    }
  }
  // Defensively re-validate rigidity here even though the only current
  // caller (`cad-occt-bridge`'s `Transform`) always constructs an
  // orthonormal, proper-rotation linear part: a bug in that pure-Rust
  // math layer must never silently corrupt kernel geometry (AGENTS.md
  // evidence rule) -- it must be rejected at this boundary instead.
  const double col0[3] = {matrix[0], matrix[4], matrix[8]};
  const double col1[3] = {matrix[1], matrix[5], matrix[9]};
  const double col2[3] = {matrix[2], matrix[6], matrix[10]};
  const double n0 = Norm3(col0);
  const double n1 = Norm3(col1);
  const double n2 = Norm3(col2);
  const double kTol = 1e-6;
  if (std::fabs(n0 - 1.0) > kTol || std::fabs(n1 - 1.0) > kTol || std::fabs(n2 - 1.0) > kTol) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  auto dot3 = [](const double a[3], const double b[3]) {
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  };
  if (std::fabs(dot3(col0, col1)) > kTol || std::fabs(dot3(col0, col2)) > kTol ||
      std::fabs(dot3(col1, col2)) > kTol) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  // determinant of the 3x3 linear part must be +1 (proper rotation, not
  // a reflection) for gp_Trsf to represent it as a rigid displacement.
  const double det = matrix[0] * (matrix[5] * matrix[10] - matrix[6] * matrix[9]) -
                      matrix[1] * (matrix[4] * matrix[10] - matrix[6] * matrix[8]) +
                      matrix[2] * (matrix[4] * matrix[9] - matrix[5] * matrix[8]);
  if (std::fabs(det - 1.0) > kTol) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = context->shapes.Lookup(handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    gp_Trsf trsf;
    trsf.SetValues(matrix[0], matrix[1], matrix[2], matrix[3], matrix[4], matrix[5], matrix[6],
                    matrix[7], matrix[8], matrix[9], matrix[10], matrix[11]);
    BRepBuilderAPI_Transform transform(*shape, trsf, /*Copy=*/Standard_True);
    if (!transform.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, transform.Shape());
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

aicad_occt_status_t aicad_occt_shape_area(aicad_occt_context_t* context,
                                           aicad_shape_handle_t handle,
                                           double* out_area) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  status = CheckHandleContext(context, handle);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_area == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = context->shapes.Lookup(handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    GProp_GProps props;
    BRepGProp::SurfaceProperties(*shape, props);
    *out_area = props.Mass();
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_bounding_box(aicad_occt_context_t* context,
                                                   aicad_shape_handle_t handle,
                                                   double out_min[3],
                                                   double out_max[3]) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  status = CheckHandleContext(context, handle);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_min == nullptr || out_max == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = context->shapes.Lookup(handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    Bnd_Box box;
    BRepBndLib::Add(*shape, box);
    if (box.IsVoid()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    double xmin, ymin, zmin, xmax, ymax, zmax;
    box.Get(xmin, ymin, zmin, xmax, ymax, zmax);
    out_min[0] = xmin;
    out_min[1] = ymin;
    out_min[2] = zmin;
    out_max[0] = xmax;
    out_max[1] = ymax;
    out_max[2] = zmax;
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

}  // extern "C"
