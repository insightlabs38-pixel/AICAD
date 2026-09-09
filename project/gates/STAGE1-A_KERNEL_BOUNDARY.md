# Stage 1, Batch 1A — Kernel Boundary checkpoint

Batch 1A (`project/TASKS.yaml` AICAD-015 through AICAD-019) is complete.
This checkpoint consolidates its evidence per the batch-checkpoint
requirement in the scheduled-task brief and `AGENTS.md`'s Stage-1 kernel
policies. It is a checkpoint, not a stage gate: the agent may prepare
and pass a batch checkpoint to unblock the next batch inside an
already-owner-approved stage; it may not approve a roadmap stage itself
(that remains an owner decision, per `project/DECISION_LOG.md#DL-10` for
Stage 1's own approval).

## Task reports

- `project/reports/AICAD-015.md` — OCCT discovery/probe CMake target
- `project/reports/AICAD-016.md` — native C ABI boundary
- `project/reports/AICAD-017.md` — `cad-kernel-api` handle/error types
- `project/reports/AICAD-018.md` — `cad-occt-bridge` safe Rust wrapper
- `project/reports/AICAD-019.md` — kernel context lifecycle and
  shape-handle table

## Checkpoint properties (required minimum)

### Reproducible OCCT discovery
`native/occt_bridge/CMakeLists.txt`'s `find_package(OpenCASCADE CONFIG
REQUIRED)` was re-verified from a clean, deleted-and-recreated build
directory in AICAD-015, AICAD-016, and AICAD-019's reports; each time it
resolves OCCT 7.6.3 at the same `/usr/include/opencascade` /
`/usr/lib/x86_64-linux-gnu` locations. `crates/cad-occt-bridge/build.rs`
independently locates the same headers (AICAD-018) via an env-var
override or a short list of known install paths, failing loudly if none
match rather than guessing.

### C ABI builds successfully
`native/occt_bridge`'s CMake project (`occt_probe`, `aicad_occt_bridge`,
`abi_smoke_test`) and `crates/cad-occt-bridge` (via `cc` in `build.rs`)
both build cleanly from a clean state, with zero warnings attributable to
AICAD code (only OCCT's own header deprecation warnings appear, as noted
in every report from AICAD-015 onward).

### No OCCT type leakage
`native/occt_bridge/include/aicad/occt_bridge.h` was grepped for OCCT
type/symbol names in AICAD-016's report (`grep -in
"TopoDS|BRep|Standard_|GProp|gp_"`) with no match beyond the word "OCCT"
inside doc comments; this remains true after AICAD-019's header changes
(only `uint32_t generation` was added). `crates/cad-occt-bridge`'s public
API (`OcctContext::create_box`/`shape_volume`/`release_shape`) is typed
entirely in `cad-kernel-api` terms (`KernelSolid`, `KernelResult`); its
`ffi` module (the only place any native type name appears on the Rust
side) is private to the crate.

### Exception containment
Every native function that calls into OCCT (`aicad_occt_create_box`,
`aicad_occt_shape_volume`, `aicad_occt_release_shape`) wraps the call in
`catch (const Standard_Failure&)` / `catch (const std::exception&)` /
`catch (...)`, normalizing into `AicadStatus` (AICAD-016, extended
unchanged in AICAD-019). `aicad_occt_context_create`/`_destroy` are
similarly guarded. AICAD-016's report records honestly that no OCCT-
thrown failure was actually reached by `create_box`'s specific inputs
after this bridge's own pre-validation rejects invalid ones first — the
`try`/`catch` is verified present and structurally correct by code
inspection and by the identical pattern already exercised successfully
in AICAD-015's probe, not claimed to have been hit by a real OCCT
exception in this batch's tests.

### Invalid handles rejected
Both the native smoke test and `cad-occt-bridge`'s Rust tests check an
out-of-range index against a real context and get
`AICAD_STATUS_INVALID_HANDLE` / `KernelError::InvalidHandle` (AICAD-016
introduced this; AICAD-019 folded it into the same `ValidateHandle`
helper used by every handle-consuming operation).

### Stale handles rejected
This is AICAD-019's own subject: a released handle is rejected
immediately (liveness check) and, after its slot is reused for an
unrelated shape, stays rejected forever via a generation mismatch —
proven in both the native smoke test (`stage the old (pre-release)
handle stays rejected even after its slot is reused, never silently
aliasing the new box`) and `cad-occt-bridge`'s
`stale_handle_never_aliases_a_reused_slot` test, both passing against the
real compiled kernel, not a mock.

### Foreign-context handles rejected
A handle issued by one `AicadOcctContext`/`OcctContext` used against a
different one returns `AICAD_STATUS_FOREIGN_CONTEXT_HANDLE` /
`KernelError::ForeignContextHandle` in both the native smoke test and
`cad-occt-bridge`'s `handle_from_one_context_is_rejected_as_foreign_by_another`
test (AICAD-016/018), unaffected by AICAD-019's generational-table
change (context-id mismatch is checked before index/generation in
`ValidateHandle`).

### Rust/native ownership behavior tested
`OcctContext`'s `Drop` impl calls `aicad_occt_context_destroy` exactly
once (AICAD-018's `dropping_a_context_does_not_panic_or_crash` test);
`NonNull<ffi::AicadOcctContext>` makes `OcctContext` `!Send`/`!Sync` by
default with no extra code, satisfying `AGENTS.md` policy #9
("single-thread-affine initially") without an explicit unsafe impl in
either direction. `valgrind --leak-check=full` was run against both the
native `abi_smoke_test` (AICAD-016/019: 0 errors, 0 definite/indirect
leaks each time) and the compiled Rust test binary (AICAD-018/019: 0
definite/indirect leaks; the one "possibly lost" block was traced by its
full allocation backtrace to Rust's own `libtest` harness internals,
confirmed by an identical check against a trivial control binary with no
test harness — not attributable to this batch's FFI/ownership code).

### Required workspace checks pass
`cargo fmt --all -- --check` and `cargo clippy --workspace
--all-targets --all-features -- -D warnings` both exit 0 (zero warnings)
as of this batch's final commit; `cargo test --workspace` passes for
every crate, including `cad-kernel-api` (6 tests) and `cad-occt-bridge`
(11 tests) added by this batch. Re-verified fresh at checkpoint time
(see "Final re-verification" below), not only carried over from each
task's own report.

## Environment versions (AGENTS.md Stage-1 kernel policy §15)

```text
OS:            Ubuntu 24.04.4 LTS (noble), x86_64
Compiler:      gcc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0
Rust:          rustc 1.98.1 (48a229cea 2026-09-01), cargo 1.98.1
OCCT:          7.6.3 (libocct-*-dev 7.6.3+dfsg1-7.1build1, Ubuntu noble)
CMake:         3.28.3
```

Unchanged from AICAD-015/016's reports; re-confirmed at checkpoint time.

## Final re-verification

Re-ran from a clean state at checkpoint time, on top of AICAD-019's
commit, rather than only citing each task's own report:

```
$ cd native/occt_bridge && rm -rf build && mkdir build && cd build
$ cmake -DCMAKE_BUILD_TYPE=Release .. && cmake --build . -j"$(nproc)"
[100%] Built target abi_smoke_test

