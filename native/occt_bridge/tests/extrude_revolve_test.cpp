// AICAD-024: extrude and revolve (face -> solid).

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

aicad_shape_handle_t MakeSquareFace(aicad_occt_context_t* ctx) {
  const double sq0[3] = {0, 0, 0};
  const double sq1[3] = {1, 0, 0};
  const double sq2[3] = {1, 1, 0};
  const double sq3[3] = {0, 1, 0};
  aicad_shape_handle_t e0, e1, e2, e3;
  aicad_occt_make_line_edge(ctx, sq0, sq1, &e0);
  aicad_occt_make_line_edge(ctx, sq1, sq2, &e1);
  aicad_occt_make_line_edge(ctx, sq2, sq3, &e2);
  aicad_occt_make_line_edge(ctx, sq3, sq0, &e3);
  aicad_shape_handle_t edges[4] = {e0, e1, e2, e3};
  aicad_shape_handle_t wire = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_make_wire_from_edges(ctx, edges, 4, &wire);
  aicad_shape_handle_t face = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_make_face_from_wire(ctx, wire, &face);
  return face;
}

// A rectangular profile in the y=0 plane, offset from the Z axis --
// revolving it around the Z axis by a given angle produces a tube
// (annular sector) with a known analytic volume.
aicad_shape_handle_t MakeOffsetRectangleFace(aicad_occt_context_t* ctx,
                                              double inner_radius,
                                              double outer_radius,
                                              double height) {
  const double p0[3] = {inner_radius, 0, 0};
  const double p1[3] = {outer_radius, 0, 0};
  const double p2[3] = {outer_radius, 0, height};
  const double p3[3] = {inner_radius, 0, height};
  aicad_shape_handle_t e0, e1, e2, e3;
  aicad_occt_make_line_edge(ctx, p0, p1, &e0);
  aicad_occt_make_line_edge(ctx, p1, p2, &e1);
  aicad_occt_make_line_edge(ctx, p2, p3, &e2);
  aicad_occt_make_line_edge(ctx, p3, p0, &e3);
  aicad_shape_handle_t edges[4] = {e0, e1, e2, e3};
  aicad_shape_handle_t wire = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_make_wire_from_edges(ctx, edges, 4, &wire);
  aicad_shape_handle_t face = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_make_face_from_wire(ctx, wire, &face);
  return face;
}

}  // namespace

