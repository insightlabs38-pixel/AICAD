// Stage-1 OCCT discovery/probe (AICAD-015).
//
// This program is not part of the AICAD kernel bridge (AICAD-016 onward
// owns that). It exists only to prove, mechanically, that the discovered
// OCCT installation can build a real B-rep solid, validate its topology,
// and reproduce correct analytic properties from it -- per AGENTS.md's
// evidence rule, "the package/headers were found" is not accepted as
// sufficient evidence that OCCT is actually usable on this toolchain.

#include <Standard_Version.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <GProp_GProps.hxx>
#include <TopoDS_Shape.hxx>

#include <cmath>
#include <cstdio>

namespace {

bool NearlyEqual(double a, double b, double tol) {
  return std::fabs(a - b) <= tol;
}

}  // namespace

int main() {
  std::printf("occt_probe: OCCT version %s\n", OCC_VERSION_COMPLETE);

  const double dx = 1.0;
  const double dy = 2.0;
  const double dz = 3.0;

  BRepPrimAPI_MakeBox make_box(dx, dy, dz);
  make_box.Build();
  if (!make_box.IsDone()) {
    std::fprintf(stderr, "occt_probe: FAIL - BRepPrimAPI_MakeBox did not complete\n");
    return 1;
  }

  const TopoDS_Shape box = make_box.Shape();

  BRepCheck_Analyzer analyzer(box);
  if (!analyzer.IsValid()) {
    std::fprintf(stderr,
                 "occt_probe: FAIL - BRepCheck_Analyzer rejected the constructed box as invalid\n");
    return 1;
  }

  GProp_GProps volume_props;
  BRepGProp::VolumeProperties(box, volume_props);
  const double volume = volume_props.Mass();
  const double expected_volume = dx * dy * dz;
  const double volume_tol = 1e-9;
  if (!NearlyEqual(volume, expected_volume, volume_tol)) {
    std::fprintf(stderr,
                 "occt_probe: FAIL - box volume %.12f does not match expected %.12f (tol %.3g)\n",
                 volume, expected_volume, volume_tol);
    return 1;
  }

  GProp_GProps surface_props;
  BRepGProp::SurfaceProperties(box, surface_props);
  const double area = surface_props.Mass();
  const double expected_area = 2.0 * (dx * dy + dy * dz + dz * dx);
  const double area_tol = 1e-9;
  if (!NearlyEqual(area, expected_area, area_tol)) {
    std::fprintf(stderr,
                 "occt_probe: FAIL - box surface area %.12f does not match expected %.12f (tol %.3g)\n",
                 area, expected_area, area_tol);
    return 1;
  }

  std::printf("occt_probe: PASS - valid B-rep box, volume=%.6f area=%.6f\n", volume, area);
  return 0;
}
