// AICAD-016: native/occt_bridge C ABI boundary implementation.
//
// This is the only translation unit permitted to include OCCT headers
// and the only place OCCT types (TopoDS_Shape, Standard_Failure, ...)
// exist in this bridge's own state -- they never appear in
// aicad_occt_bridge.h and never cross the extern "C" functions below.

#include "aicad_occt_bridge.h"

#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepAlgoAPI_BooleanOperation.hxx>
#include <BRepAlgoAPI_Common.hxx>
#include <BRepClass3d_SolidClassifier.hxx>
#include <BRepAlgoAPI_Cut.hxx>
#include <BRepAlgoAPI_Fuse.hxx>
#include <BRepBndLib.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakeSolid.hxx>
#include <BRepBuilderAPI_MakeVertex.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepBuilderAPI_Sewing.hxx>
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
#include <BRepTools.hxx>
#include <BRepTools_ReShape.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <Bnd_Box.hxx>
#include <GC_MakeArcOfCircle.hxx>
#include <GProp_GProps.hxx>
#include <GeomAbs_CurveType.hxx>
#include <GeomAbs_SurfaceType.hxx>
#include <Geom_Curve.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <Poly_Triangulation.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_StepModelType.hxx>
#include <STEPControl_Writer.hxx>
#include <ShapeFix_Shape.hxx>
#include <ShapeUpgrade_UnifySameDomain.hxx>
#include <Standard_Failure.hxx>
#include <TopAbs_ShapeEnum.hxx>
#include <TopExp.hxx>
#include <TopLoc_Location.hxx>
#include <TopTools_IndexedDataMapOfShapeListOfShape.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <TopTools_ListOfShape.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Compound.hxx>
#include <TopoDS_Edge.hxx>
#include <TopoDS_Face.hxx>
#include <TopoDS_Shape.hxx>
#include <TopoDS_Shell.hxx>
#include <TopoDS_Solid.hxx>
#include <TopoDS_Vertex.hxx>
#include <TopoDS_Wire.hxx>
#include <gp_Ax1.hxx>
#include <gp_Ax2.hxx>
#include <gp_Ax3.hxx>
#include <gp_Circ.hxx>
#include <gp_Cone.hxx>
#include <gp_Cylinder.hxx>
#include <gp_Dir.hxx>
#include <gp_Pln.hxx>
#include <gp_Pnt.hxx>
#include <gp_Pnt2d.hxx>
#include <gp_Sphere.hxx>
#include <gp_Torus.hxx>
#include <gp_Trsf.hxx>
#include <gp_Vec.hxx>

#include <algorithm>
#include <atomic>
#include <cmath>
#include <deque>
#include <exception>
#include <mutex>
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

// AICAD-086: one operation's own captured Generated/Modified/IsDeleted
// lineage (docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md §8) for a
// single input face/edge. OCCT's own Generated/Modified/IsDeleted
// answers are only defined while the builder object that performed the
// operation is still alive (its own internal history is destroyed with
// it) -- this entry is therefore a caller-owned snapshot, captured once
// at operation time (see CaptureLineage below), never re-queried from a
// builder later.
struct LineageEntry {
  TopoDS_Shape input;
  bool deleted = false;
  std::vector<TopoDS_Shape> generated;
  std::vector<TopoDS_Shape> modified;
};

// Mirrors ShapeSlot/ShapeTable's own slot/generation/free-list design
// exactly (see ShapeTable's doc comment) for a second, independently
// handled table, so a lineage handle can never be confused with or
// accidentally satisfy a shape-handle lookup.
struct LineageSlot {
  std::vector<LineageEntry> entries;
  uint32_t generation = 0;
  bool occupied = false;
};

class LineageTable {
 public:
  aicad_lineage_handle_t Insert(uint64_t context_id, std::vector<LineageEntry> entries) {
    uint32_t slot_index;
    if (!free_slots_.empty()) {
      slot_index = free_slots_.back();
      free_slots_.pop_back();
    } else {
      slot_index = static_cast<uint32_t>(slots_.size());
      slots_.emplace_back();
    }
    LineageSlot& slot = slots_[slot_index];
    slot.entries = std::move(entries);
    slot.occupied = true;
    if (slot.generation == 0) {
      slot.generation = 1;
    }
    return aicad_lineage_handle_t{context_id, slot_index, slot.generation};
  }

  aicad_occt_status_t Lookup(aicad_lineage_handle_t handle,
                              const std::vector<LineageEntry>** out) const {
    if (handle.slot >= slots_.size()) {
      return AICAD_OCCT_ERR_INVALID_HANDLE;
    }
    const LineageSlot& slot = slots_[handle.slot];
    if (!slot.occupied || slot.generation != handle.generation) {
      return AICAD_OCCT_ERR_STALE_HANDLE;
    }
    *out = &slot.entries;
    return AICAD_OCCT_OK;
  }

  aicad_occt_status_t Release(aicad_lineage_handle_t handle) {
    if (handle.slot >= slots_.size()) {
      return AICAD_OCCT_ERR_INVALID_HANDLE;
    }
    LineageSlot& slot = slots_[handle.slot];
    if (!slot.occupied || slot.generation != handle.generation) {
      return AICAD_OCCT_ERR_STALE_HANDLE;
    }
    slot.entries.clear();
    slot.occupied = false;
    slot.generation += 1;
    free_slots_.push_back(handle.slot);
    return AICAD_OCCT_OK;
  }

 private:
  std::vector<LineageSlot> slots_;
  std::vector<uint32_t> free_slots_;
};

uint64_t NextContextId() {
  static std::atomic<uint64_t> counter{1};  // 0 is reserved (never valid).
  return counter.fetch_add(1, std::memory_order_relaxed);
}

}  // namespace

// Owner-authorized context lifetime-safety fix (see
// project/reports/reviews/STAGE1-INDEPENDENT-REVIEW.md, "Context
// lifetime safety fix" section, for the full alternatives comparison and
// rationale this design was chosen from).
//
// `aicad_occt_context`'s own memory is now NEVER individually freed --
// every context ever created lives inside `ContextRegistry`'s
// process-wide `std::deque` for the remainder of the process
// (`std::deque::emplace_back` never relocates or invalidates the
// address of an already-constructed element, unlike `std::vector`,
// which is exactly the stable-address property this fix needs). This
// makes dereferencing a destroyed context pointer to read `live` a
// well-defined, standard-legal memory access -- not the use-after-free
// a `delete`-based design produces -- for the in-scope defect class: a
// pointer value this bridge itself once handed out, used again after
// its own `aicad_occt_context_destroy` call. It does not, and no design
// built on an opaque raw-pointer C ABI can, make a wholly fabricated/
// foreign pointer value safe to dereference (the C standard library's
// own `FILE*` has the identical, universally-accepted limitation for
// e.g. `fclose`); this bridge already had, and still has, that same
// inherent property for garbage pointers -- unchanged by this fix.
//
// `live` is the sole safety-critical field: set exactly once (true, at
// create, before the pointer is ever handed to a caller -- no concurrent
// reader can exist yet) and cleared exactly once (false, by whichever
// caller's `aicad_occt_context_destroy` wins an atomic compare-exchange
// race), using acquire/release ordering. This lets every ordinary bridge
// call check liveness with a single lock-free atomic load in
// `CheckContext` -- no process-wide mutex on the per-call hot path (the
// registry's own mutex is taken only inside `Allocate`, i.e. only at
// context-creation time, not on every geometry call).
struct aicad_occt_context {
  std::atomic<bool> live{false};
  uint64_t id = 0;
  std::thread::id owning_thread{};
  ShapeTable shapes;
  // AICAD-086: independent from `shapes` -- see LineageTable's own doc
  // comment.
  LineageTable lineages;
};

namespace {

class ContextRegistry {
 public:
  aicad_occt_context* Allocate() {
    std::lock_guard<std::mutex> lock(mutex_);
    records_.emplace_back();
    return &records_.back();
  }

