# AICAD-015 — Add OCCT discovery/probe CMake target

## Objective

Add a CMake target that locates a usable OCCT (Open CASCADE Technology)
installation and proves — by actually compiling, linking, and running a
real OCCT construction, not merely by `find_package` succeeding — that
this machine can build the native kernel bridge that `crates/cad-occt-bridge`
will wrap starting at AICAD-016, per `project/TASKS.yaml` (AICAD-015),
`docs/plan/01_SYSTEM_ARCHITECTURE.md` §4 ("a deliberately narrow C++
bridge around OCCT"), and `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-01.
This is the first task of Stage 1 (Batch 1A, kernel boundary), begun after
the owner recorded Stage-0 approval (`project/DECISION_LOG.md` DL-10).

## Dependencies checked

AICAD-014 (Stage-0 gate packet) — complete; Stage 0 owner-approved per
DL-10, `project/CURRENT_STAGE.md` updated to Stage 1 active in the commit
immediately preceding this task's work.

## What was done

1. **`CMakeLists.txt`** (repo root, new): minimal entry point that
   `add_subdirectory(native/occt_bridge)`. Per
   `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` §1's monorepo skeleton
   ("`CMakeLists.txt` # OCCT bridge only if needed") this file exists only
   to build the native bridge; the Rust workspace `Cargo.toml` remains the
   actual compiler/runtime build entry point.
2. **`native/occt_bridge/CMakeLists.txt`** (new): a standalone CMake
   project (so the bridge can be configured/built independently of the
   Rust workspace) that:
   - calls `find_package(OpenCASCADE REQUIRED CONFIG)` and fails the
     configure step with an explicit, actionable `FATAL_ERROR` message if
     OCCT is not found (verified — see §"Negative case" below);
   - additionally checks the discovered `OpenCASCADE_MAJOR_VERSION` is
     `>= 7` and fails clearly otherwise (AICAD's Stage-1 kernel bridge
     targets OCCT 7.x; this is a deliberate, narrow version floor, not a
     new architecture decision — no `OWNER_DECISIONS.md` item is
     implicated by pinning a native dependency's minimum major version);
   - builds `occt_discovery_probe` linked only against the specific OCCT
     module libraries the probe actually calls (`TKernel`, `TKMath`,
     `TKBRep`, `TKG3d`, `TKGeomBase`, `TKTopAlgo`, `TKPrim`), not the
     full `OpenCASCADE_LIBRARIES` list — AICAD-016 will widen this
     deliberately as the real bridge grows, rather than starting from an
     unnecessarily broad link surface.
3. **`native/occt_bridge/probe/occt_discovery_probe.cpp`** (new): a
   standalone C++ program, not part of the eventual C ABI (that boundary
   is AICAD-016's job), that:
   - prints `OCC_VERSION_MAJOR`/`MINOR`/`MAINTENANCE`/`COMPLETE` as
     `KEY=VALUE` lines read from `Standard_Version.hxx` (compile-time
     evidence the headers resolve correctly, cross-checked against the
     installed package version below);
   - constructs a `BRepPrimAPI_MakeBox` (10mm x 20mm x 30mm at the
     origin), checks `IsDone()`, checks the result shape is non-null,
     runs `BRepCheck_Analyzer` to confirm B-rep validity, computes the
     shape's bounding-box diagonal via `BRepBndLib`/`Bnd_Box`, and
     asserts it equals the analytically expected
     `sqrt(10^2+20^2+30^2) = sqrt(1400) ≈ 37.4166` to within `1e-6` — this
     is link-level and computation-level evidence (an analytic property
     check, not "OCCT returned a shape" or "the render looks right", per
     `AGENTS.md`'s evidence rule), not merely a compile-level one;
   - wraps the whole construction in `try`/`catch` for both
     `Standard_Failure` (OCCT's own exception root) and `std::exception`,
     printing a clean `FAIL` report and returning exit code 1 rather than
     letting an OCCT exception escape uncaught — establishing the
     exception-containment discipline the Stage-1 kernel policies require
     of the real ABI boundary (AICAD-016), even though this probe itself
     is not yet that boundary;
   - exits 0 only if every check passed, printing `PROBE_RESULT=PASS`.
4. **`.gitignore`**: added `/build/` (the root CMake project's default
   out-of-tree build directory lands at the repo root, since the new root
   `CMakeLists.txt` lives there — the pre-existing `native/**/build/`
   pattern does not cover it).

No Rust source was touched; `crates/cad-occt-bridge` remains the
placeholder AICAD-002/AICAD-003 created (its safe Rust wrapper is
AICAD-018, which depends on the C ABI boundary, AICAD-016, which in turn
depends on this task).

## Implementation decisions (reversible/local, within AGENTS.md's
autonomously-allowed scope)

- **No version pin beyond "7.x".** `OpenCASCADEConfigVersion.cmake`
  (installed by the `libocct-*-dev` packages) uses CMake's `ExactVersion`
  compatibility policy and does not accept a two-component
  `MAJOR.MINOR` version request from `find_package(OpenCASCADE 7.6 ...)`
  — verified by direct reproduction (see §"Debugging notes"). Requesting
  no version and checking `OpenCASCADE_MAJOR_VERSION` manually avoids this
  without weakening the actual check (still fails clearly below OCCT 7).
- **Probe links only the module libraries it uses**, not
  `OpenCASCADE_LIBRARIES` (the full ~50-library list) — smaller, faster
  build, and it also means a future accidental broadening of the probe's
  scope is visible as a diff.
- **Probe validated `Shape()`/`IsDone()` ordering bug found during this
  task** (see debugging notes below) rather than accepting a false `FAIL`
  — this is exactly the kind of native-debugging effort
  `AGENTS.md`/the Stage-1 kernel policies expect to be done rather than
  worked around.

None of these decisions touch public AICAD syntax/semantics, the
kernel-independence contract, or any open `OWNER_DECISIONS.md` item — the
probe is not part of the public kernel-neutral API surface (that starts
at AICAD-017/`cad-kernel-api`) and mentions OCCT class names freely, which
is correct and expected for a native-bridge-internal discovery tool per
`docs/plan/01_SYSTEM_ARCHITECTURE.md` §2.6 ("the first implementation can
map them to OCCT" — internally).

## Debugging notes

`BRepBuilderAPI_MakeShape::IsDone()` (the base of `BRepPrimAPI_MakeBox`)
only reflects whether `Build()` has run; `Build()` itself is called
lazily, from inside `Shape()`. Checking `IsDone()` *before* calling
`Shape()` therefore always reports `false`, independent of whether the
construction would actually have succeeded — this is documented behavior
of OCCT's `BRepBuilderAPI_Command`/`BRepBuilderAPI_MakeShape` split (the
"done" flag lives on `Command`; `MakeShape::Shape()` is what triggers
`Build()`), not a bug in OCCT itself. First run of the probe reproduced
this exactly (`FAIL: BRepPrimAPI_MakeBox did not complete`, despite valid
10x20x30 input); fixed by calling `.Shape()` first, then checking
`IsDone()` against the already-built result. Recorded here per the
native-crash/hang policy's spirit (preserve the input, record what
happened, minimize the reproducer) even though this was a logic bug, not
a crash/hang — the same rigor applies to any adversarial/edge-case native
result.

## Files changed

- Added: `CMakeLists.txt` (repo root).
- Added: `native/occt_bridge/CMakeLists.txt`.
- Added: `native/occt_bridge/probe/occt_discovery_probe.cpp`.
- Edited: `.gitignore` (`/build/` entry).

## Verification (exact commands/results)

```
$ rm -rf build
$ cmake -S . -B build -DCMAKE_BUILD_TYPE=RelWithDebInfo
...
-- AICAD occt_bridge: OpenCASCADE 7.6.3 found at /usr/lib/x86_64-linux-gnu
-- Configuring done (0.5s)
-- Generating done (0.0s)
-- Build files have been written to: /home/user/AICAD/build

$ cmake --build build -- -j"$(nproc)"
...
[100%] Linking CXX executable occt_discovery_probe
[100%] Built target occt_discovery_probe
(one warning: OCCT's own NCollection_StlIterator.hxx uses the C++17-deprecated
std::iterator base — from OCCT's headers, not this task's code; not a
warning our -Wall/-Wextra flags introduced)

$ ./build/native/occt_bridge/occt_discovery_probe; echo "EXIT=$?"
OCC_VERSION_MAJOR=7
OCC_VERSION_MINOR=6
OCC_VERSION_MAINTENANCE=3
OCC_VERSION_COMPLETE=7.6.3
PROBE_BBOX_DIAGONAL=37.4166
PROBE_RESULT=PASS
EXIT=0
```

Cross-checked `OCC_VERSION_COMPLETE=7.6.3` (compiled into the probe from
`Standard_Version.hxx`) against the installed Debian package version:
```
$ dpkg -l | grep libocct-foundation-dev
ii  libocct-foundation-dev:amd64  7.6.3+dfsg1-7.1build1  amd64  ...
```
Matches (7.6.3).

### Negative case — discovery failure is a hard configure error

```
$ cmake -S . -B /tmp/neg-build2 -DCMAKE_DISABLE_FIND_PACKAGE_OpenCASCADE=ON
...
CMake Error at native/occt_bridge/CMakeLists.txt:31 (find_package):
  find_package for module OpenCASCADE called with REQUIRED, but
  CMAKE_DISABLE_FIND_PACKAGE_OpenCASCADE is enabled.  A REQUIRED package
  cannot be disabled.

CMake Error at native/occt_bridge/CMakeLists.txt:34 (message):
  AICAD-015: OpenCASCADE (OCCT) was not found via find_package(OpenCASCADE).
  Install OCCT development packages (headers + CMake config) and/or set
  OpenCASCADE_DIR to the directory containing OpenCASCADEConfig.cmake.

-- Configuring incomplete, errors occurred!
```
Confirms the discovery-failure path halts configuration with an
actionable message rather than silently proceeding.

(Note: setting `-DOpenCASCADE_DIR=/nonexistent` alone does *not* trigger
this path — CMake's config-mode `find_package` falls back to the standard
system search paths when the hint itself contains no config file, which
is correct CMake behavior, not a gap in this task's CMake logic; the
`CMAKE_DISABLE_FIND_PACKAGE_OpenCASCADE` test above is the actual
"OCCT truly unavailable" case.)

### Rust workspace unaffected (required checks still pass)

```
$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
```

### Environment versions (Stage-1 kernel policy #15)

- OS: Ubuntu 24.04.4 LTS
- Architecture: x86_64
- Compiler: g++ (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0
- Rust toolchain: rustc 1.98.1 (48a229cea 2026-09-01), cargo 1.98.1 (797e8a9bc 2026-08-05)
- OCCT version: 7.6.3 (Debian/Ubuntu package `libocct-*-dev` 7.6.3+dfsg1-7.1build1)
- CMake version: 3.28.3

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS** (no Rust source touched).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**.
- Task-specific check: `occt_discovery_probe` builds and exits 0 with
  `PROBE_RESULT=PASS`, and the discovery-failure path was independently
  confirmed to fail configuration cleanly — both verified above.

## Limitations / follow-up

- This task only proves *discovery and a minimal sanity construction*.
  The actual C ABI boundary (opaque handles, exception containment across
  the ABI, structured error results) is AICAD-016 — this probe's
  exception-catching is a local pattern demonstration, not the ABI itself.
- OCCT's own license/redistribution terms remain an open owner decision
  (`project/OWNER_DECISIONS.md` D13) — unaffected by this task, which only
  builds against the system-installed OCCT for local development/CI use
  and does not redistribute OCCT binaries.
- The probe links against `libocct-*-dev` 7.6.3 as packaged for Ubuntu
  24.04; a from-source OCCT build was not attempted (unnecessary — the
  packaged dev libraries provide both headers and the CMake config, which
  is what `find_package(OpenCASCADE CONFIG)` requires).
- `CMAKE_DISABLE_FIND_PACKAGE_OpenCASCADE=ON` was used to force the
  discovery-failure branch since this machine has OCCT installed; a
  machine that genuinely lacks OCCT would hit the same `FATAL_ERROR`
  message via the ordinary "config file not found" path (both routes lead
  to the same `find_package(... REQUIRED ...)` failure handling).
