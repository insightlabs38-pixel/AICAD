// AICAD-016: C ABI boundary smoke test.
//
// Exercises native/occt_bridge's C ABI exactly as crates/cad-occt-bridge
// (AICAD-018) will — through aicad/occt_bridge.h only, never touching
// OCCT directly — to check the STAGE1-A checkpoint properties this task
// is responsible for: the ABI builds and runs, no OCCT type is needed by
// a caller, invalid handles are rejected, foreign-context handles are
// rejected, and adversarial (zero/negative/extreme-scale) dimensions are
// handled without crashing.
//
// Exit code 0 = all checks passed. Non-zero = at least one failed;
// stderr names which one(s).

#include "aicad/occt_bridge.h"

#include <cmath>
#include <cstdio>
#include <cstdlib>

namespace {

int g_failures = 0;

void Expect(bool condition, const char* description) {
  if (!condition) {
    std::fprintf(stderr, "FAIL: %s\n", description);
    ++g_failures;
  } else {
    std::printf("ok: %s\n", description);
  }
}

bool RelativelyClose(double actual, double expected, double relative_tolerance) {
  const double scale = std::fabs(expected) > 0.0 ? std::fabs(expected) : 1.0;
  return std::fabs(actual - expected) <= relative_tolerance * scale;
}

}  // namespace

