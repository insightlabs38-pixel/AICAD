# AICAD-019 — Implement kernel context lifecycle and shape-handle table

## Objective
Close the gap AICAD-016 and AICAD-017 explicitly deferred: turn the
native bridge's plain per-context shape vector into a real generational
shape-handle table, so a released handle can never accidentally alias
whatever new shape later reuses its slot — `AGENTS.md` non-negotiable
"Released/stale handles must never accidentally alias newly created
geometry." Per `project/TASKS.yaml` (AICAD-019), the last task in Batch
1A (kernel boundary).

## Dependencies checked
- AICAD-016 (native C ABI) — its own report's "Limitations" section
  explicitly named this gap and deferred it here by name.
- AICAD-017 (`cad-kernel-api` handle/error types) — `KernelHandle`
  extended in place (see below); no other AICAD-017 type changed.
- AICAD-018 (`cad-occt-bridge` safe wrapper) — its `OcctContext` extended
  with `release_shape`; `create_box`/`shape_volume` unchanged in
  behavior, only in the handle shape they carry.

## What was done

1. **`native/occt_bridge/include/aicad/occt_bridge.h`**:
   - added `generation: uint32_t` to `AicadShapeHandle`;
   - added `aicad_occt_release_shape(ctx, handle, out_status)`;
   - updated `aicad_occt_shape_volume`'s doc comment to state that a
     released or generation-mismatched (stale) handle is also
     `AICAD_STATUS_INVALID_HANDLE`, not only an out-of-range index.
2. **`native/occt_bridge/src/occt_bridge.cpp`**:
   - `AicadOcctContext`'s single `std::vector<TopoDS_Shape> shapes`
     became `std::vector<Slot> slots` (`Slot { TopoDS_Shape shape;
     uint32_t generation; bool live; }`) plus `std::vector<uint32_t>
     free_indices` (a free list);
   - added `ValidateHandle(ctx, handle, out_status) -> Slot*`, a shared
     helper enforcing context/index/liveness/generation checks
     identically for every operation that consumes a handle (`shape_volume`
     and the new `release_shape`) — refactored rather than duplicated,
     since AICAD-016 already had two near-identical validation blocks
     and a third would have been the "premature duplication"
     `AGENTS.md` asks this agent to avoid;
   - `create_box` now takes a slot from `free_indices` if one exists
     (reusing its already-bumped generation), else grows `slots` with a
     fresh generation-0 slot;
   - `release_shape` clears the slot's `TopoDS_Shape` (dropping OCCT's
     reference to it), marks it `!live`, bumps its `generation` by
     exactly one, and pushes its index onto `free_indices`. A
     double-release or a release of any other invalid handle fails
     (`AICAD_STATUS_INVALID_HANDLE`), it is not a silent no-op.
3. **`native/occt_bridge/tests/abi_smoke_test.cpp`**: nine new checks
   (27 total, up from 18) proving the property end-to-end: release a
   live handle -> succeeds; use it again -> rejected; release it again
   -> rejected (not a no-op); create a new box -> reuses the freed slot
   index with a *different* generation; query the new box's volume ->
   matches the new dimensions, not the old ones; query the *old*
   (pre-release) handle again, after the slot has been reused -> still
   rejected. That last check is the actual non-aliasing property, not
   merely "an invalid handle is rejected in isolation."
4. **`crates/cad-kernel-api/src/lib.rs`**: `KernelHandle<Kind>` gained a
   `generation: u32` field; `new()` now takes it as a third parameter;
   `PartialEq`/`Hash`/`Debug` extended to include it. Updated this
   crate's own six tests accordingly (added a `generation` argument to
   every `KernelHandle::new` call; added one new inequality case for
   handles that differ only by generation).
5. **`crates/cad-occt-bridge/src/lib.rs`**: `ffi::AicadShapeHandle`
   gained `generation: u32` (matching the native struct field-for-field);
   added `aicad_occt_release_shape`'s FFI declaration; added
   `OcctContext::release_shape(handle) -> KernelResult<()>`; factored the
   repeated `KernelSolid -> ffi::AicadShapeHandle` conversion (now used
   by three methods, not two) into one `to_ffi_handle` function. Added
   two new tests: `released_handle_is_rejected_and_double_release_fails`
   and `stale_handle_never_aliases_a_reused_slot` (11 tests total, up
   from 9) — the latter is the Rust-level equivalent of the native
   smoke test's core non-aliasing check, run through the real compiled
   FFI, not mocked.

## Implementation decisions
- **Generation bumps exactly once, at release, not again at reuse.**
  A released slot is already `!live`, so any handle against it is
  rejected on the liveness check alone before generation is even
  compared. Once reused, the slot becomes `live` again but keeps the
  generation value set at release time — the *old* handle (which
  recorded the pre-release generation) then fails the generation
  comparison specifically. Bumping twice (once at release, again at
  reuse) would work too but adds no additional safety, since no handle
  is ever issued carrying an intermediate value; kept the simpler rule.
- **`ValidateHandle` returns a mutable `Slot*` rather than a `bool` plus
  separate accessor**, since both of this task's two operations that use
  it (`shape_volume`, `release_shape`) need to read or mutate the same
  slot they just validated — returning the pointer avoids a second
  lookup and keeps the validate-then-use pattern atomic within one
  function call, with no possibility of the index changing in between
  (this bridge has no concurrency yet; `AicadOcctContext`'s single-
  thread-affine contract, `AGENTS.md` policy #9, makes that a non-issue
  for now).
- **`KernelHandle::new`'s signature changed** (two args -> three) rather
  than adding a fourth optional field or a builder — this is Stage 1,
  the type has no external consumers besides this workspace's own two
  crates, and both call sites were updated in this same commit; there is
  no compatibility surface yet to preserve.
