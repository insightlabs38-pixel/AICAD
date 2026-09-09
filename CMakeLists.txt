# AICAD root CMake entry point.
#
# The AICAD compiler/runtime is Rust (see the workspace `Cargo.toml`); this
# file exists only to build the native OCCT bridge
# (`native/occt_bridge`), per docs/plan/22_REPOSITORY_WORK_PACKAGES.md's
# monorepo layout ("CMakeLists.txt # OCCT bridge only if needed") and
# docs/plan/01_SYSTEM_ARCHITECTURE.md §4 ("a deliberately narrow C++
# bridge around OCCT"). Do not add non-OCCT-bridge native targets here
# without a corresponding work-package entry.

cmake_minimum_required(VERSION 3.16)
project(aicad_native)

# Must be called at the top level for `ctest` run from the root build
# directory to discover tests registered by add_subdirectory'd
# CMakeLists.txt files (CMake only writes a root CTestTestfile.cmake if
# enable_testing() was seen before/at the directory that adds them).
enable_testing()

add_subdirectory(native/occt_bridge)
