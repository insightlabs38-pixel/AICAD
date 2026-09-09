# AICAD-019 — Implement kernel context lifecycle and shape-handle table

## Objective

Harden `native/occt_bridge`'s kernel-context lifecycle on top of
AICAD-016's minimal handle registry, per `project/TASKS.yaml` (AICAD-019),
closing the specific gap AICAD-016/017/018's reports each flagged and
deferred here: a stale/destroyed **context** pointer was previously
dereferenced directly (`context->shapes...`) rather than validated,
meaning misuse (using a context after destroying it) was undefined
behavior rather than a clean rejection. This is the last task of Batch 1A
(kernel boundary); the Stage-1A gate packet follows this report.

## Dependencies checked

AICAD-018 (`cad-occt-bridge` safe Rust wrapper) — complete
(`project/reports/AICAD-018.md`). This task changes the native ABI (one
new status value, one new function) and the Rust binding together, in one
commit, so neither side is ever inconsistent with the other on the
branch.

## What was done

### Native (`native/occt_bridge`)

1. **Process-wide live-context registry.** `bridge.cpp` now keeps a
   `std::mutex`-guarded `std::unordered_set<AicadKernelContext*>` of every
   currently-live context. `aicad_kernel_context_create` inserts into it;
   `aicad_kernel_context_destroy` removes from it *before* freeing, and
   — critically — only calls `delete` if the pointer was actually found
   in the registry, making a second `destroy` call on the same pointer a
   safe no-op instead of a double-free.
2. **`check_context()`**, a new internal helper every `extern "C"`
   function calls first: returns `AICAD_STATUS_NULL_CONTEXT` for a null
   pointer (unchanged behavior) or the new `AICAD_STATUS_INVALID_CONTEXT`
   for a non-null pointer that is not in the live-context registry —
   **without ever dereferencing that pointer** (only its integer value is
   looked up in the registry). This is what turns "call a function with a
   context pointer after destroying it" from a use-after-free into a
   clean, structured rejection. Documented in the code as the same
   pointer-value-only-comparison idiom used by allocators/object pools
   for exactly this purpose.
3. **`AICAD_STATUS_INVALID_CONTEXT = 6`** added to `AicadStatus`
   (`aicad_occt_bridge.h`), appended rather than inserted/reordered so no
   existing status's numeric value changes.
4. **`aicad_kernel_context_live_shape_count`**, a new introspection
   function (context-validated the same way as every other function)
   returning how many shapes a context currently owns — added
   specifically to make "a create/release cycle leaves nothing behind"
   directly assertable in tests, rather than only inferred indirectly.
5. Every existing function (`aicad_create_box`,
   `aicad_shape_bbox_diagonal`, `aicad_shape_release`,
   `aicad_kernel_context_last_error`) now routes its context check through
   `check_context()` instead of a bare `context == nullptr` check.
   `last_error` on an invalid/stale context now returns the same safe
   empty static string it already returned for a null context (no
   behavior change for null; new safe behavior for stale).

### Rust (`crates/cad-occt-bridge`, `crates/cad-kernel-api`)

- Mirrored `AicadStatus::InvalidContext = 6` in `cad-occt-bridge`'s `ffi`
  module and `aicad_kernel_context_live_shape_count` in the `unsafe
  extern "C"` block.
- Added `cad_kernel_api::KernelError::InvalidContext` (no payload —
  matches `InvalidHandle`'s shape, since neither carries backend-specific
  text) with a doc comment explaining it should be unreachable through
  ordinary safe `Context` usage (Rust's ownership model already makes
  "call a method after `drop`" a compile error) and exists for the
  underlying ABI's own guarantee, relevant to any future non-Rust or
  unsafe-Rust caller of the native bridge.
- Added `Context::live_shape_count(&mut self) -> KernelResult<u64>`.
- `error_for` now maps `ffi::AicadStatus::InvalidContext` to
  `KernelError::InvalidContext`.

## Deliberately not implemented (recorded, not silently skipped)

