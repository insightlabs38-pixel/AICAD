// AICAD-021: aicad_occt_transform_shape (rigid transforms).
//
// Row-major 3x4 matrices are constructed by hand here, exactly as
// `cad-kernel-api::Transform::to_row_major_3x4()` will produce them, to
// test this native operation independently of that Rust layer.

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

  aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_create_box(ctx, 2.0, 3.0, 4.0, &box) == AICAD_OCCT_OK, "create_box succeeds");

  // --- identity transform: no-op on volume and bounding box ---
  const double identity[12] = {1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0};
  aicad_shape_handle_t moved = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_transform_shape(ctx, box, identity, &moved) == AICAD_OCCT_OK,
        "transform_shape succeeds with an identity matrix");
  double volume = 0.0;
  Check(aicad_occt_shape_volume(ctx, moved, &volume) == AICAD_OCCT_OK &&
            NearlyEqual(volume, 24.0, 1e-9),
        "identity transform preserves volume (2*3*4=24)");
  double bb_min[3], bb_max[3];
  Check(aicad_occt_shape_bounding_box(ctx, moved, bb_min, bb_max) == AICAD_OCCT_OK &&
            NearlyEqual(bb_min[0], 0.0, 1e-6) && NearlyEqual(bb_max[0], 2.0, 1e-6),
        "identity transform preserves the bounding box");
  Check(moved.slot != box.slot || moved.generation != box.generation,
        "identity transform still produces a NEW shape handle (functional/value semantics, DL-2)");

  // --- translation: bounding box shifts by exactly the given vector,
  //     volume is unchanged ---
  const double translate[12] = {1, 0, 0, 10, 0, 1, 0, -5, 0, 0, 1, 100};
  aicad_shape_handle_t translated = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_transform_shape(ctx, box, translate, &translated) == AICAD_OCCT_OK,
        "transform_shape succeeds with a pure translation");
  Check(aicad_occt_shape_volume(ctx, translated, &volume) == AICAD_OCCT_OK &&
            NearlyEqual(volume, 24.0, 1e-9),
        "translation preserves volume");
  Check(aicad_occt_shape_bounding_box(ctx, translated, bb_min, bb_max) == AICAD_OCCT_OK &&
            NearlyEqual(bb_min[0], 10.0, 1e-6) && NearlyEqual(bb_max[0], 12.0, 1e-6) &&
            NearlyEqual(bb_min[1], -5.0, 1e-6) && NearlyEqual(bb_max[1], -2.0, 1e-6) &&
            NearlyEqual(bb_min[2], 100.0, 1e-6) && NearlyEqual(bb_max[2], 104.0, 1e-6),
        "translation shifts the bounding box by exactly the given vector");

  // --- rotation: 90 degrees about +Z maps (x extent, y extent) -> (y
  //     extent, x extent); volume unchanged ---
  const double rot_z_90[12] = {0, -1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0};
  aicad_shape_handle_t rotated = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_transform_shape(ctx, box, rot_z_90, &rotated) == AICAD_OCCT_OK,
        "transform_shape succeeds with a 90-degree Z rotation");
  Check(aicad_occt_shape_volume(ctx, rotated, &volume) == AICAD_OCCT_OK &&
            NearlyEqual(volume, 24.0, 1e-9),
        "rotation preserves volume");
  Check(aicad_occt_shape_bounding_box(ctx, rotated, bb_min, bb_max) == AICAD_OCCT_OK &&
            NearlyEqual(bb_max[0] - bb_min[0], 3.0, 1e-6) &&
            NearlyEqual(bb_max[1] - bb_min[1], 2.0, 1e-6) &&
            NearlyEqual(bb_max[2] - bb_min[2], 4.0, 1e-6),
        "90-degree Z rotation swaps the box's X/Y extents (2x3x4 -> 3x2x4)");

  // --- repeated transforms: rotating four times by 90 degrees returns to
  //     the original bounding box (within tolerance) ---
  aicad_shape_handle_t cur = box;
  aicad_shape_handle_t next = AICAD_NULL_SHAPE_HANDLE;
  bool repeated_ok = true;
  for (int i = 0; i < 4; ++i) {
    if (aicad_occt_transform_shape(ctx, cur, rot_z_90, &next) != AICAD_OCCT_OK) {
      repeated_ok = false;
      break;
    }
    cur = next;
  }
  Check(repeated_ok, "four repeated 90-degree Z rotations all succeed");
  Check(aicad_occt_shape_bounding_box(ctx, cur, bb_min, bb_max) == AICAD_OCCT_OK &&
            NearlyEqual(bb_max[0] - bb_min[0], 2.0, 1e-6) &&
            NearlyEqual(bb_max[1] - bb_min[1], 3.0, 1e-6),
        "four repeated 90-degree Z rotations return to the original extents");

  // --- non-rigid matrices are rejected: scale, reflection, shear ---
  const double scale_x2[12] = {2, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0};
  Check(aicad_occt_transform_shape(ctx, box, scale_x2, &moved) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "transform_shape rejects a non-uniform-scale matrix (not rigid)");
  const double reflect_x[12] = {-1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0};
  Check(aicad_occt_transform_shape(ctx, box, reflect_x, &moved) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "transform_shape rejects a reflection matrix (determinant -1, not a proper rotation)");
  const double shear[12] = {1, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0};
  Check(aicad_occt_transform_shape(ctx, box, shear, &moved) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "transform_shape rejects a shear matrix (columns not orthogonal)");

  // --- NaN/infinity in the matrix is rejected ---
  double nan_matrix[12] = {1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0};
  nan_matrix[3] = std::numeric_limits<double>::quiet_NaN();
  Check(aicad_occt_transform_shape(ctx, box, nan_matrix, &moved) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "transform_shape rejects a matrix containing NaN");
  double inf_matrix[12] = {1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0};
  inf_matrix[7] = std::numeric_limits<double>::infinity();
  Check(aicad_occt_transform_shape(ctx, box, inf_matrix, &moved) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "transform_shape rejects a matrix containing infinity");

  // --- stale/foreign/invalid handle rejection (shared machinery, spot-checked here) ---
  aicad_shape_handle_t stale = box;
  Check(aicad_occt_release_shape(ctx, stale) == AICAD_OCCT_OK, "release_shape succeeds");
  Check(aicad_occt_transform_shape(ctx, stale, identity, &moved) == AICAD_OCCT_ERR_STALE_HANDLE,
        "transform_shape rejects a stale handle");

  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "context_destroy succeeds");

  if (g_failures > 0) {
    std::fprintf(stderr, "transform_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("transform_test: all checks PASSED\n");
  return 0;
}
