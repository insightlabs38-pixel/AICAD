// AICAD-015: OCCT discovery/probe.
//
// This program is deliberately *not* part of the C ABI boundary (that is
// AICAD-016). Its only job is to give the build reproducible, checkable
// evidence that:
//
//   1. OCCT headers found by CMake actually compile;
//   2. OCCT libraries found by CMake actually link;
//   3. a real OCCT geometric construction (not just header inclusion)
//      executes and produces a valid result on this machine;
//   4. any OCCT-side failure surfaces as a clean, caught report rather
//      than an uncaught C++ exception or a crash.
//
// It prints machine-parseable `KEY=VALUE` lines (OCCT version fields) for
// project/reports/AICAD-015.md to capture verbatim, and exits 0 only if
// every check below passed.

#include <cmath>
#include <exception>
#include <iostream>

#include <Standard_Version.hxx>
#include <Standard_Failure.hxx>

#include <gp_Pnt.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <TopoDS_Shape.hxx>
#include <TopoDS_Solid.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <Bnd_Box.hxx>
#include <BRepBndLib.hxx>

namespace {

// A degenerate box (zero-width) that a correct OCCT build must accept as
// geometrically valid input (a genuinely zero-volume box is not itself
// under test here — only that OCCT's own box-construction + validity
// checker agree) is deliberately *not* used for this probe: this probe
// only needs one clearly well-formed sanity case. Degenerate/adversarial
// cases belong to the geometry-operation tasks themselves (AICAD-020+),
// not to kernel discovery.
bool run_sanity_construction(double &out_volume_bbox_diagonal) {
    // 10mm x 20mm x 30mm box at the origin. Units here are bare OCCT
    // doubles (millimeters, OCCT's own convention) — this probe has no
    // opinion on AICAD's typed-units model (cad-units/RFC-0004); that
    // mapping is owned by cad-occt-bridge (AICAD-018), not this file.
    BRepPrimAPI_MakeBox box_builder(gp_Pnt(0.0, 0.0, 0.0), 10.0, 20.0, 30.0);
    // BRepBuilderAPI_MakeShape::Build() (and thus the `myDone` flag) only
    // runs lazily inside Shape() — IsDone() is meaningless before Shape()
    // has been called at least once. Call Shape() first, then check.
    TopoDS_Shape shape = box_builder.Shape();
    if (!box_builder.IsDone()) {
        std::cerr << "FAIL: BRepPrimAPI_MakeBox did not complete\n";
        return false;
    }
    if (shape.IsNull()) {
        std::cerr << "FAIL: constructed box shape is null\n";
        return false;
    }

    BRepCheck_Analyzer analyzer(shape);
    if (!analyzer.IsValid()) {
        std::cerr << "FAIL: BRepCheck_Analyzer rejected the constructed box\n";
        return false;
    }

    Bnd_Box bounds;
    BRepBndLib::Add(shape, bounds);
    if (bounds.IsVoid()) {
        std::cerr << "FAIL: bounding box of a valid solid was void\n";
        return false;
    }

    double xmin, ymin, zmin, xmax, ymax, zmax;
    bounds.Get(xmin, ymin, zmin, xmax, ymax, zmax);
    const double dx = xmax - xmin;
    const double dy = ymax - ymin;
    const double dz = zmax - zmin;
    out_volume_bbox_diagonal = std::sqrt(dx * dx + dy * dy + dz * dz);

    // Expected diagonal of a 10x20x30 box is sqrt(100+400+900) = sqrt(1400).
    const double expected = std::sqrt(1400.0);
    const double diff = out_volume_bbox_diagonal - expected;
    const double abs_diff = diff < 0.0 ? -diff : diff;
    if (abs_diff > 1e-6) {
        std::cerr << "FAIL: bounding-box diagonal " << out_volume_bbox_diagonal
                   << " does not match expected " << expected << "\n";
        return false;
    }

    return true;
}

} // namespace

int main() {
    std::cout << "OCC_VERSION_MAJOR=" << OCC_VERSION_MAJOR << "\n";
    std::cout << "OCC_VERSION_MINOR=" << OCC_VERSION_MINOR << "\n";
    std::cout << "OCC_VERSION_MAINTENANCE=" << OCC_VERSION_MAINTENANCE << "\n";
    std::cout << "OCC_VERSION_COMPLETE=" << OCC_VERSION_COMPLETE << "\n";

    double bbox_diagonal = 0.0;
    bool ok = false;
    try {
        ok = run_sanity_construction(bbox_diagonal);
    } catch (const Standard_Failure &occt_exception) {
        std::cerr << "FAIL: uncaught Standard_Failure escaped OCCT call: "
                   << occt_exception.GetMessageString() << "\n";
        ok = false;
    } catch (const std::exception &generic_exception) {
        std::cerr << "FAIL: uncaught std::exception escaped OCCT call: "
                   << generic_exception.what() << "\n";
        ok = false;
    }

    if (!ok) {
        std::cout << "PROBE_RESULT=FAIL\n";
        return 1;
    }

    std::cout << "PROBE_BBOX_DIAGONAL=" << bbox_diagonal << "\n";
    std::cout << "PROBE_RESULT=PASS\n";
    return 0;
}
