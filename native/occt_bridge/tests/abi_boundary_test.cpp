// AICAD-016: C ABI boundary tests.
//
// Deliberately includes ONLY the public header, exactly as
// crates/cad-occt-bridge will see it -- this test exercises the ABI
// boundary itself (status codes, handle validity/staleness/foreign-
// context/thread-affinity rejection), not OCCT internals (those are
// covered by the create_box/is_valid/volume correctness checks in
// probe/occt_probe.cpp).
//
// No test framework dependency: each check prints PASS/FAIL and the
// process exits non-zero on the first failure, which is enough for
// CTest to report pass/fail and for a human/agent to see exactly which
// check failed.

#include "aicad_occt_bridge.h"

#include <cstdio>
#include <cstdlib>
#include <thread>

namespace {

int g_failures = 0;

void Check(bool condition, const char* what) {
  if (condition) {
    std::printf("PASS: %s\n", what);
  } else {
    std::printf("FAIL: %s\n", what);
    g_failures += 1;
  }
}

}  // namespace

int main() {
  aicad_occt_context_t* ctx = nullptr;
  Check(aicad_occt_context_create(&ctx) == AICAD_OCCT_OK && ctx != nullptr,
        "context_create succeeds");

  // --- create_box + is_valid + volume: the happy path ---
  aicad_shape_handle_t box_handle = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_create_box(ctx, 1.0, 2.0, 3.0, &box_handle) == AICAD_OCCT_OK,
        "create_box succeeds for positive dimensions");

  int is_valid = 0;
  Check(aicad_occt_shape_is_valid(ctx, box_handle, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "is_valid reports the box as valid");

  double volume = 0.0;
  Check(aicad_occt_shape_volume(ctx, box_handle, &volume) == AICAD_OCCT_OK,
        "shape_volume succeeds for a valid handle");
  Check(volume > 5.999999 && volume < 6.000001, "box volume is exactly 6.0 within tolerance");

  // --- invalid argument rejection ---
  aicad_shape_handle_t unused_handle = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_create_box(ctx, 0.0, 1.0, 1.0, &unused_handle) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "create_box rejects a zero dimension");
  Check(aicad_occt_create_box(ctx, -1.0, 1.0, 1.0, &unused_handle) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "create_box rejects a negative dimension");
  Check(aicad_occt_context_create(nullptr) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "context_create rejects a null out-param");

  // --- invalid handle: a slot index that was never allocated ---
  aicad_shape_handle_t never_allocated = box_handle;
  never_allocated.slot += 1000;
  Check(aicad_occt_shape_volume(ctx, never_allocated, &volume) == AICAD_OCCT_ERR_INVALID_HANDLE,
        "an out-of-range slot is rejected as INVALID_HANDLE");

  // --- stale handle: release, then reuse the released handle ---
  aicad_shape_handle_t released = box_handle;
  Check(aicad_occt_release_shape(ctx, released) == AICAD_OCCT_OK, "release_shape succeeds once");
  Check(aicad_occt_shape_volume(ctx, released, &volume) == AICAD_OCCT_ERR_STALE_HANDLE,
        "using a released handle is rejected as STALE_HANDLE");
  Check(aicad_occt_release_shape(ctx, released) == AICAD_OCCT_ERR_STALE_HANDLE,
        "releasing an already-released handle is rejected as STALE_HANDLE, not a double-free");

  // --- stale handle does not alias a new shape reusing the same slot ---
  aicad_shape_handle_t reused_slot_handle = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_create_box(ctx, 5.0, 5.0, 5.0, &reused_slot_handle) == AICAD_OCCT_OK,
        "create_box succeeds and is expected to reuse the just-freed slot");
  Check(reused_slot_handle.slot == released.slot,
        "test setup: the new box did reuse the released slot (otherwise this case is not exercised)");
  Check(reused_slot_handle.generation != released.generation,
        "the reused slot's new handle has a different generation than the stale one");
  Check(aicad_occt_shape_volume(ctx, released, &volume) == AICAD_OCCT_ERR_STALE_HANDLE,
        "the OLD (pre-release) handle value still does not resolve to the NEW shape in the same slot");
  double reused_volume = 0.0;
  Check(aicad_occt_shape_volume(ctx, reused_slot_handle, &reused_volume) == AICAD_OCCT_OK &&
            reused_volume > 124.999999 && reused_volume < 125.000001,
        "the NEW handle correctly resolves to the new 5x5x5 box (volume 125)");

  // --- foreign-context handle rejection ---
  aicad_occt_context_t* other_ctx = nullptr;
  Check(aicad_occt_context_create(&other_ctx) == AICAD_OCCT_OK, "second context_create succeeds");
  Check(aicad_occt_shape_volume(other_ctx, reused_slot_handle, &volume) == AICAD_OCCT_ERR_FOREIGN_CONTEXT,
        "a handle from context A used against context B is rejected as FOREIGN_CONTEXT");
  Check(aicad_occt_context_destroy(other_ctx) == AICAD_OCCT_OK, "second context_destroy succeeds");

  // --- wrong-thread rejection: contexts are single-thread-affine ---
  aicad_occt_status_t wrong_thread_status = AICAD_OCCT_OK;
  std::thread other_thread([&]() {
    aicad_shape_handle_t ignored = AICAD_NULL_SHAPE_HANDLE;
    wrong_thread_status = aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &ignored);
  });
  other_thread.join();
  Check(wrong_thread_status == AICAD_OCCT_ERR_WRONG_THREAD,
        "calling a context from a thread other than its creator is rejected as WRONG_THREAD");

  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "context_destroy succeeds");

  if (g_failures > 0) {
    std::fprintf(stderr, "abi_boundary_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("abi_boundary_test: all checks PASSED\n");
  return 0;
}
