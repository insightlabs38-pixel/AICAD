// AICAD-025: sweep and loft, minimal supported forms.

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

aicad_shape_handle_t MakeSquareWire(aicad_occt_context_t* ctx, double side, double z) {
  const double p0[3] = {0, 0, z};
  const double p1[3] = {side, 0, z};
  const double p2[3] = {side, side, z};
  const double p3[3] = {0, side, z};
  aicad_shape_handle_t e0, e1, e2, e3;
  aicad_occt_make_line_edge(ctx, p0, p1, &e0);
  aicad_occt_make_line_edge(ctx, p1, p2, &e1);
  aicad_occt_make_line_edge(ctx, p2, p3, &e2);
  aicad_occt_make_line_edge(ctx, p3, p0, &e3);
  aicad_shape_handle_t edges[4] = {e0, e1, e2, e3};
  aicad_shape_handle_t wire = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_make_wire_from_edges(ctx, edges, 4, &wire);
  return wire;
}

aicad_shape_handle_t MakeSquareFace(aicad_occt_context_t* ctx, double side, double z) {
  aicad_shape_handle_t wire = MakeSquareWire(ctx, side, z);
  aicad_shape_handle_t face = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_make_face_from_wire(ctx, wire, &face);
  return face;
}

// A square, centered on the origin in the XY plane, so a family of these
// at increasing Z with different side lengths forms a true frustum of a
// pyramid when lofted with straight (ruled) generatrices between
// corresponding corners.
aicad_shape_handle_t MakeCenteredSquareWire(aicad_occt_context_t* ctx, double side, double z) {
  const double h = side / 2.0;
  const double p0[3] = {-h, -h, z};
  const double p1[3] = {h, -h, z};
  const double p2[3] = {h, h, z};
  const double p3[3] = {-h, h, z};
  aicad_shape_handle_t e0, e1, e2, e3;
  aicad_occt_make_line_edge(ctx, p0, p1, &e0);
  aicad_occt_make_line_edge(ctx, p1, p2, &e1);
  aicad_occt_make_line_edge(ctx, p2, p3, &e2);
  aicad_occt_make_line_edge(ctx, p3, p0, &e3);
  aicad_shape_handle_t edges[4] = {e0, e1, e2, e3};
  aicad_shape_handle_t wire = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_make_wire_from_edges(ctx, edges, 4, &wire);
  return wire;
}

aicad_shape_handle_t MakeStraightLineSpine(aicad_occt_context_t* ctx,
                                            const double p0[3],
                                            const double p1[3]) {
  aicad_shape_handle_t edge = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_make_line_edge(ctx, p0, p1, &edge);
  aicad_shape_handle_t edges[1] = {edge};
  aicad_shape_handle_t wire = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_make_wire_from_edges(ctx, edges, 1, &wire);
  return wire;
}

// An L-shaped, two-segment polygonal spine (sharp 90-degree corner, not
// G1-continuous) -- used to probe sweep's documented spine limitation.
aicad_shape_handle_t MakeLShapedSpine(aicad_occt_context_t* ctx) {
  const double p0[3] = {0, 0, 0};
  const double p1[3] = {0, 0, 3};
  const double p2[3] = {3, 0, 3};
  aicad_shape_handle_t e0, e1;
  aicad_occt_make_line_edge(ctx, p0, p1, &e0);
  aicad_occt_make_line_edge(ctx, p1, p2, &e1);
  aicad_shape_handle_t edges[2] = {e0, e1};
  aicad_shape_handle_t wire = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_make_wire_from_edges(ctx, edges, 2, &wire);
  return wire;
}

// A spine that folds back on itself along the same line (the tangent
// reverses by ~180 degrees at the corner) -- a genuinely degenerate sweep
// path, more extreme than a right-angle corner.
aicad_shape_handle_t MakeFoldedBackSpine(aicad_occt_context_t* ctx) {
  const double p0[3] = {0, 0, 0};
  const double p1[3] = {0, 0, 3};
  const double p2[3] = {0, 0, 0.5};
  aicad_shape_handle_t e0, e1;
  aicad_occt_make_line_edge(ctx, p0, p1, &e0);
  aicad_occt_make_line_edge(ctx, p1, p2, &e1);
  aicad_shape_handle_t edges[2] = {e0, e1};
  aicad_shape_handle_t wire = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_make_wire_from_edges(ctx, edges, 2, &wire);
  return wire;
}

}  // namespace

