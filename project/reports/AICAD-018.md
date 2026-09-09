# AICAD-018 — Create cad-occt-bridge safe Rust wrapper

## Objective

Implement `crates/cad-occt-bridge` as a safe Rust wrapper around
`native/occt_bridge`'s C ABI, so Rust code can drive the OCCT kernel
through `cad-kernel-api`'s backend-neutral types with no `unsafe` at the
call site. Per `project/TASKS.yaml` (AICAD-018, Stage 1 / Batch 1A).

## Dependencies checked

AICAD-017 (`cad-kernel-api` handle/error types) — complete, see
`project/reports/AICAD-017.md`; `KernelSolid`, `KernelError`, and
`KernelResult` are used directly by this task rather than duplicated.
AICAD-016 (native C ABI) — complete, see `project/reports/AICAD-016.md`;
this task links against `aicad_occt_bridge.h`/`.cpp` unchanged.

## What was done

1. Added `crates/cad-occt-bridge/build.rs`: shells out to `cmake` to
   configure and build `native/occt_bridge`'s `aicad_occt_bridge` CMake
   target into `$OUT_DIR/native-build`, then emits
   `cargo:rustc-link-search`/`cargo:rustc-link-lib` for the static
   library and the specific OCCT libraries `aicad_occt_bridge.cpp`
   actually calls into (`TKBRep TKPrim TKGeomAlgo TKTopAlgo TKG3d TKG2d
   TKGeomBase TKMath TKernel`), plus `stdc++`. Panics with a clear
   message (not a silent skip) if `cmake` is missing or the
   configure/build step fails — a missing/broken native kernel must
   fail the Rust build loudly.
