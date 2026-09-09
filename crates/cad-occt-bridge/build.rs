//! AICAD-018 build script: compiles `native/occt_bridge`'s C ABI
//! (AICAD-016) directly via the `cc` crate and links it plus the OCCT
//! toolkits it needs.
//!
//! Deliberately independent of `native/occt_bridge/CMakeLists.txt`
//! (AICAD-015): that CMake project exists for native-only development
//! and testing (`occt_probe`, `abi_smoke_test`) outside of Cargo. This
//! build script does not invoke CMake — it recompiles the same source
//! file directly, which is the simpler and more standard integration
//! for a small `-sys`-style native dependency, and keeps `cargo build`
//! from requiring a separate CMake configure/build step.

use std::env;
use std::path::{Path, PathBuf};

/// OCCT toolkits this bridge's implementation currently calls into.
/// Mirrors `native/occt_bridge/CMakeLists.txt`'s
/// `AICAD_OCCT_BRIDGE_TOOLKITS` list; grows in lockstep with it as
/// AICAD-016's operation set grows.
const OCCT_TOOLKITS: &[&str] = &[
    "TKernel",
    "TKMath",
    "TKG2d",
    "TKG3d",
    "TKGeomBase",
    "TKBRep",
    "TKGeomAlgo",
    "TKTopAlgo",
    "TKPrim",
];

/// Candidate OCCT header install locations, checked in order, when
/// `AICAD_OCCT_INCLUDE_DIR` is not set. `/usr/include/opencascade` is
/// the path AICAD-015 confirmed OCCT 7.6.3's `find_package(OpenCASCADE
/// CONFIG)` resolves to on this project's Ubuntu-based development/CI
/// image; the others cover common alternate package layouts so this
/// build script fails with a clear message rather than a confusing
/// compiler error when neither matches.
const OCCT_INCLUDE_CANDIDATES: &[&str] = &[
    "/usr/include/opencascade",
    "/usr/local/include/opencascade",
    "/usr/include/opencascade-7.6.3",
];

fn find_occt_include_dir() -> PathBuf {
    if let Ok(dir) = env::var("AICAD_OCCT_INCLUDE_DIR") {
        return PathBuf::from(dir);
    }
    for candidate in OCCT_INCLUDE_CANDIDATES {
        let path = Path::new(candidate);
        if path.join("Standard_Version.hxx").is_file() {
            return path.to_path_buf();
        }
    }
    panic!(
        "AICAD: could not find OCCT headers (looked for Standard_Version.hxx under {:?}); \
         set AICAD_OCCT_INCLUDE_DIR to override",
        OCCT_INCLUDE_CANDIDATES
    );
}

fn main() {
    // native/occt_bridge/ lives two directories above this crate
    // (repo_root/crates/cad-occt-bridge -> repo_root/native/occt_bridge).
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let bridge_dir = manifest_dir
        .parent() // crates/
        .and_then(Path::parent) // repo root
        .expect("crates/cad-occt-bridge should be two directories under the repo root")
        .join("native")
        .join("occt_bridge");

    let bridge_include = bridge_dir.join("include");
    let bridge_source = bridge_dir.join("src").join("occt_bridge.cpp");
    let bridge_header = bridge_include.join("aicad").join("occt_bridge.h");

    if let Ok(lib_dir) = env::var("AICAD_OCCT_LIB_DIR") {
        println!("cargo:rustc-link-search=native={lib_dir}");
    }

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .include(&bridge_include)
        .include(find_occt_include_dir())
        .file(&bridge_source)
        .warnings(false) // OCCT's own headers emit warnings this crate does not own
        .compile("aicad_occt_bridge");

    for toolkit in OCCT_TOOLKITS {
        println!("cargo:rustc-link-lib=dylib={toolkit}");
    }

    println!("cargo:rerun-if-changed={}", bridge_source.display());
    println!("cargo:rerun-if-changed={}", bridge_header.display());
    println!("cargo:rerun-if-env-changed=AICAD_OCCT_INCLUDE_DIR");
    println!("cargo:rerun-if-env-changed=AICAD_OCCT_LIB_DIR");
}
