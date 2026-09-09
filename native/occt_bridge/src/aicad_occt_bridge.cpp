// AICAD-016: native/occt_bridge C ABI boundary implementation.
//
// This is the only translation unit permitted to include OCCT headers
// and the only place OCCT types (TopoDS_Shape, Standard_Failure, ...)
// exist in this bridge's own state -- they never appear in
// aicad_occt_bridge.h and never cross the extern "C" functions below.

#include "aicad_occt_bridge.h"

#include <BRepBndLib.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <BRepAlgoAPI_Common.hxx>
#include <BRepAlgoAPI_Cut.hxx>
#include <BRepAlgoAPI_Fuse.hxx>
#include <BRepOffsetAPI_MakePipe.hxx>
#include <BRepOffsetAPI_ThruSections.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <BRepPrimAPI_MakeRevol.hxx>
#include <Bnd_Box.hxx>
#include <GProp_GProps.hxx>
#include <Standard_Failure.hxx>
#include <TopAbs_ShapeEnum.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Edge.hxx>
#include <TopoDS_Face.hxx>
#include <TopoDS_Shape.hxx>
#include <TopoDS_Wire.hxx>
#include <gp_Ax1.hxx>
#include <gp_Ax2.hxx>
#include <gp_Circ.hxx>
#include <gp_Dir.hxx>
#include <gp_Pnt.hxx>
#include <gp_Trsf.hxx>
#include <gp_Vec.hxx>

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

// --- AICAD-022 helpers (shared by later Batch 1B tasks too) ---

bool IsFinite3(const double v[3]) {
  return std::isfinite(v[0]) && std::isfinite(v[1]) && std::isfinite(v[2]);
}

gp_Pnt ToPnt(const double v[3]) { return gp_Pnt(v[0], v[1], v[2]); }

// Returns false (leaving `out` untouched) if `v` is not finite or is too
// close to the zero vector to normalize reliably.
bool TryToDir(const double v[3], gp_Dir* out) {
  if (!IsFinite3(v) || Norm3(v) < 1e-12) {
    return false;
  }
  *out = gp_Dir(v[0], v[1], v[2]);
  return true;
}

