# AICAD-019 — Implement kernel context lifecycle and shape-handle table

## Objective
Complete Batch 1A's kernel-lifecycle work by extending the generation-
counted shape-handle table and context lifecycle already implemented in
AICAD-016/018 with the evidence this task's title specifically calls
for: lifecycle correctness **at scale** (many create/release cycles, not
just one) and **under real concurrency** (multiple kernel contexts used
from multiple actual OS threads), plus leak/error-freedom evidence
(valgrind) across the full context-create -> many-shape-cycles ->
context-destroy lifecycle. This is also the last task in Batch 1A, so
this report additionally sets up `project/gates/STAGE1-A_KERNEL_BOUNDARY.md`.

## Scope note (why this task does not re-implement the table)
AICAD-016's report and AICAD-018's report both recorded, at the time,
that a correct generation-counted shape table and RAII lifecycle already
existed and passed single-shot tests for every safety property Batch
1A's checkpoint requires (stale/foreign/invalid-handle rejection,
exception containment, no OCCT-type leakage) — building it twice would
not add evidence. Both reports flagged the same expectation: AICAD-019
should focus on lifecycle correctness *across the whole stack, at scale
and under concurrency*, since a topology-*mutating* operation (needed to
exercise "epoch bump on mutation" specifically, as opposed to
"epoch bump on release") does not exist until Batch 1B. This report
follows that plan.

## Dependencies checked
AICAD-018 (`cad-occt-bridge` safe Rust wrapper) — complete, see
`project/reports/AICAD-018.md`.

## What was done

1. **`native/occt_bridge/tests/lifecycle_test.cpp`** (new), registered as
   the `lifecycle_test` CTest test:
   - **Free-list reuse at scale**: 1000 create/release cycles of a single
     shape; asserts the maximum slot index ever observed is 0 (a
     free-list that failed to reuse slots would instead grow to
     `kIterations - 1`).
   - **Generation-counter correctness at scale**: 1000 create/release
     cycles of the same slot; asserts the generation value is strictly
     monotonically increasing on every cycle and that all 1000 observed
     generation values are pairwise distinct (via `std::set`).
   - **Multi-context concurrency**: 8 real `std::thread`s, each creating
     its own kernel context, creating/querying/releasing 100
     distinctly-dimensioned boxes, and destroying its own context — with
     per-thread volume checks against each thread's own expected value
     (a cross-thread aliasing bug would surface as a wrong volume, not
     just a crash). Zero failures across all 800 shape operations.
2. **`crates/cad-occt-bridge/src/lib.rs`**: added
   `many_contexts_are_safe_across_real_threads`, the same concurrency
   property proven at the Rust wrapper layer using `std::thread::scope`
   (8 threads x 50 shapes each, each thread creating its own
   `OcctContext` inside its own closure — `OcctContext` being neither
   `Send` nor `Sync` means the type system would reject any attempt to
   create one thread-side and share/move it elsewhere, which this test's
   structure relies on rather than works around).
3. **`native/occt_bridge/CMakeLists.txt`**: registered the new
   `lifecycle_test` executable/CTest test alongside `occt_probe` and
   `abi_boundary_test`.
4. **`native/occt_bridge/README.md`**: documented the new test file and
   the recommended (not CI-wired — see Limitations) valgrind commands.
5. **Valgrind verification** (see Verification below): both
   `abi_boundary_test` and `lifecycle_test` run clean under
   `valgrind --leak-check=full` — 0 errors, 0 bytes definitely/indirectly/
   possibly lost, across a combined ~693,000 native allocations
   (dominated by `lifecycle_test`'s 1000+1000+800 shape create/release
   cycles). Also ran the Rust `cad-occt-bridge` test binary under
   valgrind; see the one caveat recorded in Verification.
6. **`project/gates/STAGE1-A_KERNEL_BOUNDARY.md`** (new): the Batch 1A
   checkpoint required by the active scheduled-task brief before Batch 1B
   may begin. See that file for its own content; summarized in this
   report's Verification section.

