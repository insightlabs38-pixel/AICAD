# STAGE1-A_KERNEL_BOUNDARY — Batch 1A checkpoint

Batch 1A (AICAD-015 through AICAD-019, "kernel boundary") checkpoint
evidence. This is an internal batch checkpoint within Stage 1, not a
roadmap stage-exit gate — Stage 1 itself is only exited by AICAD-037's
owner gate packet (`project/gates/stage-1-gate.md`, not yet written) and
a `project/DECISION_LOG.md` owner ruling, per `project/gates/README.md`
and `AGENTS.md`. This document exists to satisfy the standing Stage-1
work plan's requirement that Batch 1B (AICAD-020 onward) may only begin
once Batch 1A's checklist below is satisfied with evidence, and is
self-certified by the implementation agent, not an owner approval.

## 1. Exact git revision

```text
3f5222cda4b71fcf49804676c113ad1536fae341 (AICAD-019: Implement kernel
context lifecycle and shape-handle table)
```

Working tree was clean at the time this checkpoint was written; each of
AICAD-015 through AICAD-019 is its own commit on
`branch/loving-feynman-2tw2sf`, in order.

## 2. Environment (Stage-1 kernel policy #15)

- OS: Ubuntu 24.04.4 LTS (x86_64)
- Compiler: g++ (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0
- CMake: 3.28.3
- OCCT: 7.6.3+dfsg1-7.1build1 (Debian/Ubuntu packages;
  upstream OCCT version string `7.6.3`)
- Rust toolchain: rustc/cargo 1.98.1 (per `rust-toolchain.toml`)

## 3. Checklist evidence

### Reproducible OCCT discovery

`native/occt_bridge/CMakeLists.txt`'s `find_package(OpenCASCADE
REQUIRED)` succeeds and reports version 7.6.3, include dir
`/usr/include/opencascade`, library dir `/usr/lib/x86_64-linux-gnu`
(AICAD-015, `project/reports/AICAD-015.md`). Independently re-confirmed
via `cad-occt-bridge`'s `build.rs` invoking the same CMake project
(AICAD-018, `project/reports/AICAD-018.md`) — two separate build paths
(standalone CMake, and Cargo-driven) both discover it identically.

### C ABI builds successfully

`native/occt_bridge` builds `aicad_occt_bridge` (static lib),
`occt_discovery_probe`, and `bridge_abi_tests` cleanly (AICAD-015/016).
`cargo build -p cad-occt-bridge` builds the same static library via its
`build.rs` and links against it successfully, confirmed by locating
`libaicad_occt_bridge.a` under `target/debug/build/.../out/native-build/`
and by every downstream test binary linking and running (AICAD-018).

### No OCCT type leakage

`native/occt_bridge/include/aicad_occt_bridge.h` includes only
`<stdint.h>`; every OCCT header/type (`TopoDS_Shape`,
`BRepPrimAPI_MakeBox`, `Standard_Failure`, etc.) appears only in
`src/aicad_occt_bridge.cpp`, never in the public header (AICAD-016).
`crates/cad-occt-bridge/src/ffi.rs` mirrors the header with plain
integer/pointer types only; `crates/cad-kernel-api` has zero OCCT
dependency by construction (it does not depend on `cad-occt-bridge` or
the native crate at all — the dependency edge runs the other way,
confirmed by `crates/cad-kernel-api/Cargo.toml` having no path/external
dependencies) (AICAD-017/018).

### Exception containment

Every `extern "C"` function in `aicad_occt_bridge.cpp` wraps its body in
`catch (const Standard_Failure&) / catch (const std::exception&) /
catch (...)`, each returning a status code (AICAD-016). Verified against
a real OCCT-thrown exception (not just argument-validation paths): a
`1e-300`-extent box construction returned
`AICAD_STATUS_NATIVE_EXCEPTION` (status code 5) with process exit 0 — see
`project/reports/AICAD-016.md`'s "Additional verification" section for
the exact reproduction command/output.

### Invalid handles rejected

`bridge_abi_tests.cpp` (native, AICAD-016): an out-of-range slot index is
rejected as `AICAD_STATUS_INVALID_HANDLE`.
`tests/lifecycle.rs::out_of_range_slot_is_rejected_as_invalid_handle`
(Rust, AICAD-019): the same case through the safe wrapper, rejected as
`KernelError::InvalidHandle`.

### Stale handles rejected

Native: `bridge_abi_tests.cpp` — querying/destroying a just-destroyed
handle is rejected as `AICAD_STATUS_STALE_HANDLE` (including a
double-destroy case). Rust: `tests/lifecycle.rs::stale_handle_is_rejected_after_destroy`
and `::double_destroy_is_rejected_as_stale_not_silently_accepted`
(AICAD-019).

### Foreign-context handles rejected

Native: `bridge_abi_tests.cpp` — a handle from `ctx1` presented to
`ctx2` is rejected as `AICAD_STATUS_FOREIGN_CONTEXT`. Rust:
`tests/lifecycle.rs::foreign_context_handle_is_rejected` (two live
contexts) and
`::handle_from_a_destroyed_context_is_rejected_by_a_later_context_not_aliased`
(a handle from an already-`drop`ped context presented to an
independently created later context — possible to guarantee because
native context ids are a monotonic `std::atomic<uint64_t>` counter, never
a memory address) (AICAD-016/019).

### Rust/native ownership behavior tested

`crates/cad-occt-bridge/tests/lifecycle.rs` (6 tests, AICAD-019) proves
the full handle-safety contract — including the no-aliasing property
(`released_slot_does_not_alias_newly_created_geometry`: destroying one
solid and creating another that may reuse its slot leaves the old handle
permanently stale and resolves the new handle to its own, distinct,
volume) — driven exclusively through the public Rust API with no
`unsafe` in the test file itself. `KernelContext` is neither `Send` nor
`Sync` (inferred automatically from its raw-pointer field; Stage-1
kernel policy #9), and its `Drop` impl destroys the native context
deterministically (exercised by every test in `lifecycle.rs`/`smoke.rs`
via implicit end-of-scope drop).

### Required workspace checks pass

```
$ cargo fmt --all -- --check                                              # exit 0
$ cargo clippy --workspace --all-targets --all-features -- -D warnings    # exit 0, zero warnings
$ cargo build --workspace --all-targets                                   # exit 0
$ cargo test --workspace                                                  # exit 0, all suites ok
$ cd native/occt_bridge && cmake -S . -B build && cmake --build build && \
    (cd build && ctest --output-on-failure)                               # 2/2 tests passed