// Looks up a handle and additionally requires its shape to be exactly
// `kind` (e.g. TopAbs_EDGE) -- a caller-contract violation (wrong handle
// kind passed to an operation that only makes sense for one topological
// kind), so this rejects with INVALID_ARGUMENT rather than relying on an
// OCCT-level TopoDS:: cast exception to be caught and mapped later.
aicad_occt_status_t LookupTyped(aicad_occt_context_t* context,
                                 aicad_shape_handle_t handle,
                                 TopAbs_ShapeEnum kind,
                                 const TopoDS_Shape** out) {
  aicad_occt_status_t status = CheckHandleContext(context, handle);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  status = context->shapes.Lookup(handle, out);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if ((*out)->ShapeType() != kind) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
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

aicad_occt_status_t aicad_occt_make_line_edge(aicad_occt_context_t* context,
                                               const double p0[3],
                                               const double p1[3],
                                               aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (p0 == nullptr || p1 == nullptr || out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  if (!IsFinite3(p0) || !IsFinite3(p1)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const double diff[3] = {p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]};
  if (Norm3(diff) < 1e-12) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;  // coincident endpoints
  }
  try {
    BRepBuilderAPI_MakeEdge make_edge(ToPnt(p0), ToPnt(p1));
    if (!make_edge.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, make_edge.Edge());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_make_circle_wire(aicad_occt_context_t* context,
                                                 const double center[3],
                                                 const double normal[3],
                                                 double radius,
                                                 aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (center == nullptr || normal == nullptr || out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  if (!IsFinite3(center) || !(radius > 0.0) || !std::isfinite(radius)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  gp_Dir normal_dir;
  if (!TryToDir(normal, &normal_dir)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  try {
    gp_Ax2 axes(ToPnt(center), normal_dir);
    gp_Circ circle(axes, radius);
    BRepBuilderAPI_MakeEdge make_edge(circle);
    if (!make_edge.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    BRepBuilderAPI_MakeWire make_wire(make_edge.Edge());
    if (!make_wire.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, make_wire.Wire());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_make_wire_from_edges(aicad_occt_context_t* context,
                                                     const aicad_shape_handle_t* edges,
                                                     size_t edge_count,
                                                     aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (edges == nullptr || out_handle == nullptr || edge_count == 0) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  try {
    BRepBuilderAPI_MakeWire make_wire;
    for (size_t i = 0; i < edge_count; ++i) {
      const TopoDS_Shape* edge_shape = nullptr;
      status = LookupTyped(context, edges[i], TopAbs_EDGE, &edge_shape);
      if (status != AICAD_OCCT_OK) {
        return status;
      }
      make_wire.Add(TopoDS::Edge(*edge_shape));
      if (!make_wire.IsDone()) {
        // BRepBuilderAPI_MakeWire's own error enum (Error()) distinguishes
        // disconnected/non-manifold input from a generic failure, but all
        // of it is "this edge sequence cannot form a wire" from the
        // caller's point of view -- an operation failure, not an adapter
        // defect.
        return AICAD_OCCT_ERR_OPERATION_FAILED;
      }
    }
    *out_handle = context->shapes.Insert(context->id, make_wire.Wire());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_make_face_from_wire(aicad_occt_context_t* context,
                                                    aicad_shape_handle_t wire_handle,
                                                    aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* wire_shape = nullptr;
  status = LookupTyped(context, wire_handle, TopAbs_WIRE, &wire_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    const TopoDS_Wire& wire = TopoDS::Wire(*wire_shape);
    BRepBuilderAPI_MakeFace make_face(wire, /*OnlyPlane=*/Standard_True);
    if (!make_face.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, make_face.Face());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_extrude(aicad_occt_context_t* context,
                                        aicad_shape_handle_t face_handle,
                                        const double direction[3],
                                        double distance,
                                        aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (direction == nullptr || out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  if (!(distance > 0.0) || !std::isfinite(distance)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  gp_Dir dir;
  if (!TryToDir(direction, &dir)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* face_shape = nullptr;
  status = LookupTyped(context, face_handle, TopAbs_FACE, &face_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    gp_Vec vec(dir);
    vec *= distance;
    BRepPrimAPI_MakePrism make_prism(*face_shape, vec);
    if (!make_prism.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, make_prism.Shape());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_revolve(aicad_occt_context_t* context,
                                        aicad_shape_handle_t face_handle,
                                        const double axis_origin[3],
                                        const double axis_direction[3],
                                        double angle_radians,
                                        aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (axis_origin == nullptr || axis_direction == nullptr || out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  if (!IsFinite3(axis_origin)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  constexpr double kTwoPi = 6.283185307179586476925286766559;
  if (!std::isfinite(angle_radians) || !(angle_radians > 0.0) || angle_radians > kTwoPi + 1e-9) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  gp_Dir dir;
  if (!TryToDir(axis_direction, &dir)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* face_shape = nullptr;
  status = LookupTyped(context, face_handle, TopAbs_FACE, &face_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    gp_Ax1 axis(ToPnt(axis_origin), dir);
    // Clamp to exactly 2*pi when within tolerance so OCCT treats it as a
    // full revolution rather than rejecting a value that overshoots
    // 2*pi by floating-point noise.
    const double angle = std::min(angle_radians, kTwoPi);
    BRepPrimAPI_MakeRevol make_revol(*face_shape, axis, angle, /*Copy=*/Standard_True);
    if (!make_revol.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, make_revol.Shape());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_sweep(aicad_occt_context_t* context,
                                      aicad_shape_handle_t profile_face_handle,
                                      aicad_shape_handle_t spine_wire_handle,
                                      aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* profile_shape = nullptr;
  status = LookupTyped(context, profile_face_handle, TopAbs_FACE, &profile_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  const TopoDS_Shape* spine_shape = nullptr;
  status = LookupTyped(context, spine_wire_handle, TopAbs_WIRE, &spine_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    const TopoDS_Wire& spine = TopoDS::Wire(*spine_shape);
    BRepOffsetAPI_MakePipe make_pipe(spine, *profile_shape);
    if (!make_pipe.IsDone()) {
      // The most common cause here is a spine that is not G1-continuous
      // (e.g. a polygonal path with sharp corners) -- see this function's
      // header doc comment and project/reports/AICAD-025.md.
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, make_pipe.Shape());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_loft(aicad_occt_context_t* context,
                                     const aicad_shape_handle_t* sections,
                                     size_t section_count,
                                     aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (sections == nullptr || out_handle == nullptr || section_count < 2) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  try {
    // isSolid=true (caps the first/last sections into a closed solid);
    // ruled=true (straight-line generatrix between consecutive sections
    // -- the minimal, analytically-predictable loft form; see this
    // function's header doc comment).
    BRepOffsetAPI_ThruSections thru_sections(/*isSolid=*/Standard_True, /*ruled=*/Standard_True);
    for (size_t i = 0; i < section_count; ++i) {
      const TopoDS_Shape* wire_shape = nullptr;
      status = LookupTyped(context, sections[i], TopAbs_WIRE, &wire_shape);
      if (status != AICAD_OCCT_OK) {
        return status;
      }
      thru_sections.AddWire(TopoDS::Wire(*wire_shape));
    }
    thru_sections.Build();
    if (!thru_sections.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, thru_sections.Shape());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

namespace {

// Shared lookup for both boolean operands: unlike LookupTyped, no
// topological-kind restriction is imposed (see this function's callers'
// header doc comment for why).
aicad_occt_status_t LookupBooleanOperand(aicad_occt_context_t* context,
                                          aicad_shape_handle_t handle,
                                          const TopoDS_Shape** out) {
  aicad_occt_status_t status = CheckHandleContext(context, handle);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  return context->shapes.Lookup(handle, out);
}

}  // namespace

aicad_occt_status_t aicad_occt_boolean_union(aicad_occt_context_t* context,
                                              aicad_shape_handle_t a,
                                              aicad_shape_handle_t b,
                                              aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape_a = nullptr;
  status = LookupBooleanOperand(context, a, &shape_a);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  const TopoDS_Shape* shape_b = nullptr;
  status = LookupBooleanOperand(context, b, &shape_b);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepAlgoAPI_Fuse fuse(*shape_a, *shape_b);
    if (!fuse.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, fuse.Shape());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_boolean_cut(aicad_occt_context_t* context,
                                            aicad_shape_handle_t a,
                                            aicad_shape_handle_t b,
                                            aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape_a = nullptr;
  status = LookupBooleanOperand(context, a, &shape_a);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  const TopoDS_Shape* shape_b = nullptr;
  status = LookupBooleanOperand(context, b, &shape_b);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepAlgoAPI_Cut cut(*shape_a, *shape_b);
    if (!cut.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, cut.Shape());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_boolean_intersect(aicad_occt_context_t* context,
                                                  aicad_shape_handle_t a,
                                                  aicad_shape_handle_t b,
                                                  aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape_a = nullptr;
  status = LookupBooleanOperand(context, a, &shape_a);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  const TopoDS_Shape* shape_b = nullptr;
  status = LookupBooleanOperand(context, b, &shape_b);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepAlgoAPI_Common common(*shape_a, *shape_b);
    if (!common.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, common.Shape());
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
