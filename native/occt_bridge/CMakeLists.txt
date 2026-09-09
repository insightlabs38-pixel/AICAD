# native/occt_bridge — AICAD-015 (OCCT discovery/probe) + AICAD-016 (C ABI
# boundary).
#
# AICAD-015 scope: locate a usable OCCT installation and build
# `occt_discovery_probe`, a small executable that proves OCCT headers
# resolve *and* OCCT libraries link and run correctly on this machine (not
# merely that `find_package` succeeded).
#
# AICAD-016 scope: build `aicad_occt_bridge`, the narrow C ABI library
# `crates/cad-occt-bridge` (AICAD-018) will link against, plus its two
# native tests (`abi_smoke_test`, `abi_c_linkage_test`).
#
# Kept as a standalone CMake project (rather than requiring the whole
# AICAD workspace to configure through it) so the probe/bridge can be
# built and iterated on independently of the Rust workspace, per
# docs/plan/22_REPOSITORY_WORK_PACKAGES.md's monorepo layout
# ("CMakeLists.txt # OCCT bridge only if needed").

cmake_minimum_required(VERSION 3.16)
# C is enabled alongside CXX because tests/abi_c_linkage_test.c (AICAD-016)
# needs a working C compiler/toolchain: a CXX-only `project()` does not
# enable C, and CMake then silently fails to produce an object for a .c
# source (found empirically — the link step ran with no object file for
# abi_c_linkage_test.c at all, failing with "undefined reference to
# `main`" instead of a clearer configure-time error).
project(aicad_occt_bridge CXX C)

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
# Machine-parseable line `crates/cad-occt-bridge/build.rs` (AICAD-018)
# greps for out of this configure step's stdout, so it can emit
# `cargo:rustc-link-search` for OCCT's actual library directory without
# reimplementing OCCT discovery in Rust. Keep this exact `KEY=VALUE`
# format (and keep it a `message(STATUS ...)`, not e.g. `message(NOTICE
# ...)`) if this line is ever touched — build.rs depends on it.
message(STATUS "AICAD_OCCT_LIBRARY_DIR=${OpenCASCADE_LIBRARY_DIR}")

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

# ---------------------------------------------------------------------------
# AICAD-016: the C ABI boundary library.
# ---------------------------------------------------------------------------

# Same module set as the probe (box construction + bounding-box query);
# widen only when a later Stage-1 task's bridge function needs a module
# this list does not already cover.
set(AICAD_OCCT_BRIDGE_LIBS
  TKernel
  TKMath
  TKBRep
  TKG3d
  TKGeomBase
  TKTopAlgo
  TKPrim
)

add_library(aicad_occt_bridge STATIC src/bridge.cpp)
target_include_directories(aicad_occt_bridge
  PUBLIC include
  PRIVATE ${OpenCASCADE_INCLUDE_DIR})
target_link_directories(aicad_occt_bridge PRIVATE ${OpenCASCADE_LIBRARY_DIR})
target_link_libraries(aicad_occt_bridge PUBLIC ${AICAD_OCCT_BRIDGE_LIBS})
# The library is public C++ (uses OCCT headers/exceptions internally) but
# its *published* symbols are all `extern "C"`, matching the ABI contract
# in include/aicad_occt_bridge.h; -fvisibility is intentionally left at
# CMake's default here since this is a STATIC library linked directly
# into a consuming crate, not a shared object with its own export surface.

enable_testing() # harmless if already called by a parent CMakeLists.txt
                 # (e.g. the repo root); required when this directory is
                 # configured standalone, per this file's own header
                 # comment about independent iteration.

add_executable(abi_smoke_test tests/abi_smoke_test.cpp)
target_link_libraries(abi_smoke_test PRIVATE aicad_occt_bridge)
add_test(NAME abi_smoke_test COMMAND abi_smoke_test)

add_executable(abi_c_linkage_test tests/abi_c_linkage_test.c)
target_link_libraries(abi_c_linkage_test PRIVATE aicad_occt_bridge)
add_test(NAME abi_c_linkage_test COMMAND abi_c_linkage_test)