**True epoch/generation-based bulk handle invalidation**
(`docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` §5's "a raw handle
belongs to a geometry epoch... topology mutation may invalidate handles")
is **not** implemented by this task. No operation in this bridge mutates
an existing shape in place yet (`create_box`/`bbox_diagonal`/`release`
never invalidate a handle except via explicit `release`) — there is
nothing yet to exercise or validate real epoch semantics against.
Implementing epoch bookkeeping now, with no operation to test it against,
would be exactly the kind of unrequested architecture invented ahead of
evidence `AGENTS.md` asks this agent to avoid. This task instead closes
the concrete, evidenced gap that existed (context-pointer lifecycle
safety) and records this boundary explicitly in
`aicad_occt_bridge.h`'s own top comment, so AICAD-020 onward adds epoch
tracking together with the first operation that actually needs it, rather
than this task guessing its shape in advance.

## Implementation decisions (within AGENTS.md's autonomously-allowed
scope)

- **`AICAD_STATUS_INVALID_CONTEXT` appended as value `6`**, not inserted
  next to `AICAD_STATUS_NULL_CONTEXT` (which would have been the more
  "logical" grouping) — chosen to keep every existing tested status value
  numerically stable across this task's diff, since nothing yet depends
  on this ABI being frozen/stable but minimizing gratuitous churn is
  still good practice.
