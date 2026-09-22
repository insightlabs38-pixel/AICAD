// AICAD-131: Bezier/B-spline curve/surface -> kernel topology
// construction (make_bezier_edge/make_bspline_edge/
// make_face_on_bezier_surface/make_face_on_bspline_surface).

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

}  // namespace

int main() {
  aicad_occt_context_t* ctx = nullptr;
  Check(aicad_occt_context_create(&ctx) == AICAD_OCCT_OK, "context_create succeeds");

  // --- make_bezier_edge: a quadratic (3-point) non-rational Bezier ---
  const double bezier_pts[9] = {0, 0, 0, 5, 10, 0, 10, 0, 0};
  aicad_shape_handle_t bezier_edge = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_bezier_edge(ctx, bezier_pts, 3, nullptr, &bezier_edge) == AICAD_OCCT_OK,
        "make_bezier_edge succeeds for a plain quadratic Bezier");
  int is_valid = 0;
  Check(aicad_occt_shape_is_valid(ctx, bezier_edge, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "a freshly built Bezier edge is a valid B-rep");
  double bb_min[3], bb_max[3];
  Check(aicad_occt_shape_bounding_box(ctx, bezier_edge, bb_min, bb_max) == AICAD_OCCT_OK &&
            std::fabs(bb_max[0] - bb_min[0] - 10.0) <= 1e-6,
        "Bezier edge bounding box spans its own control points' own x-extent");

  // --- make_bezier_edge: a rational Bezier (weights supplied) ---
  const double weights3[3] = {1.0, 2.0, 1.0};
  aicad_shape_handle_t rational_edge = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_bezier_edge(ctx, bezier_pts, 3, weights3, &rational_edge) ==
            AICAD_OCCT_OK,
        "make_bezier_edge succeeds for a rational Bezier");

  // --- make_bezier_edge: adversarial ---
  Check(aicad_occt_make_bezier_edge(ctx, bezier_pts, 1, nullptr, &bezier_edge) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_bezier_edge rejects fewer than 2 control points");
  const double nan_pts[6] = {0, 0, 0, std::numeric_limits<double>::quiet_NaN(), 0, 0};
  Check(aicad_occt_make_bezier_edge(ctx, nan_pts, 2, nullptr, &bezier_edge) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_bezier_edge rejects a non-finite control point");
  const double bad_weights[3] = {1.0, 0.0, 1.0};
  Check(aicad_occt_make_bezier_edge(ctx, bezier_pts, 3, bad_weights, &bezier_edge) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_bezier_edge rejects a non-positive weight");

  // --- make_bspline_edge: a clamped cubic B-spline (5 control points,
  // one interior knot -- the same shape as the frozen freeform corpus's
  // own held_out/07_spline_edge_construction_unsupported hook) ---
  const double hook_pts[15] = {0, 0, 0, 0, 45, 0, 25, 60, 0, 45, 40, 0, 30, 15, 0};
  const double hook_knots[3] = {0.0, 0.5, 1.0};
  const size_t hook_mults[3] = {4, 1, 4};
  aicad_shape_handle_t hook_edge = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_bspline_edge(ctx, 3, hook_pts, 5, hook_knots, hook_mults, 3, nullptr,
                                      &hook_edge) == AICAD_OCCT_OK,
        "make_bspline_edge succeeds for a clamped cubic B-spline hook");
  Check(aicad_occt_shape_is_valid(ctx, hook_edge, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "the clamped-cubic hook edge is a valid B-rep");

  // --- make_bspline_edge: adversarial ---
  Check(aicad_occt_make_bspline_edge(ctx, 3, hook_pts, 5, hook_knots, hook_mults, 3, nullptr,
                                      nullptr) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_bspline_edge rejects a null out_handle");
  const double non_increasing_knots[3] = {0.0, 0.5, 0.5};
  Check(aicad_occt_make_bspline_edge(ctx, 3, hook_pts, 5, non_increasing_knots, hook_mults, 3,
                                      nullptr, &hook_edge) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_bspline_edge rejects non-strictly-increasing knots");
  Check(aicad_occt_make_bspline_edge(ctx, 5, hook_pts, 5, hook_knots, hook_mults, 3, nullptr,
                                      &hook_edge) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_bspline_edge rejects a degree too high for its own control-point count");

  // --- make_face_on_bezier_surface: a bilinear (2x2) doubly-curved
  // patch, bounded by a real 4-edge wire matching its own four corners ---
  const double c00[3] = {0, 0, 0};
  const double c01[3] = {0, 10, 2};
  const double c10[3] = {10, 0, 2};
  const double c11[3] = {10, 10, 0};
  const double blade_pts[12] = {c00[0], c00[1], c00[2], c01[0], c01[1], c01[2],
                                 c10[0], c10[1], c10[2], c11[0], c11[1], c11[2]};
  aicad_shape_handle_t e0 = AICAD_NULL_SHAPE_HANDLE, e1 = AICAD_NULL_SHAPE_HANDLE,
                        e2 = AICAD_NULL_SHAPE_HANDLE, e3 = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_line_edge(ctx, c00, c01, &e0) == AICAD_OCCT_OK, "blade edge 0 built");
  Check(aicad_occt_make_line_edge(ctx, c01, c11, &e1) == AICAD_OCCT_OK, "blade edge 1 built");
  Check(aicad_occt_make_line_edge(ctx, c11, c10, &e2) == AICAD_OCCT_OK, "blade edge 2 built");
  Check(aicad_occt_make_line_edge(ctx, c10, c00, &e3) == AICAD_OCCT_OK, "blade edge 3 built");
  aicad_shape_handle_t blade_edges[4] = {e0, e1, e2, e3};
  aicad_shape_handle_t blade_wire = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_wire_from_edges(ctx, blade_edges, 4, &blade_wire) == AICAD_OCCT_OK,
        "blade wire assembled from its own 4 edges");
  aicad_shape_handle_t blade_face = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_face_on_bezier_surface(ctx, blade_wire, nullptr, 0, blade_pts, 2, 2,
                                                nullptr, 0, &blade_face) == AICAD_OCCT_OK,
        "make_face_on_bezier_surface succeeds for a bilinear doubly-curved patch");
  // Construction success is not evidence of validity (Stage-1 kernel
  // policy #14, exactly as for make_face_on_cylinder/_cone/_sphere/
  // _torus's own established "not required to be valid" precedent) --
  // validate() must simply complete without error.
  Check(aicad_occt_shape_is_valid(ctx, blade_face, &is_valid) == AICAD_OCCT_OK,
        "validate() completes for the bilinear Bezier-surface face");

  // --- make_face_on_bezier_surface: adversarial ---
  Check(aicad_occt_make_face_on_bezier_surface(ctx, blade_wire, nullptr, 0, blade_pts, 1, 2,
                                                nullptr, 0, &blade_face) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_face_on_bezier_surface rejects fewer than 2 rows");
  aicad_shape_handle_t box_handle = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &box_handle) == AICAD_OCCT_OK,
        "a box (wrong shape kind) is created for the next check");
  Check(aicad_occt_make_face_on_bezier_surface(ctx, box_handle, nullptr, 0, blade_pts, 2, 2,
                                                nullptr, 0, &blade_face) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_face_on_bezier_surface rejects a handle that does not address a wire (a solid)");

  // --- make_face_on_bspline_surface: same 2x2 patch, expressed as a
  // degree_u=degree_v=1 clamped B-spline (bit-identical geometry) ---
  const double linear_knots[2] = {0.0, 1.0};
  const size_t linear_mults[2] = {2, 2};
  aicad_shape_handle_t bspline_face = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_face_on_bspline_surface(
            ctx, blade_wire, nullptr, 0, 1, 1, blade_pts, 2, 2, linear_knots, linear_mults, 2,
            linear_knots, linear_mults, 2, nullptr, 0, &bspline_face) == AICAD_OCCT_OK,
        "make_face_on_bspline_surface succeeds for a degree-1 tensor-product patch");
  Check(aicad_occt_shape_is_valid(ctx, bspline_face, &is_valid) == AICAD_OCCT_OK,
        "validate() completes for the degree-1 B-spline-surface face");

  // --- make_face_on_bspline_surface: adversarial ---
  Check(aicad_occt_make_face_on_bspline_surface(ctx, blade_wire, nullptr, 0, 1, 1, blade_pts, 1,
                                                 2, linear_knots, linear_mults, 2, linear_knots,
                                                 linear_mults, 2, nullptr, 0,
                                                 &bspline_face) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_face_on_bspline_surface rejects a control-point grid too small for its own degree");

  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "context_destroy succeeds");

  if (g_failures > 0) {
    std::fprintf(stderr, "freeform_topology_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("freeform_topology_test: all checks PASSED\n");
  return 0;
}
