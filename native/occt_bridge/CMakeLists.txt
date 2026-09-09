# native/occt_bridge — OCCT discovery/probe (AICAD-015).
#
# Stage 1, Batch 1A. This CMake project only proves OCCT is reliably
# discoverable and linkable in this environment; it does not implement
# the C ABI boundary yet (that is AICAD-016). Keep it a standalone CMake
# project (not glued into Cargo) until AICAD-016 defines how the Rust
# build actually drives the native build.
cmake_minimum_required(VERSION 3.16)
project(occt_bridge_probe LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# AICAD kernel policy #6: the adapter must fail closed on a broken/absent
# kernel rather than silently degrading, so OCCT discovery is REQUIRED.
find_package(OpenCASCADE REQUIRED)

if(NOT OpenCASCADE_FOUND)
  message(FATAL_ERROR "OpenCASCADE was not discovered by find_package(OpenCASCADE)")
endif()

message(STATUS "AICAD-015 OCCT probe: OpenCASCADE ${OpenCASCADE_MAJOR_VERSION}.${OpenCASCADE_MINOR_VERSION}.${OpenCASCADE_MAINTENANCE_VERSION}")
message(STATUS "AICAD-015 OCCT probe: include dir ${OpenCASCADE_INCLUDE_DIR}")
message(STATUS "AICAD-015 OCCT probe: library dir ${OpenCASCADE_LIBRARY_DIR}")
message(STATUS "AICAD-015 OCCT probe: modules ${OpenCASCADE_MODULES}")

add_executable(occt_discovery_probe probe/occt_probe.cpp)
target_include_directories(occt_discovery_probe PRIVATE ${OpenCASCADE_INCLUDE_DIR})
target_link_libraries(occt_discovery_probe PRIVATE TKernel TKMath)

# Cross-check that the version CMake's find_package() discovered matches
# the version the compiled headers/libraries actually report at runtime,
# rather than trusting either source alone.
target_compile_definitions(occt_discovery_probe PRIVATE
  AICAD_OCCT_PROBE_CMAKE_VERSION="${OpenCASCADE_MAJOR_VERSION}.${OpenCASCADE_MINOR_VERSION}.${OpenCASCADE_MAINTENANCE_VERSION}"
)

enable_testing()
add_test(NAME occt_discovery_probe COMMAND occt_discovery_probe)
set_tests_properties(occt_discovery_probe PROPERTIES
  PASS_REGULAR_EXPRESSION "OCCT_PROBE_OK version=[0-9]+\\.[0-9]+\\.[0-9]+"
)

# AICAD-016: the C ABI boundary itself. A static library so the Rust
# side (crates/cad-occt-bridge, AICAD-018) can link it directly with no
# separately-installed shared object to manage. Public surface is
# include/aicad_occt_bridge.h only -- no OCCT header is installed or
# re-exported alongside it.
add_library(aicad_occt_bridge STATIC src/aicad_occt_bridge.cpp)
target_include_directories(aicad_occt_bridge
  PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include
  PRIVATE ${OpenCASCADE_INCLUDE_DIR}
)
target_link_libraries(aicad_occt_bridge PUBLIC
  TKBRep TKPrim TKGeomAlgo TKTopAlgo TKG3d TKG2d TKGeomBase TKMath TKernel
)

add_executable(bridge_abi_tests tests/bridge_abi_tests.cpp)
target_link_libraries(bridge_abi_tests PRIVATE aicad_occt_bridge)

add_test(NAME bridge_abi_tests COMMAND bridge_abi_tests)
set_tests_properties(bridge_abi_tests PROPERTIES
  PASS_REGULAR_EXPRESSION "AICAD_BRIDGE_ABI_TESTS_OK"
)
