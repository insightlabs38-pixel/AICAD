// AICAD-030: length and center-of-mass queries (volume/area/bounding_box
// already covered by earlier batches' own tests).

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

  // --- length: a single line edge's length is the Euclidean distance
  //     between its endpoints ---
  {
    const double p0[3] = {0, 0, 0};
    const double p1[3] = {3, 4, 0};
    aicad_shape_handle_t line = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_make_line_edge(ctx, p0, p1, &line);
    double length = 0.0;
    Check(aicad_occt_shape_length(ctx, line, &length) == AICAD_OCCT_OK && NearlyEqual(length, 5.0, 1e-9),
          "a 3-4-5 line edge has length exactly 5.0");
  }

  // --- length: a box's total edge length is 4*(dx+dy+dz) (12 edges: 4
  //     each of the 3 distinct lengths) ---
  {
    const double dx = 2.0, dy = 3.0, dz = 4.0;
    aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_box(ctx, dx, dy, dz, &box);
    double length = 0.0;
    Check(aicad_occt_shape_length(ctx, box, &length) == AICAD_OCCT_OK &&
              NearlyEqual(length, 4.0 * (dx + dy + dz), 1e-9),
          "a box's total edge length matches 4*(dx+dy+dz)");
  }

  // --- center_of_mass: a box centered at the origin (via transform) has
  //     its centroid at the origin; an axis-aligned box built from
  //     create_box (which places one corner at the origin) has its
  //     centroid at (dx/2, dy/2, dz/2) ---
  {
    const double dx = 2.0, dy = 3.0, dz = 4.0;
    aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_box(ctx, dx, dy, dz, &box);
    double centre[3] = {999, 999, 999};
    Check(aicad_occt_shape_center_of_mass(ctx, box, centre) == AICAD_OCCT_OK &&
              NearlyEqual(centre[0], dx / 2.0, 1e-9) && NearlyEqual(centre[1], dy / 2.0, 1e-9) &&
              NearlyEqual(centre[2], dz / 2.0, 1e-9),
          "a create_box's volume-weighted center of mass is its geometric center (dx/2,dy/2,dz/2)");
  }

  // --- center_of_mass: a planar square face's centroid is its geometric
  //     center (an area-weighted centroid, since the shape has faces but
  //     no solid) ---
  {
    const double sq0[3] = {0, 0, 0};
    const double sq1[3] = {2, 0, 0};
    const double sq2[3] = {2, 2, 0};
    const double sq3[3] = {0, 2, 0};
    aicad_shape_handle_t e0, e1, e2, e3;
    aicad_occt_make_line_edge(ctx, sq0, sq1, &e0);
    aicad_occt_make_line_edge(ctx, sq1, sq2, &e1);
    aicad_occt_make_line_edge(ctx, sq2, sq3, &e2);
    aicad_occt_make_line_edge(ctx, sq3, sq0, &e3);
    aicad_shape_handle_t square_edges[4] = {e0, e1, e2, e3};
    aicad_shape_handle_t square_wire = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_make_wire_from_edges(ctx, square_edges, 4, &square_wire);
    aicad_shape_handle_t square_face = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_make_face_from_wire(ctx, square_wire, &square_face);
    double centre[3] = {999, 999, 999};
    Check(aicad_occt_shape_center_of_mass(ctx, square_face, centre) == AICAD_OCCT_OK &&
              NearlyEqual(centre[0], 1.0, 1e-9) && NearlyEqual(centre[1], 1.0, 1e-9) &&
              NearlyEqual(centre[2], 0.0, 1e-9),
          "a square face's area-weighted center of mass is its geometric center (1,1,0)");
  }

  // --- center_of_mass: a standalone edge's centroid is its midpoint (a
  //     length-weighted centroid, since the shape has edges but no
  //     face/solid) ---
  {
    const double p0[3] = {0, 0, 0};
    const double p1[3] = {10, 0, 0};
    aicad_shape_handle_t line = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_make_line_edge(ctx, p0, p1, &line);
    double centre[3] = {999, 999, 999};
    Check(aicad_occt_shape_center_of_mass(ctx, line, centre) == AICAD_OCCT_OK &&
              NearlyEqual(centre[0], 5.0, 1e-9) && NearlyEqual(centre[1], 0.0, 1e-9) &&
              NearlyEqual(centre[2], 0.0, 1e-9),
          "a standalone line edge's length-weighted center of mass is its midpoint");
  }

  // --- adversarial: center_of_mass on a bare Vertex (no edges, faces, or
  //     solids) fails cleanly rather than returning a meaningless (0,0,0) ---
  {
    aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &box);
    aicad_shape_handle_t vertex = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_shape_get_vertex(ctx, box, 0, &vertex);
    double centre[3] = {999, 999, 999};
    Check(aicad_occt_shape_center_of_mass(ctx, vertex, centre) == AICAD_OCCT_ERR_OPERATION_FAILED,
          "center_of_mass on a bare Vertex (no edge/face/solid content) fails cleanly");
  }

  // --- adversarial: length of a shape with no edges (a bare Vertex) is 0,
  //     not an error (BRepGProp::LinearProperties itself does not fail on
  //     an empty edge set -- it reports zero mass, unlike the
  //     center-of-mass dispatch above, which explicitly guards against a
  //     meaningless zero-centroid) ---
  {
    aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &box);
    aicad_shape_handle_t vertex = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_shape_get_vertex(ctx, box, 0, &vertex);
    double length = 999.0;
    Check(aicad_occt_shape_length(ctx, vertex, &length) == AICAD_OCCT_OK && NearlyEqual(length, 0.0, 1e-12),
          "length of a bare Vertex (no edges) is exactly 0, not an error");
  }

  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "context_destroy succeeds");

  if (g_failures > 0) {
    std::fprintf(stderr, "measurement_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("measurement_test: all checks PASSED\n");
  return 0;
}
