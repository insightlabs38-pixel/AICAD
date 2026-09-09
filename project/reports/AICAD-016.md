# AICAD-016 — Create native/occt_bridge C ABI boundary

## Objective
Create the C ABI boundary in `native/occt_bridge` — the only contract
`crates/cad-occt-bridge` (AICAD-018) will link against — enforcing
`AGENTS.md`'s Stage-1 kernel policies from the very first line of code:
no OCCT type crosses it, no C++/OCCT exception crosses it, resource
identity is an opaque POD handle rather than a raw object pointer, and
native failures are normalized into a structured status result. Per
`project/TASKS.yaml` (AICAD-016).

## Dependencies checked
AICAD-015 (OCCT discovery/probe) — complete, see
`project/reports/AICAD-015.md`; its `CMakeLists.txt` and confirmed
OCCT 7.6.3 discovery are extended, not replaced, by this task.

## Scope decision (recorded per AGENTS.md "record material implementation
choices")
`project/TASKS.yaml`'s Batch 1A splits this work across AICAD-016
("C ABI boundary") through AICAD-019 ("kernel context lifecycle and
shape-handle table") with no finer-grained brief than the titles. This
task implements: the ABI header/types, context create/destroy, exception
containment, and two real operations (`create_box`, `shape_volume`)
end-to-end, with handle validation that already rejects an out-of-range
index and a handle from a different context. It deliberately does **not**
yet add a `release_shape`/generation-counter mechanism for detecting a
*stale* handle (one whose slot was freed and its index reused) — there is
no operation yet that frees a shape, so there is nothing for a generation
counter to guard against, and RFC-0002 §5's "topology mutation may
invalidate handles" scenario does not arise until Batch 1B's real
operations exist. That generational table is left to AICAD-019, which
this report's "Limitations" section restates explicitly so it is not
mistaken for a completed part of this task.

## What was done

1. **`native/occt_bridge/include/aicad/occt_bridge.h`**: the public ABI.
   `extern "C"`, C-only types (`stdint.h`/`stddef.h`, no C++ or OCCT
   headers). Declares:
   - `AicadOcctContext` — opaque, forward-declared only, never defined in
     the header;
   - `AicadStatusCode`/`AicadStatus` — a fixed `char message[256]`
     buffer, never a `std::string`, so no ownership-ambiguous object
     crosses the boundary;
   - `AicadShapeHandle{context_id: uint64_t, index: uint32_t}` — a POD
     value type, copyable, intentionally exposing its own fields (this is
     the raw/unsafe handle model RFC-0002 §5 describes, not an
     information-hiding abstraction);
   - `aicad_occt_context_create`/`_destroy`, `aicad_occt_create_box`,
     `aicad_occt_shape_volume`.
2. **`native/occt_bridge/src/occt_bridge.cpp`**: the only file allowed to
   mix OCCT types with the ABI types.
   - `AicadOcctContext` is defined here as `{ uint64_t id;
     std::vector<TopoDS_Shape> shapes; }` — never exposed past this file.
   - Context ids come from a global `std::atomic<uint64_t>` counter
     starting at **1** (not 0), so a zero-initialized/garbage handle
     (`context_id == 0`) can never spuriously match a real context —
     every such handle is rejected as foreign-context by construction,
     not by a special-cased check.
   - `create_box` validates `dx, dy, dz` are each finite and strictly
     positive *before* calling into OCCT (rejecting zero, negative, and
     NaN dimensions as `AICAD_STATUS_INVALID_ARGUMENT` without ever
     reaching the kernel), reuses AICAD-015's finding (`Build()` must be
     called explicitly on this OCCT version before `IsDone()`/`Shape()`
     are valid), and checks the result is a non-null `TopAbs_SOLID`
     before accepting it.
   - `shape_volume` validates `handle.context_id == ctx->id` (else
     `AICAD_STATUS_FOREIGN_CONTEXT_HANDLE`) and `handle.index <
     ctx->shapes.size()` (else `AICAD_STATUS_INVALID_HANDLE`) before
     touching the stored shape.
   - Every function that calls into OCCT wraps the call in
     `catch (const Standard_Failure&)`, `catch (const std::exception&)`,
     `catch (...)` and converts the result into `AicadStatus`; no
     exception can reach a caller across this boundary.
