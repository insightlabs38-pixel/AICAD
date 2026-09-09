// AICAD-031: normalized B-rep validation report.

#include "aicad_occt_bridge.h"

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

bool AllZero(const aicad_validation_report_t& r) {
  return r.invalid_vertex_count == 0 && r.invalid_edge_count == 0 && r.invalid_wire_count == 0 &&
         r.invalid_face_count == 0;
}

}  // namespace

int main() {
  aicad_occt_context_t* ctx = nullptr;
  Check(aicad_occt_context_create(&ctx) == AICAD_OCCT_OK, "context_create succeeds");

  // --- a valid box: overall valid, every per-kind count is zero ---
  {
    aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_box(ctx, 1.0, 2.0, 3.0, &box);
    aicad_validation_report_t report{};
    Check(aicad_occt_shape_validate(ctx, box, &report) == AICAD_OCCT_OK, "shape_validate succeeds on a box");
    Check(report.is_valid == 1, "a valid box's report says is_valid");
    Check(AllZero(report), "a valid box's report has all invalid-* counts at zero");
    int is_valid_bool = 0;
    aicad_occt_shape_is_valid(ctx, box, &is_valid_bool);
    Check(report.is_valid == is_valid_bool,
          "shape_validate's is_valid matches shape_is_valid's own bool for the same shape");
  }

  // --- a valid circular face ---
  {
    const double center[3] = {0, 0, 0};
    const double normal_z[3] = {0, 0, 1};
    aicad_shape_handle_t circle_wire = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_make_circle_wire(ctx, center, normal_z, 2.0, &circle_wire);
    aicad_shape_handle_t circle_face = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_make_face_from_wire(ctx, circle_wire, &circle_face);
    aicad_validation_report_t report{};
    aicad_occt_shape_validate(ctx, circle_face, &report);
    Check(report.is_valid == 1 && AllZero(report), "a valid circular face's report is fully clean");
  }

  // --- an invalid face bounded by an open (unclosed) wire: is_valid is
  //     false and the report attributes it specifically to the face, not
  //     the vertices/edges/wire that make it up (verified empirically,
  //     not assumed) ---
  {
    const double p0[3] = {0, 0, 0};
    const double p1[3] = {1, 0, 0};
    const double p2[3] = {1, 1, 0};
    const double p3[3] = {0, 1, 0};
    aicad_shape_handle_t e0, e1, e2;
    aicad_occt_make_line_edge(ctx, p0, p1, &e0);
    aicad_occt_make_line_edge(ctx, p1, p2, &e1);
    aicad_occt_make_line_edge(ctx, p2, p3, &e2);
    aicad_shape_handle_t open_edges[3] = {e0, e1, e2};
    aicad_shape_handle_t open_wire = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_make_wire_from_edges(ctx, open_edges, 3, &open_wire);
    aicad_shape_handle_t open_face = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_make_face_from_wire(ctx, open_wire, &open_face);

    aicad_validation_report_t report{};
    Check(aicad_occt_shape_validate(ctx, open_face, &report) == AICAD_OCCT_OK,
          "shape_validate succeeds (returns a report) even for an invalid shape -- validity is data, "
          "not a rejection");
    Check(report.is_valid == 0, "the open-wire face's report says NOT is_valid");
    Check(report.invalid_face_count == 1,
          "the open-wire face's report attributes exactly 1 invalid face");
    Check(report.invalid_vertex_count == 0 && report.invalid_edge_count == 0 &&
              report.invalid_wire_count == 0,
          "the open-wire face's report does not spuriously flag its vertices/edges/wire as invalid "
          "(empirically verified, not assumed)");
  }

  // --- adversarial: null out_report is rejected ---
  {
    aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &box);
    Check(aicad_occt_shape_validate(ctx, box, nullptr) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
          "shape_validate rejects a null out_report");
  }

  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "context_destroy succeeds");

  if (g_failures > 0) {
    std::fprintf(stderr, "validation_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("validation_test: all checks PASSED\n");
  return 0;
}
