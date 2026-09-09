// AICAD-020: box adversarial cases + cylinder.
//
// abi_boundary_test.cpp already covers create_box's happy path and basic
// zero/negative rejection as incidental setup for handle-safety tests;
// this file is the dedicated adversarial campaign for both primitives
// (Stage-1 kernel policy: zero/near-zero/very-small/very-large/mixed-scale
// dimensions), per the active scheduled-task brief's adversarial-cases
// list.

#include "aicad_occt_bridge.h"

#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <limits>

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

bool NearlyEqual(double a, double b, double tol) { return std::fabs(a - b) <= tol; }

}  // namespace

int main() {
  aicad_occt_context_t* ctx = nullptr;
  Check(aicad_occt_context_create(&ctx) == AICAD_OCCT_OK, "context_create succeeds");

  // --- box: very small dimensions ---
  aicad_shape_handle_t h = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_create_box(ctx, 1e-6, 1e-6, 1e-6, &h) == AICAD_OCCT_OK,
        "create_box succeeds for very small (1e-6) dimensions");
  int is_valid = 0;
  Check(aicad_occt_shape_is_valid(ctx, h, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "a very small box is still a valid B-rep");
  double volume = 0.0;
  Check(aicad_occt_shape_volume(ctx, h, &volume) == AICAD_OCCT_OK &&
            NearlyEqual(volume, 1e-18, 1e-24),
        "very small box has the expected (tiny) volume");

  // --- box: very large dimensions ---
  Check(aicad_occt_create_box(ctx, 1e9, 1e9, 1e9, &h) == AICAD_OCCT_OK,
        "create_box succeeds for very large (1e9) dimensions");
  Check(aicad_occt_shape_is_valid(ctx, h, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "a very large box is still a valid B-rep");
  Check(aicad_occt_shape_volume(ctx, h, &volume) == AICAD_OCCT_OK &&
            NearlyEqual(volume, 1e27, 1e27 * 1e-9),
        "very large box has the expected volume within relative tolerance");

  // --- box: mixed-scale dimensions (thin plate) ---
  Check(aicad_occt_create_box(ctx, 1000.0, 1000.0, 1e-6, &h) == AICAD_OCCT_OK,
        "create_box succeeds for mixed-scale (thin plate) dimensions");
  Check(aicad_occt_shape_is_valid(ctx, h, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "a mixed-scale thin-plate box is still a valid B-rep");

  // --- box: infinity/NaN rejection beyond the zero/negative cases
  //     abi_boundary_test.cpp already covers ---
  Check(aicad_occt_create_box(ctx, std::numeric_limits<double>::infinity(), 1.0, 1.0, &h) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "create_box rejects +infinity");
  Check(aicad_occt_create_box(ctx, 1.0, std::numeric_limits<double>::quiet_NaN(), 1.0, &h) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "create_box rejects NaN in a non-first argument too");

  // --- cylinder: happy path with an analytically-known volume/area ---
  aicad_shape_handle_t cyl = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_create_cylinder(ctx, 2.0, 5.0, &cyl) == AICAD_OCCT_OK,
        "create_cylinder succeeds for positive radius/height");
  Check(aicad_occt_shape_is_valid(ctx, cyl, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "a freshly created cylinder is a valid B-rep");
  double cyl_volume = 0.0;
  const double kPi = 3.14159265358979323846;
  const double expected_volume = kPi * 2.0 * 2.0 * 5.0;  // pi r^2 h
  Check(aicad_occt_shape_volume(ctx, cyl, &cyl_volume) == AICAD_OCCT_OK &&
            NearlyEqual(cyl_volume, expected_volume, expected_volume * 1e-6),
        "cylinder volume matches pi*r^2*h analytically");
  double cyl_area = 0.0;
  // Total surface area of a capped cylinder: 2*pi*r*h (lateral) + 2*pi*r^2 (two caps).
  const double expected_area = 2.0 * kPi * 2.0 * 5.0 + 2.0 * kPi * 2.0 * 2.0;
  Check(aicad_occt_shape_area(ctx, cyl, &cyl_area) == AICAD_OCCT_OK &&
            NearlyEqual(cyl_area, expected_area, expected_area * 1e-6),
        "cylinder surface area matches the analytic capped-cylinder formula");

  // --- cylinder: bounding box sanity (centered on origin, axis along +Z) ---
  double bb_min[3] = {0, 0, 0};
  double bb_max[3] = {0, 0, 0};
  Check(aicad_occt_shape_bounding_box(ctx, cyl, bb_min, bb_max) == AICAD_OCCT_OK &&
            NearlyEqual(bb_min[0], -2.0, 1e-6) && NearlyEqual(bb_max[0], 2.0, 1e-6) &&
            NearlyEqual(bb_min[2], 0.0, 1e-6) && NearlyEqual(bb_max[2], 5.0, 1e-6),
        "cylinder bounding box matches radius/height placed at the origin along +Z");

  // --- cylinder: adversarial dimensions ---
  Check(aicad_occt_create_cylinder(ctx, 0.0, 5.0, &cyl) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "create_cylinder rejects a zero radius");
  Check(aicad_occt_create_cylinder(ctx, 2.0, 0.0, &cyl) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "create_cylinder rejects a zero height");
  Check(aicad_occt_create_cylinder(ctx, -1.0, 5.0, &cyl) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "create_cylinder rejects a negative radius");
  Check(aicad_occt_create_cylinder(ctx, 2.0, -5.0, &cyl) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "create_cylinder rejects a negative height");
  Check(aicad_occt_create_cylinder(ctx, std::numeric_limits<double>::quiet_NaN(), 5.0, &cyl) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "create_cylinder rejects NaN radius");
  Check(aicad_occt_create_cylinder(ctx, 1e-7, 1e-7, &cyl) == AICAD_OCCT_OK,
        "create_cylinder succeeds for very small (1e-7) radius/height");
  Check(aicad_occt_create_cylinder(ctx, 1e8, 1e8, &cyl) == AICAD_OCCT_OK,
        "create_cylinder succeeds for very large (1e8) radius/height");
  Check(aicad_occt_create_cylinder(ctx, 1e6, 1e-6, &cyl) == AICAD_OCCT_OK,
        "create_cylinder succeeds for a mixed-scale (wide, thin) cylinder");

  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "context_destroy succeeds");

  if (g_failures > 0) {
    std::fprintf(stderr, "box_cylinder_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("box_cylinder_test: all checks PASSED\n");
  return 0;
}
