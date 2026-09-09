# Stage 1, Batch 1A — Kernel Boundary gate evidence

Covers AICAD-015 through AICAD-019. Per `AGENTS.md` ("Stage gates") and
`AICAD_AGENT_OPERATING_MODEL.md` §8, this is a **checkpoint**, not a
stage-exit gate: it records the evidence required before Batch 1B
(constructive geometry, AICAD-020..024) begins, per the roadmap window in
this invocation's operating instructions. It does not require, and does
not constitute, an owner decision — Stage 1 itself still exits only at
AICAD-037, with its own owner gate packet.

## 1. Exact git revision

Evidence below was gathered against the working tree immediately prior to
this batch's final commit (AICAD-019), on branch
`branch/loving-feynman-emgbof`, HEAD at commit `4311715` (AICAD-018) plus
the uncommitted AICAD-019 changes at packet-preparation time; this file
and the AICAD-019 commit land together. `git status --short` was clean of
anything unexpected (only this batch's own edits) at every task's own
verification point, per each task's report.

## 2. Reproducible OCCT discovery

`native/occt_bridge/CMakeLists.txt` (AICAD-015) calls
`find_package(OpenCASCADE REQUIRED CONFIG)` and fails configuration with
an explicit, actionable `FATAL_ERROR` if OCCT is not found or is below
major version 7. Verified:
- **Positive** (`project/reports/AICAD-015.md`): `cmake -S . -B build`
  finds OpenCASCADE 7.6.3 at `/usr/lib/x86_64-linux-gnu`, cross-checked
  against the installed `libocct-*-dev` Debian package version and the
  `OCC_VERSION_COMPLETE` macro compiled into `occt_discovery_probe`.
- **Negative** (`project/reports/AICAD-015.md`): configuring with
  `-DCMAKE_DISABLE_FIND_PACKAGE_OpenCASCADE=ON` fails configuration
  cleanly with the expected message, confirming the failure path is a
  hard error, not a silent skip.
- Environment recorded (Stage-1 kernel policy #15): Ubuntu 24.04.4 LTS,
  x86_64, g++ 13.3.0, rustc/cargo 1.98.1, OCCT 7.6.3
  (`libocct-*-dev` 7.6.3+dfsg1-7.1build1), CMake 3.28.3.

## 3. C ABI builds successfully

Three independent build paths all succeed on a clean tree (re-verified
fresh at this packet's preparation time, not only reused from individual
task reports):

1. `native/occt_bridge` configured **standalone**
   (`cmake -S native/occt_bridge -B build`) — this is the path that
   surfaced and let this batch fix a real bug (AICAD-018: the CXX-only
   `project()` call silently dropped the one C test source; fixed by
   `project(aicad_occt_bridge CXX C)`).
2. Repo **root** `CMakeLists.txt` (`cmake -S . -B build`,
   `add_subdirectory(native/occt_bridge)`).
3. **`cargo build --workspace`**, via `crates/cad-occt-bridge/build.rs`
   (AICAD-018), which itself drives CMake to build exactly the
   `aicad_occt_bridge` library target and links it (plus the OCCT module
   libraries and `libstdc++`) into the Rust crate.

All three, re-run clean from scratch immediately before writing this
packet:
```
$ rm -rf build build-asan native/occt_bridge/build target
$ cmake -S . -B build && cmake --build build
[100%] Built target occt_discovery_probe
[100%] Built target aicad_occt_bridge
[100%] Built target abi_smoke_test
[100%] Built target abi_c_linkage_test
$ cd build && ctest --output-on-failure
100% tests passed, 0 tests failed out of 2

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.71s
```

## 4. No OCCT type leakage

Checked by direct inspection at packet-preparation time (not merely
asserted in task reports):
```
$ grep -n "TopoDS\|BRep\|gp_\|Standard_\|OCCT" crates/cad-occt-bridge/src/lib.rs crates/cad-kernel-api/src/lib.rs
(every match is inside a doc comment, a private mod ffi internal, or a
test's string literal — never in a `pub fn` signature or a public struct
field)
```
- `crates/cad-kernel-api` (the vendor-neutral vocabulary,
  `KernelCurve`/`KernelSurface`/`KernelShape`/`KernelVertex`/`KernelEdge`/
  `KernelWire`/`KernelFace`/`KernelShell`/`KernelSolid`/`KernelError`) has
  **zero** dependencies (`Cargo.toml` confirmed empty `[dependencies]`
  section) — it cannot name an OCCT type even by accident, since it has
  no way to reach one.
- `crates/cad-occt-bridge`'s `mod ffi` (the only place OCCT-shaped ABI
  types like `AicadKernelContext`/`AicadShapeHandle`/`AicadStatus`
  appear) is a private module (`mod ffi`, not `pub mod ffi`) — nothing
  outside the crate can name it.
- The native ABI itself (`aicad_occt_bridge.h`) exposes only opaque
  handles, a plain status enum, and primitive types — never a `TopoDS_*`,
  `BRep*`, or other OCCT class name (confirmed by the header's own
  content, unchanged in this respect since AICAD-016).

## 5. Exception containment

Every `native/occt_bridge` `extern "C"` function routes OCCT-side work
through `guard()`, which catches `Standard_Failure` (OCCT's exception
root), then `std::exception`, then `...`, before returning a structured
`AicadStatus` — verified with a real adversarial case, not merely
inspected: a degenerate zero-dimension box independently confirmed (via a
standalone experiment, `project/reports/AICAD-016.md`) to throw
`Standard_DomainError` deep inside OCCT is caught and reported as
`AICAD_STATUS_KERNEL_INTERNAL_ERROR`, with a descriptive `last_error`,
in every one of: the native `abi_smoke_test`, that same test built and
run under AddressSanitizer + UndefinedBehaviorSanitizer, and
`cad-occt-bridge`'s own Rust test
(`degenerate_box_is_a_contained_internal_error_not_a_panic`).

## 6. Invalid handles rejected

`aicad_shape_bbox_diagonal`/`aicad_shape_release` with handle id `0`
(never issued) return `AICAD_STATUS_INVALID_HANDLE`. Verified in
`native/occt_bridge/tests/abi_smoke_test.cpp` ("handle id 0 (never
issued) is rejected") and `cad-occt-bridge`'s Rust tests indirectly via
the released/foreign-context cases below (which also cover "never
issued" in spirit — a handle id neither context has ever minted).

## 7. Stale handles rejected

A released handle is never reissued (handle ids are minted from one
process-wide, monotonically increasing counter — AICAD-016) and querying
or re-releasing it after release returns `AICAD_STATUS_INVALID_HANDLE`
rather than aliasing a new shape. Verified natively
(`abi_smoke_test.cpp`, "querying a released (stale) handle is rejected,
not aliased" / "double-release... is rejected") and in Rust
(`cad-occt-bridge::tests::released_handle_is_rejected_not_aliased`).

**Extended in AICAD-019 to the *context* itself**, closing a gap the
AICAD-016/017/018 reports each explicitly flagged and deferred: a
destroyed context pointer is now validated against a process-wide
live-context registry (by pointer *value* only, never dereferenced) and
rejected as the new `AICAD_STATUS_INVALID_CONTEXT`, and
`aicad_kernel_context_destroy` is now safe to call twice on the same
pointer (a no-op the second time, not a double-free). Verified natively —
including under AddressSanitizer + UndefinedBehaviorSanitizer, which
would flag a real use-after-free/double-free had the fix been
incorrect — and in Rust
(`cad-occt-bridge::tests::stale_context_pointer_is_rejected_at_the_ffi_layer_not_dereferenced`).

## 8. Foreign-context handles rejected

A handle minted on one context is rejected (`AICAD_STATUS_INVALID_HANDLE`)
when presented to a different context, and two contexts' handle ids never
collide — both guaranteed by construction (one process-wide handle-id
counter, never per-context) rather than by incidental table design.
Verified natively (`abi_smoke_test.cpp`, "a handle minted on context A is
rejected by context B" / "context B's handle id never collides with
context A's handle ids") and in Rust
(`cad-occt-bridge::tests::foreign_context_handle_is_rejected` /
`two_contexts_never_mint_colliding_handle_ids`).

## 9. Rust/native ownership behavior tested

- `Context::new`/`Drop` round-trip through the real native
  `aicad_kernel_context_create`/`_destroy` (not mocked) — every Rust test
  exercises this implicitly; `dropping_a_context_with_outstanding_shapes_does_not_panic_or_abort`
  exercises it explicitly for the case of a context still owning shapes
  at drop time.
- `aicad_kernel_context_live_shape_count` (new in AICAD-019) makes shape
  ownership directly assertable rather than only inferred: verified going
  from 0 -> 1 (after `create_box`) -> 0 (after `release`) in both the
  native test and `cad-occt-bridge::tests::create_query_release_round_trip`.
- Native `Drop`-adjacent safety (context destruction cleans up all
  outstanding shapes without crashing; double-destroy is a safe no-op) is
  verified clean under AddressSanitizer + UndefinedBehaviorSanitizer, not
  merely "ran without visibly crashing."
- `Context` is `!Send`/`!Sync` by ordinary Rust auto-trait rules (it
  wraps a raw pointer, no explicit opt-out needed), matching Stage-1
  kernel policy #9; documented in `cad-occt-bridge`'s module doc.

## 10. Required workspace checks pass

Re-run fresh, on the clean tree, immediately before writing this packet:
```
$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.71s

$ cargo test --workspace
(all pass: 54 untouched-placeholder-crate `0 tests` results,
cad-occt-bridge 7/7, cad-kernel-api 8/8)

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.29s
```

## 11. Known limitations (carried forward honestly, not hidden)

- **Epoch/generation-based bulk handle invalidation is not implemented.**
  `docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` §5 describes raw
  handles as belonging to a geometry epoch that topology mutation can
  invalidate in bulk; nothing in this bridge yet mutates an existing
  shape in place (only create/query/release exist), so there is nothing
  to validate real epoch semantics against yet. Recorded in
  `aicad_occt_bridge.h`'s own top comment as explicitly deferred to
  AICAD-020 onward, together with the first operation that needs it —
  not silently skipped.
- **Only one operation triple exists**
  (`create_box`/`shape_bbox_diagonal`/`shape_release`). The full bridge
  catalog (`docs/plan/01_SYSTEM_ARCHITECTURE.md` §4's `create_box ..
  export_step` list) is Batch 1B onward's job, added incrementally as
  each safe-wrapper need arises, per that section's own "add capabilities
  only as required" rule.
- **`KernelCurve`/`KernelSurface`/`KernelVertex`/`KernelEdge`/`KernelWire`/
  `KernelFace`/`KernelShell`/`KernelSolid`** (from `cad-kernel-api`,
  AICAD-017) have no native operation minting them yet — only
  `KernelShape` does. Expected: these are committed API vocabulary ahead
  of the native capability that will back each one, not unused dead
  surface.
- **No thread-safety beyond the live-context registry itself.** Contexts
  remain documented as single-thread-affine (Stage-1 kernel policy #9);
  the registry's own mutex only protects concurrent create/destroy of
  *different* contexts, not concurrent use of the *same* context.
- **Sanitizer runs (ASan/UBSan) are manual, not yet wired into standing
  CI.** Regular sanitizer runs are an explicit Stage-1-hardening-mode
  activity (once AICAD-037 completes) per this routine's own operating
  instructions, not a Batch-1A requirement; they were run manually in
  AICAD-016, AICAD-018, and AICAD-019 specifically because each of those
  tasks' own changes had a plausible memory-safety failure mode worth
  checking directly.
- **OCCT's license/redistribution terms remain an open owner decision**
  (`project/OWNER_DECISIONS.md` D13), unaffected by this batch, which
  only builds against the system-installed OCCT for local
  development/CI use.

## 12. Recommendation

**Proceed to Batch 1B** (AICAD-020..024, constructive geometry). This is
a checkpoint recommendation only, consistent with `AGENTS.md`'s
"the agent may prepare gate evidence and recommend pass/do-not-pass; it
may not approve a roadmap stage" — Batch 1A is a within-stage checkpoint,
not the Stage-1 exit gate itself (which remains AICAD-037, still
requiring its own owner-recorded decision before Stage 2). All nine items
this checklist requires (§2-10 above) have direct, reproduced evidence;
no item was found unmet or waived.
