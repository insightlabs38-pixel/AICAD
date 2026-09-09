cmake_minimum_required(VERSION 3.16)
project(occt_bridge LANGUAGES CXX)

# Stage 1 (AICAD-015): OCCT discovery/probe target only.
#
# This CMakeLists.txt does not yet build the C ABI bridge (AICAD-016) or
# any AICAD-facing library -- it exists to prove that OCCT can be reliably
# discovered and linked against on the host toolchain, and to fail loudly
# and specifically (missing package vs. missing module vs. build failure)
# rather than silently, before any later Stage-1 task depends on it.
#
# README.md and docs/plan/01_SYSTEM_ARCHITECTURE.md §4 describe the
# eventual bridge operation surface (create_box, boolean_union, ...); this
# target intentionally stays narrower than that until AICAD-016.

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
  set(CMAKE_BUILD_TYPE RelWithDebInfo CACHE STRING "Build type" FORCE)
endif()

find_package(OpenCASCADE REQUIRED CONFIG)

if(NOT OpenCASCADE_FOUND)
  message(FATAL_ERROR
    "OCCT discovery failed: the 'OpenCASCADE' CMake package was not found. "
    "Install OCCT development packages (e.g. libocct-foundation-dev, "
    "libocct-modeling-data-dev, libocct-modeling-algorithms-dev on "
    "Debian/Ubuntu) or set OpenCASCADE_DIR to the directory containing "
    "OpenCASCADEConfig.cmake.")
endif()

message(STATUS "OCCT discovery: found OpenCASCADE ${OpenCASCADE_MAJOR_VERSION}.${OpenCASCADE_MINOR_VERSION}.${OpenCASCADE_MAINTENANCE_VERSION}")
message(STATUS "OCCT discovery: include dir = ${OpenCASCADE_INCLUDE_DIR}")
message(STATUS "OCCT discovery: library dir = ${OpenCASCADE_LIBRARY_DIR}")
message(STATUS "OCCT discovery: available modules = ${OpenCASCADE_MODULES}")

# Minimal module set needed to construct a primitive solid, validate its
# topology, and compute analytic volume/area properties -- enough to prove
# FoundationClasses + ModelingData + ModelingAlgorithms are all usable,
# without depending on Visualization/ApplicationFramework/DataExchange
# (those are exercised by later Stage-1 tasks, not this discovery probe).
set(OCCT_PROBE_LIBS
  TKernel
  TKMath
  TKG3d
  TKGeomBase
  TKBRep
  TKGeomAlgo
  TKTopAlgo
  TKPrim
)

foreach(_lib IN LISTS OCCT_PROBE_LIBS)
  if(NOT _lib IN_LIST OpenCASCADE_LIBRARIES)
    message(FATAL_ERROR
      "OCCT discovery: required module library '${_lib}' is not among "
      "OpenCASCADE_LIBRARIES reported by the discovered installation "
      "(${OpenCASCADE_LIBRARIES}). The installed OCCT build is missing a "
      "module this probe (and later Stage-1 work) requires.")
  endif()
endforeach()

add_executable(occt_probe probe/occt_probe.cpp)
target_include_directories(occt_probe SYSTEM PRIVATE ${OpenCASCADE_INCLUDE_DIR})
target_link_directories(occt_probe PRIVATE ${OpenCASCADE_LIBRARY_DIR})
target_link_libraries(occt_probe PRIVATE ${OCCT_PROBE_LIBS})

enable_testing()
add_test(NAME occt_probe COMMAND occt_probe)
set_tests_properties(occt_probe PROPERTIES TIMEOUT 60)
