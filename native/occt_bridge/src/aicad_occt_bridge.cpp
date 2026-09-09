// AICAD-016: native/occt_bridge C ABI boundary implementation.
//
// This is the only translation unit permitted to include OCCT headers
// and the only place OCCT types (TopoDS_Shape, Standard_Failure, ...)
// exist in this bridge's own state -- they never appear in
// aicad_occt_bridge.h and never cross the extern "C" functions below.

#include "aicad_occt_bridge.h"

#include <BRepAlgoAPI_Common.hxx>
#include <BRepAlgoAPI_Cut.hxx>
#include <BRepAlgoAPI_Fuse.hxx>
#include <BRepBndLib.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepFilletAPI_MakeChamfer.hxx>
#include <BRepFilletAPI_MakeFillet.hxx>
#include <BRepGProp.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRepOffsetAPI_MakeOffsetShape.hxx>
#include <BRepOffsetAPI_MakePipe.hxx>
#include <BRepOffsetAPI_MakeThickSolid.hxx>
#include <BRepOffsetAPI_ThruSections.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <BRepPrimAPI_MakeRevol.hxx>
#include <BRep_Tool.hxx>
#include <Bnd_Box.hxx>
#include <GProp_GProps.hxx>
#include <Poly_Triangulation.hxx>
#include <Standard_Failure.hxx>
#include <TopAbs_ShapeEnum.hxx>
#include <TopExp.hxx>
#include <TopLoc_Location.hxx>
#include <TopTools_IndexedDataMapOfShapeListOfShape.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <TopTools_ListOfShape.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Edge.hxx>
#include <TopoDS_Face.hxx>
#include <TopoDS_Shape.hxx>
#include <TopoDS_Vertex.hxx>
#include <TopoDS_Wire.hxx>
#include <gp_Ax1.hxx>
#include <gp_Ax2.hxx>
#include <gp_Circ.hxx>
#include <gp_Dir.hxx>
#include <gp_Pnt.hxx>
#include <gp_Trsf.hxx>
#include <gp_Vec.hxx>

#include <algorithm>
#include <atomic>
#include <cmath>
#include <exception>
#include <thread>
#include <vector>

namespace {

// AICAD-032: a cached flat-shaded triangle-soup tessellation for one
// shape slot. `vertices`/`normals` are each `9 * triangle_count` doubles
// (3 vertices per triangle * 3 coordinates, not shared across triangles;
// `normals` holds each triangle's one flat normal duplicated across its
// 3 vertices). `present` distinguishes "never tessellated" from "an
// empty (zero-triangle) tessellation was cached".
struct TessellationCache {
  std::vector<double> vertices;
  std::vector<double> normals;
  size_t triangle_count = 0;
  bool present = false;
};

// One slot in a context's shape table. `generation` is bumped every time
// the slot is released, so a handle minted before release never matches
// a shape later inserted into the same slot (Stage-1 kernel policy #5).
struct ShapeSlot {
  TopoDS_Shape shape;
  uint32_t generation = 0;
  bool occupied = false;
  TessellationCache tessellation;
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
    // A new occupant never inherits a stale tessellation cache left by
    // whatever previously occupied this slot (Stage-1 kernel policy #10).
    slot.tessellation = TessellationCache();
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
    slot.tessellation = TessellationCache();
    slot.occupied = false;
    slot.generation += 1;
    free_slots_.push_back(handle.slot);
    return AICAD_OCCT_OK;
  }

  // AICAD-032: stores a freshly computed tessellation for `handle`'s own
  // slot (overwriting any previous cache for it).
  aicad_occt_status_t SetTessellation(aicad_shape_handle_t handle, TessellationCache cache) {
    if (handle.slot >= slots_.size()) {
      return AICAD_OCCT_ERR_INVALID_HANDLE;
    }
    ShapeSlot& slot = slots_[handle.slot];
    if (!slot.occupied || slot.generation != handle.generation) {
      return AICAD_OCCT_ERR_STALE_HANDLE;
    }
    slot.tessellation = std::move(cache);
    return AICAD_OCCT_OK;
  }