int main() {
  AicadStatus status;

  // --- basic lifecycle + round trip ---
  AicadOcctContext* ctx1 = aicad_occt_context_create(&status);
  Expect(ctx1 != nullptr && status.code == AICAD_STATUS_OK, "context 1 created");

  AicadShapeHandle box1{};
  aicad_occt_create_box(ctx1, 10.0, 20.0, 30.0, &box1, &status);
  Expect(status.code == AICAD_STATUS_OK, "create_box(10,20,30) on ctx1 succeeds");

  double volume = 0.0;
  aicad_occt_shape_volume(ctx1, box1, &volume, &status);
  Expect(status.code == AICAD_STATUS_OK, "shape_volume(box1) succeeds");
  Expect(RelativelyClose(volume, 6000.0, 1e-9), "box1 volume matches 10*20*30 = 6000");

  // --- adversarial dimensions (AGENTS.md adversarial geometry cases) ---
  AicadShapeHandle bad_handle{};
  aicad_occt_create_box(ctx1, 0.0, 1.0, 1.0, &bad_handle, &status);
  Expect(status.code == AICAD_STATUS_INVALID_ARGUMENT, "zero dimension rejected");

  aicad_occt_create_box(ctx1, -5.0, 1.0, 1.0, &bad_handle, &status);
  Expect(status.code == AICAD_STATUS_INVALID_ARGUMENT, "negative dimension rejected");

  aicad_occt_create_box(ctx1, std::nan(""), 1.0, 1.0, &bad_handle, &status);
  Expect(status.code == AICAD_STATUS_INVALID_ARGUMENT, "NaN dimension rejected");

  AicadShapeHandle tiny_box{};
  aicad_occt_create_box(ctx1, 1e-6, 1e-6, 1e-6, &tiny_box, &status);
  Expect(status.code == AICAD_STATUS_OK, "very small box (1e-6 per side) succeeds");
  double tiny_volume = 0.0;
  aicad_occt_shape_volume(ctx1, tiny_box, &tiny_volume, &status);
  Expect(status.code == AICAD_STATUS_OK && RelativelyClose(tiny_volume, 1e-18, 1e-6),
         "very small box volume matches 1e-18 within relative tolerance");

  AicadShapeHandle huge_box{};
  aicad_occt_create_box(ctx1, 1e6, 1e6, 1e6, &huge_box, &status);
  Expect(status.code == AICAD_STATUS_OK, "very large box (1e6 per side) succeeds");
  double huge_volume = 0.0;
  aicad_occt_shape_volume(ctx1, huge_box, &huge_volume, &status);
  Expect(status.code == AICAD_STATUS_OK && RelativelyClose(huge_volume, 1e18, 1e-9),
         "very large box volume matches 1e18 within relative tolerance");

  // --- invalid / foreign-context handle rejection ---
  AicadOcctContext* ctx2 = aicad_occt_context_create(&status);
  Expect(ctx2 != nullptr && status.code == AICAD_STATUS_OK, "context 2 created");

  aicad_occt_shape_volume(ctx2, box1, &volume, &status);
  Expect(status.code == AICAD_STATUS_FOREIGN_CONTEXT_HANDLE,
         "box1's handle used against ctx2 is rejected as foreign-context");

  AicadShapeHandle out_of_range = box1;
  out_of_range.index = 9999;
  aicad_occt_shape_volume(ctx1, out_of_range, &volume, &status);
  Expect(status.code == AICAD_STATUS_INVALID_HANDLE,
         "out-of-range handle index on ctx1 is rejected as invalid");

  // --- null-context handling ---
  aicad_occt_create_box(nullptr, 1.0, 1.0, 1.0, &bad_handle, &status);
  Expect(status.code == AICAD_STATUS_INVALID_ARGUMENT, "null ctx on create_box rejected");
  aicad_occt_shape_volume(nullptr, box1, &volume, &status);
  Expect(status.code == AICAD_STATUS_INVALID_ARGUMENT, "null ctx on shape_volume rejected");

  // --- independent contexts stay independent ---
  AicadShapeHandle box2{};
  aicad_occt_create_box(ctx2, 1.0, 2.0, 3.0, &box2, &status);
  Expect(status.code == AICAD_STATUS_OK, "create_box on ctx2 succeeds independently of ctx1");
  double volume2 = 0.0;
  aicad_occt_shape_volume(ctx2, box2, &volume2, &status);
  Expect(status.code == AICAD_STATUS_OK && RelativelyClose(volume2, 6.0, 1e-9),
         "ctx2's box2 volume matches 1*2*3 = 6, independent of ctx1's shapes");

  // --- AICAD-019: shape-handle table (release + stale-handle rejection) ---
  AicadShapeHandle to_release{};
  aicad_occt_create_box(ctx1, 4.0, 5.0, 6.0, &to_release, &status);
  Expect(status.code == AICAD_STATUS_OK, "box to be released is created successfully");

  aicad_occt_release_shape(ctx1, to_release, &status);
  Expect(status.code == AICAD_STATUS_OK, "releasing a live handle succeeds");

  double released_volume = 0.0;
  aicad_occt_shape_volume(ctx1, to_release, &released_volume, &status);
  Expect(status.code == AICAD_STATUS_INVALID_HANDLE,
         "using a just-released handle is rejected as invalid/stale");

  aicad_occt_release_shape(ctx1, to_release, &status);
  Expect(status.code == AICAD_STATUS_INVALID_HANDLE,
         "releasing an already-released handle again is rejected, not a silent no-op");

  // The freed slot must be reusable, and the OLD handle to it must stay
  // rejected forever after — this is the core AGENTS.md non-negotiable
  // ("Released/stale handles must never accidentally alias newly
  // created geometry") made concrete.
  AicadShapeHandle reused{};
  aicad_occt_create_box(ctx1, 7.0, 8.0, 9.0, &reused, &status);
  Expect(status.code == AICAD_STATUS_OK, "a new box can reuse a released slot");
  Expect(reused.index == to_release.index,
         "the new box actually reused the released slot's index (free-list worked)");
  Expect(reused.generation != to_release.generation,
         "the reused slot's generation differs from the released handle's generation");

  double reused_volume = 0.0;
  aicad_occt_shape_volume(ctx1, reused, &reused_volume, &status);
  Expect(status.code == AICAD_STATUS_OK && RelativelyClose(reused_volume, 504.0, 1e-9),
         "the reused slot's volume matches the NEW box (7*8*9 = 504), not the old one");

  double stale_volume_after_reuse = 0.0;
  aicad_occt_shape_volume(ctx1, to_release, &stale_volume_after_reuse, &status);
  Expect(status.code == AICAD_STATUS_INVALID_HANDLE,
         "the old (pre-release) handle stays rejected even after its slot is reused, "
         "never silently aliasing the new box");

  aicad_occt_context_destroy(ctx1);
  aicad_occt_context_destroy(ctx2);
  aicad_occt_context_destroy(nullptr);  // must be a safe no-op

  if (g_failures > 0) {
    std::fprintf(stderr, "%d check(s) FAILED\n", g_failures);
    return EXIT_FAILURE;
  }
  std::printf("all checks passed\n");
  return EXIT_SUCCESS;
}
