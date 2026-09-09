# AICAD-015 — Add OCCT discovery/probe CMake target

## Objective

Prove OCCT is reliably discoverable and linkable in the build environment
via a standalone CMake target, per `project/TASKS.yaml` (AICAD-015,
Stage 1 / Batch 1A). This does not implement the C ABI boundary
(AICAD-016) or any AICAD kernel abstraction (AICAD-017/018) — it only
proves the native toolchain can find and use OCCT before those are built
on top of it.

## Dependencies checked

AICAD-014 (Stage-0 gate packet) — complete. Stage 0 itself was approved
by the owner in `project/DECISION_LOG.md#DL-10` (recorded earlier in this
session; see the preceding commit), which authorizes Stage 1 through
AICAD-037.

## Starting repository state

`native/occt_bridge/` contained only `README.md` (naming the eventual
bridge operation set); no `CMakeLists.txt` existed anywhere in the
repository. `crates/cad-occt-bridge` is still the empty AICAD-002
placeholder — untouched by this task, since AICAD-015 is native-only.

## What was done

1. Added `native/occt_bridge/CMakeLists.txt`: a standalone CMake project
   (not yet wired into the Cargo workspace — that integration decision
   belongs to AICAD-016) that:
   - Calls `find_package(OpenCASCADE REQUIRED)` so a missing/broken OCCT
     install fails the CMake configure step outright (fail-closed, per
     Stage-1 kernel policy: never silently degrade past a broken kernel).
   - Prints the discovered version, include dir, library dir, and module
     list as CMake status messages (Stage-1 policy #15: record exact
     environment versions).
   - Builds `occt_discovery_probe`, linked against `TKernel`/`TKMath`
     (the `FoundationClasses` module — the minimal real OCCT libraries
     needed to exercise an actual type, not just headers).
   - Registers the probe as a CTest test with a
     `PASS_REGULAR_EXPRESSION` that is NOT hardcoded to the currently
     installed OCCT version, so the check stays valid if the installed
     OCCT version changes later.
2. Added `native/occt_bridge/probe/occt_probe.cpp`: constructs a `gp_Pnt`
   and checks its coordinates round-trip correctly (proves actual
   linking/execution, not merely that headers parse), then cross-checks
   the compiled header version (`OCC_VERSION_COMPLETE` from
   `Standard_Version.hxx`) against the version CMake's `find_package`
   discovered (passed in as `AICAD_OCCT_PROBE_CMAKE_VERSION`) — this
   proves CMake's discovery and the actual compiled/linked OCCT agree,
   rather than trusting either source alone. Prints
   `OCCT_PROBE_OK version=<X.Y.Z>` on success.
3. Verified fail-closed behavior: configuring with
   `-DCMAKE_DISABLE_FIND_PACKAGE_OpenCASCADE=ON` (simulating OCCT being
   unavailable/disabled) produces a nonzero CMake configure exit code
   and two explicit `CMake Error` messages, rather than silently
   proceeding without OCCT. See exact commands/output below.
4. `native/**/build/` was already covered by the repository `.gitignore`
   (added in AICAD-003); no gitignore change was needed. Local
   `build/`/`build-negative/` directories created during verification
   were deleted before committing — nothing under them is part of this
   change.

## Environment versions recorded (Stage-1 policy #15)