  // AICAD-032: retrieves `handle`'s own cached tessellation, if any.
  // AICAD_OCCT_ERR_INVALID_ARGUMENT (not a handle-validity error) means
  // the handle is valid but no tessellation is cached for it yet.
  aicad_occt_status_t GetTessellation(aicad_shape_handle_t handle, const TessellationCache** out) const {
    if (handle.slot >= slots_.size()) {
      return AICAD_OCCT_ERR_INVALID_HANDLE;
    }
    const ShapeSlot& slot = slots_[handle.slot];
    if (!slot.occupied || slot.generation != handle.generation) {
      return AICAD_OCCT_ERR_STALE_HANDLE;
    }
    if (!slot.tessellation.present) {
      return AICAD_OCCT_ERR_INVALID_ARGUMENT;
    }
    *out = &slot.tessellation;
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

// Shared lookup for operations that accept any topological shape kind
// (boolean operands, and fillet/chamfer's target shape): unlike
// LookupTyped, no kind restriction is imposed (see each caller's own
// header doc comment for why).
aicad_occt_status_t LookupAnyKind(aicad_occt_context_t* context,
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
  status = LookupAnyKind(context, a, &shape_a);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  const TopoDS_Shape* shape_b = nullptr;
  status = LookupAnyKind(context, b, &shape_b);
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
  status = LookupAnyKind(context, a, &shape_a);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  const TopoDS_Shape* shape_b = nullptr;
  status = LookupAnyKind(context, b, &shape_b);
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
  status = LookupAnyKind(context, a, &shape_a);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  const TopoDS_Shape* shape_b = nullptr;
  status = LookupAnyKind(context, b, &shape_b);
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

aicad_occt_status_t aicad_occt_shape_edge_count(aicad_occt_context_t* context,
                                                 aicad_shape_handle_t handle,
                                                 size_t* out_count) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_count == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = LookupAnyKind(context, handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    // TopExp::MapShapes de-duplicates: a plain TopExp_Explorer over
    // TopAbs_EDGE revisits each edge once per adjacent face (verified
    // empirically -- 24 visits for a box's 12 actual edges), which is
    // not the "unique edges" count this function promises.
    TopTools_IndexedMapOfShape edges;
    TopExp::MapShapes(*shape, TopAbs_EDGE, edges);
    *out_count = static_cast<size_t>(edges.Extent());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_get_edge(aicad_occt_context_t* context,
                                               aicad_shape_handle_t handle,
                                               size_t index,
                                               aicad_shape_handle_t* out_edge_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_edge_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = LookupAnyKind(context, handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    TopTools_IndexedMapOfShape edges;
    TopExp::MapShapes(*shape, TopAbs_EDGE, edges);
    // TopTools_IndexedMapOfShape is 1-indexed; the ABI's `index` is
    // 0-based (matching aicad_occt_make_wire_from_edges' own convention).
    if (index >= static_cast<size_t>(edges.Extent())) {
      return AICAD_OCCT_ERR_INVALID_ARGUMENT;
    }
    const TopoDS_Shape& edge = edges.FindKey(static_cast<Standard_Integer>(index) + 1);
    *out_edge_handle = context->shapes.Insert(context->id, edge);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_fillet(aicad_occt_context_t* context,
                                       aicad_shape_handle_t shape_handle,
                                       const aicad_shape_handle_t* edges,
                                       size_t edge_count,
                                       double radius,
                                       aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (edges == nullptr || out_handle == nullptr || edge_count == 0) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  if (!(radius > 0.0) || !std::isfinite(radius)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* base_shape = nullptr;
  status = LookupAnyKind(context, shape_handle, &base_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepFilletAPI_MakeFillet make_fillet(*base_shape);
    for (size_t i = 0; i < edge_count; ++i) {
      const TopoDS_Shape* edge_shape = nullptr;
      status = LookupTyped(context, edges[i], TopAbs_EDGE, &edge_shape);
      if (status != AICAD_OCCT_OK) {
        return status;
      }
      make_fillet.Add(radius, TopoDS::Edge(*edge_shape));
    }
    make_fillet.Build();
    if (!make_fillet.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, make_fillet.Shape());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_chamfer(aicad_occt_context_t* context,
                                        aicad_shape_handle_t shape_handle,
                                        const aicad_shape_handle_t* edges,
                                        size_t edge_count,
                                        double distance,
                                        aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (edges == nullptr || out_handle == nullptr || edge_count == 0) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  if (!(distance > 0.0) || !std::isfinite(distance)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* base_shape = nullptr;
  status = LookupAnyKind(context, shape_handle, &base_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepFilletAPI_MakeChamfer make_chamfer(*base_shape);
    for (size_t i = 0; i < edge_count; ++i) {
      const TopoDS_Shape* edge_shape = nullptr;
      status = LookupTyped(context, edges[i], TopAbs_EDGE, &edge_shape);
      if (status != AICAD_OCCT_OK) {
        return status;
      }
      make_chamfer.Add(distance, TopoDS::Edge(*edge_shape));
    }
    make_chamfer.Build();
    if (!make_chamfer.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, make_chamfer.Shape());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_face_count(aicad_occt_context_t* context,
                                                 aicad_shape_handle_t handle,
                                                 size_t* out_count) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_count == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = LookupAnyKind(context, handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    TopTools_IndexedMapOfShape faces;
    TopExp::MapShapes(*shape, TopAbs_FACE, faces);
    *out_count = static_cast<size_t>(faces.Extent());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_get_face(aicad_occt_context_t* context,
                                               aicad_shape_handle_t handle,
                                               size_t index,
                                               aicad_shape_handle_t* out_face_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_face_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = LookupAnyKind(context, handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    TopTools_IndexedMapOfShape faces;
    TopExp::MapShapes(*shape, TopAbs_FACE, faces);
    // TopTools_IndexedMapOfShape is 1-indexed; the ABI's `index` is
    // 0-based (matching aicad_occt_shape_get_edge's own convention).
    if (index >= static_cast<size_t>(faces.Extent())) {
      return AICAD_OCCT_ERR_INVALID_ARGUMENT;
    }
    const TopoDS_Shape& face = faces.FindKey(static_cast<Standard_Integer>(index) + 1);
    *out_face_handle = context->shapes.Insert(context->id, face);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shell(aicad_occt_context_t* context,
                                      aicad_shape_handle_t shape_handle,
                                      const aicad_shape_handle_t* faces_to_remove,
                                      size_t face_count,
                                      double thickness,
                                      aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (faces_to_remove == nullptr || out_handle == nullptr || face_count == 0) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  if (thickness == 0.0 || !std::isfinite(thickness)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* base_shape = nullptr;
  status = LookupAnyKind(context, shape_handle, &base_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    TopTools_ListOfShape closing_faces;
    for (size_t i = 0; i < face_count; ++i) {
      const TopoDS_Shape* face_shape = nullptr;
      status = LookupTyped(context, faces_to_remove[i], TopAbs_FACE, &face_shape);
      if (status != AICAD_OCCT_OK) {
        return status;
      }
      closing_faces.Append(*face_shape);
    }
    BRepOffsetAPI_MakeThickSolid make_thick;
    make_thick.MakeThickSolidByJoin(*base_shape, closing_faces, thickness, 1e-6);
    if (!make_thick.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, make_thick.Shape());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_offset(aicad_occt_context_t* context,
                                       aicad_shape_handle_t shape_handle,
                                       double distance,
                                       aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  if (distance == 0.0 || !std::isfinite(distance)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* base_shape = nullptr;
  status = LookupAnyKind(context, shape_handle, &base_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepOffsetAPI_MakeOffsetShape make_offset;
    make_offset.PerformByJoin(*base_shape, distance, 1e-6);
    if (!make_offset.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, make_offset.Shape());
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

// --- AICAD-029: topology exploration helpers ---

namespace {

// Rebuilds `shape_handle`'s unique-edge map exactly as
// aicad_occt_shape_edge_count/_get_edge do, and returns the specific edge
// at `edge_index` (0-based) -- the shared basis both
// aicad_occt_shape_edge_adjacent_face_count and _get build on, so their
// own `edge_index` contract stays anchored to shape_edge_count's own
// enumeration order.
aicad_occt_status_t LookupIndexedEdge(const TopoDS_Shape& shape, size_t edge_index, TopoDS_Edge* out_edge) {
  TopTools_IndexedMapOfShape edges;
  TopExp::MapShapes(shape, TopAbs_EDGE, edges);
  if (edge_index >= static_cast<size_t>(edges.Extent())) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  *out_edge = TopoDS::Edge(edges.FindKey(static_cast<Standard_Integer>(edge_index) + 1));
  return AICAD_OCCT_OK;
}

}  // namespace

extern "C" {

aicad_occt_status_t aicad_occt_shape_vertex_count(aicad_occt_context_t* context,
                                                    aicad_shape_handle_t handle,
                                                    size_t* out_count) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_count == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = LookupAnyKind(context, handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    // TopExp::MapShapes de-duplicates, matching
    // aicad_occt_shape_edge_count/_face_count's own rationale: a shared
    // vertex is visited once per incident edge by a raw explorer.
    TopTools_IndexedMapOfShape vertices;
    TopExp::MapShapes(*shape, TopAbs_VERTEX, vertices);
    *out_count = static_cast<size_t>(vertices.Extent());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_get_vertex(aicad_occt_context_t* context,
                                                 aicad_shape_handle_t handle,
                                                 size_t index,
                                                 aicad_shape_handle_t* out_vertex_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_vertex_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = LookupAnyKind(context, handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    TopTools_IndexedMapOfShape vertices;
    TopExp::MapShapes(*shape, TopAbs_VERTEX, vertices);
    // 0-based (matching aicad_occt_shape_get_edge/_get_face's own
    // convention); TopTools_IndexedMapOfShape itself is 1-indexed.
    if (index >= static_cast<size_t>(vertices.Extent())) {
      return AICAD_OCCT_ERR_INVALID_ARGUMENT;
    }
    const TopoDS_Shape& vertex = vertices.FindKey(static_cast<Standard_Integer>(index) + 1);
    *out_vertex_handle = context->shapes.Insert(context->id, vertex);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_edge_vertices(aicad_occt_context_t* context,
                                              aicad_shape_handle_t edge_handle,
                                              aicad_shape_handle_t* out_v0,
                                              aicad_shape_handle_t* out_v1) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_v0 == nullptr || out_v1 == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* edge_shape = nullptr;
  status = LookupTyped(context, edge_handle, TopAbs_EDGE, &edge_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    TopoDS_Vertex v0, v1;
    // TopExp::Vertices' 3-argument form (no CumOri) returns the edge's
    // vertices in its own orientation sense: v0 = "first", v1 = "last".
    // For a closed edge (e.g. a full circle) both are the same vertex --
    // documented in the header, not treated as an error here.
    TopExp::Vertices(TopoDS::Edge(*edge_shape), v0, v1);
    if (v0.IsNull() || v1.IsNull()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_v0 = context->shapes.Insert(context->id, v0);
    *out_v1 = context->shapes.Insert(context->id, v1);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_edge_adjacent_face_count(aicad_occt_context_t* context,
                                                                aicad_shape_handle_t shape_handle,
                                                                size_t edge_index,
                                                                size_t* out_count) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_count == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = LookupAnyKind(context, shape_handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    TopoDS_Edge edge;
    status = LookupIndexedEdge(*shape, edge_index, &edge);
    if (status != AICAD_OCCT_OK) {
      return status;
    }
    TopTools_IndexedDataMapOfShapeListOfShape edge_face_map;
    TopExp::MapShapesAndAncestors(*shape, TopAbs_EDGE, TopAbs_FACE, edge_face_map);
    if (!edge_face_map.Contains(edge)) {
      // Can happen if `shape` itself is a bare Edge/Wire with no
      // containing Face at all (e.g. a standalone spine wire) --
      // zero adjacent faces, not an error.
      *out_count = 0;
      return AICAD_OCCT_OK;
    }
    *out_count = static_cast<size_t>(edge_face_map.FindFromKey(edge).Extent());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_edge_adjacent_face_get(aicad_occt_context_t* context,
                                                              aicad_shape_handle_t shape_handle,
                                                              size_t edge_index,
                                                              size_t adjacent_index,
                                                              aicad_shape_handle_t* out_face_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_face_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = LookupAnyKind(context, shape_handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    TopoDS_Edge edge;
    status = LookupIndexedEdge(*shape, edge_index, &edge);
    if (status != AICAD_OCCT_OK) {
      return status;
    }
    TopTools_IndexedDataMapOfShapeListOfShape edge_face_map;
    TopExp::MapShapesAndAncestors(*shape, TopAbs_EDGE, TopAbs_FACE, edge_face_map);
    if (!edge_face_map.Contains(edge)) {
      return AICAD_OCCT_ERR_INVALID_ARGUMENT;
    }
    const TopTools_ListOfShape& faces = edge_face_map.FindFromKey(edge);
    if (adjacent_index >= static_cast<size_t>(faces.Extent())) {
      return AICAD_OCCT_ERR_INVALID_ARGUMENT;
    }
    TopTools_ListIteratorOfListOfShape it(faces);
    for (size_t i = 0; i < adjacent_index; ++i) {
      it.Next();
    }
    *out_face_handle = context->shapes.Insert(context->id, it.Value());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_length(aicad_occt_context_t* context,
                                             aicad_shape_handle_t handle,
                                             double* out_length) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  status = CheckHandleContext(context, handle);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_length == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = context->shapes.Lookup(handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    GProp_GProps props;
    // SkipShared=true: without it, BRepGProp::LinearProperties counts an
    // edge once per adjacent face (empirically verified -- a box reports
    // 72, exactly double the true 4*(dx+dy+dz)=36 unique-edge total),
    // the same double-counting aicad_occt_shape_edge_count's own doc
    // comment already identified for a raw TopExp_Explorer traversal.
    // SkipShared=true matches this bridge's established "unique edges"
    // semantics.
    BRepGProp::LinearProperties(*shape, props, Standard_True);
    *out_length = props.Mass();
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_center_of_mass(aicad_occt_context_t* context,
                                                      aicad_shape_handle_t handle,
                                                      double out_center[3]) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  status = CheckHandleContext(context, handle);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_center == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = context->shapes.Lookup(handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    // Dispatch on the shape's own highest-dimensional content: volume >
    // area > length, matching physical "center of mass" intuition (a
    // solid's mass comes from its volume, not incidentally from its
    // boundary faces' area). Empirically, BRepGProp::VolumeProperties on
    // a shape with no Solid returns a zero-mass, origin-centred result
    // rather than failing -- that would silently produce a meaningless
    // (0,0,0) centroid for e.g. a bare face, so the dispatch is explicit
    // rather than relying on VolumeProperties' own fallback behavior.
    TopTools_IndexedMapOfShape solids;
    TopExp::MapShapes(*shape, TopAbs_SOLID, solids);
    GProp_GProps props;
    if (solids.Extent() > 0) {
      BRepGProp::VolumeProperties(*shape, props);
    } else {
      TopTools_IndexedMapOfShape faces;
      TopExp::MapShapes(*shape, TopAbs_FACE, faces);
      if (faces.Extent() > 0) {
        BRepGProp::SurfaceProperties(*shape, props);
      } else {
        TopTools_IndexedMapOfShape edges;
        TopExp::MapShapes(*shape, TopAbs_EDGE, edges);
        if (edges.Extent() > 0) {
          BRepGProp::LinearProperties(*shape, props, Standard_True);
        } else {
          return AICAD_OCCT_ERR_OPERATION_FAILED;
        }
      }
    }
    const gp_Pnt centre = props.CentreOfMass();
    out_center[0] = centre.X();
    out_center[1] = centre.Y();
    out_center[2] = centre.Z();
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

}  // extern "C"

namespace {

// Counts how many of `subshapes`' unique elements `analyzer` reports as
// invalid.
size_t CountInvalid(const BRepCheck_Analyzer& analyzer, const TopTools_IndexedMapOfShape& subshapes) {
  size_t invalid = 0;
  for (Standard_Integer i = 1; i <= subshapes.Extent(); ++i) {
    if (!analyzer.IsValid(subshapes.FindKey(i))) {
      invalid += 1;
    }
  }
  return invalid;
}

}  // namespace

extern "C" {

aicad_occt_status_t aicad_occt_shape_validate(aicad_occt_context_t* context,
                                               aicad_shape_handle_t handle,
                                               aicad_validation_report_t* out_report) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  status = CheckHandleContext(context, handle);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_report == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = context->shapes.Lookup(handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepCheck_Analyzer analyzer(*shape);
    TopTools_IndexedMapOfShape vertices, edges, wires, faces;
    TopExp::MapShapes(*shape, TopAbs_VERTEX, vertices);
    TopExp::MapShapes(*shape, TopAbs_EDGE, edges);
    TopExp::MapShapes(*shape, TopAbs_WIRE, wires);
    TopExp::MapShapes(*shape, TopAbs_FACE, faces);
    out_report->is_valid = analyzer.IsValid() ? 1 : 0;
    out_report->invalid_vertex_count = CountInvalid(analyzer, vertices);
    out_report->invalid_edge_count = CountInvalid(analyzer, edges);
    out_report->invalid_wire_count = CountInvalid(analyzer, wires);
    out_report->invalid_face_count = CountInvalid(analyzer, faces);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

}  // extern "C"

namespace {

// AICAD-032: builds a flat-shaded triangle-soup TessellationCache for
// `shape` at the given deflections. Returns false (leaving `out`
// untouched) if BRepMesh_IncrementalMesh itself did not complete.
bool ExtractTessellation(const TopoDS_Shape& shape,
                          double linear_deflection,
                          double angular_deflection,
                          TessellationCache* out) {
  BRepMesh_IncrementalMesh mesher(shape, linear_deflection, Standard_False, angular_deflection,
                                   Standard_False);
  if (!mesher.IsDone()) {
    return false;
  }

  std::vector<double> vertices;
  std::vector<double> normals;
  size_t triangle_count = 0;

  TopTools_IndexedMapOfShape faces;
  TopExp::MapShapes(shape, TopAbs_FACE, faces);
  for (Standard_Integer fi = 1; fi <= faces.Extent(); ++fi) {
    const TopoDS_Face& face = TopoDS::Face(faces.FindKey(fi));
    TopLoc_Location location;
    const Handle(Poly_Triangulation)& triangulation = BRep_Tool::Triangulation(face, location);
    if (triangulation.IsNull()) {
      // BRepMesh_IncrementalMesh reported IsDone() overall but this
      // particular face still has no triangulation -- contribute no
      // triangles from it rather than failing the whole operation.
      continue;
    }
    const gp_Trsf& trsf = location.Transformation();
    const bool reversed = (face.Orientation() == TopAbs_REVERSED);
    for (Standard_Integer ti = 1; ti <= triangulation->NbTriangles(); ++ti) {
      Standard_Integer n1, n2, n3;
      triangulation->Triangle(ti).Get(n1, n2, n3);
      if (reversed) {
        // A REVERSED face's triangle node order is defined relative to
        // its underlying surface's natural (non-reversed) parametrization
        // -- swapping two nodes flips the winding to match the face's
        // own actual (outward) orientation.
        std::swap(n2, n3);
      }
      const gp_Pnt p1 = triangulation->Node(n1).Transformed(trsf);
      const gp_Pnt p2 = triangulation->Node(n2).Transformed(trsf);
      const gp_Pnt p3 = triangulation->Node(n3).Transformed(trsf);
      const gp_Vec edge1(p1, p2);
      const gp_Vec edge2(p1, p3);
      const gp_Vec raw_normal = edge1.Crossed(edge2);
      const double normal_length = raw_normal.Magnitude();
      double nx = 0.0, ny = 0.0, nz = 0.0;
      if (normal_length > 1e-12) {
        nx = raw_normal.X() / normal_length;
        ny = raw_normal.Y() / normal_length;
        nz = raw_normal.Z() / normal_length;
      }
      const gp_Pnt* corners[3] = {&p1, &p2, &p3};
      for (const gp_Pnt* corner : corners) {
        vertices.push_back(corner->X());
        vertices.push_back(corner->Y());
        vertices.push_back(corner->Z());
        normals.push_back(nx);
        normals.push_back(ny);
        normals.push_back(nz);
      }
      triangle_count += 1;
    }
  }

  out->vertices = std::move(vertices);
  out->normals = std::move(normals);
  out->triangle_count = triangle_count;
  out->present = true;
  return true;
}

}  // namespace

extern "C" {

aicad_occt_status_t aicad_occt_tessellate(aicad_occt_context_t* context,
                                           aicad_shape_handle_t handle,
                                           double linear_deflection,
                                           double angular_deflection,
                                           aicad_tessellation_counts_t* out_counts) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  status = CheckHandleContext(context, handle);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_counts == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  if (!(linear_deflection > 0.0) || !std::isfinite(linear_deflection) ||
      !(angular_deflection > 0.0) || !std::isfinite(angular_deflection)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = context->shapes.Lookup(handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    TessellationCache cache;
    if (!ExtractTessellation(*shape, linear_deflection, angular_deflection, &cache)) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    out_counts->triangle_count = cache.triangle_count;
    status = context->shapes.SetTessellation(handle, std::move(cache));
    if (status != AICAD_OCCT_OK) {
      return status;
    }
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_tessellation_get(aicad_occt_context_t* context,
                                                 aicad_shape_handle_t handle,
                                                 double* out_vertices,
                                                 double* out_normals) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  status = CheckHandleContext(context, handle);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_vertices == nullptr || out_normals == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TessellationCache* cache = nullptr;
  status = context->shapes.GetTessellation(handle, &cache);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    std::copy(cache->vertices.begin(), cache->vertices.end(), out_vertices);
    std::copy(cache->normals.begin(), cache->normals.end(), out_normals);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

}  // extern "C"
