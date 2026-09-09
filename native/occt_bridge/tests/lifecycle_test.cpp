// AICAD-019: kernel context lifecycle and shape-handle table stress
// tests.
//
// AICAD-016 already proved the shape table's core safety properties
// (stale-handle rejection, non-aliasing on slot reuse, foreign-context
// rejection) with single-shot cases in abi_boundary_test.cpp. This file
// extends that evidence in three ways the earlier task did not cover:
//
//  1. free-list reuse actually keeps the table's slot count bounded
//     under many create/release cycles, rather than growing unboundedly;
//  2. a slot's generation counter stays correct (monotonic, never
//     repeating) across many reuse cycles, not just one;
//  3. multiple kernel contexts are safe to use concurrently from
//     multiple real OS threads, each confined to its own context, which
//     is the concurrency pattern the single-thread-affine design (Stage-1
//     kernel policy #9) is actually meant to support.
//
// Run under valgrind (see native/occt_bridge/README.md) for leak/error
// evidence covering the full context-create -> many shape cycles ->
// context-destroy lifecycle.

#include "aicad_occt_bridge.h"

#include <atomic>
#include <cstdio>
#include <cstdlib>
#include <set>
#include <thread>
#include <vector>

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

// Bumped atomically from worker threads in the concurrency test below;
// plain int is fine everywhere else since the rest of this file is
// single-threaded.
std::atomic<int> g_thread_failures{0};

void CheckFromThread(bool condition, const char* what) {
  if (!condition) {
    std::fprintf(stderr, "FAIL (thread): %s\n", what);
    g_thread_failures.fetch_add(1, std::memory_order_relaxed);
  }
}

}  // namespace

int main() {
  // --- 1. Free-list reuse keeps the slot count bounded ---
  {
    aicad_occt_context_t* ctx = nullptr;
    Check(aicad_occt_context_create(&ctx) == AICAD_OCCT_OK, "lifecycle: context_create succeeds");

    constexpr int kIterations = 1000;
    uint32_t max_slot_seen = 0;
    for (int i = 0; i < kIterations; ++i) {
      aicad_shape_handle_t handle = AICAD_NULL_SHAPE_HANDLE;
      if (aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &handle) != AICAD_OCCT_OK) {
        Check(false, "lifecycle: create_box succeeds in free-list reuse loop");
        break;
      }
      if (handle.slot > max_slot_seen) {
        max_slot_seen = handle.slot;
      }
      if (aicad_occt_release_shape(ctx, handle) != AICAD_OCCT_OK) {
        Check(false, "lifecycle: release_shape succeeds in free-list reuse loop");
        break;
      }
    }
    // Only one shape is ever alive at a time in this loop, so a working
    // free list should never need more than slot 0 after the first
    // iteration. A slot count that grew with every iteration (no reuse)
    // would instead reach kIterations - 1.
    std::printf("lifecycle: max slot index seen across %d create/release cycles = %u\n", kIterations, max_slot_seen);
    Check(max_slot_seen == 0, "lifecycle: free list reuses slot 0 instead of growing the table unboundedly");

    Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "lifecycle: context_destroy succeeds");
  }

  // --- 2. Generation counter stays correct across many reuse cycles ---
  {
    aicad_occt_context_t* ctx = nullptr;
    Check(aicad_occt_context_create(&ctx) == AICAD_OCCT_OK, "generation: context_create succeeds");

    constexpr int kIterations = 1000;
    std::set<uint32_t> seen_generations;
    uint32_t previous_generation = 0;
    bool monotonic = true;
    for (int i = 0; i < kIterations; ++i) {
      aicad_shape_handle_t handle = AICAD_NULL_SHAPE_HANDLE;
      if (aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &handle) != AICAD_OCCT_OK) {
        Check(false, "generation: create_box succeeds in generation loop");
        break;
      }
      if (i > 0 && handle.generation <= previous_generation) {
        monotonic = false;
      }
      previous_generation = handle.generation;
      seen_generations.insert(handle.generation);
      if (aicad_occt_release_shape(ctx, handle) != AICAD_OCCT_OK) {
        Check(false, "generation: release_shape succeeds in generation loop");
        break;
      }
    }
    Check(monotonic, "generation: generation strictly increases across every reuse of the same slot");
    Check(seen_generations.size() == static_cast<size_t>(kIterations),
          "generation: every one of the 1000 cycles produced a distinct generation value (no repeats)");

    Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "generation: context_destroy succeeds");
  }

  // --- 3. Multiple contexts are safe under real concurrent use ---
  {
    constexpr int kThreadCount = 8;
    constexpr int kShapesPerThread = 100;
    std::vector<std::thread> threads;
    threads.reserve(kThreadCount);

    for (int t = 0; t < kThreadCount; ++t) {
      threads.emplace_back([t]() {
        aicad_occt_context_t* ctx = nullptr;
        CheckFromThread(aicad_occt_context_create(&ctx) == AICAD_OCCT_OK, "concurrency: context_create succeeds on worker thread");
        if (ctx == nullptr) {
          return;
        }

        std::vector<aicad_shape_handle_t> handles;
        handles.reserve(kShapesPerThread);
        for (int i = 0; i < kShapesPerThread; ++i) {
          // Distinct dimensions per (thread, iteration) so a
          // cross-thread aliasing bug would show up as a wrong volume,
          // not just a crash.
          const double side = 1.0 + static_cast<double>(t) * 0.01 + static_cast<double>(i) * 0.0001;
          aicad_shape_handle_t handle = AICAD_NULL_SHAPE_HANDLE;
          if (aicad_occt_create_box(ctx, side, side, side, &handle) != AICAD_OCCT_OK) {
            CheckFromThread(false, "concurrency: create_box succeeds on worker thread");
            continue;
          }
          double volume = 0.0;
          if (aicad_occt_shape_volume(ctx, handle, &volume) != AICAD_OCCT_OK) {
            CheckFromThread(false, "concurrency: shape_volume succeeds on worker thread");
            continue;
          }
          const double expected = side * side * side;
          CheckFromThread(volume > expected - 1e-6 && volume < expected + 1e-6,
                           "concurrency: volume matches this thread's own box dimensions (no cross-thread aliasing)");
          handles.push_back(handle);
        }

        for (const aicad_shape_handle_t& handle : handles) {
          CheckFromThread(aicad_occt_release_shape(ctx, handle) == AICAD_OCCT_OK,
                           "concurrency: release_shape succeeds on worker thread");
        }

        CheckFromThread(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "concurrency: context_destroy succeeds on worker thread");
      });
    }

    for (std::thread& thread : threads) {
      thread.join();
    }

    Check(g_thread_failures.load() == 0, "concurrency: zero failures across all worker threads");
    std::printf("concurrency: %d threads x %d shapes/thread completed\n", kThreadCount, kShapesPerThread);
  }

  if (g_failures > 0) {
    std::fprintf(stderr, "lifecycle_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("lifecycle_test: all checks PASSED\n");
  return 0;
}
