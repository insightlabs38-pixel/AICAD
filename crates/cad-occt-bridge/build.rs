//! Builds and installs `native/occt_bridge` (AICAD-015/016), then links
//! `cad-occt-bridge` against the resulting `aicad_occt_bridge` shared
//! library.
//!
//! Reuses `native/occt_bridge/CMakeLists.txt` directly via `cmake`/
//! `cmake --build`/`cmake --install` (no `cmake` Cargo crate dependency)
//! so OCCT discovery logic exists in exactly one place
//! (`native/occt_bridge/CMakeLists.txt`, already proven in
//! `project/reports/AICAD-015.md`/`AICAD-016.md`) rather than being
//! re-implemented here.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo"));
    // crates/cad-occt-bridge -> crates -> workspace root.
    let workspace_root = manifest_dir
        .parent()
        .expect("crates/cad-occt-bridge has a parent directory")
        .parent()
        .expect("crates/ has a parent directory (the workspace root)");
    let native_dir = workspace_root.join("native").join("occt_bridge");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));
    let build_dir = out_dir.join("native-build");
    let install_dir = out_dir.join("native-install");

    run(Command::new("cmake")
        .arg("-S")
        .arg(&native_dir)
        .arg("-B")
        .arg(&build_dir)
        .arg(format!("-DCMAKE_INSTALL_PREFIX={}", install_dir.display()))
        .arg("-DCMAKE_BUILD_TYPE=RelWithDebInfo"));

    run(Command::new("cmake")
        .arg("--build")
        .arg(&build_dir)
        .arg("--target")
        .arg("aicad_occt_bridge")
        .arg("--parallel"));

    run(Command::new("cmake").arg("--install").arg(&build_dir));

    let lib_dir = install_dir.join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=aicad_occt_bridge");
    // So the built test/bench binaries can find libaicad_occt_bridge.so
    // at runtime without requiring LD_LIBRARY_PATH to be set by hand.
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());

    println!(
        "cargo:rerun-if-changed={}",
        native_dir.join("CMakeLists.txt").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        native_dir
            .join("include")
            .join("aicad_occt_bridge.h")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        native_dir
            .join("src")
            .join("aicad_occt_bridge.cpp")
            .display()
    );
}

fn run(command: &mut Command) {
    let status = command
        .status()
        .unwrap_or_else(|e| panic!("failed to execute {command:?}: {e}"));
    if !status.success() {
        panic!("command failed ({status}): {command:?}");
    }
}
