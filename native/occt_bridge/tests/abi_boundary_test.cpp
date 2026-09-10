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

  // --- Context lifetime safety (owner-authorized fix; see
  // project/reports/reviews/STAGE1-INDEPENDENT-REVIEW.md's "Context
  // lifetime safety fix" section for the design this regresses). `ctx`
  // above is now destroyed but its pointer value is deliberately reused
  // below: this bridge's own contract is that this must always fail
  // cleanly, never crash, never corrupt state -- not merely "usually
  // work in practice". ---

  // create -> destroy -> destroy: a second destroy of the same
  // formerly-valid context must not access freed memory (this bridge
  // never frees a context's own memory at all) and must be rejected
  // deterministically, not treated as a silent no-op or a crash.
  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "destroying an already-destroyed context is rejected cleanly, not a double-free");

  // create -> destroy -> ordinary operation: using a destroyed context
  // for a normal call must fail cleanly, never dereference freed memory.
  aicad_shape_handle_t ignored_after_destroy = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &ignored_after_destroy) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "create_box on a destroyed context is rejected cleanly, not a crash");

  // stale context with a shape/resource operation, and resource release
  // after context destruction: `reused_slot_handle` was a real, valid
  // handle in `ctx` before `ctx` was destroyed above.
  Check(aicad_occt_shape_volume(ctx, reused_slot_handle, &volume) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "a shape query against a destroyed context is rejected cleanly, not a crash");
  Check(aicad_occt_release_shape(ctx, reused_slot_handle) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "releasing a shape whose context has been destroyed is rejected cleanly, not a crash");

  // two independent contexts: destroying one must never affect another
  // still-live context's own usability.
  aicad_occt_context_t* live_ctx = nullptr;
  aicad_occt_context_t* doomed_ctx = nullptr;
  Check(aicad_occt_context_create(&live_ctx) == AICAD_OCCT_OK, "live_ctx context_create succeeds");
  Check(aicad_occt_context_create(&doomed_ctx) == AICAD_OCCT_OK, "doomed_ctx context_create succeeds");
  Check(aicad_occt_context_destroy(doomed_ctx) == AICAD_OCCT_OK, "doomed_ctx context_destroy succeeds");
  aicad_shape_handle_t live_box = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_create_box(live_ctx, 2.0, 2.0, 2.0, &live_box) == AICAD_OCCT_OK,
        "live_ctx remains fully usable after an unrelated context was destroyed");
  double live_volume = 0.0;
  Check(aicad_occt_shape_volume(live_ctx, live_box, &live_volume) == AICAD_OCCT_OK &&
            live_volume > 7.999999 && live_volume < 8.000001,
        "live_ctx's own shape is unaffected by doomed_ctx's destruction");
  // ...and a handle from the still-live context used against the
  // destroyed one is rejected as the context itself being invalid, not
  // as a crash or a misleading FOREIGN_CONTEXT/OK result.
  Check(aicad_occt_shape_volume(doomed_ctx, live_box, &live_volume) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "using a live handle against a destroyed context is rejected cleanly, not mistaken for live");
  Check(aicad_occt_context_destroy(live_ctx) == AICAD_OCCT_OK, "live_ctx context_destroy succeeds");

  // repeated create/destroy cycles: must never crash, corrupt state, or
  // let a stale pointer from an earlier cycle alias a later cycle's
  // context (this bridge never reuses a context's own memory for a
  // different context, precisely to make that aliasing impossible).
  {
    constexpr int kCycles = 2000;
    aicad_occt_context_t* first_cycle_ctx = nullptr;
    Check(aicad_occt_context_create(&first_cycle_ctx) == AICAD_OCCT_OK,
          "repeated-cycle: first context_create succeeds");
    Check(aicad_occt_context_destroy(first_cycle_ctx) == AICAD_OCCT_OK,
          "repeated-cycle: first context_destroy succeeds");
    bool all_cycles_ok = true;
    for (int i = 0; i < kCycles; ++i) {
      aicad_occt_context_t* cycle_ctx = nullptr;
      if (aicad_occt_context_create(&cycle_ctx) != AICAD_OCCT_OK ||
          aicad_occt_context_destroy(cycle_ctx) != AICAD_OCCT_OK) {
        all_cycles_ok = false;
        break;
      }
    }
    Check(all_cycles_ok, "repeated-cycle: 2000 create/destroy cycles all succeed cleanly");
    // The very first cycle's now-long-destroyed pointer must still be
    // safely (not crash-ily) rejected after thousands of unrelated
    // contexts have since been created and destroyed.
    aicad_shape_handle_t ignored_stale = AICAD_NULL_SHAPE_HANDLE;
    Check(aicad_occt_create_box(first_cycle_ctx, 1.0, 1.0, 1.0, &ignored_stale) ==
              AICAD_OCCT_ERR_INVALID_ARGUMENT,
          "repeated-cycle: the very first (long-destroyed) context is still safely rejected, "
          "never aliased onto a later cycle's context");
  }

  if (g_failures > 0) {
    std::fprintf(stderr, "abi_boundary_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("abi_boundary_test: all checks PASSED\n");
  return 0;
}
