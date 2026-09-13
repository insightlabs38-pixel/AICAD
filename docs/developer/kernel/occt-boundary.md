# OCCT boundary

## Why the boundary exists

OCCT provides the exact B-rep implementation, but AICAD does not make OCCT the definition of its public language or semantic identity. Kernel replacement/version changes should not require rewriting HIR, source bindings, parameter identity, or feature identity.

`cad-kernel-api` therefore exposes domain-level operations and kernel-neutral value types. No OCCT class or enum is allowed through that API.

## Rust adapter and native bridge

`crates/cad-occt-bridge` is the Rust-side adapter. Its `build.rs` configures/builds/installs `native/occt_bridge` with CMake and links the resulting shared library.

`native/occt_bridge/include/aicad_occt_bridge.h` is the narrow C ABI. The C++ implementation catches OCCT/C++ exceptions internally and communicates failures as status values rather than unwinding an exception through Rust FFI.

Native shape identity uses opaque/context-scoped handle data rather than exposing a C++ pointer. Stale or foreign-context handles are rejected. Kernel contexts are thread-affine according to the current conservative policy.

## Topology identity

A kernel shape/edge/face handle is valid only in its kernel context/epoch. It is not durable source identity. Operation-local lineage evidence may be exposed upward, but persistence and semantic reference resolution belong to the AICAD semantic layer above the adapter.

Similarly, raw face/edge indices in current Geometry IR are enumeration selectors, not a loophole for persistent topology identity.

## Native build

Normal Cargo builds invoke the native bridge automatically. For isolated diagnosis:

```sh
cmake -S native/occt_bridge -B native/occt_bridge/build
cmake --build native/occt_bridge/build
ctest --test-dir native/occt_bridge/build --output-on-failure
```

The CMake project requires an `OpenCASCADE` package configuration and checks its required component libraries explicitly.

## Licensing boundary

OCCT remains an external dependency. The current development policy keeps it dynamically/shared-library compatible and preserves third-party licensing separation; a formal distribution/license review remains a later gate before public binary/commercial distribution policy is frozen.