$ ./occt_probe && echo PROBE_OK
$ ./abi_smoke_test && echo SMOKE_OK
$ ctest --output-on-failure    # 2/2 passed

$ cd /home/user/AICAD
$ cargo fmt --all -- --check                                            # exit 0
$ cargo clippy --workspace --all-targets --all-features -- -D warnings  # exit 0
$ cargo test --workspace                                                # all pass
```

All commands passed; see the individual task reports for full transcripts.

## Known limitations carried forward (not blockers)

- Only two real operations exist (`create_box`, `shape_volume`) plus
  `release_shape`'s lifecycle-only role; the full bridge operation
  catalog (`native/occt_bridge/README.md`) is Batch 1B/1C/1D's scope.
- No geometry-operations trait exists yet in `cad-kernel-api`; deferred
  until more than the current handful of operations exist to generalize
  over (AICAD-017/018's own recorded scope decisions).
- The shape-handle table's free list never shrinks the underlying
  `slots` vector (AICAD-019's own recorded limitation) — acceptable for
  Stage 1's current operation set; revisit only if an actual resource
  budget requirement arises.

## Recommendation

**PASS.** Every required checkpoint property above has direct,
re-verified evidence from real (not mocked) execution against the
actual compiled kernel, across both the native and Rust layers. No
finding in this batch required an `project/OWNER_DECISIONS.md` entry or
touched an open decision. This checkpoint does not itself constitute
stage approval (Stage 1 was already owner-approved,
`project/DECISION_LOG.md#DL-10`) — it only certifies that Batch 1A's own
exit criteria are met, so Batch 1B (AICAD-020 through AICAD-024,
constructive geometry) may begin in a future invocation, within the
per-invocation work budget.
