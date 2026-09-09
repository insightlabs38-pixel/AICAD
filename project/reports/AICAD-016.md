# AICAD-016 — Create native/occt_bridge C ABI boundary

## Objective

Create the C ABI boundary in `native/occt_bridge` that `crates/cad-occt-bridge`
(AICAD-018) will be the *only* crate allowed to call into, per
`project/TASKS.yaml` (AICAD-016), `docs/plan/01_SYSTEM_ARCHITECTURE.md`
§2.6/§4, and `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-01. This task
establishes the ABI mechanics required by Stage-1 kernel policies #2-8
(no OCCT type escapes; no raw OCCT pointer as resource identity; opaque
handles; released/stale handles never alias new geometry; no exception
crosses the boundary; native failures normalized to structured
status/error results; no STL/std::string/ownership-ambiguous C++ object
crosses the boundary) — not the full geometry catalog, and not the final
handle-table hardening, which is AICAD-019's explicit scope.

## Dependencies checked

AICAD-015 (OCCT discovery/probe) — complete
(`project/reports/AICAD-015.md`); this task builds directly on its CMake
scaffolding (`native/occt_bridge/CMakeLists.txt`, the root
`CMakeLists.txt`) rather than duplicating it.

## What was done

### ABI header — `native/occt_bridge/include/aicad_occt_bridge.h`

Defines, all `extern "C"`:

- `AicadKernelContext` — an incomplete (opaque) struct type; callers only
  ever hold a pointer to it.
- `AicadShapeHandle` — a POD struct wrapping a single `uint64_t id`; no
  pointer arithmetic or OCCT-derived meaning is exposed. Ids are minted
  from **one process-wide monotonic counter shared by every context**
  (not reset per context, not a per-context free list) — deliberately, so
  that:
  - a released handle can never be reissued and accidentally alias a
    newly created shape (Stage-1 kernel policy #5);
  - a handle minted by context A is guaranteed to never coincide with an
    id context B has ever seen, so passing A's handle to B's functions is
    rejected as unknown rather than risking silent aliasing (Stage-1A
    gate requirement: "foreign-context handles rejected").
- `AicadStatus` — a structured status enum (`OK`, `NULL_CONTEXT`,
  `INVALID_ARGUMENT`, `INVALID_HANDLE`, `KERNEL_INTERNAL_ERROR`,
  `UNKNOWN_ERROR`); every function returns one of these instead of
  throwing.
- `aicad_kernel_context_create` / `_destroy` — context lifecycle. (Full
  multi-context/epoch lifecycle semantics remain AICAD-019's job; this
  task provides the create/destroy primitives AICAD-019 builds on.)
- `aicad_kernel_context_last_error` — returns a `const char*` owned by the
  context (valid until the next call on that context or its destruction),
  never a `std::string` or other container crossing the ABI.
- `aicad_create_box` / `aicad_shape_bbox_diagonal` / `aicad_shape_release`
  — one minimal real geometry round-trip, included only to prove the ABI
  mechanics end-to-end with a genuine OCCT operation rather than a bare
  create/destroy no-op. The full bridge catalog
  (`docs/plan/01_SYSTEM_ARCHITECTURE.md` §4's `create_box .. export_step`
  list) is added incrementally starting at AICAD-020, per that plan
  section's own "add capabilities only as required" rule.

### Implementation — `native/occt_bridge/src/bridge.cpp`

- `AicadKernelContext` is defined here (not in the public header) as
  `{ std::unordered_map<uint64_t, TopoDS_Shape> shapes; std::string
  last_error; }` — an OCCT-owned `TopoDS_Shape` and an STL container/string
  live inside the context, but neither ever crosses the ABI directly,
  satisfying Stage-1 kernel policy #8.
- Every `extern "C"` function does its own null/argument checks first
  (returning a structured status with no OCCT call at all for those
  cases), then routes any actual OCCT work through a `guard()` helper
  template that catches, in order: `Standard_Failure` (OCCT's exception
  root, recording `DynamicType()->Name()` + `GetMessageString()`),
  `std::exception`, and `...` (anything else) — nothing is allowed to
  propagate out of an `extern "C"` function. This directly implements
  Stage-1 kernel policy #6/#7.
- `aicad_kernel_context_create` itself is wrapped in `try { new ... }
  catch (...) { return nullptr; }` since allocation failure is the one
  thing that can go wrong before a context (and thus a place to record
  `last_error`) exists.

### Tests

- `native/occt_bridge/tests/abi_smoke_test.cpp` (C++): the primary
  behavioral test, covering (see "Verification" for the pass list):
  ordinary create/query/release round-trip; a never-issued handle id (0)
  rejected; a released (stale) handle rejected rather than aliased;
  double-release rejected; a freshly minted handle never reuses a
  just-released id; a handle minted on context A rejected by context B
  (foreign-context); context B's own handle ids never collide with
  context A's; a null-context call rejected without crashing; a
  degenerate zero-dimension box (independently confirmed below to throw
  inside OCCT) contained as a structured error rather than crashing or
  propagating; context destruction with outstanding shapes still owned
  does not crash.
- `native/occt_bridge/tests/abi_c_linkage_test.c` (plain C): a smaller,
  separate test proving the header is genuinely consumable from a C
  translation unit (not merely "compiles under `extern "C"` but was only
  ever built as C++") — relevant because AICAD-018's Rust FFI binding will
  link against this ABI the same way a C caller does, through un-mangled
  symbol names, not C++ name-mangled ones.
- Both registered via CTest (`add_test`); `enable_testing()` was added to
  the **root** `CMakeLists.txt` (not only the subdirectory one) because
  CMake only writes a root `CTestTestfile.cmake` — needed for `ctest` run
  from the natural root build directory — if `enable_testing()` was seen
  at or before the directory that adds the test-registering subdirectory;
  it remains in `native/occt_bridge/CMakeLists.txt` too since that file is
  designed (per AICAD-015's own comment) to be configurable standalone.

## Debugging notes (native, per AGENTS.md's crash/hang and evidence rules)

1. **`Bnd_Box`'s default gap silently pads bounding-box queries.**
   `BRepBndLib::Add` leaves a small default gap (empirically measured:
   `~9.9999999999999995e-08`) on the `Bnd_Box`, added symmetrically to
   every side by `Get()`. A 2x3x6 box's exact diagonal is `sqrt(4+9+36) =
   7` exactly (IEEE-754 `sqrt` of a perfect square is exact), but the
   padded box measured `dx≈2.0000002`, giving diagonal `≈7.0000003142857`
   — a `~3.14e-7` error, large enough to fail a `1e-9` tolerance (used in
   this task's tests) though small enough to have silently passed
   AICAD-015's looser `1e-6` tolerance. Reproduced and isolated with a
   standalone experiment before touching either file (see commands
   below). **Fix**: call `bounds.SetGap(0.0)` — but only *after*
   `BRepBndLib::Add`, not before: empirically, setting the gap before
   `Add` has no effect (`Add` sets its own gap internally), while setting
   it after and before `Get()` zeroes the padding exactly (verified:
   diagonal became exactly `7.0`, `diff=0`). Applied to both
   `bridge.cpp`'s `aicad_shape_bbox_diagonal` (this task) and, as a
   correctness follow-up, `occt_discovery_probe.cpp` (AICAD-015, edited in
   this task's commit — its result was already a PASS under the looser
   tolerance, so this is a precision correction, not a report retraction;
   its tolerance was also tightened from `1e-6` to `1e-9` to match).
2. **`BRepPrimAPI_MakeBox` throws `Standard_DomainError` for a
   zero-dimension box**, but does *not* throw for a negative-dimension box
   (`IsDone()==true`, non-null shape — OCCT evidently treats a negative
   extent as a valid, differently-oriented box rather than an error).
   Verified with a standalone experiment before writing `aicad_create_box`'s
   exception-containment logic, so the adversarial test case in
   `abi_smoke_test.cpp` (zero dimension) is chosen because it is
   independently confirmed to actually exercise the catch path, not
   assumed to.

## Implementation decisions (reversible/local; within AGENTS.md's
autonomously-allowed scope)

- Chose a process-wide atomic `uint64_t` counter over a per-context
  counter for handle-id minting specifically to make the
  aliasing/foreign-context-rejection properties true by construction
  rather than by incidental table design — this is a direct, minimal
  implementation of Stage-1 kernel policy #5 and the Stage-1A gate's
  foreign-context-handle-rejection requirement, not a new architecture
  decision (it does not touch any public AICAD syntax/semantics or an
  open `OWNER_DECISIONS.md` item).
- Kept the handle registry as a single `std::unordered_map` with no
  epoch/generation field yet. This already gets every *observable*
  safety property the Stage-1A gate lists (invalid/stale/foreign-context
  rejection — see the smoke test) because ids are never reused
  process-wide, but AICAD-019 owns turning this into the real
  epoch-based table the plan describes (`docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`
  §5's "raw handle belongs to a geometry epoch" rule) — e.g. bulk
  invalidation of a whole epoch's handles after a topology-changing
  operation, which a flat map cannot express cheaply. Recorded here so
  AICAD-019 does not mistake this task's registry for the final design.
- `STATIC` library (`aicad_occt_bridge`), not `SHARED` — this is linked
  directly into the one crate (`cad-occt-bridge`, AICAD-018) allowed to
  use it; no runtime dynamic-loading requirement exists yet that would
  justify a shared object and its own export-visibility surface.

## Files changed

- Added: `native/occt_bridge/include/aicad_occt_bridge.h`.
- Added: `native/occt_bridge/src/bridge.cpp`.
- Added: `native/occt_bridge/tests/abi_smoke_test.cpp`.
- Added: `native/occt_bridge/tests/abi_c_linkage_test.c`.
- Edited: `native/occt_bridge/CMakeLists.txt` (added the
  `aicad_occt_bridge` library target and both test executables/CTest
  registrations).
- Edited: `CMakeLists.txt` (repo root; added `enable_testing()`).
- Edited: `native/occt_bridge/probe/occt_discovery_probe.cpp` (AICAD-015;
  `Bnd_Box` gap-zeroing fix + tightened tolerance, per debugging note 1
  above).

## Verification (exact commands/results)

```
$ rm -rf build && cmake -S . -B build -DCMAKE_BUILD_TYPE=RelWithDebInfo
...
-- AICAD occt_bridge: OpenCASCADE 7.6.3 found at /usr/lib/x86_64-linux-gnu
-- Configuring done / Generating done

