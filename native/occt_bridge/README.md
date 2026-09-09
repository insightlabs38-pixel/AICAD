# native/occt_bridge

WP-01 (Kernel bridge). Narrow C++ bridge around Open CASCADE Technology
(OCCT), exposing coarse, domain-shaped operations only:

```text
create_box, create_cylinder, make_edge, make_wire, make_face, extrude,
revolve, sweep, loft, boolean_union, boolean_cut, boolean_intersect,
fillet, chamfer, offset, shell, heal, validate, explore_topology,
surface_info, curve_info, export_step
```

Add capabilities only as required by the low-level API in
`docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`. `crates/cad-occt-bridge`
is the only crate that may call into this bridge. OCCT is treated as an
external dependency under `project/DECISION_LOG.md#DL-6`
(development-phase policy; a formal distribution/license review remains a
separate future gate — see `project/OWNER_DECISIONS.md` D13).

Plan references: `docs/plan/01_SYSTEM_ARCHITECTURE.md` §4;
`docs/plan/19_RESEARCH_NOTES_AND_SOURCES.md` §1-3.

## OCCT discovery/probe (AICAD-015)

`CMakeLists.txt` builds a discovery/probe target, independent of the C ABI
bridge described below. It:

- calls `find_package(OpenCASCADE REQUIRED CONFIG)` and fails with a
  specific, actionable message if OCCT is not discoverable, rather than
  an opaque CMake error;
- verifies every OCCT module library this probe (and near-term Stage-1
  work) needs is actually present in the discovered installation, and
  fails specifically naming the missing module if not;
- builds `probe/occt_probe.cpp`, which constructs a real B-rep box via
  `BRepPrimAPI_MakeBox`, validates it with `BRepCheck_Analyzer`, and
  checks its volume/surface area against the analytic expected values —
  not merely that OCCT headers/libraries link, per `AGENTS.md`'s evidence
  rule ("OCCT returned a shape" is not sufficient evidence on its own);
- registers that probe as a CTest test (`ctest` in the build directory).

Build and test:

```sh
cmake -S native/occt_bridge -B native/occt_bridge/build
cmake --build native/occt_bridge/build
ctest --test-dir native/occt_bridge/build --output-on-failure
```

Requires OCCT development packages to be installed (on Debian/Ubuntu:
`libocct-foundation-dev libocct-modeling-data-dev
libocct-modeling-algorithms-dev`, matching `CMakeLists.txt`'s
`OCCT_PROBE_LIBS`). CI installs the same packages before building — see
`.github/workflows/ci.yml`'s `native-build-smoke` job.

Exact environment this probe was verified against is recorded in
`project/reports/AICAD-015.md`.

## C ABI boundary (AICAD-016)

`include/aicad_occt_bridge.h` is the entire public surface of this
bridge, and the only file `crates/cad-occt-bridge` (AICAD-018) may bind
against. It is kernel-neutral at the type level — no OCCT class/enum name
appears in it — per RFC-0002 §3. `src/aicad_occt_bridge.cpp` is the only
translation unit permitted to include OCCT headers or hold OCCT-typed
state.

Boundary guarantees (see `project/reports/AICAD-016.md` for the exact
tests proving each one):

- **No OCCT/C++ exception crosses the boundary.** Every function catches
  `Standard_Failure` and `...` internally and returns a status code
  (`aicad_occt_status_t`) instead.
- **No raw OCCT/C++ pointer is exposed as resource identity.** Shapes are
  addressed by `aicad_shape_handle_t { context_id, slot, generation }`, a
  plain-old-data value, not a pointer.
- **Stale and foreign-context handles are rejected, never silently
  aliased.** A released slot's generation is bumped before reuse, so a
  handle minted before release can never resolve to a later shape reusing
  the same slot; a handle whose `context_id` does not match the context
  it is passed to is rejected outright.
- **Contexts are single-thread-affine** (conservatively, per Stage-1
  kernel policy): every context-taking function checks the calling thread
  against the context's creating thread and fails explicitly if they
  differ, rather than risking a data race.

Build and test (same commands as the discovery probe above; both targets
build together):

```sh
cmake -S native/occt_bridge -B native/occt_bridge/build
cmake --build native/occt_bridge/build
ctest --test-dir native/occt_bridge/build --output-on-failure
```

`tests/abi_boundary_test.cpp` includes only the public header (as
`crates/cad-occt-bridge` will) and exercises the happy path plus every
rejection case above.

Only `create_box` (plus the `shape_is_valid`/`shape_volume` query helpers
needed to prove it produced a real, valid B-rep) is implemented so far.
The rest of the operation list at the top of this file is added
incrementally by later Stage-1 tasks, per RFC-0002 §3's capability-driven
minimal-surface rule — this is not a comprehensive up-front OCCT wrapper.

## Lifecycle / shape-handle table stress tests (AICAD-019)

`tests/lifecycle_test.cpp` extends `abi_boundary_test.cpp`'s single-shot
handle-safety cases with scale and concurrency evidence: free-list reuse
stays bounded (not unbounded growth) across 1000 create/release cycles;
generation numbers stay strictly monotonic and never repeat across those
same 1000 cycles; and 8 threads, each owning its own kernel context,
create/query/release 100 shapes concurrently with zero cross-thread
failures. Registered as the `lifecycle_test` CTest test alongside
`occt_probe` and `abi_boundary_test`.

Recommended leak/error check (not wired into default CI — valgrind on a
heavy OCCT/Tcl-linked binary is slow; run manually or in a dedicated
hardening pass):

```sh
valgrind --leak-check=full --error-exitcode=99 native/occt_bridge/build/abi_boundary_test
valgrind --leak-check=full --error-exitcode=99 native/occt_bridge/build/lifecycle_test
```

Both report 0 errors and 0 leaked bytes attributable to this bridge's own
code (a small "still reachable" allocation from OCCT/Tcl's own one-time
static initialization is expected and not part of this project's
allocations) — see `project/reports/AICAD-019.md` for full output.
