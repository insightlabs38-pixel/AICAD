// AICAD-016: C ABI boundary smoke test.
//
// Exercises the ABI end-to-end (create/query/release), and specifically
// the adversarial/safety cases the Stage-1A gate
// (project/gates/STAGE1-A_KERNEL_BOUNDARY.md) will require evidence for:
// invalid handles rejected, stale (released) handles rejected,
// foreign-context handles rejected, and an OCCT-side exception
// (degenerate zero-dimension box) contained rather than escaping/crashing.
// The deeper epoch/generation semantics behind these same properties are
// AICAD-019's job; this test only proves AICAD-016's minimal registry
// already gets the observable behavior right.

#include <cmath>
#include <cstdio>
#include <cstdlib>

#include "aicad_occt_bridge.h"

namespace {

int g_failures = 0;

void expect(bool condition, const char *what) {
  if (!condition) {
    std::fprintf(stderr, "FAIL: %s\n", what);
    ++g_failures;
  } else {
    std::printf("PASS: %s\n", what);
  }
}

void expect_status(AicadStatus actual, AicadStatus expected, const char *what) {
  if (actual != expected) {
    std::fprintf(stderr, "FAIL: %s (expected status %d, got %d)\n", what,
                 static_cast<int>(expected), static_cast<int>(actual));
    ++g_failures;
  } else {
    std::printf("PASS: %s\n", what);
  }
}

} // namespace

int main() {
  // 1. Ordinary round-trip: create -> query -> release.
  AicadKernelContext *ctx_a = aicad_kernel_context_create();
  expect(ctx_a != nullptr, "context A created");

  AicadShapeHandle box_handle{0};
  AicadStatus status =
      aicad_create_box(ctx_a, 10.0, 20.0, 30.0, &box_handle);
  expect_status(status, AICAD_STATUS_OK, "create_box(10,20,30) on context A succeeds");
  expect(box_handle.id != 0, "create_box produced a non-zero handle id");

  double diagonal = 0.0;
  status = aicad_shape_bbox_diagonal(ctx_a, box_handle, &diagonal);
  expect_status(status, AICAD_STATUS_OK, "bbox_diagonal on the live handle succeeds");
  const double expected_diagonal = std::sqrt(10.0 * 10.0 + 20.0 * 20.0 + 30.0 * 30.0);
  expect(std::fabs(diagonal - expected_diagonal) < 1e-9,
         "bbox_diagonal matches the analytic expectation");

  // 2. Invalid handle: never issued (id 0 is never valid).
  AicadShapeHandle never_issued{0};
  double unused = 0.0;
  status = aicad_shape_bbox_diagonal(ctx_a, never_issued, &unused);
  expect_status(status, AICAD_STATUS_INVALID_HANDLE,
                "handle id 0 (never issued) is rejected");

  // 3. Stale handle: release, then reuse.
  status = aicad_shape_release(ctx_a, box_handle);
  expect_status(status, AICAD_STATUS_OK, "release of a live handle succeeds");
  status = aicad_shape_bbox_diagonal(ctx_a, box_handle, &unused);
  expect_status(status, AICAD_STATUS_INVALID_HANDLE,
                "querying a released (stale) handle is rejected, not aliased");
  status = aicad_shape_release(ctx_a, box_handle);
  expect_status(status, AICAD_STATUS_INVALID_HANDLE,
                "double-release of the same handle is rejected");

  // 4. A freshly created shape never reuses a just-released handle id
  //    (process-wide monotonic counter, not a per-context free list).
  AicadShapeHandle second_handle{0};
  status = aicad_create_box(ctx_a, 1.0, 1.0, 1.0, &second_handle);
  expect_status(status, AICAD_STATUS_OK, "second create_box on context A succeeds");
  expect(second_handle.id != box_handle.id,
         "a new handle never reuses a just-released handle's id");

  // 5. Foreign-context handle: a handle minted on context A is rejected
  //    by context B, not silently aliased to whatever id B happens to
  //    have internally.
  AicadKernelContext *ctx_b = aicad_kernel_context_create();
  expect(ctx_b != nullptr, "context B created");
  status = aicad_shape_bbox_diagonal(ctx_b, second_handle, &unused);
  expect_status(status, AICAD_STATUS_INVALID_HANDLE,
                "a handle minted on context A is rejected by context B");

  // Also create a shape on B and confirm B's own handle ids never
  // collide with A's (proving the shared global counter, not just this
  // particular rejection case).
  AicadShapeHandle b_handle{0};
  status = aicad_create_box(ctx_b, 5.0, 5.0, 5.0, &b_handle);
  expect_status(status, AICAD_STATUS_OK, "create_box on context B succeeds");
  expect(b_handle.id != second_handle.id && b_handle.id != box_handle.id,
         "context B's handle id never collides with context A's handle ids");

  // 6. Null-context calls are rejected cleanly, not crashed.
  status = aicad_create_box(nullptr, 1.0, 1.0, 1.0, &second_handle);
  expect_status(status, AICAD_STATUS_NULL_CONTEXT,
                "create_box(nullptr, ...) is rejected, not a crash");

  // 7. Adversarial case: a degenerate (zero-dimension) box throws
  //    Standard_DomainError deep inside OCCT (independently verified
  //    before writing this bridge, see project/reports/AICAD-016.md) —
  //    confirm it is caught and normalized, never crashes the process
  //    or leaves an unhandled exception.
  AicadShapeHandle degenerate_handle{0};
  status = aicad_create_box(ctx_a, 0.0, 20.0, 30.0, &degenerate_handle);
  expect_status(status, AICAD_STATUS_KERNEL_INTERNAL_ERROR,
                "a degenerate zero-dimension box is reported as a "
                "contained kernel error, not a crash");
  const char *error_text = aicad_kernel_context_last_error(ctx_a);
  expect(error_text != nullptr && error_text[0] != '\0',
         "last_error is populated after the degenerate-box failure");
  std::printf("INFO: degenerate-box last_error = \"%s\"\n", error_text);

  // 8. Context destruction cleans up outstanding shapes without crashing
  //    (ctx_b still owns b_handle at this point).
  aicad_kernel_context_destroy(ctx_a);
  aicad_kernel_context_destroy(ctx_b);
  aicad_kernel_context_destroy(nullptr); // must be a safe no-op.
  std::printf("PASS: destroying contexts with outstanding shapes did not crash\n");

  if (g_failures == 0) {
    std::printf("ABI_SMOKE_TEST_RESULT=PASS\n");
    return 0;
  }
  std::fprintf(stderr, "ABI_SMOKE_TEST_RESULT=FAIL (%d failure(s))\n", g_failures);
  return 1;
}
