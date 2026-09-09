// AICAD-015: OCCT discovery/probe.
//
// Confirms OCCT is discoverable and actually linkable/callable in this
// environment: exercises a real OCCT type (not just an #include) so the
// probe fails on a broken link, not only on missing headers, and
// cross-checks the header-reported version against the version CMake's
// find_package(OpenCASCADE) discovered. No AICAD kernel abstraction
// exists yet; that begins at AICAD-016/AICAD-017.

#include <Standard_Version.hxx>
#include <gp_Pnt.hxx>

#include <cstring>
#include <iostream>

int main() {
    gp_Pnt p(1.0, 2.0, 3.0);
    if (p.X() != 1.0 || p.Y() != 2.0 || p.Z() != 3.0) {
        std::cerr << "OCCT_PROBE_FAIL: gp_Pnt round-trip mismatch" << std::endl;
        return 1;
    }

#ifndef AICAD_OCCT_PROBE_CMAKE_VERSION
#error "AICAD_OCCT_PROBE_CMAKE_VERSION must be supplied by CMakeLists.txt"
#endif

    if (std::strcmp(OCC_VERSION_COMPLETE, AICAD_OCCT_PROBE_CMAKE_VERSION) != 0) {
        std::cerr << "OCCT_PROBE_FAIL: header version " << OCC_VERSION_COMPLETE
                   << " does not match CMake-discovered version "
                   << AICAD_OCCT_PROBE_CMAKE_VERSION << std::endl;
        return 1;
    }

    std::cout << "OCCT_PROBE_OK version=" << OCC_VERSION_COMPLETE << std::endl;
    return 0;
}
