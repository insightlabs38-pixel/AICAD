// AICAD-015: OCCT discovery/probe.
//
// Not part of the C ABI bridge (that begins at AICAD-016). This program
// exists only to give reproducible, checkable evidence that the OCCT
// version pinned in project/gates/STAGE1-A_KERNEL_BOUNDARY.md is
// discoverable via CMake's find_package(OpenCASCADE CONFIG) and that
// linking against it actually produces a dimensionally correct exact
// B-rep shape, not merely "a shape object" or "no crash" — see AGENTS.md
// "Geometry correctness standard."
//
// Exit code 0 = probe passed. Non-zero = probe failed; stderr has the
// reason. No OCCT/C++ exception is allowed to escape main().

#include <Standard_Failure.hxx>
#include <Standard_Version.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepGProp.hxx>
#include <GProp_GProps.hxx>
#include <TopoDS_Shape.hxx>
#include <TopAbs_ShapeEnum.hxx>

#include <cmath>
#include <cstdlib>
#include <exception>
#include <iostream>

namespace {

constexpr double kWidth = 10.0;
constexpr double kHeight = 20.0;
constexpr double kDepth = 30.0;
constexpr double kExpectedVolume = kWidth * kHeight * kDepth;
constexpr double kVolumeTolerance = 1e-6;

// Returns true and prints diagnostics on success; returns false on any
// detected geometric defect. Does not throw.
bool RunProbe() {
  std::cout << "AICAD OCCT probe: OCCT " << OCC_VERSION_COMPLETE << std::endl;

  BRepPrimAPI_MakeBox make_box(kWidth, kHeight, kDepth);
  make_box.Build();
  if (!make_box.IsDone()) {
    std::cerr << "AICAD OCCT probe FAILED: BRepPrimAPI_MakeBox did not complete"
              << std::endl;
    return false;
  }

  const TopoDS_Shape shape = make_box.Shape();
  if (shape.IsNull()) {
    std::cerr << "AICAD OCCT probe FAILED: probe box shape is null" << std::endl;
    return false;
  }
  if (shape.ShapeType() != TopAbs_SOLID) {
    std::cerr << "AICAD OCCT probe FAILED: probe box shape type is "
              << static_cast<int>(shape.ShapeType()) << ", expected TopAbs_SOLID ("
              << static_cast<int>(TopAbs_SOLID) << ")" << std::endl;
    return false;
  }

  // Dimensional check, not just "OCCT returned a shape": an exact-B-rep
  // box's volume must match width*height*depth within a tight numeric
  // tolerance.
  GProp_GProps props;
  BRepGProp::VolumeProperties(shape, props);
  const double volume = props.Mass();
  const double volume_error = std::fabs(volume - kExpectedVolume);
  if (volume_error > kVolumeTolerance) {
    std::cerr << "AICAD OCCT probe FAILED: box volume " << volume
              << " does not match expected " << kExpectedVolume
              << " (error " << volume_error << " > tolerance "
              << kVolumeTolerance << ")" << std::endl;
    return false;
  }

  std::cout << "AICAD OCCT probe OK: box volume " << volume
            << " matches expected " << kExpectedVolume << " within tolerance"
            << std::endl;
  return true;
}

}  // namespace

int main() {
  try {
    return RunProbe() ? EXIT_SUCCESS : EXIT_FAILURE;
  } catch (const Standard_Failure& e) {
    std::cerr << "AICAD OCCT probe FAILED: uncaught Standard_Failure: "
              << e.GetMessageString() << std::endl;
    return EXIT_FAILURE;
  } catch (const std::exception& e) {
    std::cerr << "AICAD OCCT probe FAILED: uncaught std::exception: "
              << e.what() << std::endl;
    return EXIT_FAILURE;
  } catch (...) {
    std::cerr << "AICAD OCCT probe FAILED: uncaught non-standard exception"
              << std::endl;
    return EXIT_FAILURE;
  }
}
