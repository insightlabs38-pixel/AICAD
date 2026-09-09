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

`CMakeLists.txt` currently builds only a discovery/probe target, not the
C ABI bridge (AICAD-016 onward). It:

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