```

CI (`.github/workflows/ci.yml`) mirrors these: `fmt`, `clippy`,
`build-and-test` (now installing OCCT dev packages, AICAD-018), and
`native-build-smoke` (now installing OCCT dev packages and running
`ctest`, AICAD-016).

## 4. Test/benchmark inventory added this batch

- `native/occt_bridge/tests/bridge_abi_tests.cpp`: 21 checks (AICAD-016).
- `crates/cad-kernel-api`: 4 unit tests (AICAD-017).
- `crates/cad-occt-bridge/src/ffi.rs`: 2 unit tests (AICAD-018).
- `crates/cad-occt-bridge/tests/smoke.rs`: 2 integration tests
  (AICAD-018).
- `crates/cad-occt-bridge/tests/lifecycle.rs`: 6 integration tests
  (AICAD-019).

No benchmarks were added — not required at this batch (no performance
claim is made yet; Stage-1 policy on measurement applies once real
constructive/hard-geometry operations exist in Batch 1B/1C).

## 5. Known limitations carried forward

- Only two real geometry operations exist end-to-end
  (`create_box`/`shape_volume`, plus `destroy`) — the full operation
  catalog in `native/occt_bridge/README.md` is Batch 1B (constructive)
  and 1C (hard ops) work.
- Dimensions/volumes are plain `f64`, not a typed `Length`/`Volume` —
  `cad-units` does not exist yet (Stage 3); the native ABI is
  deliberately unit-unaware (documented in its own header).
- `cargo build --workspace` now requires `cmake` and the OCCT
  development packages on any machine building this workspace; CI was
  updated accordingly, but no top-level `README.md`/`CONTRIBUTING.md`
  documents this yet for a human contributor (flagged, not blocking).
- `AicadKernelContext` has no internal synchronization; concurrent use
  from multiple threads is undefined behavior by design (Stage-1 policy
  #9). Not fuzz/thread-sanitizer tested in this batch — nothing yet
  exercises concurrency to test against.

## 6. Unresolved owner decisions touched

None. No item in `project/OWNER_DECISIONS.md` was touched, resolved, or
silently assumed by this batch — all five tasks stayed within the
already-approved kernel-boundary policies in DL-5/DL-6 and the Stage-1
kernel policies.

## 7. Recommendation

**Ready to proceed to Batch 1B (AICAD-020 onward).** Every checklist
item above has direct evidence from this batch's own commits and test
runs, not merely restated intent. This is the agent's own batch-
readiness self-certification, not a roadmap stage-exit approval —
Stage 1's own exit gate remains AICAD-037's job, and Stage 1 as a whole
still requires an owner decision in `project/DECISION_LOG.md` before
`AICAD-038`/Stage 2, exactly as `project/CURRENT_STAGE.md` already
states.
