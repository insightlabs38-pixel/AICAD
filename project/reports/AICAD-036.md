# AICAD-036 — Add bounded geometry regression/fuzz harness

## Objective
Add bounded regression/fuzz coverage over adversarial geometry cases
(AGENTS.md's "ADVERSARIAL GEOMETRY CASES" list, as applicable to the
operations this bridge already implements) and, per the native
crash/hang policy, fully investigate and fix the genuine native
concurrency defect this batch's own AICAD-034 work surfaced.

## Dependencies checked
AICAD-035 (STEP verification harness) — complete, this session, same
batch.

## Part 1: root-causing and fixing the `BRepFilletAPI_MakeFillet` concurrency defect

### Symptom
AICAD-034's own bracket test suite
(`crates/cad-occt-bridge/tests/stage1_bracket.rs`) intermittently failed
its `is_valid()` assertion under `cargo test`'s default (parallel,
multi-threaded) execution — not a crash, but a *silently invalid* B-rep
(`validate()` reporting 1-2 `invalid_face_count`, all other counts
zero). This is more insidious than a crash: an operation that reports
success (`AICAD_OCCT_OK`) while quietly producing corrupt geometry is
exactly the class of failure AGENTS.md's "Geometry operations must not
silently perform topology-changing healing merely to convert failure
into success" / "Never accept... 'OCCT returned a shape'... as
sufficient evidence" guards against — this was caught only because
AICAD-034's tests call `is_valid()`/`validate()` rather than trusting a
successful return code.

### Isolation (bounded, evidence-based, not guessed)
A sequence of scoped stress tests (each run with real OS threads via
`std::thread::scope`, not just `cargo test`'s own test-level
parallelism, to control the exact concurrency pattern) isolated the
defect precisely:

| Experiment | Threads × reps | Failures |
|---|---|---|
| `union`/`cut` alone (the drilled L-shape, no fillet/chamfer) | 3×30 | 0/30 |
| `fillet`-all-edges on a bare box (convex edges only) | 8×30 (240 total) | 0/240 |
| single convex `chamfer` on a bare box | 8×30 (240 total) | 0/240 |
| convex `chamfer` on the drilled L-shape (real shape, not a bare box) | 3×30 | 0/30 |
| **concave/reentrant-edge `fillet` on the drilled L-shape** | 3×30 | **14/30 (~47%)** |
| full bracket pipeline (union+holes+fillet+chamfer), 3 heterogeneous concurrent variants (plain / +export / +export+import) | 40 rounds × 3 threads | **29/40 rounds had ≥1 failure** |
| the exact concave fillet construction, run with **zero concurrency** (purely sequential) | 300 reps, 1 thread | **0/300** |

This isolates the defect specifically to `BRepFilletAPI_MakeFillet` (not
`union`/`cut`/`chamfer`, which were independently confirmed safe under
the identical concurrent conditions) and specifically to concurrent
execution (the identical construction never failed once with no
concurrency across 300 repetitions). `chamfer`, despite using the
sibling `BRepFilletAPI_MakeChamfer` class, was independently confirmed
NOT to exhibit this — the mutex fix below is scoped to `fillet` only, on
that evidence, not applied speculatively to both.

### Fix
Mirrors AICAD-033's own STEP-translator remediation exactly: a single
process-wide `std::mutex` (`FilletMutex`, function-local static)
guarding `aicad_occt_fillet`'s entire `BRepFilletAPI_MakeFillet`
construction, edge-adding, and `Build()` call
(`native/occt_bridge/src/aicad_occt_bridge.cpp`). This serializes calls
to `aicad_occt_fillet` specifically across all contexts; every other
bridge function (including `chamfer`) remains fully concurrent, per the
isolation evidence above. `native/occt_bridge/include/aicad_occt_bridge.h`'s
own doc comment on `aicad_occt_fillet` documents this inline, matching
the existing `aicad_occt_export_step` documentation style.

### Confirmation
After the fix, every experiment above was re-run:
- Concave fillet under 3×30 concurrency: **0/30 failures** (down from
  14/30).
- The full heterogeneous 3-thread reproduction: **0/40 rounds with any
  failure** (down from 29/40).
- AICAD-034's own `stage1_bracket` test suite: **15/15 clean runs** in
  `cargo test`'s default parallel mode (previously intermittent).