2. Added `crates/cad-occt-bridge/src/ffi.rs`: hand-written, `unsafe
   extern "C"` declarations mirroring
   `native/occt_bridge/include/aicad_occt_bridge.h` field-for-field.
   `AicadStatus` (the C enum) crosses the boundary as a raw `c_int`,
   decoded on the Rust side by `AicadStatus::from_raw` rather than
   assumed to already be a well-formed Rust enum value — `#[repr(C)]`
   layout for a C plain `enum` (no fixed underlying-type syntax in this
   codebase's plain-C-compatible header) is compiler/platform dependent,
   so trusting an FFI return value to already be a correct Rust
   discriminant would be an unverified assumption, not a guarantee.
   `AicadShapeHandle` (all fixed-width integer fields) is `#[repr(C)]`
   and mirrors the C struct directly, which is a safe, standard pattern
   for POD FFI structs.
3. Added `crates/cad-occt-bridge/src/context.rs`: the safe wrapper.
   `KernelContext` holds the raw `*mut AicadKernelContext` (so it is
   automatically neither `Send` nor `Sync` — Stage-1 kernel policy #9,
   enforced by the type system rather than by convention alone), creates
   the native context in `new()`, destroys it in `Drop`, and exposes
   `create_box`/`shape_volume`/`destroy_solid` returning
   `cad_kernel_api::KernelResult<_>`. Every native status code is
   translated to a `KernelError` variant via `status_to_error`; no raw
   FFI type or status code is visible outside this crate.
4. `crates/cad-occt-bridge/src/lib.rs` now re-exports only
   `KernelContext` and the `cad-kernel-api` types it uses
   (`KernelError`, `KernelResult`, `KernelSolid`) — `ffi` stays a private
   module.
5. Added `crates/cad-occt-bridge/tests/smoke.rs`: an integration test
   proving the full stack (Rust -> FFI -> native -> OCCT) end to end —
   create a context, construct a 2x3x4 box, confirm its volume is 24,
   destroy it — plus one argument-rejection case. This is a smoke test
   only; AICAD-019 owns the dedicated ownership/lifecycle suite
   (foreign-context, stale-handle, no-aliasing) exercised through this
   same wrapper.
6. Updated `.github/workflows/ci.yml`'s `build-and-test` job to install
   the OCCT dev packages before `cargo build`/`cargo test`, since
   `cad-occt-bridge`'s `build.rs` now makes every workspace build depend
   on OCCT being present — this was not yet true after AICAD-015/016
   (only the standalone native CMake project needed it then).
7. `crates/cad-occt-bridge/Cargo.toml` gained one dependency:
   `cad-kernel-api = { path = "../cad-kernel-api" }` (the intended
   dependency direction per both crates' README.md; no new external
   dependency was added anywhere in this task).

## Implementation decisions

- Manual FFI declarations instead of `bindgen`: the ABI is five
  functions and two POD types, entirely hand-maintained already on the
  C side (AICAD-016); adding a codegen dependency (and the OCCT-header
  parsing it would imply, which would risk leaking OCCT-specific
  declarations into generated Rust code) is unjustified ceremony for
  this surface area. Revisit if/when the operation catalog in
  `native/occt_bridge/README.md` grows large enough that hand-
  maintenance becomes error-prone.
- `build.rs` uses `std::process::Command` to invoke `cmake` directly
  rather than adding the `cmake` crate as a build-dependency — avoids
  a new external dependency for a straightforward two-command
  configure/build sequence; this workspace currently has zero external
  dependencies (`Cargo.lock` before this task listed only in-workspace
  crates) and this task does not need to be the first to change that.
- `destroy_solid` and `shape_volume` are scoped to `KernelSolid` only
  (not every `cad-kernel-api` handle kind) because `create_box` is the
  only operation that currently produces a handle; adding
  vertex/edge/wire/face/shell/curve/surface variants ahead of any
  operation that produces them would be speculative, not "the smallest
  change that satisfies the task."

## Verification (exact commands/results)

```
$ cargo build -p cad-occt-bridge
   Compiling cad-occt-bridge v0.0.0 (.../crates/cad-occt-bridge)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.13s
$ find target/debug/build -iname 'libaicad_occt_bridge.a'
target/debug/build/cad-occt-bridge-9007d81d56f68e19/out/native-build/libaicad_occt_bridge.a
(confirms build.rs actually invoked cmake and produced the static lib,
not merely that the crate happened to compile without needing to link)

$ cargo test -p cad-occt-bridge
running 2 tests   (src/lib.rs unit tests, ffi::tests)
test ffi::tests::from_raw_decodes_every_documented_status ... ok
test ffi::tests::from_raw_treats_undocumented_values_as_unknown ... ok
running 2 tests   (tests/smoke.rs)
test create_box_query_volume_and_destroy_round_trip ... ok
test create_box_rejects_a_non_positive_dimension ... ok
test result: ok. 2 passed; 0 failed  (both suites)

$ cargo fmt --all -- --check          # exit 0 (after applying `cargo fmt --all` once)
$ cargo clippy --workspace --all-targets --all-features -- -D warnings   # exit 0, zero warnings
$ cargo build --workspace --all-targets   # exit 0
$ cargo test --workspace                  # all suites ok, including cad-kernel-api's and cad-occt-bridge's
```

CI YAML re-validated: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"`
→ `YAML OK`.

## Files changed

- Added: `crates/cad-occt-bridge/build.rs`
- Added: `crates/cad-occt-bridge/src/ffi.rs`
- Added: `crates/cad-occt-bridge/src/context.rs`
- Added: `crates/cad-occt-bridge/tests/smoke.rs`
- Modified: `crates/cad-occt-bridge/src/lib.rs`
- Modified: `crates/cad-occt-bridge/Cargo.toml`
- Modified: `.github/workflows/ci.yml`

## Artifacts

`target/` build output (git-ignored) was not committed. No `native/`
build directory was left behind (build.rs builds under `$OUT_DIR`, which
is itself under the git-ignored `target/`).

## Regressions added

`crates/cad-occt-bridge/tests/smoke.rs` (2 tests) plus 2 new unit tests
in `src/ffi.rs` — all run under `cargo test --workspace` going forward.

## Limitations

- `create_box`/`shape_volume`/`destroy_solid` take plain `f64`
  dimensions/volumes, not a typed `Length`/`Volume` from `cad-units` —
  that crate does not exist yet (Stage 3, AICAD-047-049) and the native
  ABI itself is explicitly unit-unaware (documented in
  `aicad_occt_bridge.h`). This is a known, deliberate gap, not an
  oversight.
- Comprehensive ownership/lifecycle testing (foreign-context rejection,
  stale-handle rejection, no-aliasing on slot reuse) through this Rust
  wrapper is AICAD-019's job, not duplicated here — this task's smoke
  test only proves the plumbing itself works.
- `cargo build --workspace` now requires `cmake` and the OCCT
  development packages on any machine building this workspace,
  including CI (addressed above) and any future contributor's machine
  (not yet documented in a top-level `README.md`/`CONTRIBUTING.md`;
  flagged as a follow-up, not blocking this task).

## Unresolved questions

None. No escalation condition in `project/TASKS.yaml` (AICAD-018) was
triggered.
