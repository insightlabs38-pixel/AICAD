cmake_minimum_required(VERSION 3.16)
project(aicad_occt_bridge LANGUAGES CXX)

# Stage 1, AICAD-015: OCCT discovery/probe (occt_probe target). Proves
# OCCT can be located and linked against reproducibly, on the exact
# toolchain/OCCT version recorded in
# project/gates/STAGE1-A_KERNEL_BOUNDARY.md, before any bridge code is
# written against it.
#
# Stage 1, AICAD-016: the aicad_occt_bridge C ABI library
# (include/aicad/occt_bridge.h + src/occt_bridge.cpp) and its
# abi_smoke_test, exercised through that header only — never through
# OCCT directly — the same way crates/cad-occt-bridge (AICAD-018) will.

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

# The C ABI bridge library. Only its own toolkit list, kept separate
# from occt_probe's per AICAD-015's report; grows as AICAD-016+ adds
# operations.
set(AICAD_OCCT_BRIDGE_TOOLKITS
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

add_library(aicad_occt_bridge STATIC src/occt_bridge.cpp)
target_include_directories(aicad_occt_bridge
  PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include
  PRIVATE ${OpenCASCADE_INCLUDE_DIR})
target_link_libraries(aicad_occt_bridge PRIVATE ${AICAD_OCCT_BRIDGE_TOOLKITS})

add_executable(abi_smoke_test tests/abi_smoke_test.cpp)
target_link_libraries(abi_smoke_test PRIVATE aicad_occt_bridge)

enable_testing()
add_test(NAME occt_probe COMMAND occt_probe)
add_test(NAME abi_smoke_test COMMAND abi_smoke_test)
