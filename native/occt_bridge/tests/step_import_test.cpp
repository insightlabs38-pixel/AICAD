// AICAD-035: STEP import (narrow, kernel-adapter-scoped -- see
// aicad_occt_bridge.h's own caveat on this NOT being the full public
// language-level import_step()).
//
// This test's own round-trip (export via this bridge, import via this
// same bridge) uses one OCCT installation on both sides, so it is
// deliberately NOT treated as independent verification of the exporter
// -- it proves the import/export pipeline itself is self-consistent for
// a shape this bridge already knows the exact analytic properties of.
// The genuinely independent (non-OCCT) verification path is the
// structural text scan in step_export_test.cpp plus the one-time
// steputils-based check documented in project/reports/AICAD-035.md.

#include "aicad_occt_bridge.h"

#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <string>

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

bool Close(double a, double b, double tol) { return std::fabs(a - b) < tol; }

}  // namespace

int main() {
  aicad_occt_context_t* ctx = nullptr;
  Check(aicad_occt_context_create(&ctx) == AICAD_OCCT_OK, "context_create succeeds");

  const std::string path = "/tmp/aicad_step_import_test_box.step";

  // --- round trip: export a box, import it back, compare analytic
  //     properties (volume/bounding box/topology counts) ---
  {
    const double dx = 3.0, dy = 4.0, dz = 5.0;
    aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_box(ctx, dx, dy, dz, &box);
    Check(aicad_occt_export_step(ctx, box, path.c_str()) == AICAD_OCCT_OK,
          "export_step succeeds for a box");

    aicad_shape_handle_t reimported = AICAD_NULL_SHAPE_HANDLE;
    Check(aicad_occt_import_step(ctx, path.c_str(), &reimported) == AICAD_OCCT_OK,
          "import_step succeeds for the just-exported file");

    int is_valid = 0;
    Check(aicad_occt_shape_is_valid(ctx, reimported, &is_valid) == AICAD_OCCT_OK && is_valid != 0,
          "the re-imported shape is a valid B-rep");

    double original_volume = 0.0, reimported_volume = 0.0;
    aicad_occt_shape_volume(ctx, box, &original_volume);
    aicad_occt_shape_volume(ctx, reimported, &reimported_volume);
    Check(Close(original_volume, dx * dy * dz, 1e-6), "the original box has the expected volume");
    Check(Close(reimported_volume, original_volume, 1e-6),
          "the re-imported shape's volume matches the original exactly (round trip)");

    double original_min[3], original_max[3];
    double reimported_min[3], reimported_max[3];
    aicad_occt_shape_bounding_box(ctx, box, original_min, original_max);
    aicad_occt_shape_bounding_box(ctx, reimported, reimported_min, reimported_max);
    bool bbox_matches = true;
    for (int i = 0; i < 3; ++i) {
      bbox_matches = bbox_matches && Close(original_min[i], reimported_min[i], 1e-6) &&
                     Close(original_max[i], reimported_max[i], 1e-6);
    }
    Check(bbox_matches, "the re-imported shape's bounding box matches the original exactly");

    size_t original_faces = 0, reimported_faces = 0;
    size_t original_edges = 0, reimported_edges = 0;
    size_t original_vertices = 0, reimported_vertices = 0;
    aicad_occt_shape_face_count(ctx, box, &original_faces);
    aicad_occt_shape_face_count(ctx, reimported, &reimported_faces);
    aicad_occt_shape_edge_count(ctx, box, &original_edges);
    aicad_occt_shape_edge_count(ctx, reimported, &reimported_edges);
    aicad_occt_shape_vertex_count(ctx, box, &original_vertices);
    aicad_occt_shape_vertex_count(ctx, reimported, &reimported_vertices);
    Check(original_faces == reimported_faces && original_faces == 6,
          "the re-imported shape has the same unique face count as the original (6)");
    Check(original_edges == reimported_edges && original_edges == 12,
          "the re-imported shape has the same unique edge count as the original (12)");
    Check(original_vertices == reimported_vertices && original_vertices == 8,
          "the re-imported shape has the same unique vertex count as the original (8)");
  }

  // --- adversarial: null/empty file_path, null out_handle rejected ---
  {
    aicad_shape_handle_t handle = AICAD_NULL_SHAPE_HANDLE;
    Check(aicad_occt_import_step(ctx, nullptr, &handle) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
          "import_step rejects a null file_path");
    Check(aicad_occt_import_step(ctx, "", &handle) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
          "import_step rejects an empty file_path");
    Check(aicad_occt_import_step(ctx, path.c_str(), nullptr) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
          "import_step rejects a null out_handle");
  }

  // --- adversarial: a nonexistent file fails cleanly, not with a crash ---
  {
    aicad_shape_handle_t handle = AICAD_NULL_SHAPE_HANDLE;
    Check(aicad_occt_import_step(ctx, "/nonexistent_directory_aicad/x.step", &handle) ==
              AICAD_OCCT_ERR_OPERATION_FAILED,
          "import_step fails cleanly (not a crash) for a nonexistent file");
  }

  // --- adversarial: a syntactically invalid (non-STEP) file fails cleanly ---
  {
    const std::string garbage_path = "/tmp/aicad_step_import_test_garbage.step";
    std::FILE* f = std::fopen(garbage_path.c_str(), "w");
    Check(f != nullptr, "garbage test file created");
    if (f != nullptr) {
      std::fputs("this is not a STEP file\n", f);
      std::fclose(f);
    }
    aicad_shape_handle_t handle = AICAD_NULL_SHAPE_HANDLE;
    Check(aicad_occt_import_step(ctx, garbage_path.c_str(), &handle) ==
              AICAD_OCCT_ERR_OPERATION_FAILED,
          "import_step fails cleanly (not a crash) for a non-STEP file");
    std::remove(garbage_path.c_str());
  }

  std::remove(path.c_str());

  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "context_destroy succeeds");

  if (g_failures > 0) {
    std::fprintf(stderr, "step_import_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("step_import_test: all checks PASSED\n");
  return 0;
}
