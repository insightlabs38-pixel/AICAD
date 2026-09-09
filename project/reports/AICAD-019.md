# AICAD-019 — Implement kernel context lifecycle and shape-handle table

## Objective

Prove the kernel context lifecycle and shape-handle table (implemented
natively in AICAD-016 and wrapped safely in AICAD-018) actually holds
its ownership/safety contract when driven exclusively through the safe
Rust API, with no `unsafe` in the proof itself. This is the last task in
Batch 1A before the checkpoint gate. Per `project/TASKS.yaml`
(AICAD-019, Stage 1 / Batch 1A).

## Dependencies checked

AICAD-018 (`cad-occt-bridge` safe wrapper) — complete,
`project/reports/AICAD-018.md`; `KernelContext` and its methods are used
unchanged.

## Why this task's scope is tests, not new production code

The shape-handle table's actual logic (slot table, generation counters,
context-id tagging, free-list reuse) was already implemented in
AICAD-016 at the native layer — that is where the data lives and where
the safety invariant must actually be enforced, since `cad-occt-bridge`
(AICAD-018) is a thin RAII/status-translation wrapper with no handle
bookkeeping of its own. AICAD-016's own native test suite
(`bridge_abi_tests.cpp`) already proved the invariant directly against
the C ABI. What AICAD-016/018 together had not yet demonstrated is that
the same invariant survives being driven purely through the public Rust
API that Batch 1B onward will actually use — that is this task's
deliverable, matching `project/TASKS.yaml`'s own acceptance criterion
("Focused implementation plus task-specific tests/benchmarks satisfy
the referenced plan requirements"). No production code changed.

## What was done

Added `crates/cad-occt-bridge/tests/lifecycle.rs`, an integration test
crate with no `unsafe` anywhere, covering exactly the Batch-1A
checkpoint's ownership-behavior items:

1. `stale_handle_is_rejected_after_destroy` — a destroyed handle no
   longer resolves (`KernelError::StaleHandle`).
2. `double_destroy_is_rejected_as_stale_not_silently_accepted` —
   destroying an already-destroyed handle fails rather than
   succeeding twice.
3. `foreign_context_handle_is_rejected` — a handle minted in one live
   `KernelContext` is rejected (`ForeignContext`) by both
   `shape_volume` and `destroy_solid` on a second, live context.
4. `out_of_range_slot_is_rejected_as_invalid_handle` — a handle with a
   slot index that context has never allocated is rejected
   (`InvalidHandle`), isolated from the foreign-context case by reusing
   that same context's own real id.
5. `released_slot_does_not_alias_newly_created_geometry` — after
   destroying one solid and creating another (which may reuse the freed
   slot), the old handle stays rejected and the new handle resolves to
   its own (not the old) volume. This is the direct Rust-level
   counterpart of the no-aliasing property AICAD-016 proved natively.
6. `handle_from_a_destroyed_context_is_rejected_by_a_later_context_not_aliased`
   — a handle from a context that has since been `drop`ped (destroying
   the native context) is rejected as `ForeignContext`, not accidentally
   accepted, by an independently created later context. This is
   possible to guarantee because native context ids come from a
   monotonic `std::atomic<uint64_t>` counter (AICAD-016), never a memory
   address, so a later context can never coincidentally reuse an earlier
   one's id.

All six tests pass; see exact output below.

## Verification (exact commands/results)

```
$ cargo test -p cad-occt-bridge
running 2 tests   (src/lib.rs unit tests)
test result: ok. 2 passed; 0 failed

running 6 tests   (tests/lifecycle.rs)
test foreign_context_handle_is_rejected ... ok
test double_destroy_is_rejected_as_stale_not_silently_accepted ... ok
test out_of_range_slot_is_rejected_as_invalid_handle ... ok
test handle_from_a_destroyed_context_is_rejected_by_a_later_context_not_aliased ... ok
test stale_handle_is_rejected_after_destroy ... ok
test released_slot_does_not_alias_newly_created_geometry ... ok
test result: ok. 6 passed; 0 failed

running 2 tests   (tests/smoke.rs, AICAD-018, unaffected)
test result: ok. 2 passed; 0 failed

$ cargo fmt --all -- --check          # exit 0
$ cargo clippy --workspace --all-targets --all-features -- -D warnings   # exit 0, zero warnings
$ cargo build --workspace --all-targets   # exit 0
$ cargo test --workspace                  # exit 0, all suites ok (incl. the 6 new tests)
```

## Files changed

- Added: `crates/cad-occt-bridge/tests/lifecycle.rs`
- Modified: `crates/cad-occt-bridge/src/lib.rs` (re-exports `RawHandle`
  in addition to the AICAD-018 re-exports, so this test file can
  construct a deliberately out-of-range handle for check #4 without
  reaching into a private module or adding `unsafe`).

## Implementation decisions

- Re-exporting `RawHandle` from `cad-occt-bridge` is not a new leak of
  an OCCT-specific type — it is the same kernel-neutral type
  `KernelSolid::raw()`/`::from_raw()` (already public via AICAD-017)
  operate on; without it, an external integration test would have no
  way to construct a deliberately-invalid handle for check #4 without
  `unsafe` FFI access.
- Did not add any new production API surface (no new `KernelContext`
  method) — every property tested here is already reachable through
  AICAD-018's existing methods.

## Regressions added

6 integration tests in `crates/cad-occt-bridge/tests/lifecycle.rs`, run
under `cargo test --workspace` going forward — the permanent Rust-level
counterpart to `bridge_abi_tests.cpp`'s native-level coverage.

## Limitations

None beyond those already recorded in AICAD-016/017/018's reports (unit-
unaware dimensions; `KernelSolid`-only operations so far; no
`Send`/`Sync` by design).

## Unresolved questions

None. No escalation condition in `project/TASKS.yaml` (AICAD-019) was
triggered.

## Batch 1A status

This completes Batch 1A (AICAD-015 through AICAD-019). Per
`project/CURRENT_STAGE.md`/the standing Stage-1 work plan, the batch
checkpoint gate `project/gates/STAGE1-A_KERNEL_BOUNDARY.md` is prepared
next, before Batch 1B (AICAD-020 onward) may begin.
