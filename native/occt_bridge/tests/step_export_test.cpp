// AICAD-033: STEP export.
//
// The structural checks here are deliberately implemented as plain
// substring/prefix scans over the exported file's own raw bytes -- NOT
// via any OCCT API -- so this test suite itself provides a genuinely
// OCCT-independent syntactic/topological-entity-count check, distinct
// from (and in addition to) the one-time deeper verification performed
// with the pure-Python `steputils` (ISO-10303-21) parser documented in
// project/reports/AICAD-033.md. See that report for exactly what each
// verification path proves and does not prove.

#include "aicad_occt_bridge.h"

#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <sstream>
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

std::string ReadFile(const std::string& path) {
  std::ifstream in(path, std::ios::binary);
  std::ostringstream buffer;
  buffer << in.rdbuf();
  return buffer.str();
}

// Counts non-overlapping occurrences of `needle` in `haystack` -- a
// plain string scan, not an ISO-10303-21 grammar parser, but sufficient
// to independently corroborate entity counts this bridge's own
// (OCCT-based) construction already establishes (e.g. a box's 6 unique
// faces, AICAD-028).
size_t CountOccurrences(const std::string& haystack, const std::string& needle) {
  size_t count = 0;
  size_t pos = 0;
  while ((pos = haystack.find(needle, pos)) != std::string::npos) {
    count += 1;
    pos += needle.size();
  }
  return count;
}

}  // namespace

int main() {
  aicad_occt_context_t* ctx = nullptr;
  Check(aicad_occt_context_create(&ctx) == AICAD_OCCT_OK, "context_create succeeds");

  const std::string box_path = "/tmp/aicad_step_export_test_box.step";

  // --- export a box and independently (no OCCT call) verify the file is
  //     structurally a valid ISO-10303-21 document whose entity counts
  //     match the box's own already-established topology (8 vertices, 12
  //     edges, 6 faces, 1 solid, 1 shell -- AICAD-027/028/029) ---
  {
    const double dx = 2.0, dy = 3.0, dz = 4.0;
    aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_box(ctx, dx, dy, dz, &box);
    Check(aicad_occt_export_step(ctx, box, box_path.c_str()) == AICAD_OCCT_OK,
          "export_step succeeds for a box");

    const std::string contents = ReadFile(box_path);
    Check(!contents.empty(), "the exported STEP file is non-empty");
    Check(contents.rfind("ISO-10303-21;", 0) == 0,
          "the exported file starts with the ISO-10303-21 magic header (independent text check)");
    Check(contents.find("HEADER;") != std::string::npos &&
              contents.find("ENDSEC;") != std::string::npos &&
              contents.find("DATA;") != std::string::npos &&
              contents.find("END-ISO-10303-21;") != std::string::npos,
          "the exported file has the required HEADER/DATA/ENDSEC/END-ISO-10303-21 structure");
    Check(contents.find("FILE_SCHEMA(") != std::string::npos,
          "the exported file declares a FILE_SCHEMA (AP identification)");

    Check(CountOccurrences(contents, "MANIFOLD_SOLID_BREP(") == 1,
          "the exported file has exactly 1 MANIFOLD_SOLID_BREP entity (independent text count)");
    Check(CountOccurrences(contents, "CLOSED_SHELL(") == 1,
          "the exported file has exactly 1 CLOSED_SHELL entity");
    Check(CountOccurrences(contents, "ADVANCED_FACE(") == 6,
          "the exported file has exactly 6 ADVANCED_FACE entities, matching the box's own 6 unique "
          "faces (AICAD-028)");
    Check(CountOccurrences(contents, "EDGE_CURVE(") == 12,
          "the exported file has exactly 12 EDGE_CURVE entities, matching the box's own 12 unique "
          "edges (AICAD-027)");
    Check(CountOccurrences(contents, "VERTEX_POINT(") == 8,
          "the exported file has exactly 8 VERTEX_POINT entities, matching the box's own 8 unique "
          "vertices (AICAD-029)");
    Check(CountOccurrences(contents, "PLANE(") == 6,
          "the exported file has exactly 6 PLANE surface entities (a box's faces are all planar)");
  }

  // --- export a curved shape (a cylinder) and confirm it uses a
  //     CYLINDRICAL_SURFACE entity, not just planes -- proves curved
  //     geometry is encoded distinctly, not flattened/approximated by
  //     export ---
  {
    const std::string cyl_path = "/tmp/aicad_step_export_test_cylinder.step";
    aicad_shape_handle_t cyl = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_cylinder(ctx, 1.0, 2.0, &cyl);
    Check(aicad_occt_export_step(ctx, cyl, cyl_path.c_str()) == AICAD_OCCT_OK,
          "export_step succeeds for a cylinder");
    const std::string contents = ReadFile(cyl_path);
    Check(contents.find("CYLINDRICAL_SURFACE(") != std::string::npos,
          "a cylinder's exported STEP file uses a CYLINDRICAL_SURFACE entity (curved geometry is not "
          "flattened to planes)");
    std::remove(cyl_path.c_str());
  }

  // --- adversarial: null/empty file_path is rejected ---
  {
    aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &box);
    Check(aicad_occt_export_step(ctx, box, nullptr) == AICAD_OCCT_ERR_INVALID_ARGUMENT,
          "export_step rejects a null file_path");
    Check(aicad_occt_export_step(ctx, box, "") == AICAD_OCCT_ERR_INVALID_ARGUMENT,
          "export_step rejects an empty file_path");
  }

  // --- adversarial: an unwritable path fails cleanly, not with a crash ---
  {
    aicad_shape_handle_t box = AICAD_NULL_SHAPE_HANDLE;
    aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &box);
    Check(aicad_occt_export_step(ctx, box, "/nonexistent_directory_aicad/x.step") ==
              AICAD_OCCT_ERR_OPERATION_FAILED,
          "export_step fails cleanly (not a crash) for an unwritable path");
  }

  std::remove(box_path.c_str());

  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "context_destroy succeeds");

  if (g_failures > 0) {
    std::fprintf(stderr, "step_export_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("step_export_test: all checks PASSED\n");
  return 0;
}