$ cmake --build build -- -j"$(nproc)"
...
[100%] Built target occt_discovery_probe
[100%] Built target aicad_occt_bridge
[100%] Built target abi_c_linkage_test
[100%] Built target abi_smoke_test
(only warning: OCCT's own NCollection_StlIterator.hxx deprecated-std::iterator
warning, from OCCT's headers, not this task's code — same as AICAD-015)

$ cd build && ctest --output-on-failure
Test project /home/user/AICAD/build
    Start 1: abi_smoke_test
1/2 Test #1: abi_smoke_test ...................   Passed    0.01 sec
    Start 2: abi_c_linkage_test
2/2 Test #2: abi_c_linkage_test ...............   Passed    0.01 sec
100% tests passed, 0 tests failed out of 2

$ ./build/native/occt_bridge/abi_smoke_test
PASS: context A created
PASS: create_box(10,20,30) on context A succeeds
PASS: create_box produced a non-zero handle id
PASS: bbox_diagonal on the live handle succeeds
PASS: bbox_diagonal matches the analytic expectation
PASS: handle id 0 (never issued) is rejected
PASS: release of a live handle succeeds
PASS: querying a released (stale) handle is rejected, not aliased
PASS: double-release of the same handle is rejected
PASS: second create_box on context A succeeds
PASS: a new handle never reuses a just-released handle's id
PASS: context B created
PASS: a handle minted on context A is rejected by context B
PASS: create_box on context B succeeds
PASS: context B's handle id never collides with context A's handle ids
PASS: create_box(nullptr, ...) is rejected, not a crash
PASS: a degenerate zero-dimension box is reported as a contained kernel error, not a crash
PASS: last_error is populated after the degenerate-box failure
INFO: degenerate-box last_error = "aicad_create_box: OCCT exception (Standard_DomainError): "
PASS: destroying contexts with outstanding shapes did not crash
ABI_SMOKE_TEST_RESULT=PASS