int main() {
  aicad_occt_context_t* ctx = nullptr;
  Check(aicad_occt_context_create(&ctx) == AICAD_OCCT_OK, "context_create succeeds");
  const double kPi = 3.14159265358979323846;

  // --- extrude: unit square by distance 5 along +Z -> volume 5.0 ---
  aicad_shape_handle_t square_face = MakeSquareFace(ctx);
  const double up[3] = {0, 0, 1};
  aicad_shape_handle_t prism = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_extrude(ctx, square_face, up, 5.0, &prism) == AICAD_OCCT_OK,
        "extrude succeeds for a planar face along +Z");
  int is_valid = 0;
  Check(aicad_occt_shape_is_valid(ctx, prism, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "the extruded solid is a valid B-rep");
  double volume = 0.0;
  Check(aicad_occt_shape_volume(ctx, prism, &volume) == AICAD_OCCT_OK &&
            NearlyEqual(volume, 5.0, 1e-9),
        "extruded unit-square prism volume is exactly 1*1*5=5.0");
  double bb_min[3], bb_max[3];
  Check(aicad_occt_shape_bounding_box(ctx, prism, bb_min, bb_max) == AICAD_OCCT_OK &&
            NearlyEqual(bb_max[2] - bb_min[2], 5.0, 1e-6),
        "extruded prism bounding box has height exactly 5.0 along the extrusion direction");

  // --- extrude: direction need not be unit-length (magnitude ignored) ---
  aicad_shape_handle_t square_face2 = MakeSquareFace(ctx);
  const double non_unit_up[3] = {0, 0, 42.0};
  aicad_shape_handle_t prism2 = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_extrude(ctx, square_face2, non_unit_up, 5.0, &prism2) == AICAD_OCCT_OK &&
            [&] {
              double v = 0.0;
              aicad_occt_shape_volume(ctx, prism2, &v);
              return NearlyEqual(v, 5.0, 1e-9);
            }(),
        "extrude direction magnitude is ignored; only distance controls extrusion length");

  // --- extrude: adversarial ---
  aicad_shape_handle_t square_face3 = MakeSquareFace(ctx);
  Check(aicad_occt_extrude(ctx, square_face3, up, 0.0, &prism) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "extrude rejects a zero distance");
  Check(aicad_occt_extrude(ctx, square_face3, up, -1.0, &prism) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "extrude rejects a negative distance");
  const double zero_dir[3] = {0, 0, 0};
  Check(aicad_occt_extrude(ctx, square_face3, zero_dir, 5.0, &prism) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "extrude rejects a zero-length direction");
  const double nan_dir[3] = {std::numeric_limits<double>::quiet_NaN(), 0, 1};
  Check(aicad_occt_extrude(ctx, square_face3, nan_dir, 5.0, &prism) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "extrude rejects a NaN direction component");
  const double p0[3] = {0, 0, 0};
  const double p1[3] = {1, 0, 0};
  aicad_shape_handle_t wire_not_face = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_make_line_edge(ctx, p0, p1, &wire_not_face);
  Check(aicad_occt_extrude(ctx, wire_not_face, up, 5.0, &prism) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "extrude rejects a handle that does not address a face (an edge, here)");
  Check(aicad_occt_extrude(ctx, square_face3, up, 1e-5, &prism) == AICAD_OCCT_OK,
        "extrude succeeds for a very small (1e-5) distance");
  Check(aicad_occt_extrude(ctx, MakeSquareFace(ctx), up, 1e8, &prism) == AICAD_OCCT_OK,
        "extrude succeeds for a very large (1e8) distance");

  // --- revolve: offset rectangle around Z axis, full revolution ->
  //     analytic tube volume pi*(R^2-r^2)*h ---
  aicad_shape_handle_t tube_face = MakeOffsetRectangleFace(ctx, /*inner=*/1.0, /*outer=*/3.0,
                                                            /*height=*/5.0);
  const double axis_origin[3] = {0, 0, 0};
  const double axis_z[3] = {0, 0, 1};
  const double kTwoPi = 6.283185307179586476925286766559;
  aicad_shape_handle_t tube = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_revolve(ctx, tube_face, axis_origin, axis_z, kTwoPi, &tube) == AICAD_OCCT_OK,
        "revolve succeeds for a full (2*pi) revolution of an axis-offset face");
  Check(aicad_occt_shape_is_valid(ctx, tube, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "the revolved solid is a valid B-rep");
  double tube_volume = 0.0;
  const double expected_tube_volume = kPi * (9.0 - 1.0) * 5.0;  // pi*(R^2-r^2)*h
  Check(aicad_occt_shape_volume(ctx, tube, &tube_volume) == AICAD_OCCT_OK &&
            NearlyEqual(tube_volume, expected_tube_volume, expected_tube_volume * 1e-6),
        "full-revolution tube volume matches pi*(R^2-r^2)*h analytically");

  // --- revolve: half revolution -> exactly half the volume ---
  aicad_shape_handle_t half_tube_face =
      MakeOffsetRectangleFace(ctx, /*inner=*/1.0, /*outer=*/3.0, /*height=*/5.0);
  aicad_shape_handle_t half_tube = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_revolve(ctx, half_tube_face, axis_origin, axis_z, kPi, &half_tube) ==
            AICAD_OCCT_OK,
        "revolve succeeds for a half (pi) revolution");
  double half_volume = 0.0;
  Check(aicad_occt_shape_volume(ctx, half_tube, &half_volume) == AICAD_OCCT_OK &&
            NearlyEqual(half_volume, expected_tube_volume / 2.0, expected_tube_volume * 1e-6),
        "half-revolution volume is exactly half the full-revolution volume");

  // --- revolve: adversarial ---
  aicad_shape_handle_t another_tube_face =
      MakeOffsetRectangleFace(ctx, /*inner=*/1.0, /*outer=*/3.0, /*height=*/5.0);
  Check(aicad_occt_revolve(ctx, another_tube_face, axis_origin, axis_z, 0.0, &tube) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "revolve rejects a zero angle");
  Check(aicad_occt_revolve(ctx, another_tube_face, axis_origin, axis_z, -kPi, &tube) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "revolve rejects a negative angle");
  Check(aicad_occt_revolve(ctx, another_tube_face, axis_origin, axis_z, kTwoPi * 2.0, &tube) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "revolve rejects an angle greater than 2*pi");
  Check(aicad_occt_revolve(ctx, another_tube_face, axis_origin, zero_dir, kPi, &tube) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "revolve rejects a zero-length axis direction");
  Check(aicad_occt_revolve(ctx, wire_not_face, axis_origin, axis_z, kPi, &tube) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "revolve rejects a handle that does not address a face (an edge, here)");

  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "context_destroy succeeds");

  if (g_failures > 0) {
    std::fprintf(stderr, "extrude_revolve_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("extrude_revolve_test: all checks PASSED\n");
  return 0;
}