3. **`native/occt_bridge/tests/abi_smoke_test.cpp`**: exercises the ABI
   exactly as `crates/cad-occt-bridge` will — through the header only,
   never touching OCCT directly. 18 checks, all passing (see
   Verification): round-trip create+query, zero/negative/NaN dimension
   rejection, very-small (`1e-6`) and very-large (`1e6`) per-side boxes
   (AGENTS.md "very small/large scales" adversarial cases) with
   relative-tolerance volume checks, foreign-context handle rejection,
   out-of-range handle rejection, null-context rejection on both
   operations, two independent contexts not interfering with each
   other's shape indices, and `aicad_occt_context_destroy(nullptr)` as a
   safe no-op.
4. **`native/occt_bridge/CMakeLists.txt`**: extended (not replaced) to
   add the `aicad_occt_bridge` static library target and the
   `abi_smoke_test` executable/CTest case, alongside AICAD-015's
   `occt_probe`. Deliberately kept the bridge's OCCT toolkit link list
   (`AICAD_OCCT_BRIDGE_TOOLKITS`) separate from the probe's, since they
   are allowed to diverge as the bridge's real operation set grows.

## Implementation decisions
- Chose a plain incrementing `index` into a per-context
  `std::vector<TopoDS_Shape>` rather than any generational/free-list
  structure for this task, since AICAD-019 owns designing the actual
  shape-handle table (see "Scope decision" above) — this task's job was
  proving the ABI mechanics (types, exception containment, two working
  operations) work correctly, not anticipating 019's data structure.
- `aicad_occt_context_destroy` wraps `delete ctx` in `try`/`catch(...)`
  even though nothing in `AicadOcctContext`'s members is documented to
  throw on destruction, purely so that *no* function in this file can
  ever let an exception cross the ABI, without exception, as a blanket
  rule rather than an OCCT-specific judgment call.
- Kept `AicadShapeHandle`'s fields public/documented rather than
  encoding them opaquely, matching RFC-0002 §5's raw/unsafe-handle model
  ("epoch/build-local", not an information-hiding abstraction) — a
  caller is expected to copy a previously-issued handle back verbatim,
  not construct one from scratch, but the ABI does not need to prevent
  that at the C level; `crates/cad-occt-bridge`'s safe Rust wrapper
  (AICAD-018) is where that discipline becomes enforced, not this task.

## Files changed
- Added: `native/occt_bridge/include/aicad/occt_bridge.h`
- Added: `native/occt_bridge/src/occt_bridge.cpp`
- Added: `native/occt_bridge/tests/abi_smoke_test.cpp`
- Modified: `native/occt_bridge/CMakeLists.txt` (added the
  `aicad_occt_bridge` library and `abi_smoke_test` targets/test)

## Verification (exact commands/results)

Clean, from-scratch build:

```
$ cd native/occt_bridge && rm -rf build && mkdir build && cd build
$ cmake -DCMAKE_BUILD_TYPE=Release ..
-- AICAD: found OpenCASCADE 7.6.3 (install prefix: /usr/lib/x86_64-linux-gnu, include dir: /usr/include/opencascade)
-- Configuring done (0.5s)
-- Generating done (0.0s)

$ cmake --build . -j"$(nproc)"
[ 50%] Linking CXX executable occt_probe
[ 66%] Linking CXX static library libaicad_occt_bridge.a
[100%] Linking CXX executable abi_smoke_test
(only pre-existing OCCT-header deprecation warnings, none from AICAD code)
```

Smoke test, all 18 checks pass:

