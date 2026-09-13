# AICAD-018 — Create cad-occt-bridge safe Rust wrapper

## Objective
Implement `crates/cad-occt-bridge` as a safe Rust wrapper around
`native/occt_bridge`'s C ABI (AICAD-016), expressed entirely in
`cad-kernel-api`'s (AICAD-017) kernel-neutral vocabulary. Per RFC-0002 §3,
this crate is the *only* crate in the workspace permitted to reference
`native/occt_bridge` or OCCT at all.

## Dependencies checked
- AICAD-016 (native C ABI boundary) — complete, see
  `project/reports/AICAD-016.md`. This task builds directly on
  `native/occt_bridge/include/aicad_occt_bridge.h`'s exact function
  signatures and status codes.
- AICAD-017 (`cad-kernel-api` handle/error types) — complete, see
  `project/reports/AICAD-017.md`. This task's public API returns
  `cad_kernel_api::{KernelResult, KernelError, KernelShape, KernelId}`
  exclusively; no `cad-occt-bridge`-specific error or handle type is
  introduced.

## What was done

1. **`crates/cad-occt-bridge/build.rs`** (new): configures, builds
   (`--target aicad_occt_bridge` only, not the probe/test executables),
   and installs `native/occt_bridge` via direct `cmake`/`cmake --build`/
   `cmake --install` invocations (`std::process::Command`, no `cmake`
   Cargo build-dependency) into `$OUT_DIR/native-install`, then emits:
   - `cargo:rustc-link-search=native=<install>/lib`
   - `cargo:rustc-link-lib=dylib=aicad_occt_bridge`
   - `cargo:rustc-link-arg=-Wl,-rpath,<install>/lib` (so built test/bench
     binaries find `libaicad_occt_bridge.so` at runtime without manual
     `LD_LIBRARY_PATH`)
   - `cargo:rerun-if-changed` for the native `CMakeLists.txt`, header, and
     `.cpp` source, so Cargo only re-runs the (otherwise-incremental)
     native build when one of those actually changes.
   Required adding `install(TARGETS aicad_occt_bridge ...)` and
   `install(FILES include/aicad_occt_bridge.h ...)` rules to
   `native/occt_bridge/CMakeLists.txt` (there were none before this task;
   AICAD-015/016 only built in-tree, never installed) — verified
   independently against a scratch install prefix before wiring it into
   `build.rs` (see Verification).
2. **`crates/cad-occt-bridge/src/ffi.rs`** (new, private `mod ffi;`): raw
   `unsafe extern "C"` declarations mirroring
   `aicad_occt_bridge.h` field-for-field —
   `aicad_occt_context_t` (opaque, zero-sized-array idiom),
   `aicad_shape_handle_t` (`#[repr(C)]`, identical field order/types to
   the C struct), and the six `aicad_occt_*` functions. Status codes are
   received as plain `c_int`, not a `#[repr(i32)]` Rust enum.
3. **`crates/cad-occt-bridge/src/lib.rs`** (new): the safe wrapper —
   - `status_result(c_int) -> KernelResult<()>`: the one-to-one status
     translation (0 -> `Ok`, 1-7 -> the matching `KernelError` variant,
     anything unrecognized -> `KernelError::Internal` defensively).
   - `OcctContext`: RAII-owns a `*mut ffi::aicad_occt_context_t`,
     `Drop`-destroys it. Exposes `new()` and `create_box(dx, dy, dz) ->
     KernelResult<Shape<'_>>`.
   - `Shape<'ctx>`: borrows its `OcctContext` for `'ctx`, `Drop`-releases
     its native handle. Exposes `handle() -> KernelShape` (the
     backend-independent identity), `is_valid()`, `volume()`.
   - Every `unsafe` block carries a `// SAFETY:` comment justifying it;
     no `unsafe` is exposed to callers of this crate's public API.
   - 6 integration tests exercising the **real** native library through
     the full FFI path (see Verification) — not mocks.

## Implementation decisions (autonomous, reversible)
- **`std::process::Command` instead of the `cmake` Cargo crate.** Full
  control and full reuse of the exact `native/occt_bridge/CMakeLists.txt`
  already proven in AICAD-015/016, with no dependence on a third-party
  crate's specific assumptions about install-step behavior or target
  selection, which I could not verify without first testing them anyway.
  Zero new external dependencies were added to make this decision.
