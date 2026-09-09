// AICAD-027: fillet and chamfer.

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

// Finds the (unique) edge of `shape` whose bounding box matches the given
// axis-aligned segment, within tolerance. Returns AICAD_NULL_SHAPE_HANDLE
// if none matches -- used to select one specific, known box edge by its
// geometry rather than by an assumed enumeration index (this bridge's own
// header doc comment is explicit that raw edge order/index is ephemeral
// and must never be treated as durable identity).
aicad_shape_handle_t FindEdgeByBoundingBox(aicad_occt_context_t* ctx,
                                            aicad_shape_handle_t shape,
                                            const double want_min[3],
                                            const double want_max[3]) {
  size_t count = 0;
  aicad_occt_shape_edge_count(ctx, shape, &count);
  for (size_t i = 0; i < count; ++i) {
    aicad_shape_handle_t edge = AICAD_NULL_SHAPE_HANDLE;
    if (aicad_occt_shape_get_edge(ctx, shape, i, &edge) != AICAD_OCCT_OK) {
      continue;
    }
    double bb_min[3], bb_max[3];
    if (aicad_occt_shape_bounding_box(ctx, edge, bb_min, bb_max) != AICAD_OCCT_OK) {
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
      return edge;
    }
  }
  return AICAD_NULL_SHAPE_HANDLE;
}

}  // namespace

