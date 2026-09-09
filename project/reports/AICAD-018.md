# AICAD-018 — Create cad-occt-bridge safe Rust wrapper

## Objective

Populate `crates/cad-occt-bridge` with a safe Rust wrapper around
`native/occt_bridge`'s C ABI (AICAD-015/AICAD-016), implementing
`cad-kernel-api`'s (AICAD-017) vendor-neutral handle/error vocabulary, per
`project/TASKS.yaml` (AICAD-018) and this crate's own `README.md`
("Safe Rust wrapper around `native/occt_bridge`... Exposes only coarse,
domain-shaped operations... never raw OCCT class names").

## Dependencies checked

- AICAD-016 (native C ABI boundary) — complete
  (`project/reports/AICAD-016.md`); this task links against exactly the
  ABI it documents (`aicad_kernel_context_create/_destroy/_last_error`,
  `aicad_create_box`, `aicad_shape_bbox_diagonal`, `aicad_shape_release`).
- AICAD-017 (`cad-kernel-api` handle/error types) — complete
  (`project/reports/AICAD-017.md`); this crate's public API returns
  `cad_kernel_api::KernelShape`/`KernelResult`/`KernelError` and nothing
  else.

## What was done

### `build.rs`

Configures and builds `native/occt_bridge`'s `aicad_occt_bridge` CMake
target from Cargo (`cmake -S native/occt_bridge -B $OUT_DIR/occt_bridge_build`,
then `cmake --build ... --target aicad_occt_bridge`), then emits
`cargo:rustc-link-search`/`cargo:rustc-link-lib` for the resulting static
library, the OCCT module libraries it needs, and `libstdc++` (needed
because `aicad_occt_bridge.a` is a C++ static library, and Rust does not
link libstdc++ on its own). Reuses `native/occt_bridge/CMakeLists.txt`'s
own `find_package(OpenCASCADE CONFIG)` discovery — added one new
machine-parseable `message(STATUS "AICAD_OCCT_LIBRARY_DIR=...")` line
there for `build.rs` to read back, rather than reimplementing OCCT path
discovery independently in Rust. `cargo:rerun-if-changed` is set on the
CMakeLists.txt, the bridge source, and the public header, so editing any
of them triggers a native rebuild on the next `cargo build`.

### `src/lib.rs`

- `mod ffi` — hand-written (no `bindgen`) `unsafe extern "C"` declarations
  mirroring `native/occt_bridge/include/aicad_occt_bridge.h` field-for-field
  and discriminant-for-discriminant (`AicadKernelContext` as a
  zero-sized-array opaque struct, `AicadShapeHandle` as a `#[repr(C)]
  { id: u64 }`, `AicadStatus` as a `#[repr(C)]` enum with explicit
  discriminants `0..=5`). Not exported from the crate — every other item
  wraps it behind a safe API. Hand-written rather than generated because
  the surface is small (6 functions, 2 structs, 1 enum) and stable;
  avoiding a `bindgen`/`libclang` build-time dependency keeps this
  crate's own build simpler and removes a fragility point for CI
  environments that may not have `libclang` available.
- `Context` — the safe wrapper's central type. Owns a raw
  `*mut ffi::AicadKernelContext`; `Context::new()` returns
  `KernelResult<Self>`; `Drop` calls `aicad_kernel_context_destroy`.
  Every FFI call site has an explicit `// SAFETY:` comment justifying why
  the call is sound at that point (context non-null and alive; out-pointers
  valid and uniquely owned for the call's duration).
- `Context::create_box(&mut self, dx, dy, dz) -> KernelResult<KernelShape>`,
  `Context::shape_bbox_diagonal(&mut self, KernelShape) -> KernelResult<f64>`,
  `Context::release_shape(&mut self, KernelShape) -> KernelResult<()>` —
  the same minimal round-trip AICAD-016 proved natively, now proved again
  through the Rust FFI binding itself (struct layout, enum representation,
  and pointer handling compatibility are not assumed — see "Verification"
  below).
- Every non-OK `AicadStatus` is converted to the matching `KernelError`
  variant (`error_for`), reading `aicad_kernel_context_last_error` (copied
  into an owned `String` immediately, respecting the header's documented
  pointer-lifetime contract) for the variants that carry backend detail.
- `#![allow(dead_code)]` was needed only on the local `ffi::AicadStatus`
  enum (rustc's dead-code analysis cannot see FFI-returned enum values as
  "constructed" since Rust code never builds an `AicadStatus::NullContext`
  literal itself — the native side does) — scoped to that one enum, not
  crate-wide.

### Units — explicit non-decision

`create_box` takes raw `f64` millimeters, not a typed `cad-units`
quantity. Documented at length in the module doc as a deliberate,
narrow-scope choice, not a violation of `AGENTS.md`'s typed-units
non-negotiable: that rule governs the public AICAD language surface, and
nothing in the public language reaches this crate directly (only
`cad-geometry-api`/`cad-geometry-runtime`, WP-05 — not yet implemented —
will call it, converting a typed `Length` to a raw millimeter `f64` at
that boundary, matching `docs/plan/01_SYSTEM_ARCHITECTURE.md` §5's IR
layering). Millimeters because that is OCCT's own native convention,
already established by AICAD-015/016.

### Thread affinity

`Context` wraps a raw pointer, so it is `!Send`/`!Sync` by Rust's ordinary
auto-trait rules — no explicit opt-out was needed, which itself matches
Stage-1 kernel policy #9 ("treat `KernelContext` conservatively as
single-thread-affine") for free. Documented in the module doc rather than
left implicit, so a future task does not accidentally add a `unsafe impl
Send` without re-deriving why that would be wrong.

## Bug found and fixed during this task (native/occt_bridge, AICAD-016 follow-up)

Building `native/occt_bridge` **standalone** with all targets (`cmake -S
native/occt_bridge -B build && cmake --build build`, the same sequence
CI's `native-build-smoke` job runs) failed to link `abi_c_linkage_test`
with `undefined reference to 'main'`. Root cause: `native/occt_bridge/CMakeLists.txt`'s
`project(aicad_occt_bridge CXX)` call only enabled the CXX language;
`abi_c_linkage_test.c` (added in AICAD-016, a plain-C source) was
silently not compiled into any object file at all (confirmed by
inspecting the verbose link command: no `.c.o` object appeared on the
link line), yet CMake still attempted to link the target and failed with
a confusing linker error rather than a clear configure-time diagnostic.
This was **not** caught by AICAD-016's own verification because that
task only ever configured from the **repo root** (`cmake -S . -B build`).
The root `CMakeLists.txt`'s `project(aicad_native)` call has no language
list, so CMake defaults it to enabling C and CXX both — and that
enablement is visible to the nested `project(aicad_occt_bridge CXX)` call
inside `add_subdirectory(native/occt_bridge)` too (CMake does not
"revoke" a language a parent scope already enabled), which is why the
root-level build built `abi_c_linkage_test` successfully throughout
AICAD-015/016 despite the subdirectory's own `project()` call never
requesting C. This task's `native/occt_bridge/CMakeLists.txt` header
comment advertises standalone configurability
(`cmake -S native/occt_bridge -B build`, independent of the Rust
workspace) — the first time anything actually exercised that path was
this task, which is what surfaced the bug. **Verified the mechanism**,
not just patched around the symptom: reverted the fix, reconfigured from
the repo root only, and confirmed the build *does* succeed even without
the fix (proving the root-level path was never broken) before
re-applying `project(aicad_occt_bridge CXX C)` and confirming both the
standalone `native/occt_bridge` configuration and the root configuration
build and pass `ctest` with the fix in place.

## Implementation decisions (within AGENTS.md's autonomously-allowed
scope)

- Hand-written FFI declarations instead of `bindgen` — see "What was
  done" above.
- `build.rs` re-runs CMake's configure step on every `cargo build`
  invocation of this crate (not only when `native/occt_bridge` changed);
  CMake's own configure is fast (~0.3-0.5s, measured) and idempotent
  (re-running it when nothing changed is a correctness no-op, just a
  small fixed cost), so this was judged simpler and more robust than
  trying to detect "does a valid CMake cache already exist at this
  `OUT_DIR` path" by hand. `cargo:rerun-if-changed` still controls when
  **Cargo** considers the whole build script worth invoking again.
- `Context`'s `create_box`/`shape_bbox_diagonal`/`release_shape` take
  `&mut self` (not `&self`) even though nothing here is literally mutating
  Rust-visible state beyond the opaque native side — matches the fact
  that a native kernel context's true state absolutely does mutate on
  every one of these calls, and keeps this crate's API from claiming a
  thread-safety property (shared, read-only access) that Stage-1 kernel
  policy #9 explicitly says not to assume.

## Files changed

- Added: `crates/cad-occt-bridge/build.rs`.
- Edited: `crates/cad-occt-bridge/Cargo.toml` (added `cad-kernel-api`
  path dependency).
- Edited: `crates/cad-occt-bridge/src/lib.rs` (replaced the
  AICAD-002/003 placeholder with the implementation above; +6 unit
  tests).
- Edited: `native/occt_bridge/CMakeLists.txt` (`AICAD_OCCT_LIBRARY_DIR`
  status line for `build.rs`; `project(... CXX C)` fix, see bug note
  above).
- Edited: `Cargo.lock` (new `cad-kernel-api` -> `cad-occt-bridge` internal
  edge; no external crates were added — this crate still has zero
  external dependencies).

## Verification (exact commands/results)

```
$ rm -rf target && cargo build --workspace --all-targets
   Compiling cad-occt-bridge v0.0.0 (/home/user/AICAD/crates/cad-occt-bridge)
   Compiling cad-kernel-api v0.0.0 (/home/user/AICAD/crates/cad-kernel-api)
   ... (every other placeholder crate) ...
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.09s

$ cargo test -p cad-occt-bridge
running 6 tests
test tests::degenerate_box_is_a_contained_internal_error_not_a_panic ... ok
test tests::foreign_context_handle_is_rejected ... ok
test tests::create_query_release_round_trip ... ok
test tests::dropping_a_context_with_outstanding_shapes_does_not_panic_or_abort ... ok
test tests::released_handle_is_rejected_not_aliased ... ok
test tests::two_contexts_never_mint_colliding_handle_ids ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test --workspace
(all crates pass; cad-occt-bridge's 6 and cad-kernel-api's 8 tests
included, all other crates remain placeholders with 0 tests)

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.97s
```

### Standalone native build (all targets — this is what surfaced the CXX/C bug above)

```
$ rm -rf native/occt_bridge/build
$ cmake -S native/occt_bridge -B native/occt_bridge/build
-- AICAD occt_bridge: OpenCASCADE 7.6.3 found at /usr/lib/x86_64-linux-gnu
-- AICAD_OCCT_LIBRARY_DIR=/usr/lib/x86_64-linux-gnu
-- Configuring done / Generating done

$ cmake --build native/occt_bridge/build
[100%] Built target aicad_occt_bridge
[100%] Built target abi_smoke_test
[100%] Built target abi_c_linkage_test

$ cd native/occt_bridge/build && ctest --output-on-failure
100% tests passed, 0 tests failed out of 2
```

### Root CMakeLists.txt build (repo root — the CI-adjacent path)

```
$ rm -rf build && cmake -S . -B build && cmake --build build
[100%] Built target occt_discovery_probe
[100%] Built target aicad_occt_bridge
[100%] Built target abi_smoke_test
[100%] Built target abi_c_linkage_test

$ cd build && ctest --output-on-failure
100% tests passed, 0 tests failed out of 2
```

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**.
- Task-specific check: `cargo test -p cad-occt-bridge` — **PASS** (6/6),
  exercising the FFI binding itself (not just the native library
  directly) for the ordinary round-trip, stale-handle rejection,
  foreign-context rejection, contained-exception reporting, and
  no-collision-across-contexts properties; standalone and root-level
  native builds both clean, including the fixed `abi_c_linkage_test`.

## Limitations / follow-up

- `build.rs` always reconfigures CMake on every `cargo build` of this
  crate; if this becomes a measurable build-time cost as the bridge grows,
  a future task could cache/skip reconfiguration when nothing under
  `native/occt_bridge` changed — not done now since the current cost is
  small (~0.3-0.5s) and correctness was prioritized over this
  micro-optimization, per "smallest correct solution."
- Only the same one operation triple AICAD-016 implemented natively
  (`create_box`/`bbox_diagonal`/`release`) is wrapped here. The full
  bridge catalog is added incrementally starting at AICAD-020, matching
  both this crate's and `native/occt_bridge`'s own recorded scope limits.
- `KernelCurve`/`KernelSurface`/`KernelVertex`/`KernelEdge`/`KernelWire`/
  `KernelFace`/`KernelShell`/`KernelSolid` (from `cad-kernel-api`,
  AICAD-017) have no `Context` methods minting them yet — expected; they
  will gain them as the corresponding native operations are added.