$ ./build/native/occt_bridge/abi_c_linkage_test
C_LINKAGE_TEST_RESULT=PASS
```

### AddressSanitizer + UndefinedBehaviorSanitizer run (extra rigor; not yet standing CI, see limitations)

```
$ rm -rf build-asan
$ cmake -S . -B build-asan -DCMAKE_BUILD_TYPE=Debug \
    -DCMAKE_CXX_FLAGS="-fsanitize=address,undefined -g -O0" \
    -DCMAKE_C_FLAGS="-fsanitize=address,undefined -g -O0" \
    -DCMAKE_EXE_LINKER_FLAGS="-fsanitize=address,undefined"
$ cmake --build build-asan --target abi_smoke_test abi_c_linkage_test -- -j"$(nproc)"
$ ./build-asan/native/occt_bridge/abi_smoke_test; echo $?
... (all PASS) ...
ABI_SMOKE_TEST_RESULT=PASS
0
$ ./build-asan/native/occt_bridge/abi_c_linkage_test; echo $?
C_LINKAGE_TEST_RESULT=PASS
0
```
Clean under both sanitizers — no memory-safety or undefined-behavior
report, including through the exception-containment paths and the
context-destroy-with-outstanding-shapes case.

### Reproduction of the two debugging-note findings (isolated before touching source)

```
$ g++ -std=c++17 -I/usr/include/opencascade t.cpp -o t -L/usr/lib/x86_64-linux-gnu \
    -lTKernel -lTKMath -lTKBRep -lTKG3d -lTKGeomBase -lTKTopAlgo -lTKPrim
