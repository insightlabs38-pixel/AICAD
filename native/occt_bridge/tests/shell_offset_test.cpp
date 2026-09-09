// AICAD-028: shell and offset (spike).

#include "aicad_occt_bridge.h"

#include <cmath>
#include <cstdio>
#include <cstdlib>

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

// Finds the (unique) face of `shape` whose bounding box matches the given
// axis-aligned box, within tolerance -- mirrors fillet_chamfer_test.cpp's
// FindEdgeByBoundingBox, applied to faces instead of edges.
aicad_shape_handle_t FindFaceByBoundingBox(aicad_occt_context_t* ctx,
                                            aicad_shape_handle_t shape,
                                            const double want_min[3],
                                            const double want_max[3]) {
  size_t count = 0;
  aicad_occt_shape_face_count(ctx, shape, &count);
  for (size_t i = 0; i < count; ++i) {
    aicad_shape_handle_t face = AICAD_NULL_SHAPE_HANDLE;
    if (aicad_occt_shape_get_face(ctx, shape, i, &face) != AICAD_OCCT_OK) {
      continue;
    }
    double bb_min[3], bb_max[3];
    if (aicad_occt_shape_bounding_box(ctx, face, bb_min, bb_max) != AICAD_OCCT_OK) {
      continue;
    }
    bool match = true;
    for (int k = 0; k < 3; ++k) {
      if (!NearlyEqual(bb_min[k], want_min[k], 1e-6) || !NearlyEqual(bb_max[k], want_max[k], 1e-6)) {
        match = false;
        break;
      }
    }
    if (match) {
      return face;
    }
  }
  return AICAD_NULL_SHAPE_HANDLE;
}

}  // namespace

