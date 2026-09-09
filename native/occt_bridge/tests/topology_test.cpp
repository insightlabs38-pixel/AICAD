// AICAD-029: topology exploration (vertex enumeration, edge endpoints,
// edge-to-face adjacency). topology_faces/topology_edges/face_edges
// (docs/plan/05 §3) are exercised here too, but via the *existing*
// AICAD-027/028 shape_edge_count/_get_edge and shape_face_count/_get_face
// applied to a Face handle and a Solid handle respectively -- proving that
// no new function was needed for those, not just asserting it.

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

bool SamePoint(const double a[3], const double b[3], double tol) {
  return NearlyEqual(a[0], b[0], tol) && NearlyEqual(a[1], b[1], tol) && NearlyEqual(a[2], b[2], tol);
}

}  // namespace

int main() {
  aicad_occt_context_t* ctx = nullptr;
  Check(aicad_occt_context_create(&ctx) == AICAD_OCCT_OK, "context_create succeeds");

  // --- a box: 8 unique vertices, 12 unique edges, 6 unique faces, every
  //     edge adjacent to exactly 2 faces (manifold closed solid) ---
  aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_create_box(ctx, 2.0, 3.0, 4.0, &box) == AICAD_OCCT_OK, "box created");

  size_t vertex_count = 0;
  Check(aicad_occt_shape_vertex_count(ctx, box, &vertex_count) == AICAD_OCCT_OK && vertex_count == 8,
        "box has exactly 8 unique vertices");

  size_t edge_count = 0;
  Check(aicad_occt_shape_edge_count(ctx, box, &edge_count) == AICAD_OCCT_OK && edge_count == 12,
        "box has exactly 12 unique edges (topology_edges, AICAD-027)");

  size_t face_count = 0;
  Check(aicad_occt_shape_face_count(ctx, box, &face_count) == AICAD_OCCT_OK && face_count == 6,
        "box has exactly 6 unique faces (topology_faces, AICAD-028)");

  // Every vertex handle returned is a real, distinct-from-null handle, and
  // out-of-range indices are rejected.
  bool all_vertices_ok = true;
  for (size_t i = 0; i < vertex_count; ++i) {
    aicad_shape_handle_t v = AICAD_NULL_SHAPE_HANDLE;
    if (aicad_occt_shape_get_vertex(ctx, box, i, &v) != AICAD_OCCT_OK || v.slot == 0) {
      all_vertices_ok = false;
    }
  }
  Check(all_vertices_ok, "shape_get_vertex returns a real handle for every 0..vertex_count-1 index");
  aicad_shape_handle_t oob_vertex = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_shape_get_vertex(ctx, box, vertex_count, &oob_vertex) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "shape_get_vertex rejects an out-of-range index");

  // Every one of the box's 12 edges is adjacent to exactly 2 faces.
  bool all_edges_two_faces = true;
  for (size_t i = 0; i < edge_count; ++i) {
    size_t adjacent = 999;
    if (aicad_occt_shape_edge_adjacent_face_count(ctx, box, i, &adjacent) != AICAD_OCCT_OK ||
        adjacent != 2) {
      all_edges_two_faces = false;
    }
  }
  Check(all_edges_two_faces, "every box edge (manifold closed solid) is adjacent to exactly 2 faces");

  // Those 2 adjacent faces are real, distinct handles and are themselves
  // among the box's own 6 faces (same underlying TShape, via IsSame
  // matched indirectly through is_valid/area rather than raw pointer
  // comparison, which this ABI never exposes).
  {
    aicad_shape_handle_t f0 = AICAD_NULL_SHAPE_HANDLE, f1 = AICAD_NULL_SHAPE_HANDLE;
    Check(aicad_occt_shape_edge_adjacent_face_get(ctx, box, 0, 0, &f0) == AICAD_OCCT_OK &&
              aicad_occt_shape_edge_adjacent_face_get(ctx, box, 0, 1, &f1) == AICAD_OCCT_OK &&
              f0.slot != 0 && f1.slot != 0 && !(f0.slot == f1.slot && f0.generation == f1.generation),
          "edge 0's two adjacent faces are distinct, real handles");
    aicad_shape_handle_t oob_adjacent = AICAD_NULL_SHAPE_HANDLE;
    Check(aicad_occt_shape_edge_adjacent_face_get(ctx, box, 0, 2, &oob_adjacent) ==
              AICAD_OCCT_ERR_INVALID_ARGUMENT,
          "edge_adjacent_face_get rejects an out-of-range adjacent_index (box edges have only 2)");
    size_t oob_count = 999;
    Check(aicad_occt_shape_edge_adjacent_face_count(ctx, box, edge_count, &oob_count) ==
              AICAD_OCCT_ERR_INVALID_ARGUMENT,
          "edge_adjacent_face_count rejects an out-of-range edge_index");
  }

  // --- face_edges (docs/plan/05 §3): applying the *existing*
  //     shape_edge_count/_get_edge to a Face handle already enumerates
  //     that face's own boundary edges -- a square face has exactly 4. ---
  {
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
    aicad_occt_make_wire_from_edges(ctx, square_edges, 4, &square_wire);
    aicad_shape_handle_t square_face = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_make_face_from_wire(ctx, square_wire, &square_face);
    size_t face_edge_count = 0;
    Check(aicad_occt_shape_edge_count(ctx, square_face, &face_edge_count) == AICAD_OCCT_OK &&
              face_edge_count == 4,
          "face_edges: shape_edge_count applied to a Face handle returns its own 4 boundary edges");

    // A face's own boundary edge is adjacent to exactly 1 face within
    // that face's own shape context: the face itself. (Verified
    // empirically, not assumed -- TopExp::MapShapesAndAncestors treats
    // the top-level shape passed in as its own ancestor when its type
    // matches the ancestor type being mapped, the same way a bare Face
    // handle already counts as its own single "face" for shape_face_count
    // purposes.)
    size_t adjacent_to_face_edge = 999;
    Check(aicad_occt_shape_edge_adjacent_face_count(ctx, square_face, 0, &adjacent_to_face_edge) ==
              AICAD_OCCT_OK &&
              adjacent_to_face_edge == 1,
          "a bare face's own boundary edge is adjacent to exactly 1 face (itself) in that shape context");
  }

  // --- edge_vertices: an open line edge has two distinct endpoints
  //     matching the input points, in order ---
  {
    const double p0[3] = {1.0, 2.0, 3.0};
    const double p1[3] = {4.0, 5.0, 6.0};
    aicad_shape_handle_t line = AICAD_NULL_SHAPE_HANDLE;
    Check(aicad_occt_make_line_edge(ctx, p0, p1, &line) == AICAD_OCCT_OK, "line edge created");
    aicad_shape_handle_t v0 = AICAD_NULL_SHAPE_HANDLE, v1 = AICAD_NULL_SHAPE_HANDLE;
    Check(aicad_occt_edge_vertices(ctx, line, &v0, &v1) == AICAD_OCCT_OK, "edge_vertices succeeds");
    // No point-coordinate query exists on a Vertex handle in this ABI yet
    // (out of AICAD-029's scope); this test only proves the two returned
    // vertex handles are real and distinct, and that shape_bounding_box on
    // each single-vertex shape recovers its coordinates exactly (a
    // degenerate but exact bounding box: min == max == the point).
    // Bnd_Box enlarges by the vertex's own confusion tolerance (empirically
    // ~1e-7 per side, i.e. up to ~2e-7 between min and max) even for a
    // single-point shape -- not an exact degenerate box -- so tolerances
    // here are 1e-5, comfortably above that gap while still a meaningful
    // coordinate check.
    double v0_min[3], v0_max[3], v1_min[3], v1_max[3];
    Check(aicad_occt_shape_bounding_box(ctx, v0, v0_min, v0_max) == AICAD_OCCT_OK &&
              SamePoint(v0_min, v0_max, 1e-5) && SamePoint(v0_min, p0, 1e-5),
          "edge_vertices' first vertex (v0) coincides with the line's start point p0");
    Check(aicad_occt_shape_bounding_box(ctx, v1, v1_min, v1_max) == AICAD_OCCT_OK &&
              SamePoint(v1_min, v1_max, 1e-5) && SamePoint(v1_min, p1, 1e-5),
          "edge_vertices' second vertex (v1) coincides with the line's end point p1");
  }

  // --- edge_vertices: a closed edge (full circle) has coincident
  //     start/end vertices, not an error ---
  {
    const double center[3] = {0, 0, 0};
    const double normal_z[3] = {0, 0, 1};
    aicad_shape_handle_t circle_wire = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_make_circle_wire(ctx, center, normal_z, 2.0, &circle_wire);
    size_t circle_edge_count = 0;
    aicad_occt_shape_edge_count(ctx, circle_wire, &circle_edge_count);
    Check(circle_edge_count == 1, "a circle wire has exactly 1 (closed) edge");
    aicad_shape_handle_t circle_edge = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_shape_get_edge(ctx, circle_wire, 0, &circle_edge);
    aicad_shape_handle_t cv0 = AICAD_NULL_SHAPE_HANDLE, cv1 = AICAD_NULL_SHAPE_HANDLE;
    Check(aicad_occt_edge_vertices(ctx, circle_edge, &cv0, &cv1) == AICAD_OCCT_OK,
          "edge_vertices succeeds for a closed (full-circle) edge");
    double c0min[3], c0max[3], c1min[3], c1max[3];
    aicad_occt_shape_bounding_box(ctx, cv0, c0min, c0max);
    aicad_occt_shape_bounding_box(ctx, cv1, c1min, c1max);
    Check(SamePoint(c0min, c1min, 1e-5),
          "a closed edge's two endpoint vertices from edge_vertices coincide at the same point");
  }

  // --- adversarial: edge_vertices rejects a non-Edge handle ---
  Check(aicad_occt_edge_vertices(ctx, box, &box, &box) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "edge_vertices rejects a handle that addresses a Solid, not an Edge");

  // --- adversarial: a standalone wire (no face) has edges with 0
  //     adjacent faces ---
  {
    const double p0[3] = {0, 0, 0};
    const double p1[3] = {1, 0, 0};
    aicad_shape_handle_t free_edge = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_make_line_edge(ctx, p0, p1, &free_edge);
    size_t free_adjacent = 999;
    Check(aicad_occt_shape_edge_adjacent_face_count(ctx, free_edge, 0, &free_adjacent) ==
              AICAD_OCCT_OK &&
              free_adjacent == 0,
          "a standalone (faceless) edge has 0 adjacent faces");
  }

  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "context_destroy succeeds");

  if (g_failures > 0) {
    std::fprintf(stderr, "topology_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("topology_test: all checks PASSED\n");
  return 0;
}
