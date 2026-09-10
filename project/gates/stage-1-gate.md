# Stage-1 Owner Gate Packet

**Read alongside `project/reports/reviews/STAGE1-INDEPENDENT-REVIEW.md`**
(an independent adversarial re-review performed after this packet was
written). That review reproduces this packet's evidence and reaches the
same substance, but narrows one claim in §2.3/§8 below: the "two
disclosed layers" of STEP verification independently confirm the
exported file's *structure* (entity counts, ISO-10303-21 grammar), not
its re-imported *geometry* (volume, bounding box, coordinates) — the
only check of re-imported geometric properties is Layer 1, which both
this packet and the independent review agree is not independent (same
OCCT bridge both directions). Read §2.3 below with that distinction in
mind rather than as full independent geometric verification.

Prepared by AICAD-037, per `project/gates/README.md` and
`AICAD_AGENT_OPERATING_MODEL.md` §8. **This packet recommends; it does
not approve.** Per `AGENTS.md` "Stage gates": "The agent may prepare gate
evidence and recommend pass/do-not-pass. The agent may not approve a
roadmap stage. Stage progression is an owner decision." Nothing in this
document, and no subsequent task, treats Stage 1 as passed until the
owner records that decision in `project/DECISION_LOG.md`. Per the active
scheduled-task brief, roadmap advancement STOPS here: AICAD-038 and all
Stage-2 work are forbidden until that owner decision exists.

## 1. Exact git commit/revision

```text
2a6a970c3bb210cf16564587ab6c72b5f636ed5e
```