$ ./t
negative dims: IsDone=1 IsNull=0
zero dim: THREW Standard_Failure:  (dynamic type: Standard_DomainError)

$ ./t4   # Bnd_Box gap experiment
gap before reset=9.9999999999999995e-08
gap after reset=0
diag=7 expected=7 diff=0.000e+00
```

### Rust workspace unaffected (required checks still pass)

```
$ cargo fmt --all -- --check
(exit 0)
$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
```

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS** (no Rust source touched).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**.
- Task-specific check: `ctest` (both native tests) — **PASS**; ASan/UBSan
  run — clean.

## Limitations / follow-up

- The handle registry is intentionally minimal (flat map, no
  epoch/generation field) — see "Implementation decisions" above.
  AICAD-019 ("Implement kernel context lifecycle and shape-handle table")
  owns replacing/extending it with the real epoch-based semantics
  `docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` §5 describes.
- Sanitizer runs (ASan/UBSan) were done once, manually, as extra rigor for
  this task's own confidence — they are not yet wired into a standing
  CI/verification command any task is required to run. Regular
  sanitizer runs are explicitly a Stage-1-hardening-mode activity per the
  operating routine's instructions, once AICAD-037 completes, not a
  Stage-1A requirement.
- Thread-safety is not implemented or tested; per Stage-1 kernel policy
  #9, a context is documented (in the header) as single-thread-affine —
  concurrent use from multiple threads is out of scope until an approved
  architecture decision strengthens this.
- The `guard()` exception-handling helper distinguishes
  `Standard_Failure` from generic `std::exception`/`...`, but does not
  yet map specific OCCT exception subtypes (e.g. `Standard_DomainError`
  vs. `Standard_ConstructionError`) to distinct `AicadStatus` values —
  every OCCT-side exception currently normalizes to the single
  `AICAD_STATUS_KERNEL_INTERNAL_ERROR`. Finer-grained status codes (if
  ever needed) belong with the operations that would actually
  distinguish them (AICAD-020+), not invented speculatively here.
