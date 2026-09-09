// AICAD-023: planar face from closed wire.

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

}  // namespace

int main() {
  aicad_occt_context_t* ctx = nullptr;
  Check(aicad_occt_context_create(&ctx) == AICAD_OCCT_OK, "context_create succeeds");

  // --- face from a circle wire: area = pi*r^2 ---
  const double center[3] = {0, 0, 0};
  const double normal_z[3] = {0, 0, 1};
  const double radius = 3.0;
  aicad_shape_handle_t circle_wire = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_circle_wire(ctx, center, normal_z, radius, &circle_wire) == AICAD_OCCT_OK,
        "circle wire created");
  aicad_shape_handle_t circle_face = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_face_from_wire(ctx, circle_wire, &circle_face) == AICAD_OCCT_OK,
        "make_face_from_wire succeeds for a closed planar circle wire");
  int is_valid = 0;
  Check(aicad_occt_shape_is_valid(ctx, circle_face, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "the resulting circular face is a valid B-rep");
  double area = 0.0;
  const double kPi = 3.14159265358979323846;
  Check(aicad_occt_shape_area(ctx, circle_face, &area) == AICAD_OCCT_OK &&
            NearlyEqual(area, kPi * radius * radius, 1e-6),
        "circular face area matches pi*r^2 analytically");

  // --- face from a square wire (4 line edges): area = 1.0 ---
  const double sq0[3] = {0, 0, 0};
  const double sq1[3] = {1, 0, 0};
  const double sq2[3] = {1, 1, 0};
  const double sq3[3] = {0, 1, 0};
  aicad_shape_handle_t e0, e1, e2, e3;
  aicad_occt_make_line_edge(ctx, sq0, sq1, &e0);
  aicad_occt_make_line_edge(ctx, sq1, sq2, &e1);
  aicad_occt_make_line_edge(ctx, sq2, sq3, &e2);
  aicad_occt_make_line_edge(ctx, sq3, sq0, &e3);
  aicad_shape_handle_t square_edges[4] = {e0, e1, e2, e3};
  aicad_shape_handle_t square_wire = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_wire_from_edges(ctx, square_edges, 4, &square_wire) == AICAD_OCCT_OK,
        "square wire created");
  aicad_shape_handle_t square_face = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_face_from_wire(ctx, square_wire, &square_face) == AICAD_OCCT_OK,
        "make_face_from_wire succeeds for a closed planar square wire");
  Check(aicad_occt_shape_area(ctx, square_face, &area) == AICAD_OCCT_OK &&
            NearlyEqual(area, 1.0, 1e-9),
        "square face area is exactly 1.0");

  // --- adversarial: wrong handle kind (an edge, not a wire) ---
  Check(aicad_occt_make_face_from_wire(ctx, e0, &square_face) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "make_face_from_wire rejects a handle that addresses an edge, not a wire");

  // --- adversarial: an open (unclosed) wire does not become a *valid*
  //     face. BRepBuilderAPI_MakeFace's own IsDone() only reports whether
  //     the builder algorithm ran to completion, not whether the result
  //     is topologically valid -- it happily builds a structurally
  //     complete but invalid face bounded by an open wire. Stage-1 kernel
  //     policy #14 ("validation and repair/healing are distinct semantic
  //     concepts") means this bridge does not silently fold a validity
  //     check into construction; the separate `shape_is_valid` query
  //     (already exposed for exactly this purpose) is what must catch
  //     this, and this test is the evidence that it does. ---
  aicad_shape_handle_t open_edges[3] = {e0, e1, e2};
  aicad_shape_handle_t open_wire = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_wire_from_edges(ctx, open_edges, 3, &open_wire) == AICAD_OCCT_OK,
        "an open (3-edge, unclosed) wire is itself constructible");
  aicad_shape_handle_t open_face = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_face_from_wire(ctx, open_wire, &open_face) == AICAD_OCCT_OK,
        "make_face_from_wire's builder completes even for an open wire (IsDone is not a validity check)");
  int open_face_valid = 1;
  Check(aicad_occt_shape_is_valid(ctx, open_face, &open_face_valid) == AICAD_OCCT_OK &&
            open_face_valid == 0,
        "shape_is_valid correctly reports the open-wire face as INVALID -- callers must check this,"
        " not just the construction status");

  // --- adversarial: a non-planar closed wire cannot become a (planar-only) face ---
  const double np0[3] = {0, 0, 0};
  const double np1[3] = {1, 0, 0};
  const double np2[3] = {1, 1, 1};  // out of the Z=0 plane
  const double np3[3] = {0, 1, 0};
  aicad_shape_handle_t ne0, ne1, ne2, ne3;
  aicad_occt_make_line_edge(ctx, np0, np1, &ne0);
  aicad_occt_make_line_edge(ctx, np1, np2, &ne1);
  aicad_occt_make_line_edge(ctx, np2, np3, &ne2);
  aicad_occt_make_line_edge(ctx, np3, np0, &ne3);
  aicad_shape_handle_t nonplanar_edges[4] = {ne0, ne1, ne2, ne3};
  aicad_shape_handle_t nonplanar_wire = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_wire_from_edges(ctx, nonplanar_edges, 4, &nonplanar_wire) == AICAD_OCCT_OK,
        "a non-planar closed wire is itself constructible");
  aicad_shape_handle_t nonplanar_face = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_face_from_wire(ctx, nonplanar_wire, &nonplanar_face) ==
            AICAD_OCCT_ERR_OPERATION_FAILED,
        "make_face_from_wire rejects a non-planar closed wire (planar-only construction)");

  // --- adversarial: very small and very large circle radii still face correctly ---
  aicad_shape_handle_t tiny_wire = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_make_circle_wire(ctx, center, normal_z, 1e-6, &tiny_wire);
  aicad_shape_handle_t tiny_face = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_face_from_wire(ctx, tiny_wire, &tiny_face) == AICAD_OCCT_OK,
        "make_face_from_wire succeeds for a very small (1e-6 radius) circle wire");
  aicad_shape_handle_t huge_wire = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_make_circle_wire(ctx, center, normal_z, 1e8, &huge_wire);
  aicad_shape_handle_t huge_face = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_make_face_from_wire(ctx, huge_wire, &huge_face) == AICAD_OCCT_OK,
        "make_face_from_wire succeeds for a very large (1e8 radius) circle wire");

  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "context_destroy succeeds");

  if (g_failures > 0) {
    std::fprintf(stderr, "face_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("face_test: all checks PASSED\n");
  return 0;
}
