# AICAD-016 — Create native/occt_bridge C ABI boundary

## Objective
Create the C ABI boundary in `native/occt_bridge`: the extern-"C" surface
`crates/cad-occt-bridge` (AICAD-018) will bind against, implementing the
kernel-neutral/exception-containment/opaque-handle guarantees frozen in
RFC-0002 §3-4 and the active scheduled-task brief's Stage-1 kernel
policies (#2-3 no OCCT type/raw pointer crosses the boundary; #5 stale
handles never alias new geometry; #6-7 no exception crosses the boundary,
normalize failures into status codes; #8 no STL/OCCT-owned object crosses
the boundary directly; #9 contexts are conservatively single-thread-affine;
#10 handles are epoch/build-local).

Per `project/TASKS.yaml`, this task depends on AICAD-015 (OCCT discovery)
and is depended on by AICAD-017 (kernel-api Rust types) and AICAD-018
(safe Rust wrapper).

## Dependencies checked
AICAD-015 (OCCT discovery/probe) — complete, see
`project/reports/AICAD-015.md`. `native/occt_bridge/CMakeLists.txt`
already discovers OCCT and verifies the module libraries this task's
implementation links against.

## What was done

1. **`native/occt_bridge/include/aicad_occt_bridge.h`** (new): the entire
   public ABI. Defines:
   - `aicad_occt_status_t`: an 8-value status enum (`OK`,
     `ERR_INVALID_ARGUMENT`, `ERR_INVALID_HANDLE`, `ERR_STALE_HANDLE`,
     `ERR_FOREIGN_CONTEXT`, `ERR_WRONG_THREAD`, `ERR_OPERATION_FAILED`,
     `ERR_INTERNAL`) returned by every function — no OCCT-specific status
     value anywhere in it.
   - `aicad_shape_handle_t { uint64_t context_id; uint32_t slot; uint32_t
     generation; }`: a plain-old-data value, never a raw OCCT/C++
     pointer, addressing a shape within one context's shape table.
   - `aicad_occt_context_t`: an opaque (incomplete) struct; only
     `aicad_occt_context_create`/`_destroy` construct/destroy it.
   - `aicad_occt_create_box`, `aicad_occt_release_shape`,
     `aicad_occt_shape_is_valid`, `aicad_occt_shape_volume`: the Stage-1
     starting operation (per `native/occt_bridge/README.md`'s list) plus
     the minimum query surface needed to prove it actually produced valid
     geometry, not just a handle.
2. **`native/occt_bridge/src/aicad_occt_bridge.cpp`** (new): the only
   translation unit that includes OCCT headers or holds OCCT-typed state.
   Implements:
   - `ShapeTable`, an internal generation-counted slot table
     (`std::vector<ShapeSlot>` + a free-list). `Insert` reuses a freed
     slot when available and always returns the slot's *current*
     generation; `Release` bumps the slot's generation before returning
     it to the free list, so a handle value captured before release can
     never resolve to whatever shape later reuses that slot. `Lookup`/
     `Release` reject an out-of-range slot as `ERR_INVALID_HANDLE` and a
     generation mismatch (released-or-superseded) as `ERR_STALE_HANDLE`
     — these are deliberately different codes so a caller/agent can tell
     "this handle never existed" from "this handle existed but is now
     stale."
   - `aicad_occt_context`: holds a process-unique `id` (an
     `std::atomic<uint64_t>` counter, 0 reserved/never assigned) and the
     `std::thread::id` that created it. Every context-taking function
     calls `CheckContext` (null-context and wrong-thread checks) and, for
     handle-taking functions, `CheckHandleContext` (the handle's
     `context_id` must match this context's `id`) before touching the
     shape table at all — a handle from a different context can never
     even reach `ShapeTable::Lookup` with a slot index that happens to
     collide with a real local slot.
   - Every function body is wrapped in `try { ... } catch (const
     Standard_Failure&) { return ERR_OPERATION_FAILED; } catch (...) {
     return ERR_INTERNAL; }` (or just the `catch (...)` where no OCCT call
     is reachable, e.g. `context_create`) — no exception can propagate out
     of an `extern "C"` function.
   - `create_box` additionally rejects non-positive or NaN dimensions
     itself (a `> 0.0` comparison is false for NaN) before ever calling
     OCCT, returning `ERR_INVALID_ARGUMENT`.
3. **`native/occt_bridge/tests/abi_boundary_test.cpp`** (new): includes
   *only* `aicad_occt_bridge.h` (exactly as `crates/cad-occt-bridge` will)
   and exercises, with plain pass/fail assertions (no test-framework
   dependency introduced): the happy path (create_box -> is_valid ->
   volume); invalid-argument rejection (zero/negative dimension, null
   out-param); an out-of-range slot (`ERR_INVALID_HANDLE`); release then
   reuse of a stale handle (`ERR_STALE_HANDLE`), including double-release;
   **the specific non-aliasing case** — release a handle, create a new
   shape that reuses the same slot, and confirm the *old* handle value
   still returns `ERR_STALE_HANDLE` (not the new shape's data) while the
   *new* handle correctly resolves to the new shape; a handle from one
   context used against a second context (`ERR_FOREIGN_CONTEXT`); and a
   context used from a thread other than its creator (`ERR_WRONG_THREAD`,
   via `std::thread`).
4. **`native/occt_bridge/CMakeLists.txt`**: added `find_package(Threads
   REQUIRED)`, the `aicad_occt_bridge` shared library target (public
   include dir = `include/`, private OCCT include/link dirs, linked
   against the same `OCCT_PROBE_LIBS` module set AICAD-015 already
   verified), and the `abi_boundary_test` executable (linked only against
   `aicad_occt_bridge` + `Threads::Threads`), registered as a second
   CTest test alongside `occt_probe`.
5. **`native/occt_bridge/README.md`**: documented the ABI's guarantees and
   build/test commands, and narrowed the "Stage 1 CMakeLists.txt" phrasing
   left over from AICAD-015 (it no longer builds only a probe).

## Implementation decisions (autonomous, reversible)
- **Handle shape chosen up front as `{context_id, slot, generation}`**
  rather than a bare index, even though AICAD-019 ("kernel context
  lifecycle and shape-handle table") is the next task. A generation
  counter is required to satisfy Stage-1 kernel policy #5 for even the
  single `create_box`/`release_shape` pair this task adds — there was no
  materially simpler correct version to build first and harden later.
  AICAD-019 is expected to focus on lifecycle correctness across the full
  native+Rust stack (multi-context integration tests once
  `cad-occt-bridge` exists, and epoch-bumping behavior once a
  topology-*mutating* operation, e.g. boolean cut in Batch 1B, exists to
  test against) rather than redoing this table.
- **`ERR_INVALID_HANDLE` vs. `ERR_STALE_HANDLE` as distinct codes.** The
  task brief and RFC-0002 §4 both treat "never existed" and "existed but
  is now stale" as conceptually different failure classes (a stale handle
  is expected/routine after a topology-mutating operation; an invalid
  handle usually indicates a caller bug). Collapsing them into one code
  would lose that distinction for diagnostics built on top of this layer
  later (RFC-0005).
- **`ERR_WRONG_THREAD` as an explicit status rather than an assertion/
  crash.** Per Stage-1 kernel policy #7 ("normalize native failures into
  explicit structured status/error results"), a thread-affinity violation
  is treated the same way as any other misuse — reported, not fatal —
  even though it is also arguably a caller bug.
- **No test-framework dependency added.** `abi_boundary_test.cpp` uses
  plain `Check(condition, description)` + process exit code, registered
  as a CTest test — sufficient for this task's scope and consistent with
  `AGENTS.md`'s "smallest correct solution"; introducing a native test
  framework (e.g. GoogleTest) is a larger, reusable-elsewhere decision
  better made once more native test suites exist to justify it.
- **`OCC_CATCH_SIGNALS` (OCCT's hardware-signal-to-C++-exception
  conversion) was not enabled.** This task's exception containment covers
  ordinary C++ exceptions (`Standard_Failure` and `std::exception`/`...`),
  which is what "no exception crosses the ABI" requires; converting
  hardware faults (segfault, FPE) into catchable exceptions is a deeper
  native-crash-hardening concern (the scheduled-task brief's "Native
  crash/hang policy" section) better addressed deliberately, with its own
  adversarial test cases, in a dedicated hardening pass rather than as a
  side effect of this task. Recorded as a limitation below, not silently
  assumed equivalent.

## Files changed
- Added: `native/occt_bridge/include/aicad_occt_bridge.h`
- Added: `native/occt_bridge/src/aicad_occt_bridge.cpp`
- Added: `native/occt_bridge/tests/abi_boundary_test.cpp`
- Edited: `native/occt_bridge/CMakeLists.txt`
- Edited: `native/occt_bridge/README.md`

## Verification (exact commands/results)

Environment: unchanged from `project/reports/AICAD-015.md` (Ubuntu
24.04.4 LTS, x86_64, GCC/G++ 13.3.0, CMake 3.28.3, OCCT 7.6.3, Rust
1.98.1) — re-confirmed by this task's own build below.

```
$ cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3
...
-- Found Threads: TRUE
-- Configuring done (0.4s)

$ cmake --build native/occt_bridge/build
[ 16%] Building CXX object CMakeFiles/occt_probe.dir/probe/occt_probe.cpp.o
[ 33%] Linking CXX executable occt_probe
[ 50%] Building CXX object CMakeFiles/aicad_occt_bridge.dir/src/aicad_occt_bridge.cpp.o
[ 66%] Linking CXX shared library libaicad_occt_bridge.so
[ 83%] Building CXX object CMakeFiles/abi_boundary_test.dir/tests/abi_boundary_test.cpp.o
[100%] Linking CXX executable abi_boundary_test
[100%] Built target abi_boundary_test

$ ctest --test-dir native/occt_bridge/build --output-on-failure
Test project .../native/occt_bridge/build
    Start 1: occt_probe
1/2 Test #1: occt_probe .......................   Passed    0.01 sec
    Start 2: abi_boundary_test
2/2 Test #2: abi_boundary_test ................   Passed    0.01 sec
100% tests passed, 0 tests failed out of 2

$ ./native/occt_bridge/build/abi_boundary_test
PASS: context_create succeeds
PASS: create_box succeeds for positive dimensions
PASS: is_valid reports the box as valid
PASS: shape_volume succeeds for a valid handle
PASS: box volume is exactly 6.0 within tolerance
PASS: create_box rejects a zero dimension
PASS: create_box rejects a negative dimension
PASS: context_create rejects a null out-param
PASS: an out-of-range slot is rejected as INVALID_HANDLE
PASS: release_shape succeeds once
PASS: using a released handle is rejected as STALE_HANDLE
PASS: releasing an already-released handle is rejected as STALE_HANDLE, not a double-free
PASS: create_box succeeds and is expected to reuse the just-freed slot
PASS: test setup: the new box did reuse the released slot (otherwise this case is not exercised)
PASS: the reused slot's new handle has a different generation than the stale one
PASS: the OLD (pre-release) handle value still does not resolve to the NEW shape in the same slot
PASS: the NEW handle correctly resolves to the new 5x5x5 box (volume 125)
PASS: second context_create succeeds
PASS: a handle from context A used against context B is rejected as FOREIGN_CONTEXT
PASS: second context_destroy succeeds
PASS: calling a context from a thread other than its creator is rejected as WRONG_THREAD
PASS: context_destroy succeeds
abi_boundary_test: all checks PASSED
(exit 0)
```

Compiler-warning check (not wired into CMakeLists.txt permanently, run
directly to confirm the new translation unit is clean under a stricter
flag set than CMake's default):
```
$ g++ -std=c++17 -Wall -Wextra -Wpedantic -Iinclude -isystem /usr/include/opencascade \
    -c src/aicad_occt_bridge.cpp -o /tmp/test_warn.o
(no output — zero warnings)
```

Required checks per task ticket:
```
$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s
(exit 0 — no Rust source added/changed by this task)
```

## Regressions added
None. `occt_probe` (AICAD-015) still builds and passes unchanged;
`abi_boundary_test` is new and additive.

## Limitations / follow-up
- **Hardware-signal containment (`OCC_CATCH_SIGNALS`) is not yet
  implemented** — see "Implementation decisions" above. A future
  hardening task (per the scheduled-task brief's native crash/hang
  policy, or Stage-1 Batch 1C/hardening mode) should decide whether and
  how to convert OCCT-triggered hardware faults into containable failures
  at this boundary, with its own adversarial fixtures.
- **No adversarial trigger for `ERR_INTERNAL`/`ERR_OPERATION_FAILED` via a
  genuine OCCT-thrown `Standard_Failure` exists yet** — every current
  code path either succeeds or is rejected by this bridge's own
  pre-validation (e.g. non-positive dimensions) before reaching OCCT, so
  the `catch (const Standard_Failure&)` branches are exercised by code
  review, not yet by a test that provably reaches them. This should be
  revisited once a later Stage-1 operation (e.g. a boolean op or fillet
  with a genuinely-degenerate-to-OCCT input) provides a natural, honest
  way to trigger it, rather than fabricating an artificial internal-state
  corruption just to hit the branch.
- Only `create_box` is implemented; the rest of
  `native/occt_bridge/README.md`'s Stage-1 operation list is added
  incrementally by Batch 1B (AICAD-020..024) and later, per RFC-0002 §3.
- Next task: AICAD-017 (create `cad-kernel-api` backend-independent
  handle/error Rust types), which is pure Rust and does not yet bind
  against this C ABI (AICAD-018 does that).

No escalation condition was triggered: this task implements the
kernel-boundary mechanics already frozen by DL-5/DL-6 and RFC-0002 §3-4;
it introduces no new public AICAD language syntax/semantics, does not
weaken any gate/benchmark/test, does not let an OCCT type leak above
`native/occt_bridge` (the header is OCCT-type-free), and does not touch
any open `OWNER_DECISIONS.md` item.
