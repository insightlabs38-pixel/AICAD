//! Build script (AICAD-018): configures and builds
//! `native/occt_bridge`'s `aicad_occt_bridge` static library via CMake,
//! then links it (and the specific OCCT libraries it requires) into
//! this crate.
//!
//! Fails the build loudly (`panic!`) if `cmake` is missing or the
//! native configure/build fails, rather than silently producing a crate
//! that links but calls into nothing — a missing/broken kernel must be
//! a build-time error, not a latent runtime one (Stage-1 kernel
//! policy: fail closed on a broken kernel).

use std::path::PathBuf;
use std::process::Command;

const REQUIRED_OCCT_LIBS: &[&str] = &[
    "TKBRep",
    "TKPrim",
    "TKGeomAlgo",
    "TKTopAlgo",
    "TKG3d",
    "TKG2d",
    "TKGeomBase",
    "TKMath",
    "TKernel",
];

fn main() {
    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo"),
    );
    // crates/cad-occt-bridge -> repository root -> native/occt_bridge
    let native_dir = manifest_dir.join("../../native/occt_bridge");
    let native_dir = native_dir
        .canonicalize()
        .unwrap_or_else(|e| panic!("native/occt_bridge not found at {native_dir:?}: {e}"));

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));
    let build_dir = out_dir.join("native-build");

    run_cmake(
        &[
            "-S",
            native_dir.to_str().unwrap(),
            "-B",
            build_dir.to_str().unwrap(),
            "-DCMAKE_BUILD_TYPE=RelWithDebInfo",
        ],
        "configure",
    );
    run_cmake(
        &[
            "--build",
            build_dir.to_str().unwrap(),
            "--target",
            "aicad_occt_bridge",
        ],
        "build",
    );

    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=static=aicad_occt_bridge");
    for lib in REQUIRED_OCCT_LIBS {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }
    // aicad_occt_bridge.cpp is compiled as C++ and uses the C++ standard
    // library (std::vector, std::atomic, exceptions).
    println!("cargo:rustc-link-lib=dylib=stdc++");

    println!(
        "cargo:rerun-if-changed={}",
        native_dir.join("CMakeLists.txt").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        native_dir.join("include").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        native_dir.join("src").display()
    );
}

fn run_cmake(args: &[&str], step: &str) {
    let status = Command::new("cmake").args(args).status().unwrap_or_else(|e| {
        panic!(
            "failed to invoke `cmake` to {step} native/occt_bridge (is cmake installed and on PATH?): {e}"
        )
    });
    if !status.success() {
        panic!("cmake {step} of native/occt_bridge failed: {status}");
    }
}