- OS: Ubuntu 24.04.4 LTS (x86_64)
- Compiler: g++ (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0
- CMake: 3.28.3
- OCCT: 7.6.3+dfsg1-7.1build1 (Debian/Ubuntu package
  `libocct-foundation-dev` and siblings), upstream OCCT version string
  `7.6.3`
- Rust toolchain: rustc/cargo 1.98.1 (matches `rust-toolchain.toml`)

## Files changed

- Added: `native/occt_bridge/CMakeLists.txt`
- Added: `native/occt_bridge/probe/occt_probe.cpp`

## Implementation decisions

- Kept the probe as a fully standalone CMake project rather than
  pre-deciding how the Rust build will eventually invoke it (via
  `cmake` crate, a `build.rs` shelling to `cmake`, or a separate build
  step) — that is an AICAD-016 decision (native C ABI boundary), not
  this task's. This is a reversible, private implementation detail per
  `AGENTS.md`'s "Implementation decision authority."
- Linked only against `TKernel`/`TKMath` (not the full module list) —
  sufficient to prove real linkage without committing to which OCCT
  modules the eventual bridge needs; that is decided incrementally as
  Batch 1B/1C tasks require specific operations, per
  `native/occt_bridge/README.md`'s "add capabilities only as required."
- Did not hardcode the installed OCCT version into the CTest pass
  regex, so this check does not silently start failing (or silently stop
  meaning anything) if the installed OCCT version changes; the
  cross-check inside the probe itself (compiled header vs.
  CMake-discovered version) is the part that would actually catch a
  real discovery/version mismatch.

## Verification (exact commands/results)

```
$ cd native/occt_bridge && cmake -S . -B build -DCMAKE_BUILD_TYPE=RelWithDebInfo
-- The CXX compiler identification is GNU 13.3.0
-- AICAD-015 OCCT probe: OpenCASCADE 7.6.3
-- AICAD-015 OCCT probe: include dir /usr/include/opencascade
-- AICAD-015 OCCT probe: library dir /usr/lib/x86_64-linux-gnu
-- AICAD-015 OCCT probe: modules FoundationClasses;ModelingData;ModelingAlgorithms;Visualization;ApplicationFramework;DataExchange;Draw
-- Configuring done
-- Generating done

$ cmake --build build -j"$(nproc)"
[100%] Built target occt_discovery_probe
(one benign deprecation warning from OCCT's own
NCollection_StlIterator.hxx, not AICAD source)

$ cd build && ctest --output-on-failure
1/1 Test #1: occt_discovery_probe .............   Passed    0.00 sec
100% tests passed, 0 tests failed out of 1

$ ./occt_discovery_probe
OCCT_PROBE_OK version=7.6.3
$ echo $?
0

$ cd .. && rm -rf build-negative && cmake -S . -B build-negative -DCMAKE_DISABLE_FIND_PACKAGE_OpenCASCADE=ON
CMake Error at CMakeLists.txt:17 (find_package):
  find_package for module OpenCASCADE called with REQUIRED, but
  CMAKE_DISABLE_FIND_PACKAGE_OpenCASCADE is enabled.
CMake Error at CMakeLists.txt:20 (message):
  OpenCASCADE was not discovered by find_package(OpenCASCADE)
-- Configuring incomplete, errors occurred!
$ echo $?
1
```

Required Rust-workspace checks (unaffected by this native-only change,
re-run fresh to confirm no regression):

```
$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.40s
(exit 0)

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.17s
(exit 0)

$ cargo test --workspace
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
(exit 0)
```

## Artifacts

`native/occt_bridge/build/` (local CMake build tree) is git-ignored and
was deleted after verification; nothing from it is committed. The
committed artifacts are the two source files listed above.

## Regressions added

None. `occt_discovery_probe`'s CTest registration is itself a new,
permanent regression check: any future environment change that breaks
OCCT discovery, breaks linking, or produces a version mismatch between
CMake and the compiled headers will now fail `ctest` in
`native/occt_bridge/build`.

## Limitations

- This CMake project is not yet invoked from any top-level build command
  or from Cargo; there is no single `cargo build`/`cargo test` that runs
  `ctest` here yet. Wiring that up is in scope for AICAD-016 (native C
  ABI boundary) once the crate-to-native build integration is decided.
- `find_package(OpenCASCADE REQUIRED)` with no `COMPONENTS` list imports
  targets for all seven installed OCCT modules (including
  `Visualization`/`ApplicationFramework`/`Draw`, which the bridge may
  never need). This is intentionally not narrowed yet — AICAD-016/017
  will decide the actual required module set as the bridge operation
  list from `native/occt_bridge/README.md` is implemented.
- The probe only exercises `gp_Pnt` (a trivial value type in `TKMath`).
  It is deliberately not a claim that any constructive/topological OCCT
  operation works — that evidence begins at AICAD-020 (Batch 1B).

## Unresolved questions

None. No escalation condition in `project/TASKS.yaml` (AICAD-015) was
triggered.
