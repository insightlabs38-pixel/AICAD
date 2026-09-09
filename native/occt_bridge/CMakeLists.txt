cmake_minimum_required(VERSION 3.16)
project(aicad_occt_bridge LANGUAGES CXX)

# Stage 1, AICAD-015: OCCT discovery/probe only. This CMakeLists does not
# yet build the C ABI bridge itself (that starts at AICAD-016) — its job
# is to prove that OCCT can be located and linked against reproducibly,
# on the exact toolchain/OCCT version recorded in
# project/gates/STAGE1-A_KERNEL_BOUNDARY.md, before any bridge code is
# written against it.

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

find_package(OpenCASCADE CONFIG REQUIRED)

message(STATUS
  "AICAD: found OpenCASCADE ${OpenCASCADE_MAJOR_VERSION}.${OpenCASCADE_MINOR_VERSION}.${OpenCASCADE_MAINTENANCE_VERSION} "
  "(install prefix: ${OpenCASCADE_INSTALL_PREFIX}, include dir: ${OpenCASCADE_INCLUDE_DIR})")

# Narrow set of toolkits needed for the probe's box-construction smoke
# check only. The real bridge (AICAD-016+) will declare its own toolkit
# list as its operation set grows — this list is deliberately not shared
# with it.
set(AICAD_OCCT_PROBE_TOOLKITS
  TKernel
  TKMath
  TKG2d
  TKG3d
  TKGeomBase
  TKBRep
  TKGeomAlgo
  TKTopAlgo
  TKPrim
)

add_executable(occt_probe probe/occt_probe.cpp)
target_include_directories(occt_probe PRIVATE ${OpenCASCADE_INCLUDE_DIR})
target_link_libraries(occt_probe PRIVATE ${AICAD_OCCT_PROBE_TOOLKITS})

enable_testing()
add_test(NAME occt_probe COMMAND occt_probe)