### Permanent regression test
`concurrent_fillet_of_a_concave_edge_from_independent_contexts_does_not_corrupt_the_result`
(`crates/cad-occt-bridge/src/lib.rs`) reproduces the exact defect
pattern on a small, fast fixture (a 20×15×5 / 20×5×15 L-shape, not the
full 80-unit bracket, to keep the regression test itself fast): 3
threads × 15 rounds, each building and filleting the concave root edge
concurrently, asserting every result is a valid B-rep. This is a real
fix addressing the root cause (an actual OCCT-internal non-thread-safety
in `BRepFilletAPI_MakeFillet`/`ChFi3d`), not a workaround, and not a
weakened test — per AGENTS.md's native crash/hang policy.

This finding did not require an owner escalation: it is a
bridge-internal implementation-only change (a mutex inside
`aicad_occt_bridge.cpp` plus a doc-comment update), does not alter the
ABI, any public semantics, or Stage-1's kernel architecture, and does
not weaken any test or gate — squarely "internal refactor/correctness
fix" per AGENTS.md's autonomously-allowed list, identical in kind to
AICAD-033's own STEP-mutex fix.

## Part 2: bounded adversarial parameter sweep
`crates/cad-occt-bridge/tests/adversarial_sweep.rs` (new): a
fixed-seed (reproducible), bounded-iteration (100-500 cases per test, 7
tests, ~1150 total geometry-operation calls) sweep using a small inline
xorshift64* PRNG (no new crate dependency, per CLAUDE.md's
"agent-support infrastructure... minimal" policy):

- **Box/cylinder dimensions** across zero/negative/NaN/infinite/tiny
  (`1e-300`–`1e-6`)/ordinary/huge (`1e6`–`1e12`) magnitudes — asserts no
  panic, no crash, and that every non-positive/non-finite dimension is
  rejected with `InvalidArgument` (never silently accepted, never
  misclassified as `Internal`).
- **Tangent/coincident/overlapping/disjoint/mixed-scale booleans** — two
  boxes at controlled offsets (0 = coincident, exactly-touching =
  tangent, partial/full overlap, disjoint) across 5 scale-ratio pairs
  spanning `1e-9`–`1e6`, run through `union`/`cut`/`intersect`.
- **Impossible fillet/chamfer magnitudes** — radius/distance from
  negative/zero/NaN through geometrically-impossible-for-one-edge
  (larger than the box side) — asserts the non-positive/non-finite cases
  are strictly `InvalidArgument`, and that no case (however geometrically
  extreme) ever returns `Internal`.
- **Extreme shell/offset deltas** — including deliberately-invalid zero
  and adversarially large magnitudes.
- **Long chains of repeated transforms** (500 compositions of random
  rotation+translation, 20 independent chains) — pure Rust math, no
  kernel calls, checking the composed transform stays rigid
  (orthonormal, determinant 1) within a `1e-6` tolerance, catching
  cumulative floating-point drift in `Transform::compose`.
- **Unusual orientation/frames** — `Frame3::from_x` across scales
  `1e-9`–`1e9` and near-axis-aligned directions, checking the
  orthonormal/right-handed invariant holds.

**What this harness asserts, and what it deliberately does not** (stated
in the file's own header comment, so a future reader doesn't need this
report to understand the design):
- It DOES assert the process never panics for any case (every call is
  matched on `KernelResult`, never blindly `.unwrap()`ed).
- It DOES assert `KernelError::Internal` never occurs for any
  merely-extreme-but-not-malformed input — this bridge's own contract is
  that `Internal` means an adapter/backend defect.