- **Did not add a distinct `AICAD_STATUS_STALE_HANDLE` status code.**
  `AICAD_STATUS_INVALID_HANDLE` already meant "recognized context, but
  this specific handle is not currently valid for it"; a stale handle
  (right context, in-range index, wrong generation) is exactly that
  category, not a new one — adding a fourth handle-rejection code with
  no behavioral difference from the caller's perspective (both are
  `KernelError::InvalidHandle` either way) would be an unrequested
  distinction, not a needed one.

## Files changed
- Modified: `native/occt_bridge/include/aicad/occt_bridge.h`
- Modified: `native/occt_bridge/src/occt_bridge.cpp`
- Modified: `native/occt_bridge/tests/abi_smoke_test.cpp`
- Modified: `crates/cad-kernel-api/src/lib.rs`
- Modified: `crates/cad-occt-bridge/src/lib.rs`

## Verification (exact commands/results)

Native, clean rebuild:

```
$ cd native/occt_bridge && rm -rf build && mkdir build && cd build
$ cmake -DCMAKE_BUILD_TYPE=Release .. && cmake --build . -j"$(nproc)"
[100%] Built target abi_smoke_test
(only pre-existing OCCT-header deprecation warnings; none from AICAD code)

$ ./abi_smoke_test
... (27 checks, including:)
ok: using a just-released handle is rejected as invalid/stale
ok: releasing an already-released handle again is rejected, not a silent no-op
ok: a new box can reuse a released slot
ok: the new box actually reused the released slot's index (free-list worked)
ok: the reused slot's generation differs from the released handle's generation
ok: the reused slot's volume matches the NEW box (7*8*9 = 504), not the old one
ok: the old (pre-release) handle stays rejected even after its slot is reused, never silently aliasing the new box
all checks passed
EXIT: 0

$ ctest --output-on-failure
100% tests passed, 0 tests failed out of 2

$ valgrind --error-exitcode=1 --leak-check=full --show-leak-kinds=definite,indirect ./abi_smoke_test
ERROR SUMMARY: 0 errors from 0 contexts (suppressed: 0 from 0)
(16 bytes "still reachable" at exit — the same pre-existing OCCT
 internal static, unchanged from AICAD-016's report)
```

Rust, workspace-wide:

```
$ cargo test -p cad-kernel-api    # 6 passed (updated for the new field)
$ cargo test -p cad-occt-bridge   # 11 passed (9 prior + 2 new)
$ cargo test --workspace          # every crate's suite passes
$ cargo fmt --all -- --check      # exit 0
$ cargo clippy --workspace --all-targets --all-features -- -D warnings   # exit 0, zero warnings

$ valgrind --leak-check=full --show-leak-kinds=definite,indirect \
    target/debug/deps/cad_occt_bridge-<hash> --test-threads=1
LEAK SUMMARY:
   definitely lost: 0 bytes in 0 blocks
   indirectly lost: 0 bytes in 0 blocks
     possibly lost: 48 bytes in 1 blocks   # same libtest-harness artifact
                                            # identified and attributed in
                                            # AICAD-018's report; unchanged
   still reachable: 560 bytes in 2 blocks
```

## Artifacts
- `native/occt_bridge/build/abi_smoke_test` (gitignored, reproducible).

## Regressions added
None. All prior tests (AICAD-015/016/017/018) continue to pass unchanged
in behavior; the two crates' internal `KernelHandle`/`AicadShapeHandle`
shape changes were mechanical field additions with every call site
updated in this same commit, not a behavior change to any existing
passing test.

## Limitations
- The free list is unbounded and never shrinks `slots` itself — a
  context that creates and releases many shapes keeps `slots.size()` at
  its historical peak (only `free_indices` grows/shrinks). This is an
  acceptable, simple choice for Stage 1's single real operation
  (`create_box`); revisit if/when a long-running-context resource
  budget becomes an actual requirement (no task or RFC currently asks
  for one).
- `release_shape` is not yet reachable from any AICAD source-level
  construct — it exists only as a native ABI operation and a
  `cad-occt-bridge` Rust method, proven by direct tests. Higher-level
  reachability (e.g. from the low-level `unsafe geometry` block's future
  lowering) is out of Stage 1 Batch 1A's scope.
- This task does not add a `KernelContext`-level "reset" or bulk-release
  operation; the only way to invalidate every handle at once remains
  dropping/destroying the whole `OcctContext`/`AicadOcctContext`
  (AICAD-016/018's existing behavior, unchanged).

## Unresolved questions
None raised by this task. No `AICAD-019` escalation condition was
triggered: no public AICAD language syntax/semantics changed, no OCCT
type crosses any adapter boundary (verified: `Slot`/`TopoDS_Shape` stay
inside `occt_bridge.cpp`; the header still only names `uint32_t`
generation, not any OCCT type), no stage gate/benchmark/test needed
weakening (all prior tests still pass; none was loosened to accommodate
this change), no reference-resolution ambiguity exists at this layer
(this is below the semantic-reference layer entirely), no unresolved
architecture alternative needed selecting (the generational-table design
follows directly from RFC-0002 §5's already-frozen "stale-handle access
is trapped" rule), and this task did not expand scope into Batch 1B's
constructive-geometry operations.

## Batch 1A status
AICAD-015 through AICAD-019 are now all complete (see
`project/reports/AICAD-015.md` through this report). Per
`project/CURRENT_STAGE.md`'s roadmap window, the next step is preparing
`project/gates/STAGE1-A_KERNEL_BOUNDARY.md` before Batch 1B
(AICAD-020..024) begins — see that gate file for the consolidated
checkpoint evidence.
