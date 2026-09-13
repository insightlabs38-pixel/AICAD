# Installation and prerequisites

AICAD's Rust code and native geometry bridge build together. A working setup therefore needs both the Rust toolchain and an Open CASCADE Technology (OCCT) development installation.

## Required tools

The repository pins **Rust 1.98.1** in `rust-toolchain.toml` and requests the `rustfmt` and `clippy` components. It also requires:

- CMake 3.16 or newer;
- a C++17-capable compiler;
- OCCT development libraries discoverable by CMake through `find_package(OpenCASCADE CONFIG)`.

If `rustup` is installed, entering the repository and invoking Cargo will select the pinned toolchain automatically.

## OCCT on Debian/Ubuntu

The native bridge documents these base development packages:

```sh
sudo apt update
sudo apt install cmake build-essential \
  libocct-foundation-dev \
  libocct-modeling-data-dev \
  libocct-modeling-algorithms-dev
```

The complete bridge uses additional OCCT modules for operations such as booleans, fillets, meshing, and interchange. Distribution packaging differs; if a required module is absent, the CMake configuration reports the missing OCCT library by name rather than silently degrading capability. Install the corresponding `libocct-*-dev` package for your distribution.

If OCCT is installed in a nonstandard prefix, set `OpenCASCADE_DIR` to the directory containing `OpenCASCADEConfig.cmake` before building.

## Build the workspace

From the repository root:

```sh
cargo build --workspace
```

`crates/cad-occt-bridge/build.rs` configures, builds, and installs the native bridge into Cargo's build output automatically. You do not need to run a separate native build for normal Rust workspace builds.

Run the workspace tests with:

```sh
cargo test --workspace
```

For contributor checks, also see the [developer testing guide](../../developer/testing/).

## Optional native-only probe

To diagnose OCCT discovery independently of Cargo, the native bridge can be configured directly:

```sh
cmake -S native/occt_bridge -B native/occt_bridge/build
cmake --build native/occt_bridge/build
ctest --test-dir native/occt_bridge/build --output-on-failure
```

This is primarily a developer/diagnostic path; ordinary users can build through Cargo.
