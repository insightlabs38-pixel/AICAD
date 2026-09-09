//! AICAD-018 build script: configures and builds the `aicad_occt_bridge`
//! native static library (`native/occt_bridge`, AICAD-015/AICAD-016) via
//! CMake, then links it (and the OCCT module libraries it depends on)
//! into this crate.
//!
//! Deliberately reuses `native/occt_bridge/CMakeLists.txt`'s own OCCT
//! discovery (`find_package(OpenCASCADE CONFIG)`) rather than
//! reimplementing path-searching heuristics here — CMake's discovery is
//! already written, already tested (`project/reports/AICAD-015.md`), and
//! is the one place that logic should live. This script only needs the
//! resulting OCCT library directory, which it reads back from the
//! configure step's own stdout (see the `AICAD_OCCT_LIBRARY_DIR=`
//! marker line in `native/occt_bridge/CMakeLists.txt`) rather than
//! guessing it independently.

use std::env;
use std::path::PathBuf;
use std::process::Command;

/// The OCCT module libraries `aicad_occt_bridge` links against. Kept in
/// sync **by hand** with `native/occt_bridge/CMakeLists.txt`'s
/// `AICAD_OCCT_BRIDGE_LIBS` list — update both together whenever a later
/// Stage-1 task's bridge function needs a module neither list currently
/// covers. Duplicated rather than shared because build.rs is Rust and
/// CMakeLists.txt is CMake; the *directory* those libraries live in is
/// not duplicated (see `AICAD_OCCT_LIBRARY_DIR` above).
const OCCT_BRIDGE_LIBS: &[&str] = &[
    "TKernel",
    "TKMath",
    "TKBRep",
    "TKG3d",
    "TKGeomBase",
    "TKTopAlgo",
    "TKPrim",
];

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo"));
    let native_dir = manifest_dir
        .join("../../native/occt_bridge")
        .canonicalize()
        .expect(
            "native/occt_bridge must exist relative to crates/cad-occt-bridge \
             (AICAD-015/AICAD-016 create it)",
        );

    println!(
        "cargo:rerun-if-changed={}",
        native_dir.join("CMakeLists.txt").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        native_dir.join("src/bridge.cpp").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        native_dir.join("include/aicad_occt_bridge.h").display()
    );

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));
    let build_dir = out_dir.join("occt_bridge_build");

    // Match Cargo's own optimization intent rather than always building
    // one fixed CMake configuration.
    let cmake_build_type = match env::var("PROFILE").as_deref() {
        Ok("release") => "Release",
        _ => "RelWithDebInfo",
    };

    let configure = Command::new("cmake")
        .arg("-S")
        .arg(&native_dir)
        .arg("-B")
        .arg(&build_dir)
        .arg(format!("-DCMAKE_BUILD_TYPE={cmake_build_type}"))
        .output()
        .expect(
            "failed to invoke `cmake` to configure native/occt_bridge — \
             is CMake installed and on PATH?",
        );

    if !configure.status.success() {
        panic!(
            "cmake configure of native/occt_bridge failed (exit {:?}):\n--- stdout ---\n{}\n--- stderr ---\n{}",
            configure.status.code(),
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr),
        );
    }

    let configure_stdout = String::from_utf8_lossy(&configure.stdout);
    let occt_library_dir = configure_stdout
        .lines()
        .find_map(|line| line.trim().strip_prefix("-- AICAD_OCCT_LIBRARY_DIR="))
        .unwrap_or_else(|| {
            panic!(
                "cmake configure succeeded but did not print an \
                 `AICAD_OCCT_LIBRARY_DIR=` line (see \
                 native/occt_bridge/CMakeLists.txt) — configure stdout was:\n{configure_stdout}"
            )
        })
        .to_string();

    let build = Command::new("cmake")
        .arg("--build")
        .arg(&build_dir)
        .arg("--target")
        .arg("aicad_occt_bridge")
        .arg("--config")
        .arg(cmake_build_type)
        .output()
        .expect("failed to invoke `cmake --build` for the aicad_occt_bridge target");

    if !build.status.success() {
        panic!(
            "cmake --build of aicad_occt_bridge failed (exit {:?}):\n--- stdout ---\n{}\n--- stderr ---\n{}",
            build.status.code(),
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr),
        );
    }

    // `native_dir` is configured as CMake's *project root* here (`-S
    // native_dir`, not `-S <repo_root>`), so its targets' outputs land
    // directly in `build_dir`, not nested under a mirrored
    // `native/occt_bridge/` subpath (verified empirically: the .a
    // landed at `build_dir/libaicad_occt_bridge.a`, not
    // `build_dir/native/occt_bridge/libaicad_occt_bridge.a`).
    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=static=aicad_occt_bridge");

    println!("cargo:rustc-link-search=native={occt_library_dir}");
    for lib in OCCT_BRIDGE_LIBS {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }

    // aicad_occt_bridge.a is a C++ static library (uses the C++ standard
    // library and exceptions internally, even though nothing it throws
    // ever crosses its extern "C" boundary — see
    // project/reports/AICAD-016.md); Rust does not link libstdc++ on its
    // own.
    println!("cargo:rustc-link-lib=dylib=stdc++");
}
