// AICAD-016: native/occt_bridge C ABI boundary implementation.
//
// Every function exported from aicad_occt_bridge.h is implemented here
// with a try/catch that never lets a C++ exception (OCCT's
// Standard_Failure hierarchy included) escape across the ABI boundary
// (Stage-1 kernel policy #6/#7). OCCT types (TopoDS_Shape, etc.) live
// only inside this translation unit and inside AicadKernelContext's
// definition below -- never in the public header.

#include "aicad_occt_bridge.h"

#include <BRepGProp.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <GProp_GProps.hxx>
#include <Standard_Failure.hxx>
#include <TopoDS_Shape.hxx>

#include <atomic>
#include <exception>
#include <vector>

namespace {

struct Slot {
    TopoDS_Shape shape;
    uint32_t generation = 0;
    bool occupied = false;
};

std::atomic<uint64_t> g_next_context_id{1};

bool handle_is_zero(const AicadShapeHandle& h) {
    return h.context_id == 0 && h.slot == 0 && h.generation == 0;
}

} // namespace

// Defines the opaque AicadKernelContext declared in the public header.
// Single-thread-affine per Stage-1 kernel policy #9: no internal locking.
struct AicadKernelContext {
    uint64_t context_id;
    std::vector<Slot> slots;
    std::vector<uint32_t> free_list;
};

namespace {

// Resolves handle against ctx, returning the live Slot* or nullptr with
// *out_status set to the specific rejection reason. Never throws.
Slot* resolve(AicadKernelContext* ctx, AicadShapeHandle handle, AicadStatus* out_status) {
    if (handle.context_id != ctx->context_id) {
        *out_status = AICAD_STATUS_FOREIGN_CONTEXT;
        return nullptr;
    }
    if (handle.slot >= ctx->slots.size()) {
        *out_status = AICAD_STATUS_INVALID_HANDLE;
        return nullptr;
    }
    Slot& slot = ctx->slots[handle.slot];
    if (!slot.occupied || slot.generation != handle.generation) {
        *out_status = AICAD_STATUS_STALE_HANDLE;
        return nullptr;
    }
    *out_status = AICAD_STATUS_OK;
    return &slot;
}

AicadShapeHandle store_shape(AicadKernelContext* ctx, TopoDS_Shape shape) {
    if (!ctx->free_list.empty()) {
        uint32_t index = ctx->free_list.back();
        ctx->free_list.pop_back();
        Slot& slot = ctx->slots[index];
        slot.shape = std::move(shape);
        slot.occupied = true;
        // generation was already advanced when this slot was freed, so
        // any handle issued before that free is rejected as stale
        // rather than aliasing this new shape.
        return AicadShapeHandle{ctx->context_id, index, slot.generation};
    }
    Slot slot;
    slot.shape = std::move(shape);
    slot.generation = 1;
    slot.occupied = true;
    ctx->slots.push_back(std::move(slot));
    uint32_t index = static_cast<uint32_t>(ctx->slots.size() - 1);
    return AicadShapeHandle{ctx->context_id, index, ctx->slots[index].generation};
}

} // namespace

extern "C" {

AicadStatus aicad_context_create(AicadKernelContext** out_ctx) {
    if (out_ctx == nullptr) {
        return AICAD_STATUS_INVALID_ARGUMENT;
    }
    try {
        AicadKernelContext* ctx = new AicadKernelContext();
        ctx->context_id = g_next_context_id.fetch_add(1, std::memory_order_relaxed);
        *out_ctx = ctx;
        return AICAD_STATUS_OK;
    } catch (const std::exception&) {
        return AICAD_STATUS_UNKNOWN_ERROR;
    } catch (...) {
        return AICAD_STATUS_UNKNOWN_ERROR;
    }
}

AicadStatus aicad_context_destroy(AicadKernelContext* ctx) {
    if (ctx == nullptr) {
        return AICAD_STATUS_OK;
    }
    try {
        delete ctx;
        return AICAD_STATUS_OK;
    } catch (const std::exception&) {
        return AICAD_STATUS_UNKNOWN_ERROR;
    } catch (...) {
        return AICAD_STATUS_UNKNOWN_ERROR;
    }
}

AicadStatus aicad_create_box(
    AicadKernelContext* ctx,
    double dx,
    double dy,
    double dz,
    AicadShapeHandle* out_handle
) {
    if (ctx == nullptr || out_handle == nullptr) {
        return AICAD_STATUS_INVALID_ARGUMENT;
    }
    if (!(dx > 0.0) || !(dy > 0.0) || !(dz > 0.0)) {
        return AICAD_STATUS_INVALID_ARGUMENT;
    }
    try {
        BRepPrimAPI_MakeBox make_box(dx, dy, dz);
        make_box.Build();
        if (!make_box.IsDone()) {
            return AICAD_STATUS_NATIVE_EXCEPTION;
        }
        *out_handle = store_shape(ctx, make_box.Shape());
        return AICAD_STATUS_OK;
    } catch (const Standard_Failure&) {
        return AICAD_STATUS_NATIVE_EXCEPTION;
    } catch (const std::exception&) {
        return AICAD_STATUS_UNKNOWN_ERROR;
    } catch (...) {
        return AICAD_STATUS_UNKNOWN_ERROR;
    }
}

AicadStatus aicad_shape_volume(
    AicadKernelContext* ctx,
    AicadShapeHandle handle,
    double* out_volume
) {
    if (ctx == nullptr || out_volume == nullptr) {
        return AICAD_STATUS_INVALID_ARGUMENT;
    }
    if (handle_is_zero(handle)) {
        return AICAD_STATUS_INVALID_HANDLE;
    }
    AicadStatus status = AICAD_STATUS_OK;
    Slot* slot = resolve(ctx, handle, &status);
    if (slot == nullptr) {
        return status;
    }
    try {
        GProp_GProps props;
        BRepGProp::VolumeProperties(slot->shape, props);
        *out_volume = props.Mass();
        return AICAD_STATUS_OK;
    } catch (const Standard_Failure&) {
        return AICAD_STATUS_NATIVE_EXCEPTION;
    } catch (const std::exception&) {
        return AICAD_STATUS_UNKNOWN_ERROR;
    } catch (...) {
        return AICAD_STATUS_UNKNOWN_ERROR;
    }
}

AicadStatus aicad_shape_destroy(AicadKernelContext* ctx, AicadShapeHandle handle) {
    if (ctx == nullptr) {
        return AICAD_STATUS_INVALID_ARGUMENT;
    }
    if (handle_is_zero(handle)) {
        return AICAD_STATUS_INVALID_HANDLE;
    }
    AicadStatus status = AICAD_STATUS_OK;
    Slot* slot = resolve(ctx, handle, &status);
    if (slot == nullptr) {
        return status;
    }
    try {
        slot->shape = TopoDS_Shape();
        slot->occupied = false;
        slot->generation += 1;
        ctx->free_list.push_back(handle.slot);
        return AICAD_STATUS_OK;
    } catch (const std::exception&) {
        return AICAD_STATUS_UNKNOWN_ERROR;
    } catch (...) {
        return AICAD_STATUS_UNKNOWN_ERROR;
    }
}

} // extern "C"
