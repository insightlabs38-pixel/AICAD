# AICAD-015 — Add OCCT discovery/probe CMake target

## Objective
Add a CMake target under `native/occt_bridge` that reliably discovers a
system-installed Open CASCADE Technology (OCCT) build and probes it with a
real, checkable geometric computation — not merely "the package/headers
were found" — per `project/TASKS.yaml` (AICAD-015) and the Stage-1 kernel
policies in the active scheduled-task brief (record exact OS/arch/
compiler/Rust toolchain/OCCT version/CMake version; never accept "OCCT
returned a shape" as sufficient evidence).

This task does not implement the C ABI bridge (AICAD-016), the
kernel-neutral handle types (AICAD-017), or the safe Rust wrapper
(AICAD-018) — those depend on this task and come next in Batch 1A.

## Dependencies checked
AICAD-014 (Stage-0 gate packet) — complete. Stage 0 itself is now closed:
the owner recorded Stage-0 approval in `project/DECISION_LOG.md#DL-10`
(this session, immediately prior to this task) after merging in the
previously-unmerged independent Stage-0 review (commit `aad7267`, branch
`claude/aicad-stage-0-review-9leull`) that this session discovered had not
yet been merged into `branch/loving-feynman-qcrzen`.

## What was done

1. **`native/occt_bridge/CMakeLists.txt`** (new): a CMake project that
   - calls `find_package(OpenCASCADE REQUIRED CONFIG)`, and additionally
     checks `OpenCASCADE_FOUND` with a specific, actionable
     `FATAL_ERROR` message (naming the packages to install or the
     `OpenCASCADE_DIR` variable to set) rather than relying only on
     CMake's own generic "package not found" error;
   - explicitly verifies that every OCCT module library this probe (and
     near-term Stage-1 work) needs (`TKernel`, `TKMath`, `TKG3d`,
     `TKGeomBase`, `TKBRep`, `TKGeomAlgo`, `TKTopAlgo`, `TKPrim`) is
     actually present in `OpenCASCADE_LIBRARIES` for the discovered
     installation, failing with a message naming the specific missing
     module if not — this distinguishes "OCCT not found at all" from "OCCT
     found but missing a required module" (e.g. a minimal/partial OCCT
     package), which a bare `find_package` would not distinguish;
   - builds `probe/occt_probe.cpp` against the discovered include/library
     directories and registers it as a CTest test (`add_test`).
2. **`native/occt_bridge/probe/occt_probe.cpp`** (new): constructs a real
   B-rep box (`BRepPrimAPI_MakeBox`, 1x2x3), validates its topology with
   `BRepCheck_Analyzer`, and checks its volume and surface area against
   the exact analytic values (6.0 and 22.0) within a `1e-9` tolerance,
   printing `OCC_VERSION_COMPLETE`. Exits non-zero with a specific
   diagnostic message on any failure (build-not-done, invalid topology,
   volume mismatch, area mismatch). This satisfies the evidence rule for
   this task's own scope: dimensional/topology-sanity/analytic-property
   checks, not a render or a bare "did it link."
3. **`.github/workflows/ci.yml`**: `native-build-smoke` job now installs
   the exact OCCT development packages this probe needs
   (`libocct-foundation-dev`, `libocct-modeling-data-dev`,
   `libocct-modeling-algorithms-dev`) before configuring/building, and
   runs `ctest --output-on-failure` after building, so CI actually
   exercises OCCT discovery and the probe's correctness checks on every
   push/PR rather than only locally. Updated the job's own comments
   (previously described a not-yet-existent `CMakeLists.txt`; now
   describes the real build+test flow and what happens if the
   `CMakeLists.txt` is ever removed).
4. **`native/occt_bridge/README.md`**: documented the discovery/probe
   target, its build/test commands, its required OCCT packages, and
   pointed to this report for the exact verified environment. Also fixed
   a stale cross-reference ("OCCT's license... is an open owner decision")
   to cite `project/DECISION_LOG.md#DL-6` (the development-phase policy
   the owner already recorded, per Stage-0), while still pointing to
   `project/OWNER_DECISIONS.md` D13 for the still-open public-distribution
   sub-question — this is a documentation-accuracy fix, not a policy
   change.

## Implementation decisions (autonomous, reversible)
- **C++ standard: C++17.** No RFC/plan document pins a C++ standard for
  the native bridge; OCCT 7.6 only requires C++11. C++17 is a
  private/internal build-tooling choice (nothing about it is visible
  through the AICAD kernel-neutral public API), consistent with
  `AGENTS.md`'s "internal refactors/local algorithms" autonomous-allowed
  scope.
- **Minimal module set for this probe**
  (`FoundationClasses`/`ModelingData`/`ModelingAlgorithms` only, via the
  eight `TK*` libraries listed above), not the full OCCT module list. This
  keeps the CI install step and the probe's own scope narrow and
  purpose-built to what AICAD-015 needs; `DataExchange` (STEP),
  `ApplicationFramework` (OCAF), and `Visualization` are Batch 1D/AICAD-016+
  concerns, not this discovery task's.
- **Probe checks analytic properties, not just successful linking.**
  Chose volume+area+validity over a trivial "print the version and exit
  0" probe specifically because the task brief's evidence rule and
  Stage-1 geometry-correctness standard both reject "OCCT returned a
  shape" as sufficient; a box with known analytic volume/area is the
  simplest fixture that produces an independently-checkable numeric
  result.