int main() {
  aicad_occt_context_t* ctx = nullptr;
  Check(aicad_occt_context_create(&ctx) == AICAD_OCCT_OK, "context_create succeeds");
  const double kPi = 3.14159265358979323846;

  // --- face enumeration: a box has exactly 6 unique faces. ---
  const double dx = 2.0, dy = 3.0, dz = 4.0;
  aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, dx, dy, dz, &box);
  size_t face_count = 0;
  Check(aicad_occt_shape_face_count(ctx, box, &face_count) == AICAD_OCCT_OK && face_count == 6,
        "a box has exactly 6 unique faces");
  aicad_shape_handle_t out_of_range = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_shape_get_face(ctx, box, 6, &out_of_range) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "get_face rejects an out-of-range index (6, when count is 6)");

  // --- shell: hollow the box, removing the top (z=dz) face, wall
  //     thickness 0.2 built inward (negative thickness). The resulting
  //     hollow cavity is a box spanning x in [t,dx-t], y in [t,dy-t], z in
  //     [t,dz] (open at the top, since that face was removed) -> volume
  //     = dx*dy*dz - (dx-2t)*(dy-2t)*(dz-t). ---
  aicad_shape_handle_t shell_box = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, dx, dy, dz, &shell_box);
  const double top_min[3] = {0, 0, dz};
  const double top_max[3] = {dx, dy, dz};
  aicad_shape_handle_t top_face = FindFaceByBoundingBox(ctx, shell_box, top_min, top_max);
  Check(!(top_face.slot == 0 && top_face.generation == 0 && top_face.context_id == 0),
        "the top face (z=dz) is found among the box's 6 faces");
  const double t = 0.2;
  aicad_shape_handle_t faces_to_remove[1] = {top_face};
  aicad_shape_handle_t shelled = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_shell(ctx, shell_box, faces_to_remove, 1, -t, &shelled) == AICAD_OCCT_OK,
        "shell succeeds, hollowing a box with its top face removed");
  int shelled_valid = 0;
  Check(aicad_occt_shape_is_valid(ctx, shelled, &shelled_valid) == AICAD_OCCT_OK &&
            shelled_valid == 1,
        "the shelled box is a valid B-rep");
  double shelled_volume = 0.0;
  const double expected_shell_volume = dx * dy * dz - (dx - 2 * t) * (dy - 2 * t) * (dz - t);
  Check(aicad_occt_shape_volume(ctx, shelled, &shelled_volume) == AICAD_OCCT_OK &&
            NearlyEqual(shelled_volume, expected_shell_volume, expected_shell_volume * 1e-4),
        "shelled box volume matches box_volume - (dx-2t)(dy-2t)(dz-t) analytically");

  // --- offset: a positive-distance offset of a convex solid with OCCT's
  //     default GeomAbs_Arc join is geometrically equivalent to filleting
  //     every edge with that distance as radius -- both are the
  //     Minkowski sum of the original solid with a ball of that radius.
  //     Volume = dx*dy*dz + 2*delta*(dx*dy+dy*dz+dz*dx) +
  //     pi*delta^2*(dx+dy+dz) + (4/3)*pi*delta^3 (no inset term, since
  //     offset grows OUTWARD from the original faces, unlike fillet which
  //     keeps the original face planes and insets the "core"). ---
  aicad_shape_handle_t offset_box = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, dx, dy, dz, &offset_box);
  const double delta = 0.3;
  aicad_shape_handle_t offset_result = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_offset(ctx, offset_box, delta, &offset_result) == AICAD_OCCT_OK,
        "offset succeeds for a positive distance on a box");
  int offset_valid = 0;
  Check(aicad_occt_shape_is_valid(ctx, offset_result, &offset_valid) == AICAD_OCCT_OK &&
            offset_valid == 1,
        "the offset result is a valid B-rep");
  double offset_volume = 0.0;
  const double expected_offset_volume =
      dx * dy * dz + 2 * delta * (dx * dy + dy * dz + dz * dx) +
      kPi * delta * delta * (dx + dy + dz) + (4.0 / 3.0) * kPi * delta * delta * delta;
  Check(aicad_occt_shape_volume(ctx, offset_result, &offset_volume) == AICAD_OCCT_OK &&
            NearlyEqual(offset_volume, expected_offset_volume, expected_offset_volume * 1e-4),
        "positive-offset box volume matches the analytic Minkowski-sum-with-a-ball formula "
        "(the same closed form as fillet-all-edges, without the inset term)");

  // --- adversarial ---
  Check(aicad_occt_shell(ctx, box, nullptr, 1, -0.1, &shelled) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "shell rejects a null faces_to_remove pointer");
  Check(aicad_occt_shell(ctx, box, faces_to_remove, 0, -0.1, &shelled) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "shell rejects a zero face_count");
  Check(aicad_occt_shell(ctx, box, faces_to_remove, 1, 0.0, &shelled) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "shell rejects a zero thickness");
  aicad_shape_handle_t not_a_face_array[1] = {box};  // `box` is a Solid, not a Face.
  Check(aicad_occt_shell(ctx, box, not_a_face_array, 1, -0.1, &shelled) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "shell rejects a handle that is not a face (the solid itself, here)");
  Check(aicad_occt_offset(ctx, box, 0.0, &offset_result) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "offset rejects a zero distance");

  // --- documented limitation: an impossible shell, where the requested
  //     wall thickness leaves no room for a cavity (per AGENTS.md's
  //     "shell/offset failure modes" adversarial-case category) --
  //     probed rather than assumed. ---
  aicad_shape_handle_t small_box = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &small_box);
  const double small_top_min[3] = {0, 0, 1.0};
  const double small_top_max[3] = {1.0, 1.0, 1.0};
  aicad_shape_handle_t small_top_face =
      FindFaceByBoundingBox(ctx, small_box, small_top_min, small_top_max);
  aicad_shape_handle_t small_faces[1] = {small_top_face};
  aicad_shape_handle_t impossible_shell = AICAD_NULL_SHAPE_HANDLE;
  // A wall thickness of 10.0 on a 1x1x1 box leaves no room for a cavity
  // at all -- geometrically impossible.
  aicad_occt_status_t impossible_status =
      aicad_occt_shell(ctx, small_box, small_faces, 1, -10.0, &impossible_shell);
  std::printf(
      "INFO: shell with wall thickness (10.0) far exceeding the 1x1x1 box it targets returned "
      "status %d (0=OK, 6=OPERATION_FAILED) -- an oversized shell request is expected to be "
      "geometrically impossible, not silently clamped\n",
      static_cast<int>(impossible_status));
  Check(impossible_status == AICAD_OCCT_ERR_OPERATION_FAILED || impossible_status == AICAD_OCCT_OK,
        "an oversized shell request either fails cleanly or succeeds (never crashes/hangs)");

  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "context_destroy succeeds");

  if (g_failures > 0) {
    std::fprintf(stderr, "shell_offset_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("shell_offset_test: all checks PASSED\n");
  return 0;
}
