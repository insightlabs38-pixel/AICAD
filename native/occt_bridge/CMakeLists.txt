# native/occt_bridge — AICAD-015: OCCT discovery/probe CMake target.
#
# Scope of this file (AICAD-015 only): locate a usable OCCT installation and
# build `occt_discovery_probe`, a small executable that proves OCCT headers
# resolve *and* OCCT libraries link and run correctly on this machine (not
# merely that `find_package` succeeded). It does not implement the C ABI
# boundary itself — that is AICAD-016, in `src/` (not yet populated).
#
# Kept as a standalone CMake project (rather than requiring the whole
# AICAD workspace to configure through it) so the probe/bridge can be
# built and iterated on independently of the Rust workspace, per
# docs/plan/22_REPOSITORY_WORK_PACKAGES.md's monorepo layout
# ("CMakeLists.txt # OCCT bridge only if needed").

cmake_minimum_required(VERSION 3.16)
project(aicad_occt_bridge CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE "RelWithDebInfo")
endif()

# OCCT's own OpenCASCADEConfigVersion.cmake uses CMake's "ExactVersion"
# compatibility strategy without accepting a two-component (MAJOR.MINOR)
# request, so `find_package(OpenCASCADE 7.6 ...)` is rejected even against
# a compatible 7.6.3 install. Find without a version constraint and check
# the discovered version explicitly below instead.
find_package(OpenCASCADE REQUIRED CONFIG)

if(NOT OpenCASCADE_FOUND)
  message(FATAL_ERROR
    "AICAD-015: OpenCASCADE (OCCT) was not found via find_package(OpenCASCADE). "
    "Install OCCT development packages (headers + CMake config) and/or set "
    "OpenCASCADE_DIR to the directory containing OpenCASCADEConfig.cmake.")
endif()

if(OpenCASCADE_MAJOR_VERSION LESS 7)
  message(FATAL_ERROR
    "AICAD-015: found OpenCASCADE ${OpenCASCADE_MAJOR_VERSION}.${OpenCASCADE_MINOR_VERSION}.${OpenCASCADE_MAINTENANCE_VERSION}, "
    "but AICAD's Stage-1 kernel bridge targets OCCT 7.x. Set OpenCASCADE_DIR "
    "to a 7.x install.")
endif()

message(STATUS "AICAD occt_bridge: OpenCASCADE ${OpenCASCADE_MAJOR_VERSION}.${OpenCASCADE_MINOR_VERSION}.${OpenCASCADE_MAINTENANCE_VERSION} found at ${OpenCASCADE_INSTALL_PREFIX}")

# Only the modules the discovery probe actually exercises (FoundationClasses
# for Standard_Version.hxx/version macros; ModelingData + ModelingAlgorithms
# for the box-construction + validity-check sanity operation). AICAD-016
# will widen this list as the real bridge grows; do not link modules this
# probe does not use.
set(AICAD_OCCT_PROBE_LIBS
  TKernel
  TKMath
  TKBRep
  TKG3d
  TKGeomBase
  TKTopAlgo
  TKPrim
)

add_executable(occt_discovery_probe probe/occt_discovery_probe.cpp)

target_include_directories(occt_discovery_probe PRIVATE ${OpenCASCADE_INCLUDE_DIR})
target_link_directories(occt_discovery_probe PRIVATE ${OpenCASCADE_LIBRARY_DIR})
target_link_libraries(occt_discovery_probe PRIVATE ${AICAD_OCCT_PROBE_LIBS})

# OCCT's own installed CXX flags additionally pin -std=c++0x on some
# distributions; do not import them wholesale (they include distro
# hardening flags such as -ffile-prefix-map tied to OCCT's own build path,
# which are meaningless for this target). Only the include/library/link
# wiring above is reused.