 private:
  std::mutex mutex_;
  std::deque<aicad_occt_context> records_;
};

ContextRegistry& GlobalContextRegistry() {
  static ContextRegistry registry;
  return registry;
}

aicad_occt_status_t CheckContext(aicad_occt_context_t* context) {
  if (context == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  // Safe even if `context` was returned by a PRIOR
  // `aicad_occt_context_create` call and has since been destroyed: its
  // memory is never freed (see the type's own doc comment above), so
  // this load is well-defined and simply observes `false`.
  if (!context->live.load(std::memory_order_acquire)) {
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

// AICAD-086: captures Generated/Modified/IsDeleted lineage for every
// unique face and edge of each shape in `inputs`, against `builder` --
// must run while `builder` is still alive (see LineageEntry's own doc
// comment). Only faces/edges are captured (matching AICAD-083's own
// `adjacent_to`/`boundary` scope, and `generated_by`/`modified_by`'s own
// plan-doc §6 examples, which only ever name faces/edges) -- extending
// this to vertices/shells/solids is additional scope no current task
// needs.
template <class Builder>
std::vector<LineageEntry> CaptureLineage(Builder& builder,
                                          std::initializer_list<const TopoDS_Shape*> inputs) {
  std::vector<LineageEntry> entries;
  for (const TopoDS_Shape* input : inputs) {
    for (TopAbs_ShapeEnum kind : {TopAbs_FACE, TopAbs_EDGE}) {
      TopTools_IndexedMapOfShape subshapes;
      TopExp::MapShapes(*input, kind, subshapes);
      for (Standard_Integer i = 1; i <= subshapes.Extent(); ++i) {
        LineageEntry entry;
        entry.input = subshapes.FindKey(i);
        entry.deleted = builder.IsDeleted(entry.input) != Standard_False;
        if (!entry.deleted) {
          TopTools_ListIteratorOfListOfShape git(builder.Generated(entry.input));
          for (; git.More(); git.Next()) {
            entry.generated.push_back(git.Value());
          }
          TopTools_ListIteratorOfListOfShape mit(builder.Modified(entry.input));
          for (; mit.More(); mit.Next()) {
            entry.modified.push_back(mit.Value());
          }
        }
        entries.push_back(std::move(entry));
      }
    }
  }
  return entries;
}

// Finds the captured LineageEntry (if any) whose own `input` is the same
// topological entity (TopoDS_Shape::IsSame -- TShape + Location, ignoring
// Orientation, matching aicad_occt_shape_is_same's own convention) as
// `shape`. `entries` was captured against the operation's own original
// input shape(s); a caller-supplied handle obtained via
// aicad_occt_shape_get_face/_get_edge against one of those same inputs
// resolves here even though it is a structurally different handle/slot.
const LineageEntry* FindLineageEntry(const std::vector<LineageEntry>& entries,
                                      const TopoDS_Shape& shape) {
  for (const LineageEntry& entry : entries) {
    if (entry.input.IsSame(shape)) {
      return &entry;
    }
  }
  return nullptr;
}

// AICAD-086: runs a Boolean operation (FUSE/CUT/COMMON, matching
// aicad_occt_boolean_union/_cut/_intersect exactly -- this is the same
// generic `BRepAlgoAPI_BooleanOperation` the dedicated `BRepAlgoAPI_Fuse`/
// `_Cut`/`_Common` classes each wrap with one operation preset, so this
// produces an identical result) while additionally capturing lineage.
// Unlike aicad_occt_boolean_union/_cut/_intersect's own convenience
// two-shape constructor (which builds immediately, before any option can
// be set), this uses the explicit SetArguments/SetTools/Build sequence so
// SetToFillHistory(Standard_True) can be set before Build() runs --
// required for Generated/Modified to answer anything at all, since this
// bridge cannot assume a particular OCCT version's own default.
aicad_occt_status_t BooleanWithLineage(aicad_occt_context_t* context,
                                        const TopoDS_Shape& shape_a,
                                        const TopoDS_Shape& shape_b,
                                        BOPAlgo_Operation operation,
                                        aicad_shape_handle_t* out_handle,
                                        aicad_lineage_handle_t* out_lineage) {
  try {
    BRepAlgoAPI_BooleanOperation op;
    TopTools_ListOfShape args;
    args.Append(shape_a);
    op.SetArguments(args);
    TopTools_ListOfShape tools;
    tools.Append(shape_b);
    op.SetTools(tools);
    op.SetOperation(operation);
    op.SetToFillHistory(Standard_True);
    op.Build();
    if (!op.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    std::vector<LineageEntry> entries = CaptureLineage(op, {&shape_a, &shape_b});
    *out_handle = context->shapes.Insert(context->id, op.Shape());
    *out_lineage = context->lineages.Insert(context->id, std::move(entries));
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

}  // namespace

extern "C" {

aicad_occt_status_t aicad_occt_context_create(aicad_occt_context_t** out_context) {
  if (out_context == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  try {
    aicad_occt_context_t* context = GlobalContextRegistry().Allocate();
    context->id = NextContextId();
    context->owning_thread = std::this_thread::get_id();
    // `context->shapes` is already a freshly-default-constructed, empty
    // ShapeTable from `emplace_back()` -- no prior occupant to reset,
    // unlike a reused slot in `ShapeTable` itself (this registry never
    // reuses a context's own memory for a different context, precisely
    // so a caller's dangling pointer from one context can never alias a
    // later, different, legitimately-live context -- see this fix's
    // design-comparison notes for why slot reuse, which IS used for
    // shapes, is deliberately NOT used here).
    context->live.store(true, std::memory_order_release);
    *out_context = context;
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
  // Well-defined even for an already-destroyed context (see the type's
  // own doc comment): its memory is never freed, so this is an ordinary
  // atomic load, not a use-after-free.
  if (!context->live.load(std::memory_order_acquire)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  if (std::this_thread::get_id() != context->owning_thread) {
    return AICAD_OCCT_ERR_WRONG_THREAD;
  }
  try {
    bool expected = true;
    if (!context->live.compare_exchange_strong(expected, false, std::memory_order_acq_rel)) {
      // Lost a race with a concurrent destroy call that violates the
      // documented single-thread-affine contract (Stage-1 kernel policy
      // #9) -- still a clean, deterministic rejection, never a
      // double-free: this bridge never calls `delete` on a context's own
      // memory at all, ever.
      return AICAD_OCCT_ERR_INVALID_ARGUMENT;
    }
    // Release the owned geometry memory now (this IS individually
    // freed/reset); only the small, fixed-size `aicad_occt_context`
    // control block itself is kept alive forever, per this fix's design.
    context->shapes = ShapeTable();
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

aicad_occt_status_t aicad_occt_mirror_shape(aicad_occt_context_t* context,
                                             aicad_shape_handle_t handle,
                                             const double origin[3],
                                             const double normal[3],
                                             aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  status = CheckHandleContext(context, handle);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (origin == nullptr || normal == nullptr || out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  if (!IsFinite3(origin)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  gp_Dir normal_dir;
  if (!TryToDir(normal, &normal_dir)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = context->shapes.Lookup(handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    // `gp_Ax2`'s main direction is the plane's normal; `SetMirror(ax2)`
    // builds the reflection across the plane through `ax2`'s own
    // location perpendicular to that main direction -- exactly the
    // mirror-plane semantics `cad_kernel_api::Plane3` (origin + unit
    // normal) already establishes above this bridge, deliberately NOT
    // routed through `aicad_occt_transform_shape` (this produces a
    // determinant -1 matrix, which that function correctly rejects).
    gp_Ax2 mirror_plane(ToPnt(origin), normal_dir);
    gp_Trsf trsf;
    trsf.SetMirror(mirror_plane);
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

aicad_occt_status_t aicad_occt_make_arc_edge(aicad_occt_context_t* context,
                                              const double p_start[3],
                                              const double p_mid[3],
                                              const double p_end[3],
                                              aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (p_start == nullptr || p_mid == nullptr || p_end == nullptr || out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  if (!IsFinite3(p_start) || !IsFinite3(p_mid) || !IsFinite3(p_end)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  try {
    GC_MakeArcOfCircle maker(ToPnt(p_start), ToPnt(p_mid), ToPnt(p_end));
    if (!maker.IsDone()) {
      // Coincident or collinear points -- OCCT's own gce_ConfusedPoints /
      // gce_IntersectionError construction failure, a caller-input
      // problem, not an adapter defect.
      return AICAD_OCCT_ERR_INVALID_ARGUMENT;
    }
    BRepBuilderAPI_MakeEdge make_edge(maker.Value());
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

aicad_occt_status_t CheckLineageHandleContext(aicad_occt_context_t* context,
                                               aicad_lineage_handle_t handle) {
  if (handle.context_id != context->id) {
    return AICAD_OCCT_ERR_FOREIGN_CONTEXT;
  }
  return AICAD_OCCT_OK;
}

// Resolves `lineage`/`input` down to the one captured LineageEntry
// `input` (a face/edge handle belonging to one of the operation's own
// original inputs) corresponds to. AICAD_OCCT_ERR_INVALID_ARGUMENT means
// both handles are individually valid but `input` was never one of the
// shapes CaptureLineage recorded for this operation (e.g. it names an
// output shape, or an entity from an unrelated shape entirely) --
// deliberately distinct from "unchanged"/"not deleted," which would
// silently conflate "no evidence" with a real, evidenced answer.
aicad_occt_status_t ResolveLineageEntry(aicad_occt_context_t* context,
                                         aicad_lineage_handle_t lineage,
                                         aicad_shape_handle_t input,
                                         const LineageEntry** out) {
  aicad_occt_status_t status = CheckLineageHandleContext(context, lineage);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  const std::vector<LineageEntry>* entries = nullptr;
  status = context->lineages.Lookup(lineage, &entries);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  const TopoDS_Shape* input_shape = nullptr;
  status = LookupAnyKind(context, input, &input_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  const LineageEntry* entry = FindLineageEntry(*entries, *input_shape);
  if (entry == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  *out = entry;
  return AICAD_OCCT_OK;
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

// --- AICAD-086: lineage-capturing operation variants. See LineageEntry's
// own doc comment for why lineage must be captured at operation time,
// never as a separate later call. ---

aicad_occt_status_t aicad_occt_boolean_union_lineage(aicad_occt_context_t* context,
                                                       aicad_shape_handle_t a,
                                                       aicad_shape_handle_t b,
                                                       aicad_shape_handle_t* out_handle,
                                                       aicad_lineage_handle_t* out_lineage) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr || out_lineage == nullptr) {
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
  return BooleanWithLineage(context, *shape_a, *shape_b, BOPAlgo_FUSE, out_handle, out_lineage);
}

aicad_occt_status_t aicad_occt_boolean_cut_lineage(aicad_occt_context_t* context,
                                                     aicad_shape_handle_t a,
                                                     aicad_shape_handle_t b,
                                                     aicad_shape_handle_t* out_handle,
                                                     aicad_lineage_handle_t* out_lineage) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr || out_lineage == nullptr) {
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
  return BooleanWithLineage(context, *shape_a, *shape_b, BOPAlgo_CUT, out_handle, out_lineage);
}

aicad_occt_status_t aicad_occt_boolean_intersect_lineage(aicad_occt_context_t* context,
                                                           aicad_shape_handle_t a,
                                                           aicad_shape_handle_t b,
                                                           aicad_shape_handle_t* out_handle,
                                                           aicad_lineage_handle_t* out_lineage) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr || out_lineage == nullptr) {
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
  return BooleanWithLineage(context, *shape_a, *shape_b, BOPAlgo_COMMON, out_handle, out_lineage);
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

namespace {

// AICAD-036 finding: BRepFilletAPI_MakeFillet (and the ChFi3d
// fillet-construction machinery it drives internally) is not safe to
// call concurrently from independent contexts on independent threads,
// for at least some inputs -- confirmed empirically, not assumed. A
// concave/reentrant edge on a multi-boolean shape (the Stage-1 proof
// bracket's own interior root fillet, AICAD-034) intermittently produced
// an invalid B-rep (1-2 invalid faces per `aicad_occt_shape_validate`)
// when 3 independent contexts on 3 independent threads each built and
// filleted the identical shape concurrently -- a ~47% failure rate over
// 30 repetitions -- while the IDENTICAL construction run with no
// concurrency at all (300 sequential repetitions, same process) never
// failed once. `aicad_occt_boolean_union`/`_cut`/`_intersect` and
// `aicad_occt_chamfer` were independently stress-tested under the same
// concurrent conditions and never failed, isolating this specifically to
// `BRepFilletAPI_MakeFillet` (see project/reports/AICAD-036.md for the
// full investigation, reproduction counts, and the isolation
// experiments). This mirrors AICAD-033's STEP-translator finding
// (project/reports/AICAD-033.md): a real defect/limitation in this
// version of the underlying kernel library's own global state, not a
// per-context bridge bug, fixed the same way -- a single process-wide
// mutex serializing every fillet call.
std::mutex& FilletMutex() {
  static std::mutex mutex;
  return mutex;
}

}  // namespace

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
    std::lock_guard<std::mutex> lock(FilletMutex());
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

// AICAD-086: like aicad_occt_fillet, but also captures lineage (see
// LineageEntry's own doc comment) -- `BRepFilletAPI_MakeFillet` (unlike
// the BOP-based Boolean operations) needs no SetToFillHistory toggle;
// Generated/Modified/IsDeleted are always answerable directly after
// Build().
aicad_occt_status_t aicad_occt_fillet_lineage(aicad_occt_context_t* context,
                                               aicad_shape_handle_t shape_handle,
                                               const aicad_shape_handle_t* edges,
                                               size_t edge_count,
                                               double radius,
                                               aicad_shape_handle_t* out_handle,
                                               aicad_lineage_handle_t* out_lineage) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (edges == nullptr || out_handle == nullptr || out_lineage == nullptr || edge_count == 0) {
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
    std::lock_guard<std::mutex> lock(FilletMutex());
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
    std::vector<LineageEntry> entries = CaptureLineage(make_fillet, {base_shape});
    *out_handle = context->shapes.Insert(context->id, make_fillet.Shape());
    *out_lineage = context->lineages.Insert(context->id, std::move(entries));
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

// AICAD-086: like aicad_occt_chamfer, but also captures lineage -- see
// aicad_occt_fillet_lineage's own doc comment.
aicad_occt_status_t aicad_occt_chamfer_lineage(aicad_occt_context_t* context,
                                                aicad_shape_handle_t shape_handle,
                                                const aicad_shape_handle_t* edges,
                                                size_t edge_count,
                                                double distance,
                                                aicad_shape_handle_t* out_handle,
                                                aicad_lineage_handle_t* out_lineage) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (edges == nullptr || out_handle == nullptr || out_lineage == nullptr || edge_count == 0) {
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
    std::vector<LineageEntry> entries = CaptureLineage(make_chamfer, {base_shape});
    *out_handle = context->shapes.Insert(context->id, make_chamfer.Shape());
    *out_lineage = context->lineages.Insert(context->id, std::move(entries));
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

aicad_occt_status_t aicad_occt_shape_duplicate(aicad_occt_context_t* context,
                                                aicad_shape_handle_t handle,
                                                aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = LookupAnyKind(context, handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    // A cheap second, independently-releasable handle onto the exact same
    // underlying TopoDS_Shape (OCCT's own TopoDS_Shape is a lightweight
    // handle+location value; re-inserting it is a map entry, never a real
    // geometry copy) -- used by `cad-query`'s own `Candidate::with_root`
    // (`AICAD-100A`) to give a candidate its own independently-owned
    // handle onto the whole shape it was enumerated from, without the
    // caller needing to keep the original handle alive/aliased.
    *out_handle = context->shapes.Insert(context->id, *shape);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_shell_count(aicad_occt_context_t* context,
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
    TopTools_IndexedMapOfShape shells;
    TopExp::MapShapes(*shape, TopAbs_SHELL, shells);
    *out_count = static_cast<size_t>(shells.Extent());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_get_shell(aicad_occt_context_t* context,
                                                aicad_shape_handle_t handle,
                                                size_t index,
                                                aicad_shape_handle_t* out_shell_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_shell_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = LookupAnyKind(context, handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    TopTools_IndexedMapOfShape shells;
    TopExp::MapShapes(*shape, TopAbs_SHELL, shells);
    if (index >= static_cast<size_t>(shells.Extent())) {
      return AICAD_OCCT_ERR_INVALID_ARGUMENT;
    }
    const TopoDS_Shape& shell = shells.FindKey(static_cast<Standard_Integer>(index) + 1);
    *out_shell_handle = context->shapes.Insert(context->id, shell);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_solid_count(aicad_occt_context_t* context,
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
    TopTools_IndexedMapOfShape solids;
    TopExp::MapShapes(*shape, TopAbs_SOLID, solids);
    *out_count = static_cast<size_t>(solids.Extent());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_get_solid(aicad_occt_context_t* context,
                                                aicad_shape_handle_t handle,
                                                size_t index,
                                                aicad_shape_handle_t* out_solid_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_solid_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = LookupAnyKind(context, handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    TopTools_IndexedMapOfShape solids;
    TopExp::MapShapes(*shape, TopAbs_SOLID, solids);
    if (index >= static_cast<size_t>(solids.Extent())) {
      return AICAD_OCCT_ERR_INVALID_ARGUMENT;
    }
    const TopoDS_Shape& solid = solids.FindKey(static_cast<Standard_Integer>(index) + 1);
    *out_solid_handle = context->shapes.Insert(context->id, solid);
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
    TopTools_IndexedMapOfShape vertices, edges, wires, faces, shells, solids;
    TopExp::MapShapes(*shape, TopAbs_VERTEX, vertices);
    TopExp::MapShapes(*shape, TopAbs_EDGE, edges);
    TopExp::MapShapes(*shape, TopAbs_WIRE, wires);
    TopExp::MapShapes(*shape, TopAbs_FACE, faces);
    TopExp::MapShapes(*shape, TopAbs_SHELL, shells);
    TopExp::MapShapes(*shape, TopAbs_SOLID, solids);
    out_report->is_valid = analyzer.IsValid() ? 1 : 0;
    out_report->invalid_vertex_count = CountInvalid(analyzer, vertices);
    out_report->invalid_edge_count = CountInvalid(analyzer, edges);
    out_report->invalid_wire_count = CountInvalid(analyzer, wires);
    out_report->invalid_face_count = CountInvalid(analyzer, faces);
    out_report->invalid_shell_count = CountInvalid(analyzer, shells);
    out_report->invalid_solid_count = CountInvalid(analyzer, solids);
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

namespace {

// AICAD-033 key finding: OCCT's STEP translator (STEPControl_Writer/
// STEPControl_Reader and the XSTEP/Interface_Static machinery they drive)
// holds process-global, non-thread-safe state -- confirmed empirically,
// not assumed: calling aicad_occt_export_step concurrently from
// independent contexts on independent threads (each thread otherwise
// fully respecting Stage-1 kernel policy #9's single-thread-affine-per-
// context contract) intermittently segfaulted the process (see
// project/reports/AICAD-033.md for the exact reproduction). This is a
// defect in the underlying kernel library's own global state, not a
// per-context bridge bug, so a per-context lock cannot fix it -- only a
// single process-wide mutex serializing every STEP export/import call
// can, which is what this does. AICAD-035 shares this same mutex for
// `aicad_occt_import_step` rather than introducing a second one, since
// reader and writer drive the same underlying global XSTEP session state.
std::mutex& StepIoMutex() {
  static std::mutex mutex;
  return mutex;
}

}  // namespace

aicad_occt_status_t aicad_occt_export_step(aicad_occt_context_t* context,
                                            aicad_shape_handle_t handle,
                                            const char* file_path) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  status = CheckHandleContext(context, handle);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (file_path == nullptr || file_path[0] == '\0') {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = context->shapes.Lookup(handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    std::lock_guard<std::mutex> lock(StepIoMutex());
    STEPControl_Writer writer;
    const IFSelect_ReturnStatus transfer_status = writer.Transfer(*shape, STEPControl_AsIs);
    if (transfer_status != IFSelect_RetDone) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    const IFSelect_ReturnStatus write_status = writer.Write(file_path);
    if (write_status != IFSelect_RetDone) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_import_step(aicad_occt_context_t* context,
                                            const char* file_path,
                                            aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (file_path == nullptr || file_path[0] == '\0' || out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  try {
    std::lock_guard<std::mutex> lock(StepIoMutex());
    STEPControl_Reader reader;
    const IFSelect_ReturnStatus read_status = reader.ReadFile(file_path);
    if (read_status != IFSelect_RetDone) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    const Standard_Integer num_roots = reader.TransferRoots();
    if (num_roots <= 0) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    const TopoDS_Shape shape = reader.OneShape();
    if (shape.IsNull()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, shape);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

}  // extern "C"

namespace {

// Maps OCCT's own BRepAdaptor_Surface::GetType() into this bridge's
// AICAD-owned, ABI-stable aicad_surface_kind_t -- never re-exporting the
// OCCT enum's numeric value directly (this header's own top-of-file
// kernel-neutral-at-the-ABI contract).
aicad_surface_kind_t ToSurfaceKind(GeomAbs_SurfaceType type) {
  switch (type) {
    case GeomAbs_Plane:
      return AICAD_SURFACE_PLANE;
    case GeomAbs_Cylinder:
      return AICAD_SURFACE_CYLINDER;
    case GeomAbs_Cone:
      return AICAD_SURFACE_CONE;
    case GeomAbs_Sphere:
      return AICAD_SURFACE_SPHERE;
    case GeomAbs_Torus:
      return AICAD_SURFACE_TORUS;
    case GeomAbs_BezierSurface:
      return AICAD_SURFACE_BEZIER;
    case GeomAbs_BSplineSurface:
      return AICAD_SURFACE_BSPLINE;
    default:
      return AICAD_SURFACE_OTHER;
  }
}

aicad_curve_kind_t ToCurveKind(GeomAbs_CurveType type) {
  switch (type) {
    case GeomAbs_Line:
      return AICAD_CURVE_LINE;
    case GeomAbs_Circle:
      return AICAD_CURVE_CIRCLE;
    case GeomAbs_Ellipse:
      return AICAD_CURVE_ELLIPSE;
    case GeomAbs_BezierCurve:
      return AICAD_CURVE_BEZIER;
    case GeomAbs_BSplineCurve:
      return AICAD_CURVE_BSPLINE;
    default:
      return AICAD_CURVE_OTHER;
  }
}

void WriteAx1(const gp_Ax1& axis, double out_origin[3], double out_direction[3]) {
  const gp_Pnt& location = axis.Location();
  const gp_Dir& direction = axis.Direction();
  out_origin[0] = location.X();
  out_origin[1] = location.Y();
  out_origin[2] = location.Z();
  out_direction[0] = direction.X();
  out_direction[1] = direction.Y();
  out_direction[2] = direction.Z();
}

}  // namespace

extern "C" {

aicad_occt_status_t aicad_occt_shape_surface_type(aicad_occt_context_t* context,
                                                   aicad_shape_handle_t face_handle,
                                                   int* out_kind) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_kind == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* face_shape = nullptr;
  status = LookupTyped(context, face_handle, TopAbs_FACE, &face_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepAdaptor_Surface surface(TopoDS::Face(*face_shape));
    *out_kind = static_cast<int>(ToSurfaceKind(surface.GetType()));
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_face_radius(aicad_occt_context_t* context,
                                                  aicad_shape_handle_t face_handle,
                                                  double* out_radius) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_radius == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* face_shape = nullptr;
  status = LookupTyped(context, face_handle, TopAbs_FACE, &face_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepAdaptor_Surface surface(TopoDS::Face(*face_shape));
    switch (surface.GetType()) {
      case GeomAbs_Cylinder:
        *out_radius = surface.Cylinder().Radius();
        return AICAD_OCCT_OK;
      case GeomAbs_Sphere:
        *out_radius = surface.Sphere().Radius();
        return AICAD_OCCT_OK;
      case GeomAbs_Torus:
        *out_radius = surface.Torus().MajorRadius();
        return AICAD_OCCT_OK;
      default:
        // No single well-defined radius for this surface kind (e.g. a
        // cone's radius varies continuously along its axis) -- documented
        // in this function's own header comment, not guessed here.
        return AICAD_OCCT_ERR_INVALID_ARGUMENT;
    }
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_face_axis(aicad_occt_context_t* context,
                                                aicad_shape_handle_t face_handle,
                                                double out_origin[3],
                                                double out_direction[3]) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_origin == nullptr || out_direction == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* face_shape = nullptr;
  status = LookupTyped(context, face_handle, TopAbs_FACE, &face_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepAdaptor_Surface surface(TopoDS::Face(*face_shape));
    switch (surface.GetType()) {
      case GeomAbs_Cylinder:
        WriteAx1(surface.Cylinder().Axis(), out_origin, out_direction);
        return AICAD_OCCT_OK;
      case GeomAbs_Cone:
        WriteAx1(surface.Cone().Axis(), out_origin, out_direction);
        return AICAD_OCCT_OK;
      case GeomAbs_Torus:
        WriteAx1(surface.Torus().Axis(), out_origin, out_direction);
        return AICAD_OCCT_OK;
      default:
        return AICAD_OCCT_ERR_INVALID_ARGUMENT;
    }
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_face_normal(aicad_occt_context_t* context,
                                                  aicad_shape_handle_t face_handle,
                                                  double out_point[3],
                                                  double out_normal[3]) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_point == nullptr || out_normal == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* face_shape = nullptr;
  status = LookupTyped(context, face_handle, TopAbs_FACE, &face_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    const TopoDS_Face& face = TopoDS::Face(*face_shape);
    Standard_Real umin, umax, vmin, vmax;
    BRepTools::UVBounds(face, umin, umax, vmin, vmax);
    const Standard_Real u = 0.5 * (umin + umax);
    const Standard_Real v = 0.5 * (vmin + vmax);
    BRepAdaptor_Surface surface(face);
    gp_Pnt point;
    gp_Vec du, dv;
    surface.D1(u, v, point, du, dv);
    gp_Vec normal = du.Crossed(dv);
    if (normal.Magnitude() < 1e-12) {
      // Singular at this exact parameter (e.g. a cone's apex) -- fails
      // rather than reporting a meaningless zero-length direction.
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    normal.Normalize();
    if (face.Orientation() == TopAbs_REVERSED) {
      // A REVERSED face's actual outward material normal is the negation
      // of its underlying surface's raw D1 parametrization sense --
      // matching ExtractTessellation's own REVERSED-handling precedent
      // above (that function swaps winding; this negates the normal --
      // both correct for the same underlying reason).
      normal.Reverse();
    }
    out_point[0] = point.X();
    out_point[1] = point.Y();
    out_point[2] = point.Z();
    out_normal[0] = normal.X();
    out_normal[1] = normal.Y();
    out_normal[2] = normal.Z();
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_curve_type(aicad_occt_context_t* context,
                                                 aicad_shape_handle_t edge_handle,
                                                 int* out_kind) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_kind == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* edge_shape = nullptr;
  status = LookupTyped(context, edge_handle, TopAbs_EDGE, &edge_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepAdaptor_Curve curve(TopoDS::Edge(*edge_shape));
    *out_kind = static_cast<int>(ToCurveKind(curve.GetType()));
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_edge_radius(aicad_occt_context_t* context,
                                                  aicad_shape_handle_t edge_handle,
                                                  double* out_radius) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_radius == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* edge_shape = nullptr;
  status = LookupTyped(context, edge_handle, TopAbs_EDGE, &edge_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepAdaptor_Curve curve(TopoDS::Edge(*edge_shape));
    if (curve.GetType() != GeomAbs_Circle) {
      return AICAD_OCCT_ERR_INVALID_ARGUMENT;
    }
    *out_radius = curve.Circle().Radius();
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_edge_axis(aicad_occt_context_t* context,
                                                aicad_shape_handle_t edge_handle,
                                                double out_origin[3],
                                                double out_direction[3]) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_origin == nullptr || out_direction == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* edge_shape = nullptr;
  status = LookupTyped(context, edge_handle, TopAbs_EDGE, &edge_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepAdaptor_Curve curve(TopoDS::Edge(*edge_shape));
    if (curve.GetType() != GeomAbs_Circle) {
      return AICAD_OCCT_ERR_INVALID_ARGUMENT;
    }
    WriteAx1(curve.Circle().Axis(), out_origin, out_direction);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_is_same(aicad_occt_context_t* context,
                                              aicad_shape_handle_t a,
                                              aicad_shape_handle_t b,
                                              int* out_is_same) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_is_same == nullptr) {
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
    *out_is_same = shape_a->IsSame(*shape_b) ? 1 : 0;
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_wire_count(aicad_occt_context_t* context,
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
    TopTools_IndexedMapOfShape wires;
    TopExp::MapShapes(*shape, TopAbs_WIRE, wires);
    *out_count = static_cast<size_t>(wires.Extent());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_get_wire(aicad_occt_context_t* context,
                                               aicad_shape_handle_t handle,
                                               size_t index,
                                               aicad_shape_handle_t* out_wire_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_wire_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = LookupAnyKind(context, handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    TopTools_IndexedMapOfShape wires;
    TopExp::MapShapes(*shape, TopAbs_WIRE, wires);
    if (index >= static_cast<size_t>(wires.Extent())) {
      return AICAD_OCCT_ERR_INVALID_ARGUMENT;
    }
    const TopoDS_Shape& wire = wires.FindKey(static_cast<Standard_Integer>(index) + 1);
    *out_wire_handle = context->shapes.Insert(context->id, wire);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_is_outer_wire(aicad_occt_context_t* context,
                                                    aicad_shape_handle_t face_handle,
                                                    aicad_shape_handle_t wire_handle,
                                                    int* out_is_outer) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_is_outer == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* face_shape = nullptr;
  status = LookupTyped(context, face_handle, TopAbs_FACE, &face_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  const TopoDS_Shape* wire_shape = nullptr;
  status = LookupTyped(context, wire_handle, TopAbs_WIRE, &wire_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    const TopoDS_Wire& outer = BRepTools::OuterWire(TopoDS::Face(*face_shape));
    *out_is_outer = outer.IsSame(*wire_shape) ? 1 : 0;
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_vertex_point(aicad_occt_context_t* context,
                                                   aicad_shape_handle_t vertex_handle,
                                                   double out_point[3]) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_point == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* vertex_shape = nullptr;
  status = LookupTyped(context, vertex_handle, TopAbs_VERTEX, &vertex_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    const gp_Pnt point = BRep_Tool::Pnt(TopoDS::Vertex(*vertex_shape));
    out_point[0] = point.X();
    out_point[1] = point.Y();
    out_point[2] = point.Z();
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_classify_point(aicad_occt_context_t* context,
                                                     aicad_shape_handle_t solid_handle,
                                                     const double point[3],
                                                     double tolerance,
                                                     int* out_classification) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (point == nullptr || out_classification == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  if (!IsFinite3(point) || !(tolerance > 0.0) || !std::isfinite(tolerance)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = LookupAnyKind(context, solid_handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepClass3d_SolidClassifier classifier(*shape);
    classifier.Perform(ToPnt(point), tolerance);
    if (classifier.State() == TopAbs_IN) {
      *out_classification = static_cast<int>(AICAD_CLASSIFY_IN);
    } else if (classifier.State() == TopAbs_ON) {
      *out_classification = static_cast<int>(AICAD_CLASSIFY_ON_BOUNDARY);
    } else {
      *out_classification = static_cast<int>(AICAD_CLASSIFY_OUT);
    }
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

// --- AICAD-086: lineage query/release. See LineageEntry's own doc
// comment for the capture-at-operation-time design these query. ---

aicad_occt_status_t aicad_occt_release_lineage(aicad_occt_context_t* context,
                                                aicad_lineage_handle_t handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  status = CheckLineageHandleContext(context, handle);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  return context->lineages.Release(handle);
}

aicad_occt_status_t aicad_occt_lineage_is_deleted(aicad_occt_context_t* context,
                                                   aicad_lineage_handle_t lineage,
                                                   aicad_shape_handle_t input,
                                                   int* out_is_deleted) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_is_deleted == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const LineageEntry* entry = nullptr;
  status = ResolveLineageEntry(context, lineage, input, &entry);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  *out_is_deleted = entry->deleted ? 1 : 0;
  return AICAD_OCCT_OK;
}

aicad_occt_status_t aicad_occt_lineage_generated_count(aicad_occt_context_t* context,
                                                        aicad_lineage_handle_t lineage,
                                                        aicad_shape_handle_t input,
                                                        size_t* out_count) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_count == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const LineageEntry* entry = nullptr;
  status = ResolveLineageEntry(context, lineage, input, &entry);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  *out_count = entry->generated.size();
  return AICAD_OCCT_OK;
}

aicad_occt_status_t aicad_occt_lineage_generated_get(aicad_occt_context_t* context,
                                                      aicad_lineage_handle_t lineage,
                                                      aicad_shape_handle_t input,
                                                      size_t index,
                                                      aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const LineageEntry* entry = nullptr;
  status = ResolveLineageEntry(context, lineage, input, &entry);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (index >= entry->generated.size()) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  try {
    *out_handle = context->shapes.Insert(context->id, entry->generated[index]);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_lineage_modified_count(aicad_occt_context_t* context,
                                                       aicad_lineage_handle_t lineage,
                                                       aicad_shape_handle_t input,
                                                       size_t* out_count) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_count == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const LineageEntry* entry = nullptr;
  status = ResolveLineageEntry(context, lineage, input, &entry);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  *out_count = entry->modified.size();
  return AICAD_OCCT_OK;
}

aicad_occt_status_t aicad_occt_lineage_modified_get(aicad_occt_context_t* context,
                                                     aicad_lineage_handle_t lineage,
                                                     aicad_shape_handle_t input,
                                                     size_t index,
                                                     aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const LineageEntry* entry = nullptr;
  status = ResolveLineageEntry(context, lineage, input, &entry);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (index >= entry->modified.size()) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  try {
    *out_handle = context->shapes.Insert(context->id, entry->modified[index]);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

}  // extern "C"

namespace {

// AICAD-119: shared face-on-elementary-surface construction, used by
// aicad_occt_make_face_on_plane/_cylinder/_cone/_sphere/_torus below. OCCT
// provides an identical `BRepBuilderAPI_MakeFace(const Surface&, const
// TopoDS_Wire&, Standard_Boolean Inside)` constructor for every one of
// gp_Pln/gp_Cylinder/gp_Cone/gp_Sphere/gp_Torus, which is exactly what lets
// this be a single template instead of 5 near-duplicate functions.
// `outer_reversed` is applied to the outer wire only; each hole is added
// exactly as given -- no orientation is inferred or corrected (see
// aicad_occt_make_face_on_plane's own header doc comment).
template <class Surface>
aicad_occt_status_t MakeFaceOnSurface(aicad_occt_context_t* context,
                                       const Surface& surface,
                                       aicad_shape_handle_t outer_wire,
                                       const aicad_shape_handle_t* holes,
                                       size_t hole_count,
                                       int outer_reversed,
                                       aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr || (holes == nullptr && hole_count != 0)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* outer_shape = nullptr;
  status = LookupTyped(context, outer_wire, TopAbs_WIRE, &outer_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    TopoDS_Wire outer = TopoDS::Wire(*outer_shape);
    if (outer_reversed) {
      outer = TopoDS::Wire(outer.Reversed());
    }
    BRepBuilderAPI_MakeFace make_face(surface, outer, /*Inside=*/Standard_True);
    if (!make_face.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    for (size_t i = 0; i < hole_count; ++i) {
      const TopoDS_Shape* hole_shape = nullptr;
      status = LookupTyped(context, holes[i], TopAbs_WIRE, &hole_shape);
      if (status != AICAD_OCCT_OK) {
        return status;
      }
      make_face.Add(TopoDS::Wire(*hole_shape));
      if (!make_face.IsDone()) {
        return AICAD_OCCT_ERR_OPERATION_FAILED;
      }
    }
    *out_handle = context->shapes.Insert(context->id, make_face.Face());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

}  // namespace

extern "C" {

aicad_occt_status_t aicad_occt_make_vertex(aicad_occt_context_t* context,
                                            const double point[3],
                                            aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (point == nullptr || out_handle == nullptr || !IsFinite3(point)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  try {
    BRepBuilderAPI_MakeVertex make_vertex(ToPnt(point));
    if (!make_vertex.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, make_vertex.Vertex());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_make_face_on_plane(aicad_occt_context_t* context,
                                                   aicad_shape_handle_t outer_wire,
                                                   const aicad_shape_handle_t* holes,
                                                   size_t hole_count,
                                                   const double origin[3],
                                                   const double normal[3],
                                                   int outer_reversed,
                                                   aicad_shape_handle_t* out_handle) {
  if (origin == nullptr || normal == nullptr || !IsFinite3(origin)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  gp_Dir dir;
  if (!TryToDir(normal, &dir)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  gp_Pln plane(ToPnt(origin), dir);
  return MakeFaceOnSurface(context, plane, outer_wire, holes, hole_count, outer_reversed,
                            out_handle);
}

aicad_occt_status_t aicad_occt_make_face_on_cylinder(aicad_occt_context_t* context,
                                                      aicad_shape_handle_t outer_wire,
                                                      const aicad_shape_handle_t* holes,
                                                      size_t hole_count,
                                                      const double axis_origin[3],
                                                      const double axis_direction[3],
                                                      double radius,
                                                      int outer_reversed,
                                                      aicad_shape_handle_t* out_handle) {
  if (axis_origin == nullptr || axis_direction == nullptr || !IsFinite3(axis_origin) ||
      !std::isfinite(radius) || radius <= 0.0) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  gp_Dir dir;
  if (!TryToDir(axis_direction, &dir)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  gp_Ax3 axis(ToPnt(axis_origin), dir);
  gp_Cylinder cylinder(axis, radius);
  return MakeFaceOnSurface(context, cylinder, outer_wire, holes, hole_count, outer_reversed,
                            out_handle);
}

aicad_occt_status_t aicad_occt_make_face_on_cone(aicad_occt_context_t* context,
                                                  aicad_shape_handle_t outer_wire,
                                                  const aicad_shape_handle_t* holes,
                                                  size_t hole_count,
                                                  const double axis_origin[3],
                                                  const double axis_direction[3],
                                                  double half_angle_radians,
                                                  int outer_reversed,
                                                  aicad_shape_handle_t* out_handle) {
  const double kHalfPi = 2.0 * std::atan(1.0);
  if (axis_origin == nullptr || axis_direction == nullptr || !IsFinite3(axis_origin) ||
      !std::isfinite(half_angle_radians) || half_angle_radians <= 0.0 ||
      half_angle_radians >= kHalfPi) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  gp_Dir dir;
  if (!TryToDir(axis_direction, &dir)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  gp_Ax3 axis(ToPnt(axis_origin), dir);
  gp_Cone cone(axis, half_angle_radians, /*Radius=*/0.0);
  return MakeFaceOnSurface(context, cone, outer_wire, holes, hole_count, outer_reversed,
                            out_handle);
}

aicad_occt_status_t aicad_occt_make_face_on_sphere(aicad_occt_context_t* context,
                                                    aicad_shape_handle_t outer_wire,
                                                    const aicad_shape_handle_t* holes,
                                                    size_t hole_count,
                                                    const double center[3],
                                                    double radius,
                                                    int outer_reversed,
                                                    aicad_shape_handle_t* out_handle) {
  if (center == nullptr || !IsFinite3(center) || !std::isfinite(radius) || radius <= 0.0) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  gp_Ax3 axis(ToPnt(center), gp_Dir(0.0, 0.0, 1.0));
  gp_Sphere sphere(axis, radius);
  return MakeFaceOnSurface(context, sphere, outer_wire, holes, hole_count, outer_reversed,
                            out_handle);
}

aicad_occt_status_t aicad_occt_make_face_on_torus(aicad_occt_context_t* context,
                                                   aicad_shape_handle_t outer_wire,
                                                   const aicad_shape_handle_t* holes,
                                                   size_t hole_count,
                                                   const double axis_origin[3],
                                                   const double axis_direction[3],
                                                   double major_radius,
                                                   double minor_radius,
                                                   int outer_reversed,
                                                   aicad_shape_handle_t* out_handle) {
  if (axis_origin == nullptr || axis_direction == nullptr || !IsFinite3(axis_origin) ||
      !std::isfinite(major_radius) || major_radius <= 0.0 || !std::isfinite(minor_radius) ||
      minor_radius <= 0.0) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  gp_Dir dir;
  if (!TryToDir(axis_direction, &dir)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  gp_Ax3 axis(ToPnt(axis_origin), dir);
  gp_Torus torus(axis, major_radius, minor_radius);
  return MakeFaceOnSurface(context, torus, outer_wire, holes, hole_count, outer_reversed,
                            out_handle);
}

aicad_occt_status_t aicad_occt_make_shell(aicad_occt_context_t* context,
                                           const aicad_shape_handle_t* faces,
                                           size_t face_count,
                                           aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (faces == nullptr || out_handle == nullptr || face_count == 0) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  try {
    BRep_Builder builder;
    TopoDS_Shell shell;
    builder.MakeShell(shell);
    for (size_t i = 0; i < face_count; ++i) {
      const TopoDS_Shape* face_shape = nullptr;
      status = LookupTyped(context, faces[i], TopAbs_FACE, &face_shape);
      if (status != AICAD_OCCT_OK) {
        return status;
      }
      builder.Add(shell, TopoDS::Face(*face_shape));
    }
    *out_handle = context->shapes.Insert(context->id, shell);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_make_solid(aicad_occt_context_t* context,
                                           aicad_shape_handle_t outer_shell,
                                           const aicad_shape_handle_t* voids,
                                           size_t void_count,
                                           aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr || (voids == nullptr && void_count != 0)) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* outer_shape = nullptr;
  status = LookupTyped(context, outer_shell, TopAbs_SHELL, &outer_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepBuilderAPI_MakeSolid make_solid(TopoDS::Shell(*outer_shape));
    for (size_t i = 0; i < void_count; ++i) {
      const TopoDS_Shape* void_shape = nullptr;
      status = LookupTyped(context, voids[i], TopAbs_SHELL, &void_shape);
      if (status != AICAD_OCCT_OK) {
        return status;
      }
      make_solid.Add(TopoDS::Shell(*void_shape));
    }
    if (!make_solid.IsDone()) {
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    *out_handle = context->shapes.Insert(context->id, make_solid.Solid());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_make_compound(aicad_occt_context_t* context,
                                              const aicad_shape_handle_t* shapes,
                                              size_t shape_count,
                                              aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (shapes == nullptr || out_handle == nullptr || shape_count == 0) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  try {
    BRep_Builder builder;
    TopoDS_Compound compound;
    builder.MakeCompound(compound);
    for (size_t i = 0; i < shape_count; ++i) {
      const TopoDS_Shape* member = nullptr;
      status = LookupAnyKind(context, shapes[i], &member);
      if (status != AICAD_OCCT_OK) {
        return status;
      }
      builder.Add(compound, *member);
    }
    *out_handle = context->shapes.Insert(context->id, compound);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

}  // extern "C"

namespace {

// AICAD-120: captures Generated/Modified/IsDeleted lineage for every
// unique face/edge of each input shape against a completed sewing
// operation -- the sewing-specific analogue of `CaptureLineage<Builder>`
// above, needed because `BRepBuilderAPI_Sewing`'s own API shape differs
// from `BRepAlgoAPI_BooleanOperation`'s (`IsModified`/`Modified` return a
// single `Standard_Boolean`/`TopoDS_Shape&`, not a `TopTools_ListOfShape`,
// and there is no `IsDeleted` at all -- sewing merges/relabels coincident
// boundaries, it never deletes an entity outright or generates one with
// no traceable input, so `deleted`/`generated` are always false/empty
// here, by construction, not by omission).
std::vector<LineageEntry> CaptureSewLineage(const BRepBuilderAPI_Sewing& sewing,
                                             const std::vector<const TopoDS_Shape*>& inputs) {
  std::vector<LineageEntry> entries;
  for (const TopoDS_Shape* input : inputs) {
    for (TopAbs_ShapeEnum kind : {TopAbs_FACE, TopAbs_EDGE}) {
      TopTools_IndexedMapOfShape subshapes;
      TopExp::MapShapes(*input, kind, subshapes);
      for (Standard_Integer i = 1; i <= subshapes.Extent(); ++i) {
        LineageEntry entry;
        entry.input = subshapes.FindKey(i);
        if (sewing.IsModified(entry.input)) {
          entry.modified.push_back(sewing.Modified(entry.input));
        }
        entries.push_back(std::move(entry));
      }
    }
  }
  return entries;
}

}  // namespace

extern "C" {

aicad_occt_status_t aicad_occt_sew(aicad_occt_context_t* context,
                                    const aicad_shape_handle_t* shapes,
                                    size_t shape_count,
                                    double tolerance,
                                    aicad_shape_handle_t* out_handle,
                                    aicad_lineage_handle_t* out_lineage,
                                    aicad_sew_report_t* out_report) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (shapes == nullptr || out_handle == nullptr || out_lineage == nullptr ||
      out_report == nullptr || shape_count == 0 || !std::isfinite(tolerance) ||
      tolerance <= 0.0) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  std::vector<const TopoDS_Shape*> inputs;
  inputs.reserve(shape_count);
  for (size_t i = 0; i < shape_count; ++i) {
    const TopoDS_Shape* shape = nullptr;
    status = LookupAnyKind(context, shapes[i], &shape);
    if (status != AICAD_OCCT_OK) {
      return status;
    }
    inputs.push_back(shape);
  }
  try {
    BRepBuilderAPI_Sewing sewing(tolerance);
    for (const TopoDS_Shape* shape : inputs) {
      sewing.Add(*shape);
    }
    sewing.Perform();
    TopoDS_Shape result = sewing.SewedShape();
    BRepCheck_Analyzer analyzer(result);
    out_report->is_valid = analyzer.IsValid() ? 1 : 0;
    out_report->free_edge_count = static_cast<size_t>(sewing.NbFreeEdges());
    out_report->multiple_edge_count = static_cast<size_t>(sewing.NbMultipleEdges());
    out_report->degenerated_shape_count = static_cast<size_t>(sewing.NbDegeneratedShapes());
    std::vector<LineageEntry> entries = CaptureSewLineage(sewing, inputs);
    bool changed = false;
    for (const LineageEntry& entry : entries) {
      if (!entry.modified.empty()) {
        changed = true;
        break;
      }
    }
    out_report->changed = changed ? 1 : 0;
    *out_handle = context->shapes.Insert(context->id, result);
    *out_lineage = context->lineages.Insert(context->id, std::move(entries));
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_heal(aicad_occt_context_t* context,
                                     aicad_shape_handle_t handle,
                                     double tolerance,
                                     aicad_shape_handle_t* out_handle,
                                     aicad_heal_report_t* out_report) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr || out_report == nullptr || !std::isfinite(tolerance) ||
      tolerance <= 0.0) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = LookupAnyKind(context, handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepCheck_Analyzer before(*shape);
    out_report->is_valid_before = before.IsValid() ? 1 : 0;
    TopAbs_ShapeEnum kind_before = shape->ShapeType();
    ShapeFix_Shape fixer(*shape);
    fixer.SetPrecision(tolerance);
    Standard_Boolean changed = fixer.Perform();
    TopoDS_Shape fixed = fixer.Shape();
    BRepCheck_Analyzer after(fixed);
    out_report->changed = changed ? 1 : 0;
    out_report->is_valid_after = after.IsValid() ? 1 : 0;
    out_report->kind_changed = (fixed.ShapeType() != kind_before) ? 1 : 0;
    *out_handle = context->shapes.Insert(context->id, fixed);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_kind(aicad_occt_context_t* context,
                                           aicad_shape_handle_t handle,
                                           int* out_kind) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_kind == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = LookupAnyKind(context, handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    switch (shape->ShapeType()) {
      case TopAbs_VERTEX:
        *out_kind = AICAD_TOPOLOGY_VERTEX;
        return AICAD_OCCT_OK;
      case TopAbs_EDGE:
        *out_kind = AICAD_TOPOLOGY_EDGE;
        return AICAD_OCCT_OK;
      case TopAbs_WIRE:
        *out_kind = AICAD_TOPOLOGY_WIRE;
        return AICAD_OCCT_OK;
      case TopAbs_FACE:
        *out_kind = AICAD_TOPOLOGY_FACE;
        return AICAD_OCCT_OK;
      case TopAbs_SHELL:
        *out_kind = AICAD_TOPOLOGY_SHELL;
        return AICAD_OCCT_OK;
      case TopAbs_SOLID:
        *out_kind = AICAD_TOPOLOGY_SOLID;
        return AICAD_OCCT_OK;
      default:
        // Compound/CompSolid/generic Shape: no single classifiable
        // entity kind to report.
        return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_shape_is_forward_oriented(aicad_occt_context_t* context,
                                                          aicad_shape_handle_t handle,
                                                          int* out_is_forward) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_is_forward == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* shape = nullptr;
  status = LookupAnyKind(context, handle, &shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    *out_is_forward = (shape->Orientation() == TopAbs_FORWARD) ? 1 : 0;
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

// --- AICAD-123: functional raw topology editing (BRepTools_ReShape/
// ShapeUpgrade_UnifySameDomain-backed remove/replace/split/merge). Every
// entity argument here is a handle the caller (cad-geometry-runtime)
// already resolved by raw index against a live shape -- these functions
// perform no index resolution of their own, mirroring aicad_occt_shell's
// own faces_to_remove convention exactly. ---

aicad_occt_status_t aicad_occt_remove_face(aicad_occt_context_t* context,
                                            aicad_shape_handle_t shape_handle,
                                            const aicad_shape_handle_t* faces_to_remove,
                                            size_t face_count,
                                            aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (faces_to_remove == nullptr || out_handle == nullptr || face_count == 0) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* base_shape = nullptr;
  status = LookupAnyKind(context, shape_handle, &base_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepTools_ReShape reshape;
    for (size_t i = 0; i < face_count; ++i) {
      const TopoDS_Shape* face_shape = nullptr;
      status = LookupTyped(context, faces_to_remove[i], TopAbs_FACE, &face_shape);
      if (status != AICAD_OCCT_OK) {
        return status;
      }
      reshape.Remove(*face_shape);
    }
    TopoDS_Shape result = reshape.Apply(*base_shape);
    *out_handle = context->shapes.Insert(context->id, result);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_replace_face(aicad_occt_context_t* context,
                                             aicad_shape_handle_t shape_handle,
                                             aicad_shape_handle_t old_face_handle,
                                             aicad_shape_handle_t new_face_handle,
                                             aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (out_handle == nullptr) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* base_shape = nullptr;
  status = LookupAnyKind(context, shape_handle, &base_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  const TopoDS_Shape* old_face = nullptr;
  status = LookupTyped(context, old_face_handle, TopAbs_FACE, &old_face);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  const TopoDS_Shape* new_face = nullptr;
  status = LookupTyped(context, new_face_handle, TopAbs_FACE, &new_face);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    BRepTools_ReShape reshape;
    reshape.Replace(*old_face, *new_face);
    TopoDS_Shape result = reshape.Apply(*base_shape);
    *out_handle = context->shapes.Insert(context->id, result);
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_split_edge(aicad_occt_context_t* context,
                                           aicad_shape_handle_t edge_handle,
                                           const double* params,
                                           size_t param_count,
                                           aicad_shape_handle_t* out_handles,
                                           size_t* out_handle_count) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (params == nullptr || out_handles == nullptr || out_handle_count == nullptr ||
      param_count == 0) {
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  const TopoDS_Shape* edge_shape = nullptr;
  status = LookupTyped(context, edge_handle, TopAbs_EDGE, &edge_shape);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  try {
    Standard_Real first = 0.0;
    Standard_Real last = 0.0;
    Handle(Geom_Curve) curve = BRep_Tool::Curve(TopoDS::Edge(*edge_shape), first, last);
    if (curve.IsNull()) {
      // A degenerate edge (no underlying 3D curve) has nothing to split.
      return AICAD_OCCT_ERR_OPERATION_FAILED;
    }
    // Every split parameter must be strictly increasing and strictly
    // interior to the edge's own [first, last] range -- a param at or
    // beyond either end would produce a degenerate (zero-length) segment,
    // which this function rejects explicitly rather than constructing.
    double previous = first;
    for (size_t i = 0; i < param_count; ++i) {
      if (!std::isfinite(params[i]) || params[i] <= previous || params[i] >= last) {
        return AICAD_OCCT_ERR_INVALID_ARGUMENT;
      }
      previous = params[i];
    }
    std::vector<double> breakpoints;
    breakpoints.reserve(param_count + 2);
    breakpoints.push_back(first);
    for (size_t i = 0; i < param_count; ++i) {
      breakpoints.push_back(params[i]);
    }
    breakpoints.push_back(last);
    for (size_t i = 0; i + 1 < breakpoints.size(); ++i) {
      BRepBuilderAPI_MakeEdge make_edge(curve, breakpoints[i], breakpoints[i + 1]);
      if (!make_edge.IsDone()) {
        return AICAD_OCCT_ERR_OPERATION_FAILED;
      }
      out_handles[i] = context->shapes.Insert(context->id, make_edge.Edge());
    }
    *out_handle_count = breakpoints.size() - 1;
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

aicad_occt_status_t aicad_occt_merge_faces(aicad_occt_context_t* context,
                                            const aicad_shape_handle_t* faces,
                                            size_t face_count,
                                            aicad_shape_handle_t* out_handle) {
  aicad_occt_status_t status = CheckContext(context);
  if (status != AICAD_OCCT_OK) {
    return status;
  }
  if (faces == nullptr || out_handle == nullptr || face_count < 2) {
    // Merging fewer than 2 faces is not a meaningful edit -- explicit
    // rejection rather than a silent no-op copy.
    return AICAD_OCCT_ERR_INVALID_ARGUMENT;
  }
  try {
    BRep_Builder builder;
    TopoDS_Compound group;
    builder.MakeCompound(group);
    for (size_t i = 0; i < face_count; ++i) {
      const TopoDS_Shape* face_shape = nullptr;
      status = LookupTyped(context, faces[i], TopAbs_FACE, &face_shape);
      if (status != AICAD_OCCT_OK) {
        return status;
      }
      builder.Add(group, *face_shape);
    }
    ShapeUpgrade_UnifySameDomain unifier(group, /*UnifyEdges=*/Standard_True,
                                          /*UnifyFaces=*/Standard_True,
                                          /*ConcatBSplines=*/Standard_False);
    unifier.Build();
    *out_handle = context->shapes.Insert(context->id, unifier.Shape());
    return AICAD_OCCT_OK;
  } catch (const Standard_Failure&) {
    return AICAD_OCCT_ERR_OPERATION_FAILED;
  } catch (...) {
    return AICAD_OCCT_ERR_INTERNAL;
  }
}

}  // extern "C"