- **Live-context registry keyed by raw pointer value, never dereferenced
  for the check itself** — the one piece of this task with a genuine (if
  narrow) portability caveat: comparing/hashing a dangling pointer's
  *value* without reading through it is a widely-used, practically safe
  idiom on every mainstream compiler/platform this project targets, but
  is not, in the strictest reading of the C++ standard, unconditionally
  defined for an arbitrary dangling value. Documented directly in the
  code (`bridge.cpp`'s `check_context` comment) rather than left
  implicit, and independently verified clean under AddressSanitizer +
  UndefinedBehaviorSanitizer (see Verification) — ASan in particular is
  specifically designed to catch exactly this class of use-after-free
  issue, so a clean sanitizer run through the new dangling-context test
  cases is meaningful evidence, not just an assertion.
- **Registry guarded by its own separate `std::mutex`**, not folded into
  per-context state — contexts themselves remain single-thread-affine
  (Stage-1 kernel policy #9, unchanged), but creating/destroying
  *different* contexts from different threads (or validating one
  context's liveness while a different thread destroys a different
  context) needed its own synchronization since the registry is
  process-wide shared state.

## Files changed

- Edited: `native/occt_bridge/include/aicad_occt_bridge.h`
  (`AICAD_STATUS_INVALID_CONTEXT`, `aicad_kernel_context_live_shape_count`,
  updated lifecycle/last-error documentation).
- Edited: `native/occt_bridge/src/bridge.cpp` (live-context registry,
  `check_context()`, updated every function to use it,
  `aicad_kernel_context_live_shape_count`).
- Edited: `native/occt_bridge/tests/abi_smoke_test.cpp` (+13 new
  assertions: `live_shape_count` at three points in the round-trip;
  stale-context rejection for all four context-taking functions;
  double-destroy safety; a freshly created context after the stale one is
  destroyed still behaves correctly).
- Edited: `crates/cad-kernel-api/src/lib.rs`
  (`KernelError::InvalidContext` + updated tests).
- Edited: `crates/cad-occt-bridge/src/lib.rs`
  (`ffi::AicadStatus::InvalidContext`,
  `ffi::aicad_kernel_context_live_shape_count`,
  `Context::live_shape_count`, updated `error_for`; +1 new test exercising
  the raw FFI directly for the stale-context case, +assertions on the
  existing round-trip test).

## Verification (exact commands/results)

```
$ rm -rf build && cmake -S . -B build && cmake --build build
[100%] Built target occt_discovery_probe
[100%] Built target aicad_occt_bridge
[100%] Built target abi_smoke_test
[100%] Built target abi_c_linkage_test

$ ./build/native/occt_bridge/abi_smoke_test
PASS: context A created
... (30 PASS lines total, including all AICAD-019-specific cases below) ...
PASS: live_shape_count is 1 after one create_box
PASS: live_shape_count is 0 after releasing the only shape
PASS: create_box against a destroyed context is rejected, not a crash
PASS: bbox_diagonal against a destroyed context is rejected, not a crash
PASS: release against a destroyed context is rejected, not a crash
PASS: live_shape_count against a destroyed context is rejected, not a crash
PASS: last_error on a destroyed context returns an empty string, not a crash
PASS: double-destroying the same context did not crash
PASS: context C created after ctx_a/ctx_b destruction
PASS: live_shape_count on the fresh context C succeeds
PASS: a freshly created context owns no shapes
ABI_SMOKE_TEST_RESULT=PASS

$ cd build && ctest --output-on-failure
100% tests passed, 0 tests failed out of 2
```

### AddressSanitizer + UndefinedBehaviorSanitizer (critical for this task specifically — it deliberately exercises dangling-pointer scenarios)

```
$ rm -rf build-asan
$ cmake -S . -B build-asan -DCMAKE_BUILD_TYPE=Debug \
    -DCMAKE_CXX_FLAGS="-fsanitize=address,undefined -g -O0" \
    -DCMAKE_C_FLAGS="-fsanitize=address,undefined -g -O0" \
    -DCMAKE_EXE_LINKER_FLAGS="-fsanitize=address,undefined"
$ cmake --build build-asan -- -j"$(nproc)"
$ ./build-asan/native/occt_bridge/abi_smoke_test; echo $?
... (all 30 PASS lines, including every stale-context/double-destroy case) ...
ABI_SMOKE_TEST_RESULT=PASS
0
$ ./build-asan/native/occt_bridge/abi_c_linkage_test; echo $?
C_LINKAGE_TEST_RESULT=PASS
0
```
Clean — no heap-use-after-free, double-free, or other memory-safety/UB
report through any of the new dangling-context/double-destroy paths.

### Rust

```
$ cargo build -p cad-occt-bridge
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.25s

$ cargo test -p cad-occt-bridge
running 7 tests
test tests::degenerate_box_is_a_contained_internal_error_not_a_panic ... ok
test tests::create_query_release_round_trip ... ok
test tests::dropping_a_context_with_outstanding_shapes_does_not_panic_or_abort ... ok
test tests::foreign_context_handle_is_rejected ... ok
test tests::stale_context_pointer_is_rejected_at_the_ffi_layer_not_dereferenced ... ok
test tests::released_handle_is_rejected_not_aliased ... ok
test tests::two_contexts_never_mint_colliding_handle_ids ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p cad-kernel-api
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.33s

$ cargo test --workspace
(all crates pass; 54 workspace-wide `running 0 tests` results from
untouched placeholder crates, cad-occt-bridge's 7 and cad-kernel-api's 8
included)
```

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**.
- Task-specific check: `ctest` (native, 2/2) — **PASS**; ASan/UBSan run
  clean through every new dangling-pointer/double-destroy case;
  `cargo test -p cad-occt-bridge`/`cad-kernel-api` — **PASS** (7/7, 8/8).

## Limitations / follow-up

- Epoch/generation-based bulk invalidation remains unimplemented — see
  "Deliberately not implemented" above. AICAD-020 onward should add it
  together with the first topology-mutating operation.
- The live-context registry adds one global mutex-guarded set; if context
  creation/destruction ever becomes a hot path at high concurrency, this
  could become a contention point — no evidence of that yet (Stage 1 has
  no concurrent-context workload), so no lock-free alternative was
  pursued now, per "smallest correct solution."
- `KernelError::InvalidContext` is currently unreachable through the safe
  `cad-occt-bridge::Context` API (by construction, per Rust's ownership
  rules) — it exists for the ABI's own contract and any future
  lower-level or non-Rust caller. This is expected, not a gap: it is
  exercised directly in this task's tests (both native, under ASan/UBSan,
  and via `cad-occt-bridge`'s raw `ffi` module).
