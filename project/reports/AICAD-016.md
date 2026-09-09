# AICAD-016 — Create native/occt_bridge C ABI boundary

## Objective

Implement the C ABI boundary itself: opaque context/handle types,
exception containment, and structured status results, proving the
Stage-1 kernel policies at the native layer before any Rust code exists
to consume it (that is AICAD-017/018). Per `project/TASKS.yaml`
(AICAD-016, Stage 1 / Batch 1A).

## Dependencies checked

AICAD-015 (OCCT discovery/probe) — complete,
`project/reports/AICAD-015.md`; OCCT is discoverable and linkable in this
environment.

## What was done

1. Added `native/occt_bridge/include/aicad_occt_bridge.h` — the ABI
   contract:
   - `AicadKernelContext` is an opaque, forward-declared struct; only
     ever used through a pointer.
   - `AicadShapeHandle` is a POD struct of three integers
     (`context_id`, `slot`, `generation`) — never a pointer into
     kernel-owned memory (Stage-1 policy #3).
   - `AicadStatus` is a structured result code
     (`OK`/`INVALID_ARGUMENT`/`INVALID_HANDLE`/`STALE_HANDLE`/
     `FOREIGN_CONTEXT`/`NATIVE_EXCEPTION`/`UNKNOWN_ERROR`); every
     function returns one instead of throwing (policy #7).
   - Five functions: `aicad_context_create`/`_destroy`, and one
     representative constructive operation + one representative query +
     one release (`aicad_create_box`, `aicad_shape_volume`,
     `aicad_shape_destroy`) — enough to exercise the full handle
     lifecycle end to end. The rest of the operation list in
     `native/occt_bridge/README.md` is deliberately deferred to Batch
     1B/1C, per that README's own "add capabilities only as required."
   - No OCCT/C++ type appears in this header (checked: only `stdint.h`
     is included).
2. Added `native/occt_bridge/src/aicad_occt_bridge.cpp` — the
   implementation:
   - `AicadKernelContext` is defined here as an opaque-to-callers struct
     holding a monotonic `context_id` (from a global
     `std::atomic<uint64_t>` counter, so two live contexts never share
     an id) and a slot table (`std::vector<Slot>` + a free list) mapping
     slot index -> `TopoDS_Shape` + generation + occupied flag.
   - `resolve()` centralizes handle validation: rejects a
     foreign-context handle before even inspecting the slot table,
     then an out-of-range slot, then an unoccupied-or-generation-
     mismatched slot — the three explicit rejection reasons the Batch
     1A checkpoint requires evidence for.
   - `store_shape()` implements slot reuse: freeing a slot
     (`aicad_shape_destroy`) bumps its generation immediately, so a
     handle issued before the free is rejected as stale even after the
     slot is reused for new geometry — no aliasing (Stage-1 policy #5).
   - Every `extern "C"` function body is wrapped in
     `try { ... } catch (const Standard_Failure&) { ... } catch (const
     std::exception&) { ... } catch (...) { ... }`, each branch
     returning a status code rather than propagating (policy #6).
3. Added `native/occt_bridge/tests/bridge_abi_tests.cpp` — a native
   CTest executable that exercises, directly against the ABI: a normal
   create/query/destroy round-trip (volume of a 2x3x4 box is 24, and of
   a reused-slot 5x5x5 box is 125); an invalid-argument rejection
   (non-positive dimension, null context); an invalid-handle rejection
   (out-of-range slot); a foreign-context rejection (a handle from
   `ctx1` presented to `ctx2`); a stale-handle rejection (querying a
   just-destroyed handle); the no-aliasing property (the pre-destroy
   handle still does not resolve after its slot is reused by a new
   shape); and a double-destroy rejection. 21 checks, all passing.
4. Extended `native/occt_bridge/CMakeLists.txt`: builds
   `aicad_occt_bridge` as a static library (so the future Rust wrapper,
   AICAD-018, can link it directly with no separate shared object to
   install/rpath), and `bridge_abi_tests` as a CTest-registered
   executable.
5. Updated `.github/workflows/ci.yml`'s `native-build-smoke` job to
   `apt-get install` the OCCT dev packages before configuring/building
   (the runner does not have them by default, unlike this sandbox), and
   to run `ctest` after the build so the new ABI tests are actually
   exercised in CI, not just compiled. This was necessary now, not
   deferred: AICAD-015 already made this job attempt a real
   `cmake`/`OpenCASCADE` configure on every push, which would fail on
   `ubuntu-latest` without this fix.

## Additional verification beyond the CTest suite

To confirm exception containment against a real OCCT-thrown exception
(not only this task's own argument-validation checks), a temporary probe
was built and run outside the CMake tree, requesting a box with
`1e-300` extents — small enough to drive OCCT's internal
`BRepPrimAPI_MakeBox`/tolerance logic into its failure path:

```
$ g++ -std=c++17 -I include /tmp/exc_probe.cpp -Lbuild -laicad_occt_bridge \
    -lTKBRep -lTKPrim -lTKGeomAlgo -lTKTopAlgo -lTKG3d -lTKG2d -lTKGeomBase \
    -lTKMath -lTKernel -o /tmp/exc_probe
$ /tmp/exc_probe
status=5
$ echo $?
0
```

`status=5` is `AICAD_STATUS_NATIVE_EXCEPTION`; the process exited 0 (no
crash, no uncaught exception). This is the specific evidence the
Batch-1A checkpoint (`project/gates/STAGE1-A_KERNEL_BOUNDARY.md`) needs
for "exception containment" beyond argument-validation-only paths. The
temporary probe file and binary were deleted after use — nothing from
this step is committed.

## Files changed

- Added: `native/occt_bridge/include/aicad_occt_bridge.h`
- Added: `native/occt_bridge/src/aicad_occt_bridge.cpp`
- Added: `native/occt_bridge/tests/bridge_abi_tests.cpp`
- Modified: `native/occt_bridge/CMakeLists.txt`
- Modified: `.github/workflows/ci.yml`

## Implementation decisions

- Chose a monotonic global `context_id` counter (not e.g. the context's
  memory address) specifically so `AicadShapeHandle` never leaks or
  implies a pointer value, and so context ids stay unique even across a
  create/destroy/create cycle in the same process.
- Chose to bump a slot's generation at free time (not at reuse time) —
  behaviorally equivalent for correctness here, but means an already-
  freed-but-not-yet-reused slot is immediately distinguishable from its
  pre-free state without relying on the separate `occupied` flag alone;
  `occupied` still gates it as belt-and-suspenders.
- Context lifetime itself (`ctx` pointer validity across
  `aicad_context_destroy`) is the caller's responsibility, same as any
  opaque-handle C API (e.g. `FILE*`) — this ABI protects *handle*
  identity within/across live contexts, not use of an already-destroyed
  context pointer. Documented explicitly in the header's doc comment
  rather than left implicit.
- Linked `aicad_occt_bridge` against the specific OCCT libraries
  actually required by the two operations implemented
  (`TKBRep TKPrim TKGeomAlgo TKTopAlgo TKG3d TKG2d TKGeomBase TKMath
  TKernel`) rather than the full `OpenCASCADE_LIBRARIES` list, so a
  future missing-library error surfaces immediately as new operations
  are added in Batch 1B/1C instead of being masked by an
  already-linked superset.

## Verification (exact commands/results)

```
$ cd native/occt_bridge && cmake -S . -B build -DCMAKE_BUILD_TYPE=RelWithDebInfo
-- AICAD-015 OCCT probe: OpenCASCADE 7.6.3
-- Configuring done
-- Generating done

$ cmake --build build -j"$(nproc)"
[ 66%] Built target aicad_occt_bridge
[100%] Built target bridge_abi_tests
(plus the pre-existing occt_discovery_probe target; only benign
deprecation warnings from OCCT's own headers, not AICAD source)

$ cd build && ctest --output-on-failure
1/2 Test #1: occt_discovery_probe .............   Passed
2/2 Test #2: bridge_abi_tests .................   Passed
100% tests passed, 0 tests failed out of 2
```

`bridge_abi_tests` output (all 21 checks passed; see test source for the
full list): context create/destroy, box create + volume query (24 and
125), non-positive-dimension and null-context rejection, out-of-range
slot rejection (`INVALID_HANDLE`), foreign-context rejection
(`FOREIGN_CONTEXT`), stale-handle rejection after destroy
(`STALE_HANDLE`), no-aliasing after slot reuse, and double-destroy
rejection.

Required Rust-workspace checks (unaffected by this native-only change,
re-run fresh):

```
$ cargo fmt --all -- --check          # exit 0
$ cargo clippy --workspace --all-targets --all-features -- -D warnings   # exit 0
$ cargo build --workspace --all-targets   # exit 0
$ cargo test --workspace                  # test result: ok. 0 passed; 0 failed
```

CI YAML re-validated: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"`
→ `YAML OK`.

## Artifacts

`native/occt_bridge/build/` (git-ignored) deleted after verification.
Committed artifacts are the four source/build files listed above.

## Regressions added

`bridge_abi_tests` (21 checks) is a new permanent CTest regression
covering the full handle-safety contract. `occt_discovery_probe` from
AICAD-015 continues to run alongside it.

## Limitations

- Only two real operations exist (`create_box`, `shape_volume`) plus
  `destroy`; the full bridge operation catalog
  (`native/occt_bridge/README.md`) is Batch 1B/1C work.
- No Rust code yet calls this library; `crates/cad-occt-bridge` remains
  the AICAD-002 placeholder until AICAD-018.
- `AicadKernelContext` has no internal synchronization — concurrent use
  from multiple threads is undefined behavior by design (Stage-1 policy
  #9: treat conservatively as single-thread-affine). Not tested here
  (there is nothing yet to race against); revisit only if an approved
  architecture decision strengthens this guarantee.

## Unresolved questions

None. No escalation condition in `project/TASKS.yaml` (AICAD-016) was
triggered.
