// AICAD-022: line/circle curves and edge/wire creation.

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

  // --- make_line_edge: happy path ---
  const double p0[3] = {0, 0, 0};
  const double p1[3] = {3, 4, 0};
  aicad_shape_handle_t edge = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_line_edge(ctx, p0, p1, &edge) == AICAD_OCCT_OK,
        "make_line_edge succeeds for two distinct points");
  int is_valid = 0;
  Check(aicad_occt_shape_is_valid(ctx, edge, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "a freshly created line edge is a valid B-rep");
  double bb_min[3], bb_max[3];
  Check(aicad_occt_shape_bounding_box(ctx, edge, bb_min, bb_max) == AICAD_OCCT_OK &&
            NearlyEqual(bb_max[0] - bb_min[0], 3.0, 1e-6) &&
            NearlyEqual(bb_max[1] - bb_min[1], 4.0, 1e-6),
        "line edge bounding box matches its endpoints (3-4-5 triangle leg)");

  // --- make_line_edge: adversarial ---
  Check(aicad_occt_make_line_edge(ctx, p0, p0, &edge) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_line_edge rejects coincident endpoints");
  const double tiny_offset[3] = {1e-15, 0, 0};
  Check(aicad_occt_make_line_edge(ctx, p0, tiny_offset, &edge) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_line_edge rejects near-coincident endpoints below tolerance");
  const double nan_point[3] = {std::numeric_limits<double>::quiet_NaN(), 0, 0};
  Check(aicad_occt_make_line_edge(ctx, p0, nan_point, &edge) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_line_edge rejects a NaN endpoint");
  const double far_point[3] = {1e9, 0, 0};
  Check(aicad_occt_make_line_edge(ctx, p0, far_point, &edge) == AICAD_OCCT_OK,
        "make_line_edge succeeds for a very large (1e9) span");
  const double near_point[3] = {1e-5, 0, 0};
  Check(aicad_occt_make_line_edge(ctx, p0, near_point, &edge) == AICAD_OCCT_OK,
        "make_line_edge succeeds for a very small (1e-5) but above-tolerance span");

  // --- make_circle_wire: happy path ---
  const double center[3] = {1, 2, 3};
  const double normal_z[3] = {0, 0, 1};
  aicad_shape_handle_t circle = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_circle_wire(ctx, center, normal_z, 5.0, &circle) == AICAD_OCCT_OK,
        "make_circle_wire succeeds for a positive radius");
  Check(aicad_occt_shape_is_valid(ctx, circle, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "a freshly created circle wire is a valid B-rep");
  Check(aicad_occt_shape_bounding_box(ctx, circle, bb_min, bb_max) == AICAD_OCCT_OK &&
            NearlyEqual(bb_min[0], center[0] - 5.0, 1e-6) &&
            NearlyEqual(bb_max[0], center[0] + 5.0, 1e-6) &&
            NearlyEqual(bb_min[1], center[1] - 5.0, 1e-6) &&
            NearlyEqual(bb_max[1], center[1] + 5.0, 1e-6) &&
            NearlyEqual(bb_min[2], center[2], 1e-6) && NearlyEqual(bb_max[2], center[2], 1e-6),
        "circle wire bounding box is centered correctly and lies in the plane normal to +Z");

  // --- make_circle_wire: adversarial ---
  Check(aicad_occt_make_circle_wire(ctx, center, normal_z, 0.0, &circle) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_circle_wire rejects a zero radius");
  Check(aicad_occt_make_circle_wire(ctx, center, normal_z, -1.0, &circle) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_circle_wire rejects a negative radius");
  const double zero_normal[3] = {0, 0, 0};
  Check(aicad_occt_make_circle_wire(ctx, center, zero_normal, 5.0, &circle) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_circle_wire rejects a zero-length normal");
  Check(aicad_occt_make_circle_wire(ctx, center, normal_z, 1e-7, &circle) == AICAD_OCCT_OK,
        "make_circle_wire succeeds for a very small (1e-7) radius");
  Check(aicad_occt_make_circle_wire(ctx, center, normal_z, 1e8, &circle) == AICAD_OCCT_OK,
        "make_circle_wire succeeds for a very large (1e8) radius");
  // Non-unit normal is accepted and normalized (direction only matters).
  const double non_unit_normal[3] = {0, 0, 7.5};
  Check(aicad_occt_make_circle_wire(ctx, center, non_unit_normal, 5.0, &circle) == AICAD_OCCT_OK,
        "make_circle_wire accepts a non-unit-length normal (direction only)");

  // --- make_wire_from_edges: build a closed unit square from 4 line edges ---
  const double sq0[3] = {0, 0, 0};
  const double sq1[3] = {1, 0, 0};
  const double sq2[3] = {1, 1, 0};
  const double sq3[3] = {0, 1, 0};
  aicad_shape_handle_t e0, e1, e2, e3;
  Check(aicad_occt_make_line_edge(ctx, sq0, sq1, &e0) == AICAD_OCCT_OK, "square edge 0 created");
  Check(aicad_occt_make_line_edge(ctx, sq1, sq2, &e1) == AICAD_OCCT_OK, "square edge 1 created");
  Check(aicad_occt_make_line_edge(ctx, sq2, sq3, &e2) == AICAD_OCCT_OK, "square edge 2 created");
  Check(aicad_occt_make_line_edge(ctx, sq3, sq0, &e3) == AICAD_OCCT_OK, "square edge 3 created");
  aicad_shape_handle_t square_edges[4] = {e0, e1, e2, e3};
  aicad_shape_handle_t square_wire = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_wire_from_edges(ctx, square_edges, 4, &square_wire) == AICAD_OCCT_OK,
        "make_wire_from_edges joins 4 connected edges into one closed wire");
  Check(aicad_occt_shape_is_valid(ctx, square_wire, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "the resulting square wire is a valid B-rep");
  Check(aicad_occt_shape_bounding_box(ctx, square_wire, bb_min, bb_max) == AICAD_OCCT_OK &&
            NearlyEqual(bb_max[0] - bb_min[0], 1.0, 1e-6) &&
            NearlyEqual(bb_max[1] - bb_min[1], 1.0, 1e-6),
        "the square wire's bounding box is the unit square");

  // --- make_wire_from_edges: adversarial ---
  Check(aicad_occt_make_wire_from_edges(ctx, square_edges, 0, &square_wire) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_wire_from_edges rejects an empty edge list");
  aicad_shape_handle_t far_edge = AICAD_NULL_SHAPE_HANDLE;
  const double away0[3] = {100, 100, 100};
  const double away1[3] = {101, 100, 100};
  Check(aicad_occt_make_line_edge(ctx, away0, away1, &far_edge) == AICAD_OCCT_OK,
        "disconnected edge created");
  aicad_shape_handle_t disconnected_edges[2] = {e0, far_edge};
  Check(aicad_occt_make_wire_from_edges(ctx, disconnected_edges, 2, &square_wire) ==
            AICAD_OCCT_ERR_OPERATION_FAILED,
        "make_wire_from_edges rejects two disconnected edges");
  aicad_shape_handle_t box_handle = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &box_handle) == AICAD_OCCT_OK,
        "a box (wrong shape kind) is created for the next check");
  aicad_shape_handle_t wrong_kind_edges[1] = {box_handle};
  Check(aicad_occt_make_wire_from_edges(ctx, wrong_kind_edges, 1, &square_wire) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_wire_from_edges rejects a handle that does not address an edge (a solid, here)");

  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "context_destroy succeeds");

  if (g_failures > 0) {
    std::fprintf(stderr, "curve_edge_wire_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("curve_edge_wire_test: all checks PASSED\n");
  return 0;
}
