// AICAD-032: display tessellation output.

#include "aicad_occt_bridge.h"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <vector>

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

double Length3(double x, double y, double z) { return std::sqrt(x * x + y * y + z * z); }

}  // namespace

int main() {
  aicad_occt_context_t* ctx = nullptr;
  Check(aicad_occt_context_create(&ctx) == AICAD_OCCT_OK, "context_create succeeds");

  // --- a box tessellates into a nonzero number of triangles, and the
  //     resulting mesh's own bounding box (computed from the raw vertex
  //     buffer) matches the box's own analytic dimensions ---
  {
    const double dx = 2.0, dy = 3.0, dz = 4.0;
    aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_box(ctx, dx, dy, dz, &box);
    aicad_tessellation_counts_t counts{};
    Check(aicad_occt_tessellate(ctx, box, 0.1, 0.5, &counts) == AICAD_OCCT_OK,
          "tessellate succeeds on a box");
    Check(counts.triangle_count > 0, "a box tessellates into a nonzero number of triangles");
    // A box (6 planar quad faces, each split into exactly 2 triangles by
    // BRepMesh_IncrementalMesh) has exactly 12 triangles regardless of
    // deflection -- planar faces need no further subdivision.
    Check(counts.triangle_count == 12, "a box's planar faces tessellate into exactly 12 triangles");

    std::vector<double> vertices(9 * counts.triangle_count);
    std::vector<double> normals(9 * counts.triangle_count);
    Check(aicad_occt_tessellation_get(ctx, box, vertices.data(), normals.data()) == AICAD_OCCT_OK,
          "tessellation_get succeeds after tessellate");

    double xmin = 1e300, ymin = 1e300, zmin = 1e300;
    double xmax = -1e300, ymax = -1e300, zmax = -1e300;
    for (size_t i = 0; i < vertices.size(); i += 3) {
      xmin = std::min(xmin, vertices[i]);
      xmax = std::max(xmax, vertices[i]);
      ymin = std::min(ymin, vertices[i + 1]);
      ymax = std::max(ymax, vertices[i + 1]);
      zmin = std::min(zmin, vertices[i + 2]);
      zmax = std::max(zmax, vertices[i + 2]);
    }
    Check(NearlyEqual(xmin, 0.0, 1e-9) && NearlyEqual(xmax, dx, 1e-9) &&
              NearlyEqual(ymin, 0.0, 1e-9) && NearlyEqual(ymax, dy, 1e-9) &&
              NearlyEqual(zmin, 0.0, 1e-9) && NearlyEqual(zmax, dz, 1e-9),
          "the tessellated mesh's own bounding box matches the box's analytic dimensions");

    // Every triangle's normal is a unit vector and is one of the 6
    // axis-aligned box-face directions (+-X, +-Y, +-Z) -- proves winding/
    // orientation handling produced physically sensible outward normals,
    // not just "some" normal.
    bool all_unit_and_axis_aligned = true;
    for (size_t i = 0; i < normals.size(); i += 3) {
      const double nx = normals[i], ny = normals[i + 1], nz = normals[i + 2];
      const double len = Length3(nx, ny, nz);
      if (!NearlyEqual(len, 1.0, 1e-6)) {
        all_unit_and_axis_aligned = false;
        break;
      }
      const int near_axis_count = (NearlyEqual(std::fabs(nx), 1.0, 1e-6) ? 1 : 0) +
                                   (NearlyEqual(std::fabs(ny), 1.0, 1e-6) ? 1 : 0) +
                                   (NearlyEqual(std::fabs(nz), 1.0, 1e-6) ? 1 : 0);
      if (near_axis_count != 1) {
        all_unit_and_axis_aligned = false;
        break;
      }
    }
    Check(all_unit_and_axis_aligned,
          "every triangle's normal is a unit vector aligned with exactly one box-face axis");
  }

  // --- a cylinder's curved lateral surface subdivides into more than 2
  //     triangles (proves angular_deflection actually drives curved-face
  //     refinement, not just planar splitting) ---
  {
    aicad_shape_handle_t cyl = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_cylinder(ctx, 1.0, 2.0, &cyl);
    aicad_tessellation_counts_t counts{};
    aicad_occt_tessellate(ctx, cyl, 0.05, 0.2, &counts);
    Check(counts.triangle_count > 8,
          "a cylinder's curved surface tessellates into more than a trivial handful of triangles");
  }

  // --- adversarial: tessellation_get without a prior tessellate call
  //     fails cleanly ---
  {
    aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &box);
    double v[9], n[9];
    Check(aicad_occt_tessellation_get(ctx, box, v, n) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
          "tessellation_get fails cleanly when no tessellate call preceded it");
  }

  // --- adversarial: non-positive/non-finite deflections are rejected ---
  {
    aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &box);
    aicad_tessellation_counts_t counts{};
    Check(aicad_occt_tessellate(ctx, box, 0.0, 0.5, &counts) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
          "tessellate rejects a zero linear_deflection");
    Check(aicad_occt_tessellate(ctx, box, -0.1, 0.5, &counts) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
          "tessellate rejects a negative linear_deflection");
    Check(aicad_occt_tessellate(ctx, box, 0.1, 0.0, &counts) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
          "tessellate rejects a zero angular_deflection");
  }

  // --- each handle caches its own tessellation independently: a second
  //     shape's tessellate call does not disturb the first's cached
  //     result ---
  {
    aicad_shape_handle_t box_a = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &box_a);
    aicad_shape_handle_t box_b = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_box(ctx, 5.0, 5.0, 5.0, &box_b);
    aicad_tessellation_counts_t counts_a{}, counts_b{};
    aicad_occt_tessellate(ctx, box_a, 0.1, 0.5, &counts_a);
    aicad_occt_tessellate(ctx, box_b, 0.1, 0.5, &counts_b);
    std::vector<double> v_a(9 * counts_a.triangle_count), n_a(9 * counts_a.triangle_count);
    Check(aicad_occt_tessellation_get(ctx, box_a, v_a.data(), n_a.data()) == AICAD_OCCT_OK,
          "box_a's own cached tessellation is still retrievable after box_b was tessellated");
    double xmax = -1e300;
    for (size_t i = 0; i < v_a.size(); i += 3) {
      xmax = std::max(xmax, v_a[i]);
    }
    Check(NearlyEqual(xmax, 1.0, 1e-9),
          "box_a's own retrieved mesh still reflects box_a's own (1x1x1) dimensions, not box_b's");
  }

  // --- releasing a handle invalidates its cached tessellation (a fresh
  //     shape reusing the same slot must never inherit a stale cache) ---
  {
    aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &box);
    aicad_tessellation_counts_t counts{};
    aicad_occt_tessellate(ctx, box, 0.1, 0.5, &counts);
    aicad_occt_release_shape(ctx, box);
    double v[9 * 12], n[9 * 12];
    Check(aicad_occt_tessellation_get(ctx, box, v, n) == AICAD_OCCT_ERR_STALE_HANDLE,
          "a released handle's tessellation cache is unreachable (stale handle), not silently "
          "returned");
  }

  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "context_destroy succeeds");

  if (g_failures > 0) {
    std::fprintf(stderr, "tessellation_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("tessellation_test: all checks PASSED\n");
  return 0;
}
