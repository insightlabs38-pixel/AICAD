// AICAD-016: native C ABI boundary tests.
//
// Exercises the Stage-1 Batch-1A checkpoint properties directly against
// the C ABI (no Rust involved yet -- that is AICAD-017/018): invalid
// handles rejected, stale handles rejected, foreign-context handles
// rejected, released slots do not alias newly created geometry, and a
// deliberately invalid argument is reported as a status code rather
// than crashing or throwing across the boundary.

#include "aicad_occt_bridge.h"

#include <cstdio>
#include <cstdlib>

namespace {

int g_failures = 0;

void check(bool condition, const char* what) {
    if (!condition) {
        std::fprintf(stderr, "FAIL: %s\n", what);
        g_failures += 1;
    } else {
        std::fprintf(stdout, "ok: %s\n", what);
    }
}

} // namespace

int main() {
    AicadKernelContext* ctx1 = nullptr;
    check(aicad_context_create(&ctx1) == AICAD_STATUS_OK, "context_create ctx1 succeeds");
    check(ctx1 != nullptr, "ctx1 is non-null after create");

    AicadKernelContext* ctx2 = nullptr;
    check(aicad_context_create(&ctx2) == AICAD_STATUS_OK, "context_create ctx2 succeeds");

    // Basic construction + query round-trip: a 2x3x4 box has volume 24.
    AicadShapeHandle box1{};
    check(aicad_create_box(ctx1, 2.0, 3.0, 4.0, &box1) == AICAD_STATUS_OK, "create_box succeeds");
    double volume = -1.0;
    check(aicad_shape_volume(ctx1, box1, &volume) == AICAD_STATUS_OK, "shape_volume succeeds");
    check(volume > 23.999 && volume < 24.001, "box volume is 24 within tolerance");

    // Invalid argument: non-positive dimension must be rejected as a
    // structured status, not undefined behavior or a thrown exception
    // crossing the ABI.
    AicadShapeHandle bad{};
    check(
        aicad_create_box(ctx1, -1.0, 1.0, 1.0, &bad) == AICAD_STATUS_INVALID_ARGUMENT,
        "create_box rejects a non-positive dimension"
    );
    check(
        aicad_create_box(nullptr, 1.0, 1.0, 1.0, &bad) == AICAD_STATUS_INVALID_ARGUMENT,
        "create_box rejects a null context"
    );

    // Invalid handle: a slot index that has never been allocated.
    AicadShapeHandle invalid_slot{box1.context_id, 9999, 1};
    double unused_volume = 0.0;
    check(
        aicad_shape_volume(ctx1, invalid_slot, &unused_volume) == AICAD_STATUS_INVALID_HANDLE,
        "shape_volume rejects an out-of-range slot as INVALID_HANDLE"
    );

    // Foreign-context handle: box1 belongs to ctx1, presented to ctx2.
    check(
        aicad_shape_volume(ctx2, box1, &unused_volume) == AICAD_STATUS_FOREIGN_CONTEXT,
        "shape_volume rejects a foreign-context handle as FOREIGN_CONTEXT"
    );

    // Stale handle + no aliasing: destroy box1, then create a second box
    // in ctx1. The freed slot may be reused, but the new handle's
    // generation must differ from the old one, so the old handle must
    // not resolve to the new shape.
    check(aicad_shape_destroy(ctx1, box1) == AICAD_STATUS_OK, "shape_destroy succeeds");
    check(
        aicad_shape_volume(ctx1, box1, &unused_volume) == AICAD_STATUS_STALE_HANDLE,
        "shape_volume rejects the just-destroyed handle as STALE_HANDLE"
    );

    AicadShapeHandle box2{};
    check(aicad_create_box(ctx1, 5.0, 5.0, 5.0, &box2) == AICAD_STATUS_OK, "create_box (reuse) succeeds");
    check(
        !(box1.slot == box2.slot && box1.generation == box2.generation),
        "reused slot's new handle differs from the destroyed handle"
    );
    check(
        aicad_shape_volume(ctx1, box1, &unused_volume) == AICAD_STATUS_STALE_HANDLE,
        "the old (pre-reuse) handle still does not resolve after reuse -- no aliasing"
    );
    double box2_volume = -1.0;
    check(aicad_shape_volume(ctx1, box2, &box2_volume) == AICAD_STATUS_OK, "new handle resolves correctly");
    check(box2_volume > 124.999 && box2_volume < 125.001, "reused-slot box volume is 125 within tolerance");

    // Double-destroy of an already-freed handle must not crash and must
    // be reported as stale, not silently accepted.
    check(
        aicad_shape_destroy(ctx1, box1) == AICAD_STATUS_STALE_HANDLE,
        "destroying an already-destroyed handle reports STALE_HANDLE"
    );

    check(aicad_context_destroy(ctx1) == AICAD_STATUS_OK, "context_destroy ctx1 succeeds");
    check(aicad_context_destroy(ctx2) == AICAD_STATUS_OK, "context_destroy ctx2 succeeds");
    check(aicad_context_destroy(nullptr) == AICAD_STATUS_OK, "context_destroy(NULL) is a no-op");

    if (g_failures == 0) {
        std::fprintf(stdout, "AICAD_BRIDGE_ABI_TESTS_OK\n");
        return 0;
    }
    std::fprintf(stderr, "%d check(s) failed\n", g_failures);
    return 1;
}