```
$ ./abi_smoke_test
ok: context 1 created
ok: create_box(10,20,30) on ctx1 succeeds
ok: shape_volume(box1) succeeds
ok: box1 volume matches 10*20*30 = 6000
ok: zero dimension rejected
ok: negative dimension rejected
ok: NaN dimension rejected
ok: very small box (1e-6 per side) succeeds
ok: very small box volume matches 1e-18 within relative tolerance
ok: very large box (1e6 per side) succeeds
ok: very large box volume matches 1e18 within relative tolerance
ok: context 2 created
ok: box1's handle used against ctx2 is rejected as foreign-context
ok: out-of-range handle index on ctx1 is rejected as invalid
ok: null ctx on create_box rejected
ok: null ctx on shape_volume rejected
ok: create_box on ctx2 succeeds independently of ctx1
ok: ctx2's box2 volume matches 1*2*3 = 6, independent of ctx1's shapes
all checks passed
EXIT: 0

$ ctest --output-on-failure
1/2 Test #1: occt_probe .......................   Passed    0.01 sec
2/2 Test #2: abi_smoke_test ...................   Passed    0.01 sec
100% tests passed, 0 tests failed out of 2
```

Memory safety (FFI ownership/lifetime auditing per `AGENTS.md`'s Stage-1
hardening-mode list, run early rather than deferred):

```
$ valgrind --error-exitcode=1 --leak-check=full --show-leak-kinds=definite,indirect ./abi_smoke_test
==...== ERROR SUMMARY: 0 errors from 0 contexts (suppressed: 0 from 0)
==...== LEAK SUMMARY:
==...==    definitely lost: 0 bytes in 0 blocks
==...==    indirectly lost: 0 bytes in 0 blocks
(16 bytes "still reachable" at exit — an OCCT internal static/global,
 not AICAD-owned memory; not a leak)
```

No-OCCT-leakage check on the public header:

```
$ grep -in "TopoDS\|BRep\|Standard_\|GProp\|gp_" native/occt_bridge/include/aicad/occt_bridge.h
(no matches other than the word "OCCT" inside doc comments — no OCCT
 symbol/type name appears in the public header)
```

Workspace-level required checks (no Rust source touched by this task):

```
$ cargo fmt --all -- --check          # exit 0
$ cargo clippy --workspace --all-targets --all-features -- -D warnings   # exit 0, zero warnings
```

## Artifacts
- `native/occt_bridge/build/{occt_probe,abi_smoke_test,libaicad_occt_bridge.a}`
  (gitignored; reproducible from the commands above).

## Regressions added
None — `abi_smoke_test` and `occt_probe` are both new coverage; no prior
test suite existed for this crate/module to regress.

## Limitations
- **Stale-handle detection (a slot that was freed and its index reused)
  is not implemented by this task** — there is no shape-release
  operation yet for a handle to become stale against. This is explicitly
  AICAD-019's scope ("kernel context lifecycle and shape-handle table"),
  not a gap introduced here; do not treat this task's `AicadShapeHandle`
  as final until AICAD-019 lands.
- Only two operations exist (`create_box`, `shape_volume`); the full
  operation catalog (`native/occt_bridge/README.md`'s
  `create_cylinder, make_edge, ... export_step` list) is out of this
  task's scope and belongs to Batch 1B/1C/1D.
- This ABI is not yet reachable from Rust/Cargo — `crates/cad-occt-bridge`
  does not yet declare or link against it. That integration is AICAD-018.
- The exception-containment `catch` blocks around OCCT calls could not be
  exercised by a genuine OCCT-thrown failure in this task's tests: box
  construction has no reachable failure mode for finite, positive inputs
  after this task's own pre-validation rejects the invalid ones first.
  The `try`/`catch` remains defense-in-depth, verified by code inspection
  and by the same pattern already proven to compile and run correctly in
  AICAD-015's probe; a genuine OCCT-exception-triggering input was not
  found for this task's two operations and is not claimed to have been
  tested.

## Unresolved questions
None raised by this task. No `AICAD-016` escalation condition
(`project/TASKS.yaml`) was triggered: no public syntax/semantics changed,
no OCCT type crossed the adapter boundary (verified by grep above), no
stage gate/benchmark/test needed weakening, no semantic-reference
ambiguity arose (none exists at this layer), no unresolved architecture
alternative needed selecting, and this task's scope was not expanded
into AICAD-017/018/019's work.