Branch `branch/festive-cori-pe2fun`, which is exactly `origin/main`
(`0e9065c`) plus this session's own two commits:
```
2a6a970 AICAD-036: Add bounded adversarial geometry regression/fuzz harness
a6fe021 AICAD-034/AICAD-035: Stage-1 proof bracket, STEP import, fillet-concurrency fix
```
Working tree is clean (`git status --short` empty) at packet preparation
time. All of Batches 1A-1D (AICAD-015 through AICAD-033) are already
canonical on `origin/main` (merged via PR #2/#3/#4/#5); this session's
own work (Batch 1E, AICAD-034 through AICAD-037) is not yet on
`origin/main` — see `project/SESSION_HANDOFF.md` for canonical-publishing
status.

## 2. Stage-1 exit gate and evidence

**Exit gate** (`project/CURRENT_STAGE.md`): "A meaningfully nontrivial
exact B-rep part (not a renamed primitive), combining base geometry,
booleans, transforms, holes, and fillet/chamfer, can be built through the
AICAD kernel API, validated (topology/property checks, not render-only),
exported to STEP, and independently imported/checked — with
kernel-boundary safety (no OCCT type leakage, no C++ exception crossing
the ABI, stale/foreign handle rejection) evidenced per batch checkpoint."

**The full pipeline, end to end:**
```
Rust/native AICAD kernel API (cad-kernel-api / cad-occt-bridge)
  -> create_box x2 (base flange + upright wall, genuine 3D overlap)
  -> union (L-shaped bracket)
  -> create_cylinder x4 + Transform (translation, and a 90-degree
     rotation mapping +Z onto +Y) + cut (4 mounting through-holes)
  -> fillet (interior concave/reentrant root edge, found by geometric
     bounding box, never raw enumeration index)
  -> chamfer (exterior convex top edge, found the same way)
  -> valid exact B-rep (is_valid()==true; validate() all-zero invalid
     counts)
  -> analytic/topological verification (closed-form volume within 0.1%;
     exact bounding box; an exact mirror-symmetry invariant on center of
     mass; nontrivial face/edge/vertex counts)
  -> STEP export (export_step, AP214 via STEPControl_Writer)
  -> independent import/check (two disclosed layers -- see §2.3)
  -> preserved regression/fuzz evidence (a permanent regression test for
     a genuine concurrency defect found and fixed this batch, plus a
     bounded adversarial parameter sweep)
```
This is not a renamed primitive: the bracket combines two boxes, four
holes (including a rotated-axis cylinder), a boolean union and four
boolean cuts, a fillet, and a chamfer — matching the brief's own
"bracket-like part" guidance (base geometry, perpendicular mounting
geometry, holes, booleans, transform/orientation use, fillet and
chamfer).

### 2.1 Batch-by-batch evidence (all already-canonical on `origin/main`)

| Batch | Tasks | Checkpoint | Result |
|---|---|---|---|
| 1A — Kernel boundary | AICAD-015..019 | `project/gates/STAGE1-A_KERNEL_BOUNDARY.md` | **PASS** |
| 1B — Constructive geometry | AICAD-020..024 | `project/gates/STAGE1-B_CONSTRUCTIVE_GEOMETRY.md` | **PASS** |
| 1C — Hard geometry operations | AICAD-025..028 | `project/gates/STAGE1-C_HARD_OPS.md` | **PASS** |
| 1D — Inspection/validation/interchange | AICAD-029..033 | `project/gates/STAGE1-D_INTERCHANGE.md` | **PASS** |
| 1E — Stage-1 proof | AICAD-034..037 | this packet | see §2.2-2.4 |

Each batch checkpoint independently verifies: reproducible OCCT
discovery; the C ABI builds; no OCCT type leakage above
`cad-occt-bridge`; C++/OCCT exception containment (with the one open
caveat in §5, G1); invalid/stale/foreign-context handle rejection;
Rust/native ownership behavior; and a full fresh workspace-check run at
that batch's own commit. None weakened a prior batch's gate.

### 2.2 Batch 1E task evidence

| Task | Summary | Report |
|---|---|---|
| AICAD-034 | Built the Stage-1 proof bracket directly through `cad-kernel-api`/`cad-occt-bridge`; exact/analytic verification; STEP export. | `project/reports/AICAD-034.md` |
| AICAD-035 | Added a narrow STEP-import kernel capability; a disclosed-non-independent self round-trip check on the bracket; and a genuinely independent (non-OCCT) `steputils`-based structural re-verification of the bracket's STEP file. | `project/reports/AICAD-035.md` |
| AICAD-036 | Root-caused and fixed a genuine native concurrency defect in `BRepFilletAPI_MakeFillet` found while building AICAD-034/035 (full isolation-experiment table); added a bounded, fixed-seed adversarial geometry parameter sweep. | `project/reports/AICAD-036.md` |
| AICAD-037 | This packet. | this file |

### 2.3 STEP verification disclosure (per the active scheduled-task brief's explicit requirements)

- **Which implementation performs export**: this bridge's own
  `Shape::export_step`, via OCCT's `STEPControl_Writer` (OCCT 7.6.3) —
  unchanged from AICAD-033.
- **Which implementation/tool performs the independent check**: two
  distinct, disclosed layers.
  - **Layer 1 (self round-trip, NOT independent)**: the bracket is
    re-imported through this same bridge's new `import_step`
    (`STEPControl_Reader`, same OCCT installation), and its volume/
    bounding-box/face-count/edge-count are compared to the original —
    proves the export/import pipeline is internally self-consistent for
    a real multi-operation shape, not that OCCT's own output is
    independently correct.
  - **Layer 2 (genuinely independent)**: the pure-Python, non-OCCT
    `steputils` ISO-10303-21 parser (same tool AICAD-033 used for a box/
    cylinder) applied directly to the bracket's own STEP file. Result:
    1324 total entities (matching OCCT's own write-time report exactly),
    correctly parsed AP214 `FILE_SCHEMA`, and an independently-computed
    entity histogram (1 `MANIFOLD_SOLID_BREP`, 1 `CLOSED_SHELL`, 20
    `ADVANCED_FACE`, 48 `EDGE_CURVE`, 30 `VERTEX_POINT`, 15 `PLANE`, 5
    `CYLINDRICAL_SURFACE`) agreeing exactly with this bridge's own
    independently-established topology for the finished bracket
    (`face_count()==20`, `edge_count()==48`, `vertex_count()==30`).
  - **What this proves**: the bracket's STEP file is genuinely parseable
    per the ISO-10303-21 exchange-structure grammar by an implementation
    sharing no code with OCCT, and its topological entity counts agree
    exactly with this bridge's own established topology for a real
    multi-operation (not box/cylinder-only) shape.
  - **What this does not prove**: full AP214 EXPRESS-schema-semantic
    conformance (e.g. that referenced `CARTESIAN_POINT` coordinates are
    geometrically consistent with the surfaces citing them) — neither
    layer performs EXPRESS schema validation. See
    `project/reports/AICAD-035.md` for the complete disclosure.

### 2.4 Kernel-boundary safety (batch-checkpoint evidence, reconfirmed)
- **No OCCT type leakage**: `cad-kernel-api` remains
  `#![forbid(unsafe_code)]` and OCCT-free; `cad-occt-bridge`'s public API
  surface (re-checked this session) exposes only `OcctContext`, `Shape`,
  `BoundingBox`, `ValidationReport`, `TriangleMesh`, and
  `cad_kernel_api` re-exports — no `TopoDS_*`/OCCT symbol anywhere above
  `ffi.rs`.
- **No C++/OCCT exception crosses the ABI**: every native function added
  this batch (`aicad_occt_import_step`) follows the established
  `catch (const Standard_Failure&) { OPERATION_FAILED } catch (...) {
  INTERNAL }` pattern (see `native/occt_bridge/src/aicad_occt_bridge.cpp`).
- **Stale/foreign-context handle rejection**: unchanged this batch;
  re-confirmed still passing via the full native `ctest` suite (§3).

## 3. Test/benchmark commands and results

Full fresh re-run at commit `2a6a970`, from a clean native rebuild
(`rm -rf native/occt_bridge/build` first):

```text
$ cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3
-- Configuring done / Generating done

$ cmake --build native/occt_bridge/build -j$(nproc)
[100%] Built target step_import_test   (18 targets total, 0 errors)

$ ctest --test-dir native/occt_bridge/build --output-on-failure
100% tests passed, 0 tests failed out of 18
Total Test time (real) = 0.73 sec

$ cargo fmt --all -- --check
(exit 0, no output)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s)
(exit 0, zero warnings)

$ cargo build --workspace --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo test --workspace
(every crate) test result: ok, 0 failed
  cad-kernel-api:    23 passed
  cad-occt-bridge (lib):              84 passed
  cad-occt-bridge (adversarial_sweep): 7 passed
  cad-occt-bridge (stage1_bracket):    3 passed
  (all other crates: 0 passed, 0 failed -- no product code yet, per
  their own stage's scope)
```

**Concurrency-fix confirmation** (re-run specifically because the
underlying defect was concurrency-triggered and intermittent — a single
clean run is not sufficient evidence for this class of fix):
```text
$ for i in $(seq 1 15); do cargo test -p cad-occt-bridge --test stage1_bracket; done
(15/15 runs, cargo test's default parallel/multi-threaded mode:
 test result: ok. 3 passed; 0 failed -- every run)
```
Pre-fix, this same command intermittently failed (approximately 1 in 3
runs) with an `is_valid()` assertion failure. See
`project/reports/AICAD-036.md` for the full isolation-experiment table
(per-operation concurrency stress results) and the sequential-baseline
control (0/300 failures with zero concurrency, confirming the defect was
concurrency-specific, not algorithmic).

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1
(`rust-toolchain.toml`), CMake 3.28.3, OCCT 7.6.3 (`libocct-*-dev`
7.6.3+dfsg1-7.1build1) — unchanged across all of Stage 1
(AICAD-015..037); independently confirmed via `dpkg -S`/`--version` at
this packet's own preparation time, not assumed carried forward.
Independent verification tool: Python 3.11, `steputils` 0.1
(`pip3 install steputils`, this session's environment only, not a
project dependency — same as AICAD-033).

## 4. Known failures and limitations

Carried forward from Batches 1A-1D (all still open, all still
non-blocking, re-confirmed not worsened by Batch 1E's own work):
- **G1** (from AICAD-016/STAGE1-A, still open): exception containment's
  `catch (const Standard_Failure&)` branches remain unproven reachable by
  a genuine OCCT-thrown exception in this bridge's own test suite (every
  current adversarial input either succeeds, is rejected by
  pre-validation, or is cleanly reported as `OPERATION_FAILED` via
  `IsDone()`/similar checks rather than an actual C++ throw). Batch 1E's
  own adversarial sweep (AICAD-036) did not newly trigger a genuine
  `Standard_Failure` throw either — every extreme case in the sweep
  still resolved through `OperationFailed`/`InvalidArgument`/success, not
  a caught exception. This remains a documented, non-fabricated gap, not
  silently closed.
- **G2** (from AICAD-016/STAGE1-A, still open): valgrind remains a
  documented manual check, not wired into default per-push CI. No fresh
  valgrind run was performed this batch (time-bounded per AGENTS.md's
  "do not spend unbounded effort" guidance, given a concrete, higher-value
  investigation — the fillet concurrency defect — consumed this batch's
  investigative budget instead). Recommended for Stage-1 hardening mode
  (per the active scheduled-task brief's post-AICAD-037 plan).
- **Sweep/loft, shell/offset limitations** (from AICAD-025/AICAD-028,
  STAGE1-C): unchanged; not exercised by the Stage-1 proof bracket, which
  does not require them (its own ticket's acceptance criterion —
  booleans, transforms, holes, fillet/chamfer — does not call for
  sweep/loft/shell/offset, and AGENTS.md's "do not expand product scope
  merely to support an adversarial case" was judged to apply equally to
  not artificially broadening this proof fixture).

New, Batch-1E-specific, all non-blocking:
- **The `BRepFilletAPI_MakeFillet` concurrency defect** (found and fixed
  this batch, `project/reports/AICAD-036.md`) was isolated to a
  concave/reentrant-edge fillet on a multi-boolean shape under real
  cross-thread concurrency. The fix (a process-wide mutex) is narrowly
  scoped to `aicad_occt_fillet`, confirmed by isolation testing not to be
  needed for `union`/`cut`/`chamfer`. A broader concurrent-fuzz campaign
  across more geometry categories was not performed beyond what
  AICAD-036's own sweep covers (bounded effort, per AGENTS.md).
- **No wall-clock/subprocess watchdog** exists for the adversarial sweep
  or any other risky geometry test (AGENTS.md's own suggested mitigation
  for risky campaigns) — an honest, disclosed gap, not an oversight;
  CLAUDE.md's minimal-agent-infrastructure policy and this batch's own
  time budget were weighed against building that infrastructure now.
  Recommended for Stage-1 hardening mode.
- **Bracket topology counts are not asserted as exact** (deliberately —
  see `project/reports/AICAD-034.md`'s "Nontrivial topology" section):
  this is a design choice matching AGENTS.md's topological-naming
  guidance, not a limitation of the evidence gathered.
- **No STEP-file EXPRESS-schema-semantic conformance check** (either
  layer) — see §2.3.

None of these limitations required forcing an artificial success,
weakening a test, or hiding a failure. The one genuine defect this batch
found (the fillet concurrency issue) was root-caused, fixed, and given a
permanent regression test — the native crash/hang policy's required
response, not a suppressed or ignored flake.

## 5. Representative artifacts

- Kernel API: `crates/cad-kernel-api/` (backend-independent handles/
  errors/geometry types), `crates/cad-occt-bridge/` (safe Rust wrapper),
  `native/occt_bridge/` (C ABI + OCCT implementation).
- Stage-1 proof: `crates/cad-occt-bridge/tests/stage1_bracket.rs` (the
  bracket construction and its three verification tests).
- Regression/fuzz evidence:
  `crates/cad-occt-bridge/tests/adversarial_sweep.rs`; the permanent
  fillet-concurrency regression test in `crates/cad-occt-bridge/src/lib.rs`
  (`concurrent_fillet_of_a_concave_edge_from_independent_contexts_does_not_corrupt_the_result`).
- Batch checkpoints: `project/gates/STAGE1-A_KERNEL_BOUNDARY.md` through
  `STAGE1-D_INTERCHANGE.md`.
- Every task's own evidence report: `project/reports/AICAD-015.md`
  through `AICAD-037.md` (this packet).
- STEP export from this batch's own bracket build was scratch-verified
  (not committed — see AICAD-035's report for the exact reproduction
  commands) with the independent `steputils` parser; the permanent,
  always-run STEP evidence is `crates/cad-occt-bridge/tests/stage1_bracket.rs`'s
  own `bracket_exports_to_a_syntactically_valid_step_file` and
  `bracket_survives_an_export_then_import_round_trip_through_this_bridge`
  tests.

## 6. Regression counts / performance baselines

- **Regressions**: zero across all of Stage 1. Every prior batch's
  reports record zero regressions at that batch's own commit; this
  batch's own fresh full-workspace re-run (§3) confirms all
  pre-existing native (18/18) and Rust (84+7+3+23 = 117 across the
  crates with product code) tests still pass unchanged.
- **One genuine defect found and fixed this batch** (not a regression
  this batch introduced — the `BRepFilletAPI_MakeFillet` concurrency
  issue was a pre-existing latent defect in the underlying kernel
  library, only exposed once a concave-edge fillet on a multi-boolean
  shape was exercised under real concurrency, which no prior batch's
  test suite happened to do).
- **Performance baselines**: not yet established as a formal, tracked
  benchmark suite (`benchmarks/performance/` per its own `README.md`
  remains for a later stage) — Stage 1's own task tickets did not
  require one, and none of the four batch checkpoints established one
  either. Informally: the full bracket construction (2 boxes, 4 holes,
  1 fillet, 1 chamfer, `is_valid`/`validate`/property queries, STEP
  export, and — for the round-trip test — re-import) completes in well
  under 100ms per iteration (`cargo test -p cad-occt-bridge --test
  stage1_bracket` completes in ~0.1s total for all 3 tests combined,
  §3).

## 7. Unresolved decisions

Unchanged from Stage 0's own gate packet and every Batch 1A-1D
checkpoint — no Batch 1E task touched an `OWNER_DECISIONS.md` item.
Current status (from `project/OWNER_DECISIONS.md`'s own quick index):

| Status | IDs |
|---|---|
| Fully open | D3, D5, D10, D11, D12, D15 |
| Partially resolved, residual item tracked | D7, D8, D13 |
| Fully resolved | D1, D2, D4, D6, D9, D14 |

None of the fully-open items block any Stage-1 task (verified per-task
against each ticket's own `plan_references` and `escalate_if`
conditions across all of AICAD-015 through AICAD-037); D5 (determinism-
equivalence contract) and D11 (constraint IR) remain flagged for
resolution before their respective later-stage blocking impact
materializes, per their own existing entries. The
`BRepFilletAPI_MakeFillet` concurrency fix (§4) did not require a new
`OWNER_DECISIONS.md` entry — see `project/reports/AICAD-036.md`'s own
"Unresolved questions" section for the reasoning (internal
correctness fix, no ABI/semantics/architecture change).

## 8. Recommendation

**Recommend: PASS.**

Rationale: the Stage-1 exit gate is met with direct, checkable evidence
(§2) — a meaningfully nontrivial bracket (base geometry, a perpendicular
wall, four holes via transformed-cylinder booleans including a rotated
axis, a boolean union, four boolean cuts, a fillet, and a chamfer, not a
renamed primitive) is built entirely through the kernel-neutral Rust API,
validated by exact/analytic checks rather than a render (closed-form
volume, exact bounding box, an exact symmetry invariant, validity/
topology-count sanity), exported to STEP, and verified through two
disclosed layers (a self-consistency round trip and a genuinely
independent non-OCCT structural parse). Kernel-boundary safety
(no OCCT leakage, exception containment for every reachable path, stale/
foreign-handle rejection) is evidenced across all four prior batch
checkpoints and reconfirmed unchanged by this batch's own full test run.
Every batch checkpoint from 1A through 1D independently PASSed with no
weakened test or gate. The one genuine defect this final batch's own
work surfaced — a real, previously-undiscovered native concurrency
defect in `BRepFilletAPI_MakeFillet` — was root-caused with a bounded,
evidence-driven investigation (not guessed), fixed narrowly (not by
broadly serializing unrelated operations), and given a permanent
regression test, exactly matching the native crash/hang policy's
required response and the precedent AICAD-033 already set for the
STEP-translator's own concurrency issue. All known limitations (§4) are
disclosed, none are hidden, and none required weakening a check to
achieve this recommendation.

**This recommendation is not an approval.** Per `AGENTS.md` and
`CURRENT_STAGE.md` ("Owner approval required to advance: Yes"), Stage 2
work must not begin until the owner records a Stage-1 pass decision in
`project/DECISION_LOG.md`, following the same pattern as `DL-10`'s
Stage-0 approval. Per the active scheduled-task brief, future invocations
of this routine automatically switch into Stage-1 hardening mode
(reproducing Stage-1 claims from clean state, expanding regression/fuzz
coverage, bounded fuzzing, sanitizers, FFI/lifetime auditing, leak
investigation — including the still-open G1/G2 items above, and this
batch's own newly-flagged wall-clock-watchdog gap — STEP interoperability
testing, determinism/performance baselining) rather than beginning
AICAD-038 or any Stage-2 scope, until that owner decision is recorded.