- **CTest, not a hand-rolled shell assertion**, to register the probe as
  a test — this is the native-toolchain-idiomatic way to make "run the
  task-specific test" a single reusable command (`ctest`) for both local
  use and CI, rather than inventing a bespoke pass/fail convention.

## Files changed
- Added: `native/occt_bridge/CMakeLists.txt`
- Added: `native/occt_bridge/probe/occt_probe.cpp`
- Edited: `native/occt_bridge/README.md`
- Edited: `.github/workflows/ci.yml`

(`native/occt_bridge/build/` is produced by the commands below and is
already covered by `.gitignore`'s `native/**/build/` pattern — confirmed,
not added to git.)

## Verification (exact commands/results)

Environment (Stage-1 kernel policy #15):
```
$ cat /etc/os-release | head -5
PRETTY_NAME="Ubuntu 24.04.4 LTS"
NAME="Ubuntu"
VERSION_ID="24.04"
VERSION="24.04.4 LTS (Noble Numbat)"
VERSION_CODENAME=noble

$ uname -m
x86_64

$ g++ --version | head -1
g++ (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0

$ rustc --version
rustc 1.98.1 (48a229cea 2026-09-01)

$ cmake --version | head -1
cmake version 3.28.3

$ dpkg -s libocct-foundation-dev | grep Version
Version: 7.6.3+dfsg1-7.1build1
```
(OCCT version as reported by the probe itself, matching the installed
package: `7.6.3`, see below.)

Configure, build, test:
```
$ cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3
-- OCCT discovery: include dir = /usr/include/opencascade
-- OCCT discovery: library dir = /usr/lib/x86_64-linux-gnu
-- OCCT discovery: available modules = FoundationClasses;ModelingData;ModelingAlgorithms;Visualization;ApplicationFramework;DataExchange;Draw
-- Configuring done (0.3s)
-- Generating done (0.0s)
-- Build files have been written to: .../native/occt_bridge/build

$ cmake --build native/occt_bridge/build
[ 50%] Building CXX object CMakeFiles/occt_probe.dir/probe/occt_probe.cpp.o
[100%] Linking CXX executable occt_probe
[100%] Built target occt_probe

$ ctest --test-dir native/occt_bridge/build --output-on-failure
Test project .../native/occt_bridge/build
    Start 1: occt_probe
1/1 Test #1: occt_probe .......................   Passed    0.01 sec
100% tests passed, 0 tests failed out of 1

$ ./native/occt_bridge/build/occt_probe
occt_probe: OCCT version 7.6.3
occt_probe: PASS - valid B-rep box, volume=6.000000 area=22.000000
```

Negative-path verification (discovery/module-check failure modes, run
against a scratch copy and reverted — not committed):
```
$ cmake -S . -B build_negativetest -DCMAKE_DISABLE_FIND_PACKAGE_OpenCASCADE=ON
CMake Error at CMakeLists.txt:24 (find_package):
  find_package for module OpenCASCADE called with REQUIRED, but
  CMAKE_DISABLE_FIND_PACKAGE_OpenCASCADE is enabled...
CMake Error at CMakeLists.txt:27 (message):
  OCCT discovery failed: the 'OpenCASCADE' CMake package was not found. ...
-- Configuring incomplete, errors occurred!

# (separately, with a bogus library name injected into OCCT_PROBE_LIBS)
CMake Error at CMakeLists.txt:59 (message):
  OCCT discovery: required module library 'TKDoesNotExist' is not among
  OpenCASCADE_LIBRARIES reported by the discovered installation (...).
  The installed OCCT build is missing a module this probe (and later
  Stage-1 work) requires.
-- Configuring incomplete, errors occurred!
```
Both confirm the discovery target fails loudly and specifically rather
than silently, distinguishing "not found at all" from "found but missing
a required module."

Required checks per task ticket:
```
$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.47s
(exit 0 — no Rust source added/changed by this task; this confirms the
Rust workspace baseline is undisturbed)
```

## Regressions added
None. This task adds a new native build target; it does not modify any
Rust crate or existing test.

## Limitations / follow-up
- This target proves OCCT discovery/linkage/basic correctness on the
  current sandbox environment (Ubuntu 24.04, OCCT 7.6.3, GCC 13.3.0,
  CMake 3.28.3) and now also in CI (GitHub Actions `ubuntu-latest`, which
  is currently also Ubuntu 24.04 — the same package names were used
  deliberately for this reason). It does not attempt macOS/Windows OCCT
  discovery; that is out of scope for AICAD-015 and not required by any
  Stage-1 task read so far.
- The probe intentionally does not yet exercise `DataExchange` (STEP),
  `ApplicationFramework` (OCAF), or `Visualization` — those are validated
  by later Stage-1 batches (1C/1D) against the operations that actually
  need them, per this task's own narrower scope.
- `native/occt_bridge/README.md`'s D13 cross-reference fix is
  documentation-only; it does not alter D13's status in
  `project/OWNER_DECISIONS.md`, which remains partially resolved exactly
  as `DECISION_LOG.md#DL-6` already records.
- Next task: AICAD-016 (create `native/occt_bridge` C ABI boundary),
  depends on this task's discovery target being in place.

No escalation condition was triggered: this task added a build-tooling
discovery target and a correctness probe using already-approved kernel
boundary decisions (DL-5/DL-6); it did not touch public syntax/semantics,
a stage gate/benchmark, kernel-type leakage above the adapter (no OCCT
type is exposed by this task — there is no AICAD-facing API yet), or any
open `OWNER_DECISIONS.md` item.
