cmake_minimum_required(VERSION 3.16)
project(occt_bridge LANGUAGES CXX)

# Stage 1: builds (a) an OCCT discovery/probe target (AICAD-015) that
# proves OCCT can be reliably discovered and linked against on the host
# toolchain, failing loudly and specifically (missing package vs. missing
# module vs. build failure) rather than silently; and (b) the
# aicad_occt_bridge C ABI library (AICAD-016), the only AICAD-facing
# artifact this project produces -- crates/cad-occt-bridge's build.rs
# (AICAD-018) configures, builds, and installs this project to obtain it.
#
# README.md and docs/plan/01_SYSTEM_ARCHITECTURE.md §4 describe the
# eventual bridge operation surface (create_box, boolean_union, ...); the
# implemented operation set intentionally stays narrower than that list
# per RFC-0002 §3's capability-driven minimal-surface rule.

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

# TKOffset provides BRepOffsetAPI_MakePipe/ThruSections (AICAD-025: sweep
# and loft) and also BRepOffsetAPI_MakeOffsetShape/MakeThickSolid
# (AICAD-028: shell/offset -- same OCCT module, no new library needed).
# Linked only into aicad_occt_bridge, not into OCCT_PROBE_LIBS
# (occt_probe's discovery check stays scoped to the Batch-1A/1B module
# set it originally verified).
set(OCCT_SWEEP_LOFT_LIBS
  TKOffset
)

foreach(_lib IN LISTS OCCT_SWEEP_LOFT_LIBS)
  if(NOT _lib IN_LIST OpenCASCADE_LIBRARIES)
    message(FATAL_ERROR
      "OCCT discovery: required module library '${_lib}' (for sweep/loft, "
      "AICAD-025) is not among OpenCASCADE_LIBRARIES reported by the "
      "discovered installation (${OpenCASCADE_LIBRARIES}).")
  endif()
endforeach()

# TKBO provides BRepAlgoAPI_Fuse/Cut/Common (AICAD-026: boolean
# union/cut/intersect).
set(OCCT_BOOLEAN_LIBS
  TKBO
)

foreach(_lib IN LISTS OCCT_BOOLEAN_LIBS)
  if(NOT _lib IN_LIST OpenCASCADE_LIBRARIES)
    message(FATAL_ERROR
      "OCCT discovery: required module library '${_lib}' (for boolean "
      "union/cut/intersect, AICAD-026) is not among OpenCASCADE_LIBRARIES "
      "reported by the discovered installation (${OpenCASCADE_LIBRARIES}).")
  endif()
endforeach()

# TKFillet provides BRepFilletAPI_MakeFillet/MakeChamfer (AICAD-027:
# fillet and chamfer).
set(OCCT_FILLET_LIBS
  TKFillet
)

foreach(_lib IN LISTS OCCT_FILLET_LIBS)
  if(NOT _lib IN_LIST OpenCASCADE_LIBRARIES)
    message(FATAL_ERROR
      "OCCT discovery: required module library '${_lib}' (for fillet/"
      "chamfer, AICAD-027) is not among OpenCASCADE_LIBRARIES reported by "
      "the discovered installation (${OpenCASCADE_LIBRARIES}).")
  endif()
endforeach()

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

# --- AICAD-016: C ABI boundary library ---
#
# aicad_occt_bridge is the shared library crates/cad-occt-bridge (AICAD-018)
# links against. include/aicad_occt_bridge.h is its entire public surface;
# src/aicad_occt_bridge.cpp is the only translation unit permitted to
# include OCCT headers or reference OCCT types.
find_package(Threads REQUIRED)