## Implementation decisions (autonomous, reversible)
- **Slot-index/generation assertions as the free-list/generation-correctness
  proof**, rather than adding a new introspection API (e.g. "get current
  table size") to the C ABI. Adding an ABI-level introspection function
  purely to make a test easier would expand the native operation surface
  beyond what RFC-0002 §3's capability-driven minimal-surface rule
  intends for a task that is fundamentally about testing, not about
  adding a new domain operation; the existing `slot`/`generation` fields
  already returned in every handle are sufficient to observe both
  properties from outside.
- **8 threads / 100 (native) or 50 (Rust) shapes per thread** as the
  concurrency test's scale: large enough that a real race condition
  (e.g. a missing lock, if one were ever mistakenly needed — it is not,
  since each context's `ShapeTable` is only ever touched by its own
  single owning thread) would very likely manifest as either a crash or
  a wrong volume within a `ctest`-timeout-friendly runtime (~0.2s for the
  native test), rather than requiring a long-running fuzzing campaign to
  surface. A larger scale is deferred to a dedicated fuzzing/hardening
  pass rather than inflating this task's default `ctest` runtime.
- **Valgrind is documented, not wired into default CI.** Running a heavy
  OCCT/Tcl-linked binary under valgrind is meaningfully slower than the
  native `ctest` suite itself and is exactly the kind of check the
  scheduled-task brief's Stage-1-hardening-mode section calls out
  ("ASan/UBSan or applicable native sanitizer runs") as an unattended-cycle
  activity rather than a per-push CI gate. Recorded as a documented manual
  command (`native/occt_bridge/README.md`) and run once here as this
  task's own evidence, not automated per-commit.

## Files changed
- Added: `native/occt_bridge/tests/lifecycle_test.cpp`
- Edited: `native/occt_bridge/CMakeLists.txt`
- Edited: `native/occt_bridge/README.md`
- Edited: `crates/cad-occt-bridge/src/lib.rs` (added one test)
- Added: `project/gates/STAGE1-A_KERNEL_BOUNDARY.md`

## Verification (exact commands/results)

Environment: unchanged from `project/reports/AICAD-018.md` (Ubuntu
24.04.4 LTS, x86_64, GCC/G++ 13.3.0, CMake 3.28.3, OCCT 7.6.3, Rust
1.98.1). Valgrind: `valgrind-3.22.0` (`valgrind --version`).

```
$ rm -rf native/occt_bridge/build
$ cmake -S native/occt_bridge -B native/occt_bridge/build
... (OCCT discovery output as in AICAD-015/016/018) ...
$ cmake --build native/occt_bridge/build
[100%] Built target lifecycle_test (occt_probe, aicad_occt_bridge, abi_boundary_test also built)

$ ctest --test-dir native/occt_bridge/build --output-on-failure
    Start 1: occt_probe
1/3 Test #1: occt_probe .......................   Passed    0.01 sec
    Start 2: abi_boundary_test
2/3 Test #2: abi_boundary_test ................   Passed    0.02 sec
    Start 3: lifecycle_test
3/3 Test #3: lifecycle_test ...................   Passed    0.17 sec
100% tests passed, 0 tests failed out of 3

$ ./native/occt_bridge/build/lifecycle_test
PASS: lifecycle: context_create succeeds
lifecycle: max slot index seen across 1000 create/release cycles = 0
PASS: lifecycle: free list reuses slot 0 instead of growing the table unboundedly
PASS: lifecycle: context_destroy succeeds
PASS: generation: context_create succeeds
PASS: generation: generation strictly increases across every reuse of the same slot
PASS: generation: every one of the 1000 cycles produced a distinct generation value (no repeats)
PASS: generation: context_destroy succeeds
PASS: concurrency: zero failures across all worker threads
concurrency: 8 threads x 100 shapes/thread completed
lifecycle_test: all checks PASSED
```

Valgrind (both native test binaries):
```
$ valgrind --leak-check=full --error-exitcode=99 ./abi_boundary_test
... all 22 PASS lines ...
HEAP SUMMARY: in use at exit: 16 bytes in 1 blocks; total heap usage: 3,218 allocs, 3,217 frees
LEAK SUMMARY: definitely lost: 0 bytes; indirectly lost: 0 bytes; possibly lost: 0 bytes; still reachable: 16 bytes in 1 blocks
ERROR SUMMARY: 0 errors from 0 contexts
(exit 0)

$ valgrind --leak-check=full --error-exitcode=99 ./lifecycle_test
... all PASS lines ...
HEAP SUMMARY: in use at exit: 16 bytes in 1 blocks; total heap usage: 690,090 allocs, 690,089 frees
LEAK SUMMARY: definitely lost: 0 bytes; indirectly lost: 0 bytes; possibly lost: 0 bytes; still reachable: 16 bytes in 1 blocks
ERROR SUMMARY: 0 errors from 0 contexts
(exit 0)
```
The 16-byte "still reachable" block is identical in both runs and is
consistent with a one-time OCCT/Tcl static-initialization allocation
(reachable at process exit, not a per-shape leak) — not evidence of a
leak in this bridge's own code, whose every `TopoDS_Shape`/context
allocation is demonstrably matched by a free (690,089 frees for 690,090
allocs; the one outstanding allocation is the same 16-byte block present
even before any shape is ever created).

Valgrind on the Rust test binary (informational; the Rust binary is not
itself part of the native ABI's own evidence, since it links the same
already-verified native library through FFI):
```
$ valgrind --leak-check=full --error-exitcode=99 target/debug/deps/cad_occt_bridge-<hash>
LEAK SUMMARY: definitely lost: 0 bytes; indirectly lost: 0 bytes; possibly lost: 48 bytes in 1 blocks; still reachable: 560 bytes in 2 blocks
ERROR SUMMARY: 1 errors from 1 contexts
```
Investigated the one "possibly lost" report: re-ran with a test filter
that matches zero tests (`... nonexistent_test_filter_xyz`) and the
"possibly lost" entry disappeared entirely (0 errors), while the same
560-byte "still reachable" baseline remained. The backtrace for the
possibly-lost allocation is entirely inside Rust's own `libtest` harness
(`std::sync::mpmc::context::Context` thread-local lazy initialization,
used by `libtest`'s own result-channel plumbing when tests actually run)
— it does not pass through any `unsafe`/FFI code this crate owns. Treated
as a known Rust-test-harness-under-valgrind artifact, not a defect in
this crate or the native bridge; not blocking, but recorded here rather
than silently omitted, per the evidence rule.

Required checks per task ticket:
```
$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0)

$ cargo build --workspace --all-targets
(exit 0)

$ cargo test --workspace
(every crate: ok; cad-kernel-api 6/6, cad-occt-bridge now 7/7 including
the new concurrency test, all other crates remain empty placeholders)
```

## Regressions added
None. `occt_probe` and `abi_boundary_test` are unchanged and still pass;
`cad-occt-bridge`'s six pre-existing tests are unchanged and still pass.

## Limitations / follow-up
- Valgrind is a manual/documented check, not wired into default CI (see
  "Implementation decisions"). A future hardening-mode pass may choose to
  add it as a scheduled (not per-push) CI job.
- The concurrency test proves multiple *contexts* are safe across
  threads; it does not and should not attempt to prove that sharing a
  *single* context across threads is safe, since that is explicitly
  outside the single-thread-affine contract (Stage-1 kernel policy #9)
  and is already covered by AICAD-016's `WRONG_THREAD` rejection test.
- Epoch-bumping on a topology-*mutating* operation (as opposed to a
  release) remains untested, because no mutating operation exists until
  Batch 1B (AICAD-020..024). This is a known, explicitly-scoped gap
  (already flagged in AICAD-016/018's own limitations sections), not an
  oversight of this task; whichever Batch 1B task first adds a
  topology-mutating operation should add that specific test then.
- This concludes Batch 1A (AICAD-015 through AICAD-019). See
  `project/gates/STAGE1-A_KERNEL_BOUNDARY.md` for the batch checkpoint
  this task's own evidence, plus AICAD-015/016/017/018's reports, feeds
  into.

No escalation condition was triggered: this task extends already-approved
kernel-boundary mechanics with additional testing evidence; it introduces
no new native operation, no new public AICAD language syntax/semantics,
and does not touch any open `OWNER_DECISIONS.md` item.