int main() {
  aicad_occt_context_t* ctx = nullptr;
  Check(aicad_occt_context_create(&ctx) == AICAD_OCCT_OK, "context_create succeeds");
  const double kPi = 3.14159265358979323846;

  // --- edge enumeration: a box has exactly 12 unique edges (verified
  //     empirically that a raw explorer over-counts to 24 -- see
  //     project/reports/AICAD-027.md -- this ABI must report the
  //     de-duplicated count). ---
  const double dx = 4.0, dy = 5.0, dz = 6.0;
  aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, dx, dy, dz, &box);
  size_t edge_count = 0;
  Check(aicad_occt_shape_edge_count(ctx, box, &edge_count) == AICAD_OCCT_OK && edge_count == 12,
        "a box has exactly 12 unique edges");
  aicad_shape_handle_t first_edge = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_shape_get_edge(ctx, box, 0, &first_edge) == AICAD_OCCT_OK,
        "get_edge(0) succeeds for a box");
  int edge_is_valid = 0;
  Check(aicad_occt_shape_is_valid(ctx, first_edge, &edge_is_valid) == AICAD_OCCT_OK &&
            edge_is_valid == 1,
        "the returned edge handle is itself a valid shape");
  aicad_shape_handle_t out_of_range = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_shape_get_edge(ctx, box, 12, &out_of_range) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "get_edge rejects an out-of-range index (12, when count is 12)");

  // --- fillet ALL edges of a box with radius r: the result is a
  //     "rounded box" whose volume is the well-known closed-form
  //     Minkowski-sum-with-a-ball formula:
  //       V = Lx*Ly*Lz + 2r(Lx*Ly+Ly*Lz+Lz*Lx) + pi*r^2*(Lx+Ly+Lz) + (4/3)*pi*r^3
  //     where Lx=dx-2r, Ly=dy-2r, Lz=dz-2r (the box inset by r on every
  //     side). ---
  aicad_shape_handle_t fillet_box = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, dx, dy, dz, &fillet_box);
  size_t fillet_edge_count = 0;
  aicad_occt_shape_edge_count(ctx, fillet_box, &fillet_edge_count);
  aicad_shape_handle_t all_edges[12];
  for (size_t i = 0; i < fillet_edge_count; ++i) {
    aicad_occt_shape_get_edge(ctx, fillet_box, i, &all_edges[i]);
  }
  const double r = 1.0;
  aicad_shape_handle_t rounded = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_fillet(ctx, fillet_box, all_edges, fillet_edge_count, r, &rounded) ==
            AICAD_OCCT_OK,
        "fillet succeeds when applied to all 12 edges of a box");
  int rounded_valid = 0;
  Check(aicad_occt_shape_is_valid(ctx, rounded, &rounded_valid) == AICAD_OCCT_OK &&
            rounded_valid == 1,
        "the fully-filleted box is a valid B-rep");
  double rounded_volume = 0.0;
  const double lx = dx - 2 * r, ly = dy - 2 * r, lz = dz - 2 * r;
  const double expected_rounded_volume = lx * ly * lz + 2 * r * (lx * ly + ly * lz + lz * lx) +
                                          kPi * r * r * (lx + ly + lz) +
                                          (4.0 / 3.0) * kPi * r * r * r;
  Check(aicad_occt_shape_volume(ctx, rounded, &rounded_volume) == AICAD_OCCT_OK &&
            NearlyEqual(rounded_volume, expected_rounded_volume, expected_rounded_volume * 1e-4),
        "fully-filleted box volume matches the analytic rounded-box (Minkowski sum with a "
        "ball) formula");

  // --- chamfer exactly ONE identified edge of a box: removes a clean
  //     triangular prism (cross-section legs d,d, length = the full edge
  //     length, since neither adjacent edge is also modified) -> volume
  //     = box_volume - (d^2/2)*edge_length. ---
  aicad_shape_handle_t chamfer_box = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, dx, dy, dz, &chamfer_box);
  // The edge from (0,0,dz) to (dx,0,dz): intersection of the y=0 face and
  // the z=dz (top) face, length dx.
  const double want_min[3] = {0, 0, dz};
  const double want_max[3] = {dx, 0, dz};
  aicad_shape_handle_t target_edge = FindEdgeByBoundingBox(ctx, chamfer_box, want_min, want_max);
  Check(!(target_edge.slot == 0 && target_edge.generation == 0 && target_edge.context_id == 0),
        "the specific top-front edge (0,0,dz)-(dx,0,dz) is found among the box's 12 edges");
  const double d = 0.5;
  aicad_shape_handle_t chamfered = AICAD_NULL_SHAPE_HANDLE;
  aicad_shape_handle_t single_edge_array[1] = {target_edge};
  Check(aicad_occt_chamfer(ctx, chamfer_box, single_edge_array, 1, d, &chamfered) ==
            AICAD_OCCT_OK,
        "chamfer succeeds when applied to a single identified edge");
  int chamfered_valid = 0;
  Check(aicad_occt_shape_is_valid(ctx, chamfered, &chamfered_valid) == AICAD_OCCT_OK &&
            chamfered_valid == 1,
        "the single-edge-chamfered box is a valid B-rep");
  double chamfered_volume = 0.0;
  const double box_volume = dx * dy * dz;
  const double expected_chamfered_volume = box_volume - (d * d / 2.0) * dx;
  Check(aicad_occt_shape_volume(ctx, chamfered, &chamfered_volume) == AICAD_OCCT_OK &&
            NearlyEqual(chamfered_volume, expected_chamfered_volume, expected_chamfered_volume * 1e-6),
        "single-edge chamfer volume matches box_volume - (d^2/2)*edge_length analytically");

  // --- fillet/chamfer accept any shape kind (not restricted to Solid),
  //     matching aicad_occt_boolean_union's own rationale: fillet a
  //     Compound (a boolean-union result of two disjoint boxes). ---
  aicad_shape_handle_t comp_a = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, 2.0, 2.0, 2.0, &comp_a);
  aicad_shape_handle_t comp_b_raw = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, 2.0, 2.0, 2.0, &comp_b_raw);
  const double far_matrix[12] = {1, 0, 0, 20, 0, 1, 0, 0, 0, 0, 1, 0};
  aicad_shape_handle_t comp_b = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_transform_shape(ctx, comp_b_raw, far_matrix, &comp_b);
  aicad_shape_handle_t compound = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_boolean_union(ctx, comp_a, comp_b, &compound);
  size_t compound_edge_count = 0;
  Check(aicad_occt_shape_edge_count(ctx, compound, &compound_edge_count) == AICAD_OCCT_OK &&
            compound_edge_count == 24,
        "edge_count works on a Compound (two disjoint boxes: 12+12=24 edges)");

  // --- adversarial ---
  Check(aicad_occt_fillet(ctx, box, nullptr, 1, 1.0, &rounded) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "fillet rejects a null edges pointer");
  Check(aicad_occt_fillet(ctx, box, all_edges, 0, 1.0, &rounded) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "fillet rejects a zero edge_count");
  Check(aicad_occt_fillet(ctx, box, all_edges, 1, 0.0, &rounded) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "fillet rejects a zero radius");
  Check(aicad_occt_fillet(ctx, box, all_edges, 1, -1.0, &rounded) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "fillet rejects a negative radius");
  aicad_shape_handle_t not_an_edge_array[1] = {box};  // `box` is a Solid, not an Edge.
  Check(aicad_occt_fillet(ctx, box, not_an_edge_array, 1, 1.0, &rounded) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "fillet rejects a handle that is not an edge (the solid itself, here)");
  Check(aicad_occt_chamfer(ctx, box, all_edges, 1, 0.0, &chamfered) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "chamfer rejects a zero distance");
  Check(aicad_occt_chamfer(ctx, box, not_an_edge_array, 1, 0.5, &chamfered) ==
            AICAD_OCCT_ERR_INVALID_ARGUMENT,
        "chamfer rejects a handle that is not an edge");

  // --- documented limitation: an impossible fillet, where the requested
  //     radius is larger than the geometry can support (per AGENTS.md's
  //     adversarial-cases requirement) -- probed rather than assumed. ---
  aicad_shape_handle_t tiny_box = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &tiny_box);
  aicad_shape_handle_t tiny_edge = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_shape_get_edge(ctx, tiny_box, 0, &tiny_edge);
  aicad_shape_handle_t tiny_edge_array[1] = {tiny_edge};
  aicad_shape_handle_t impossible_fillet = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_status_t impossible_status =
      aicad_occt_fillet(ctx, tiny_box, tiny_edge_array, 1, 10.0, &impossible_fillet);
  std::printf(
      "INFO: fillet with radius (10.0) much larger than the 1x1x1 box it targets returned "
      "status %d (0=OK, 6=OPERATION_FAILED) -- an oversized fillet request is expected to be "
      "geometrically impossible, not silently clamped\n",
      static_cast<int>(impossible_status));
  Check(impossible_status == AICAD_OCCT_ERR_OPERATION_FAILED || impossible_status == AICAD_OCCT_OK,
        "an oversized fillet request either fails cleanly or succeeds (never crashes/hangs)");

  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "context_destroy succeeds");

  if (g_failures > 0) {
    std::fprintf(stderr, "fillet_chamfer_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("fillet_chamfer_test: all checks PASSED\n");
  return 0;
}