int main() {
  aicad_occt_context_t* ctx = nullptr;
  Check(aicad_occt_context_create(&ctx) == AICAD_OCCT_OK, "context_create succeeds");

  // --- sweep: unit-square profile along a straight +Z spine of length 5
  //     must match extrude's own analytic result (volume 5.0) -- a
  //     straight spine sweep is geometrically identical to an extrusion. ---
  aicad_shape_handle_t square_profile = MakeSquareFace(ctx, /*side=*/1.0, /*z=*/0.0);
  const double p0[3] = {0, 0, 0};
  const double p1[3] = {0, 0, 5};
  aicad_shape_handle_t straight_spine = MakeStraightLineSpine(ctx, p0, p1);
  aicad_shape_handle_t swept = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_sweep(ctx, square_profile, straight_spine, &swept) == AICAD_OCCT_OK,
        "sweep succeeds for a planar square profile along a straight spine");
  int is_valid = 0;
  Check(aicad_occt_shape_is_valid(ctx, swept, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "the swept solid is a valid B-rep");
  double volume = 0.0;
  Check(aicad_occt_shape_volume(ctx, swept, &volume) == AICAD_OCCT_OK &&
            NearlyEqual(volume, 5.0, 1e-6),
        "straight-spine sweep volume matches extrude's analytic result (1*1*5=5.0)");

  // --- sweep: circular profile along a straight spine -> cylinder volume ---
  const double center[3] = {0, 0, 0};
  const double normal[3] = {0, 0, 1};
  aicad_shape_handle_t circle_wire = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_make_circle_wire(ctx, center, normal, /*radius=*/2.0, &circle_wire);
  aicad_shape_handle_t circle_face = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_make_face_from_wire(ctx, circle_wire, &circle_face);
  aicad_shape_handle_t straight_spine2 = MakeStraightLineSpine(ctx, p0, p1);
  aicad_shape_handle_t swept_cyl = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_sweep(ctx, circle_face, straight_spine2, &swept_cyl) == AICAD_OCCT_OK,
        "sweep succeeds for a circular profile along a straight spine");
  double cyl_volume = 0.0;
  const double kPi = 3.14159265358979323846;
  const double expected_cyl_volume = kPi * 2.0 * 2.0 * 5.0;
  Check(aicad_occt_shape_volume(ctx, swept_cyl, &cyl_volume) == AICAD_OCCT_OK &&
            NearlyEqual(cyl_volume, expected_cyl_volume, expected_cyl_volume * 1e-6),
        "circular-profile straight-spine sweep volume matches pi*r^2*h analytically");

  // --- sweep: adversarial / wrong-handle-kind ---
  aicad_shape_handle_t another_profile = MakeSquareFace(ctx, 1.0, 0.0);
  Check(aicad_occt_sweep(ctx, straight_spine, another_profile, &swept) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "sweep rejects a profile handle that is not a face (a wire, here)");
  aicad_shape_handle_t an_edge = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_make_line_edge(ctx, p0, p1, &an_edge);
  Check(aicad_occt_sweep(ctx, another_profile, an_edge, &swept) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "sweep rejects a spine handle that is not a wire (an edge, here)");

  // --- sweep: documented limitation -- a non-G1-continuous (sharp-corner)
  //     polygonal spine. OCCT's own BRepOffsetAPI_MakePipe documentation
  //     requires G1 continuity; this probes what actually happens rather
  //     than assuming (see project/reports/AICAD-025.md). ---
  aicad_shape_handle_t l_spine = MakeLShapedSpine(ctx);
  aicad_shape_handle_t l_profile = MakeSquareFace(ctx, 0.5, 0.0);
  aicad_shape_handle_t l_swept = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_status_t l_status = aicad_occt_sweep(ctx, l_profile, l_spine, &l_swept);
  std::printf(
      "INFO: sweep along a sharp-cornered (L-shaped) spine returned status %d "
      "(0=OK, 6=OPERATION_FAILED) -- recorded as a documented limitation either way\n",
      static_cast<int>(l_status));
  Check(l_status == AICAD_OCCT_OK || l_status == AICAD_OCCT_ERR_OPERATION_FAILED,
        "sweep on a sharp-cornered spine either succeeds or fails cleanly (never crashes/hangs)");
  if (l_status == AICAD_OCCT_OK) {
    // OCCT's BRepOffsetAPI_MakePipe (IsDone) succeeding does NOT imply the
    // result is a topologically valid solid at a non-G1 corner -- the same
    // "construction vs. validity are distinct concepts" pattern already
    // established for aicad_occt_make_face_from_wire on an open wire
    // (Stage-1 kernel policy #14). Record what actually happens (checked
    // via BRepCheck_Analyzer) rather than assuming either outcome; this
    // is the documented sharp-corner-spine limitation, not a test bug.
    int l_valid = 0;
    Check(aicad_occt_shape_is_valid(ctx, l_swept, &l_valid) == AICAD_OCCT_OK,
          "shape_is_valid itself succeeds on the sharp-corner-spine sweep result");
    std::printf(
        "INFO: sharp-corner-spine sweep result validity = %s (construction succeeding does "
        "not imply topological validity at a non-G1 corner -- Stage-1 kernel policy #14)\n",
        l_valid ? "VALID" : "INVALID");
    double l_volume = 0.0;
    Check(aicad_occt_shape_volume(ctx, l_swept, &l_volume) == AICAD_OCCT_OK,
          "shape_volume itself succeeds (returns a finite number) on the sharp-corner result");
  }

  // --- sweep: a more extreme degenerate spine (folds back on itself,
  //     tangent reverses ~180 degrees) -- probes the actual failure
  //     boundary rather than assuming one. ---
  aicad_shape_handle_t folded_spine = MakeFoldedBackSpine(ctx);
  aicad_shape_handle_t folded_profile = MakeSquareFace(ctx, 0.5, 0.0);
  aicad_shape_handle_t folded_swept = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_status_t folded_status =
      aicad_occt_sweep(ctx, folded_profile, folded_spine, &folded_swept);
  std::printf(
      "INFO: sweep along a folded-back (180-degree-reversal) spine returned "
      "status %d (0=OK, 6=OPERATION_FAILED)\n",
      static_cast<int>(folded_status));
  Check(folded_status == AICAD_OCCT_OK || folded_status == AICAD_OCCT_ERR_OPERATION_FAILED,
        "sweep on a folded-back spine either succeeds or fails cleanly (never crashes/hangs)");
  if (folded_status == AICAD_OCCT_OK) {
    int folded_valid = 0;
    Check(aicad_occt_shape_is_valid(ctx, folded_swept, &folded_valid) == AICAD_OCCT_OK,
          "shape_is_valid itself succeeds on the folded-back-spine sweep result");
    std::printf("INFO: folded-back-spine sweep result validity = %s\n",
                folded_valid ? "VALID" : "INVALID");
  }

  // --- loft: two identical squares (side 1) at z=0 and z=5 -> a ruled
  //     loft between identical, aligned sections must equal a prism
  //     (matches extrude's own analytic result). ---
  aicad_shape_handle_t sec_a = MakeCenteredSquareWire(ctx, 1.0, 0.0);
  aicad_shape_handle_t sec_b = MakeCenteredSquareWire(ctx, 1.0, 5.0);
  aicad_shape_handle_t sections_prism[2] = {sec_a, sec_b};
  aicad_shape_handle_t loft_prism = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_loft(ctx, sections_prism, 2, &loft_prism) == AICAD_OCCT_OK,
        "loft succeeds for two identical square sections");
  Check(aicad_occt_shape_is_valid(ctx, loft_prism, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "the lofted solid is a valid B-rep");
  double prism_volume = 0.0;
  Check(aicad_occt_shape_volume(ctx, loft_prism, &prism_volume) == AICAD_OCCT_OK &&
            NearlyEqual(prism_volume, 5.0, 1e-6),
        "loft between two identical square sections matches prism volume 1*1*5=5.0");

  // --- loft: square frustum, side 2 at z=0 and side 4 at z=3 -> analytic
  //     frustum-of-pyramid volume h/3*(A1+A2+sqrt(A1*A2)). ---
  aicad_shape_handle_t frustum_a = MakeCenteredSquareWire(ctx, 2.0, 0.0);
  aicad_shape_handle_t frustum_b = MakeCenteredSquareWire(ctx, 4.0, 3.0);
  aicad_shape_handle_t sections_frustum[2] = {frustum_a, frustum_b};
  aicad_shape_handle_t loft_frustum = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_loft(ctx, sections_frustum, 2, &loft_frustum) == AICAD_OCCT_OK,
        "loft succeeds for two differently-sized square sections (frustum)");
  Check(aicad_occt_shape_is_valid(ctx, loft_frustum, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "the frustum loft is a valid B-rep");
  double frustum_volume = 0.0;
  const double a1 = 2.0 * 2.0;
  const double a2 = 4.0 * 4.0;
  const double h = 3.0;
  const double expected_frustum_volume = (h / 3.0) * (a1 + a2 + std::sqrt(a1 * a2));
  Check(aicad_occt_shape_volume(ctx, loft_frustum, &frustum_volume) == AICAD_OCCT_OK &&
            NearlyEqual(frustum_volume, expected_frustum_volume, expected_frustum_volume * 1e-6),
        "square-frustum loft volume matches h/3*(A1+A2+sqrt(A1*A2)) analytically");

  // --- loft: adversarial ---
  aicad_shape_handle_t only_one[1] = {frustum_a};
  Check(aicad_occt_loft(ctx, only_one, 1, &loft_prism) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "loft rejects a single section (fewer than 2)");
  Check(aicad_occt_loft(ctx, nullptr, 2, &loft_prism) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "loft rejects a null sections pointer");
  aicad_shape_handle_t wrong_kind[2] = {an_edge, frustum_b};
  Check(aicad_occt_loft(ctx, wrong_kind, 2, &loft_prism) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "loft rejects a section handle that is not a wire (an edge, here)");

  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "context_destroy succeeds");

  if (g_failures > 0) {
    std::fprintf(stderr, "sweep_loft_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("sweep_loft_test: all checks PASSED\n");
  return 0;
}