- It does NOT assert that every extreme case succeeds or produces a
  valid B-rep — OCCT is free to report `OperationFailed` for a
  geometrically-impossible fillet/chamfer magnitude, and this bridge must
  not silently heal a failure into an artificial success (Stage-1 kernel
  policy #13-14). Outcome counts are printed for visibility rather than
  hard-coded as pass/fail thresholds, since the exact success/failure
  boundary at a given adversarial magnitude is an OCCT implementation
  detail, not a contract this bridge makes.

## Implementation decisions
- **No wall-clock/subprocess watchdog infrastructure** was added for
  this sweep, despite AGENTS.md's "use killable subprocesses with
  explicit wall-clock/resource limits where practical" guidance for
  risky geometry campaigns. This is an honest, disclosed limitation (see
  below), not an oversight: building that infrastructure is nontrivial
  and CLAUDE.md's project-scope policy directs against unnecessary
  agent-support infrastructure; the sweep's own bounded case count
  (~1150 calls, ~2-3 seconds observed wall time) and the CI job's own
  overall timeout are the practical backstop instead.
- **Fixed seed** (`0x0A1C_AD00_3600_00A1`, byte-grouped to satisfy
  clippy's `unusual_byte_groupings` lint) rather than a
  time-/environment-derived one — reproducibility was judged more
  valuable than broader per-run coverage at this scope; a future
  hardening-mode session can vary the seed deliberately if broader
  coverage is wanted.
- **Mutex scoped to `fillet` only, not `chamfer`** — see Part 1's
  isolation evidence; extending the lock to chamfer "just in case" was
  considered and rejected as unjustified by the actual evidence gathered
  (over-broad serialization has a real concurrency-throughput cost with
  no corresponding safety benefit here).

## Files changed
- Added: `crates/cad-occt-bridge/tests/adversarial_sweep.rs`.
- Edited: `native/occt_bridge/include/aicad_occt_bridge.h`,
  `native/occt_bridge/src/aicad_occt_bridge.cpp` (the `FilletMutex` fix),
  `crates/cad-occt-bridge/src/lib.rs` (the permanent regression test).

## Verification (exact commands/results)
```
$ cmake --build native/occt_bridge/build -j$(nproc)
[100%] Built target ... (18 targets, 0 errors)

$ ctest --test-dir native/occt_bridge/build --output-on-failure
18/18 tests passed, 0 tests failed

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, no warnings)

$ cargo test -p cad-occt-bridge --lib
test result: ok. 84 passed; 0 failed   (includes the new fillet-concurrency regression test)

$ cargo test -p cad-occt-bridge --test adversarial_sweep
test result: ok. 7 passed; 0 failed
  box sweep (300 cases): ok_valid=19 ok_invalid=0 err_invalid_argument=218 err_other=63
  tangent/coincident/mixed-scale boolean sweep (150 cases): union_ok=120 cut_ok=120 intersect_ok=120

$ for i in $(seq 1 5); do cargo test -p cad-occt-bridge --test adversarial_sweep; done
(5/5 runs, default parallel mode: 7 passed; 0 failed each time)

$ for i in $(seq 1 15); do cargo test -p cad-occt-bridge --test stage1_bracket; done
(15/15 runs, default parallel mode: 3 passed; 0 failed each time -- confirms the FilletMutex fix holds)

$ cargo test --workspace
(every crate) test result: ok, 0 failed
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1
(`rust-toolchain.toml`), CMake 3.28.3, OCCT 7.6.3 (`libocct-*-dev`
7.6.3+dfsg1-7.1build1) — unchanged from Batch 1A-1D/AICAD-034/035.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — **PASS**.
- Task-specific: native `ctest` (18/18), `cargo test -p cad-occt-bridge
  --lib` (84/84, including the new permanent fillet-concurrency
  regression test), `cargo test -p cad-occt-bridge --test
  adversarial_sweep` (7/7), and `cargo test -p cad-occt-bridge --test
  stage1_bracket` re-run 15x in default parallel mode (15/15 clean,
  confirming the concurrency fix holds) all pass.

## Regressions added
None. One genuine pre-existing latent defect (the
`BRepFilletAPI_MakeFillet` concurrency issue) was found and fixed as
part of this batch's own work — it could only manifest once a
concave-edge fillet on a multi-boolean shape was exercised under real
concurrency, which no prior batch's test suite happened to do (prior
fillet tests used only convex edges on bare boxes, and prior concurrency
tests — AICAD-019, AICAD-033 — exercised simpler operations).

## Limitations
- **No wall-clock/subprocess watchdog** was added (see "Implementation
  decisions" above) — a genuine native hang (as opposed to the
  silent-corruption defect actually found, which was not a hang) in some
  future, not-yet-implemented operation would rely on the CI job's own
  overall timeout as backstop, not a per-case kill switch.
- **The adversarial sweep is fixed-seed** — it exercises the same ~1150
  cases every run, not a growing/varying corpus. This trades broader
  incidental coverage for perfect reproducibility; a future
  hardening-mode session (per the active scheduled-task brief's
  post-AICAD-037 "Stage-1 hardening mode") is the natural place to vary
  the seed or expand the case count if broader coverage is wanted.
- **The isolation experiments (Part 1's table) used throwaway debug test
  files, not committed** — only the final permanent regression test
  (small, fast fixture) and this report's own recorded counts persist;
  the exact reproduction rate (~47%, 29/40) is therefore historical
  evidence from this investigation, not a number future CI runs will
  re-derive (the permanent regression test instead asserts zero
  failures post-fix, which is what matters going forward).
- The concurrency defect was isolated to this specific concave-edge/
  multi-boolean-shape category; a systematically broader concurrent
  fuzz campaign across many more geometry categories (beyond what Part 2
  and this investigation covered) was not performed, consistent with
  AGENTS.md's "do not spend unbounded effort" guidance once a concrete,
  reproducible, fixable defect was found and fixed.

## Unresolved questions
None requiring owner escalation.