add_library(aicad_occt_bridge SHARED src/aicad_occt_bridge.cpp)
target_include_directories(aicad_occt_bridge PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_include_directories(aicad_occt_bridge SYSTEM PRIVATE ${OpenCASCADE_INCLUDE_DIR})
target_link_directories(aicad_occt_bridge PRIVATE ${OpenCASCADE_LIBRARY_DIR})
target_link_libraries(aicad_occt_bridge PRIVATE ${OCCT_PROBE_LIBS} ${OCCT_SWEEP_LOFT_LIBS} ${OCCT_BOOLEAN_LIBS} ${OCCT_FILLET_LIBS} Threads::Threads)

add_executable(abi_boundary_test tests/abi_boundary_test.cpp)
target_include_directories(abi_boundary_test PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_link_libraries(abi_boundary_test PRIVATE aicad_occt_bridge Threads::Threads)

# --- AICAD-019: kernel context lifecycle / shape-handle table stress tests ---
add_executable(lifecycle_test tests/lifecycle_test.cpp)
target_include_directories(lifecycle_test PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_link_libraries(lifecycle_test PRIVATE aicad_occt_bridge Threads::Threads)

# --- Batch 1B: constructive geometry (AICAD-020..024) ---
add_executable(box_cylinder_test tests/box_cylinder_test.cpp)
target_include_directories(box_cylinder_test PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_link_libraries(box_cylinder_test PRIVATE aicad_occt_bridge Threads::Threads)

add_executable(transform_test tests/transform_test.cpp)
target_include_directories(transform_test PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_link_libraries(transform_test PRIVATE aicad_occt_bridge Threads::Threads)

add_executable(curve_edge_wire_test tests/curve_edge_wire_test.cpp)
target_include_directories(curve_edge_wire_test PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_link_libraries(curve_edge_wire_test PRIVATE aicad_occt_bridge Threads::Threads)

add_executable(face_test tests/face_test.cpp)
target_include_directories(face_test PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_link_libraries(face_test PRIVATE aicad_occt_bridge Threads::Threads)

add_executable(extrude_revolve_test tests/extrude_revolve_test.cpp)
target_include_directories(extrude_revolve_test PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_link_libraries(extrude_revolve_test PRIVATE aicad_occt_bridge Threads::Threads)

# --- Batch 1C: hard geometry operations (AICAD-025..028) ---
add_executable(sweep_loft_test tests/sweep_loft_test.cpp)
target_include_directories(sweep_loft_test PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_link_libraries(sweep_loft_test PRIVATE aicad_occt_bridge Threads::Threads)

add_executable(boolean_test tests/boolean_test.cpp)
target_include_directories(boolean_test PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_link_libraries(boolean_test PRIVATE aicad_occt_bridge Threads::Threads)

add_executable(fillet_chamfer_test tests/fillet_chamfer_test.cpp)
target_include_directories(fillet_chamfer_test PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_link_libraries(fillet_chamfer_test PRIVATE aicad_occt_bridge Threads::Threads)

add_executable(shell_offset_test tests/shell_offset_test.cpp)
target_include_directories(shell_offset_test PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_link_libraries(shell_offset_test PRIVATE aicad_occt_bridge Threads::Threads)

# --- Batch 1D: inspection/validation/interchange (AICAD-029..033) ---
add_executable(topology_test tests/topology_test.cpp)
target_include_directories(topology_test PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_link_libraries(topology_test PRIVATE aicad_occt_bridge Threads::Threads)

enable_testing()
add_test(NAME occt_probe COMMAND occt_probe)
set_tests_properties(occt_probe PROPERTIES TIMEOUT 60)
add_test(NAME abi_boundary_test COMMAND abi_boundary_test)
set_tests_properties(abi_boundary_test PROPERTIES TIMEOUT 60)
add_test(NAME lifecycle_test COMMAND lifecycle_test)
set_tests_properties(lifecycle_test PROPERTIES TIMEOUT 120)
add_test(NAME box_cylinder_test COMMAND box_cylinder_test)
set_tests_properties(box_cylinder_test PROPERTIES TIMEOUT 60)
add_test(NAME transform_test COMMAND transform_test)
set_tests_properties(transform_test PROPERTIES TIMEOUT 60)
add_test(NAME curve_edge_wire_test COMMAND curve_edge_wire_test)
set_tests_properties(curve_edge_wire_test PROPERTIES TIMEOUT 60)
add_test(NAME face_test COMMAND face_test)
set_tests_properties(face_test PROPERTIES TIMEOUT 60)
add_test(NAME extrude_revolve_test COMMAND extrude_revolve_test)
set_tests_properties(extrude_revolve_test PROPERTIES TIMEOUT 60)
add_test(NAME sweep_loft_test COMMAND sweep_loft_test)
set_tests_properties(sweep_loft_test PROPERTIES TIMEOUT 60)
add_test(NAME boolean_test COMMAND boolean_test)
set_tests_properties(boolean_test PROPERTIES TIMEOUT 60)
add_test(NAME fillet_chamfer_test COMMAND fillet_chamfer_test)
set_tests_properties(fillet_chamfer_test PROPERTIES TIMEOUT 60)
add_test(NAME shell_offset_test COMMAND shell_offset_test)
set_tests_properties(shell_offset_test PROPERTIES TIMEOUT 60)
add_test(NAME topology_test COMMAND topology_test)
set_tests_properties(topology_test PROPERTIES TIMEOUT 60)

# --- AICAD-018: install rules ---
#
# crates/cad-occt-bridge's build.rs installs this project into a
# Cargo-managed OUT_DIR (via -DCMAKE_INSTALL_PREFIX=<out_dir>) rather than
# reaching into this build tree's raw layout, which is not guaranteed
# stable across CMake generators/versions. Only the library and its one
# public header are installed -- occt_probe and abi_boundary_test are
# dev-only executables, not part of the installable artifact.
include(GNUInstallDirs)
install(TARGETS aicad_occt_bridge
  LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
  ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
  RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
install(FILES include/aicad_occt_bridge.h DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