- **RAII (`OcctContext`/`Shape<'ctx>` with `Drop`)** rather than a
  hand-called `release`/`destroy` API, and **lifetime-bound `Shape<'ctx>`**
  rather than an unscoped shape type. This turns two of the Stage-1
  kernel policies from runtime-only guarantees (already proven in
  `project/reports/AICAD-016.md`) into **compile-time** guarantees at this
  layer: a shape can never outlive the context that produced it (the
  context cannot be dropped while a shape borrows it), and a shape is
  never left un-released by an author forgetting to call a cleanup
  function. This is additive to, not a replacement for, the native
  bridge's own runtime checks (unsafe/raw FFI use of `ffi` module
  internals — not reachable from outside this crate — would still need
  them).
- **`OcctContext`/`Shape` are not `Send`/`Sync`, achieved for free** by
  holding a raw pointer / a reference to a type holding one, rather than
  by writing explicit negative trait impls (which require nightly Rust).
  This matches the native bridge's single-thread-affine contract
  (Stage-1 kernel policy #9) at the type level as a side effect of the
  ownership design, not as a separate mechanism to maintain.
- **Status codes cross FFI as plain `c_int`, converted by hand**, not as
  a `#[repr(i32)]` Rust enum received directly from C — see `ffi.rs`'s doc
  comment: transmuting/reinterpreting an out-of-range C `int` as a
  `#[repr]` Rust enum is undefined behavior per the Rustonomicon's FFI
  guidance, whereas `match`-ing a plain integer defensively (mapping any
  unrecognized value to `KernelError::Internal`) is always sound, even
  though this bridge fully controls the native side and every value it
  can currently produce is enumerated.
- **`unwrap`-free production code; `unwrap`/`expect` confined to
  `#[cfg(test)]`.** The only place this crate could reasonably panic in
  production code is `debug_assert!`/`debug_assert_eq!` guarding an
  invariant that would indicate an AICAD-016 defect (e.g. `AICAD_OCCT_OK`
  returned with a null out-context) — compiled out in release builds,
  intentionally, since these are defects to catch during development/CI,
  not conditions a release build should crash on if they somehow occurred
  anyway.

## Files changed
- Added: `crates/cad-occt-bridge/build.rs`
- Added: `crates/cad-occt-bridge/src/ffi.rs`
- Edited: `crates/cad-occt-bridge/src/lib.rs` (placeholder -> full
  implementation)
- Edited: `crates/cad-occt-bridge/Cargo.toml` (added `cad-kernel-api`
  path dependency)
- Edited: `crates/cad-occt-bridge/README.md`
- Edited: `native/occt_bridge/CMakeLists.txt` (added `install()` rules
  for the `aicad_occt_bridge` library and its public header; updated the
  stale top-of-file comment that still described AICAD-015's
  probe-only scope)

## Verification (exact commands/results)

Environment: unchanged from `project/reports/AICAD-016.md` (Ubuntu
24.04.4 LTS, x86_64, GCC/G++ 13.3.0, CMake 3.28.3, OCCT 7.6.3, Rust
1.98.1).

Install-rule verification (scratch prefix, run before wiring `build.rs`
to depend on it):
```
$ cmake -S native/occt_bridge -B /tmp/aicad_native_test_build -DCMAKE_INSTALL_PREFIX=/tmp/aicad_native_test_install
... (OCCT discovery output as in AICAD-015/016) ...
$ cmake --build /tmp/aicad_native_test_build --target aicad_occt_bridge
[100%] Built target aicad_occt_bridge
$ cmake --install /tmp/aicad_native_test_build
-- Installing: /tmp/aicad_native_test_install/lib/libaicad_occt_bridge.so
-- Installing: /tmp/aicad_native_test_install/include/aicad_occt_bridge.h
```
(scratch directories removed afterward, not committed)

Cold build + test (`cargo clean -p cad-occt-bridge` first, to prove the
full cmake-configure/build/install pipeline runs correctly from a clean
`OUT_DIR`, not just incrementally):
```
$ cargo clean -p cad-occt-bridge
     Removed 314 files, 47.3MiB total
$ time cargo test -p cad-occt-bridge
   Compiling cad-occt-bridge v0.0.0 (/home/user/AICAD/crates/cad-occt-bridge)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.33s
running 6 tests
test tests::context_create_and_destroy_succeeds ... ok
test tests::create_box_rejects_invalid_dimensions ... ok
test tests::dropping_and_recreating_reuses_the_slot_without_aliasing ... ok
test tests::two_contexts_are_fully_independent ... ok
test tests::multiple_shapes_in_one_context_have_independent_lifecycles ... ok
test tests::create_box_is_valid_and_has_the_expected_volume ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
real	0m2.42s
```
(These 6 tests run against the **real** native library via the full FFI
path -- no mocking -- and include the same non-aliasing/multi-context
independence properties AICAD-016 proved in C++, now proved again through
the Rust boundary, confirming the `ffi.rs` struct layouts and calling
convention are ABI-correct: if `aicad_shape_handle_t`'s Rust layout
mismatched the C struct, `volume()` would return garbage or the process
would likely crash, not silently return the exact expected values 6.0,
1.0, 8.0, 125.0, 27.0, 64.0 seen across these tests.)

**Empirical proof that the borrow checker enforces context/shape drop
order** (scratch file, written, compiled to observe the expected error,
then deleted — not committed):
```
// crates/cad-occt-bridge/examples/scratch_dropcheck.rs (scratch only)
fn main() {
    let context = cad_occt_bridge::OcctContext::new().unwrap();
    let shape = context.create_box(1.0, 1.0, 1.0).unwrap();
    drop(context); // should fail: `shape` still borrows `context`
    println!("{:?}", shape.volume());
}
```
```
$ cargo build -p cad-occt-bridge --example scratch_dropcheck
error[E0505]: cannot move out of `context` because it is borrowed
 --> crates/cad-occt-bridge/examples/scratch_dropcheck.rs:4:10
  |
3 |     let shape = context.create_box(1.0, 1.0, 1.0).unwrap();
  |                 ------- borrow of `context` occurs here
4 |     drop(context); // should fail: `shape` still borrows `context`
  |          ^^^^^^^ move out of `context` occurs here
```
This confirms the "compile-time guarantee" claim above is not merely
asserted but demonstrated.

Compiler-warning check:
```
$ cargo clippy -p cad-occt-bridge --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.63s
(exit 0)
```

Required checks per task ticket, run against the full workspace:
```
$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.36s
(exit 0)

$ cargo build --workspace --all-targets
(exit 0)

$ cargo test --workspace
(every crate: ok; cad-kernel-api 6/6, cad-occt-bridge 6/6, all other
crates remain empty placeholders with 0 tests, unchanged by this task)
```

## Regressions added
None. `native/occt_bridge`'s existing `occt_probe`/`abi_boundary_test`
CTest targets are unaffected (this task's `install()` addition is
additive; it does not change how those targets build or run — reconfirmed
by re-running `ctest --test-dir native/occt_bridge/build` after this
task's CMakeLists.txt edit, both tests still pass).

## Limitations / follow-up
- Only `create_box`/`is_valid`/`volume` are wrapped, matching AICAD-016's
  current native operation set exactly. Later Stage-1 tasks add more
  native operations (Batch 1B onward); each will need a corresponding
  `ffi.rs` declaration and safe wrapper method here, following the same
  pattern.
- `Shape::handle()` returns a value-only `KernelShape` (per
  `cad-kernel-api`'s design) that does **not** keep the underlying native
  shape alive or track whether its owning `Shape`/`OcctContext` still
  exists — a `KernelShape` value obtained this way and stored past its
  `Shape`'s lifetime becomes meaningless (not unsafe, since nothing
  reachable from safe code dereferences it directly, but it will not
  resolve to anything if ever passed back into this crate through a
  future lookup-by-`KernelId` API). This is consistent with
  `cad-kernel-api`'s own documented "not a semantic reference" warning
  (AICAD-017) and does not need addressing by this task.
- `Drop for Shape`/`Drop for OcctContext` intentionally ignore the native
  status code on release/destroy (a `Drop` impl cannot propagate a
  `Result`) — see "Implementation decisions" above. A future hardening
  task could add a debug-only assertion here similar to `OcctContext::new`'s
  null-check, if a concrete failure mode ever needs catching this way.
- Next task: AICAD-019 (kernel context lifecycle and shape-handle table).
  Given this task and AICAD-016 already implement a working
  generation-counted table and RAII lifecycle end-to-end, AICAD-019 is
  expected to focus on integration-level lifecycle correctness across the
  full stack (e.g. once a topology-*mutating* operation exists in Batch
  1B to test epoch-bumping against) rather than rebuilding what already
  exists and passes here.

No escalation condition was triggered: this task implements the safe
Rust binding for an already-frozen C ABI (AICAD-016) using already-defined
neutral types (AICAD-017); it introduces no new public AICAD language
syntax/semantics, does not weaken any gate/benchmark/test, and does not
let any OCCT type leak outside this crate (`ffi.rs` is a private module;
no OCCT header or type is referenced anywhere in this crate's own source
— only the C ABI's plain-C-typed function signatures are).
