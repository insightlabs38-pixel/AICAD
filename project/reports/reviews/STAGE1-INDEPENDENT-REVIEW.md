# Stage-1 Independent Adversarial Gate Review

This is an independent, adversarial re-review of the Stage-1 exit gate. It
does not extend the roadmap, does not begin AICAD-038, does not mark
Stage 1 owner-approved, and does not modify `project/CURRENT_STAGE.md`.
It reconstructs and verifies Stage-1 evidence directly from the
repository and this session's own tool runs; it does not accept
`project/gates/stage-1-gate.md`'s PASS recommendation on the strength of
its own claims.

## Scope

- **Exact commit reviewed**: `a296891f045aa7a263e2751c1c0effc7d8d97336`
  (`origin/main`, `main`, merge of PR #7 — a `SESSION_HANDOFF.md`-only
  update on top of PR #6's merge `1777d03`, which carried all of Batch 1E,
  `AICAD-034`–`AICAD-037`). Working tree confirmed clean
  (`git status --short` empty) both before and after this review; `git
  fetch origin --prune` confirmed this is the current tip of `origin/main`
  before any work began.
- **Two rounds of patches were made on top of the reviewed commit.**
  Round 1 (self-initiated, after this review's own first pass
  under-applied its own patch policy): a documentation cross-reference
  (Finding 1), a missing report stub (Finding 5), and two new permanent
  test files closing the test-coverage gaps identified as Findings 3 and
  4 — all squarely inside AGENTS.md's "Autonomously allowed" list
  ("unit/integration/property/fuzz tests," "benchmark fixtures and
  adversarial cases," "documentation synchronized to implemented
  behavior"), none changing public semantics, weakening a test/gate,
  choosing an open owner decision, or adding a trusted boundary. Round 2
  (owner-authorized, not self-initiated): Finding 2 — the
  double-context-destroy use-after-free this review had correctly
  identified but declined to fix unilaterally (fixing it properly
  required an internal architecture decision this review was not
  positioned to make on its own) — was fixed under an explicit owner
  decision authorizing exactly that architecture change, with specific
  required behaviors and a required design-alternatives comparison. See
  "Context lifetime safety fix" below for the full record: original
  defect, alternatives considered, selected design, rationale, benchmark
  result, regression tests, sanitizer result, and the post-fix
  second-pass audit result this triggered for Audits 2, 3, and 10. Both
  rounds are committed together with the rest of this review's fixes, on
  top of `a296891`; all findings below describe what was true of
  `a296891` itself unless a finding's own text says it was subsequently
  fixed.
- **Environment used for verification** (independently confirmed, not
  assumed from prior reports): Ubuntu 24.04 container, `g++`/`clang++`
  13.3.0, CMake 3.28.3, `cargo`/`rustc` 1.98.1 (`rust-toolchain.toml`),
  OCCT 7.6.3 (`libocct-*-dev 7.6.3+dfsg1-7.1build1`, verified via `dpkg
  -l`), `valgrind` 3.22.0. This matches the environment every Stage-1
  task report and the gate packet itself claim to have used.
- **Files/subsystems reviewed directly** (full read, not summarized from
  reports): `native/occt_bridge/include/aicad_occt_bridge.h` (670 lines,
  complete), `native/occt_bridge/src/aicad_occt_bridge.cpp` (1955 lines,
  complete), `crates/cad-occt-bridge/src/ffi.rs` (complete),
  `crates/cad-occt-bridge/src/lib.rs` (the full public API surface, Drop
  impls, and the concurrency regression test), `crates/cad-kernel-api/src/lib.rs`
  and `geometry.rs` (handle/error types, transform math),
  `native/occt_bridge/tests/abi_boundary_test.cpp`,
  `native/occt_bridge/tests/lifecycle_test.cpp`,
  `crates/cad-occt-bridge/tests/stage1_bracket.rs` (with an independent
  hand-derivation of its analytic volume formula — see "Geometry
  findings"), `crates/cad-occt-bridge/tests/adversarial_sweep.rs`,
  `rfcs/0002-geometry-runtime-kernel-abstraction.md`,
  `project/CURRENT_STAGE.md`, `project/TASKS.yaml`,
  `project/DECISION_LOG.md`, `project/OWNER_DECISIONS.md`,
  `project/SESSION_HANDOFF.md`, `project/gates/stage-1-gate.md` and all
  four batch checkpoints, `project/reports/AICAD-033.md`,
  `AICAD-034.md`(referenced)/`AICAD-035.md`/`AICAD-036.md` in full, and
  `.github/workflows/*.yml`. Every other non-kernel crate
  (`cad-references`, `cad-feature-graph`, `cad-diagnostics`, `cad-runtime`,
  etc. — 25 crates) was confirmed by direct inspection to be an unedited
  6-line placeholder stub with no logic, ruling out Stage-2+ scope creep
  by direct evidence rather than trusting the reports' own "no product
  code yet" claim.
- **What was NOT independently re-derived**: the historical ~47%/29-of-40
  concurrency failure rates reported for the pre-fix `BRepFilletAPI_MakeFillet`
  defect (AICAD-036) were not reproduced by reverting the fix, since doing
  so would require modifying source under audit; instead, this review
  verified the *fix* holds (20/20 fresh repeated runs, see below) and
  read the isolation-experiment table as historical evidence, consistent
  with the audit brief's "second-pass" scope (verify the current state,
  not re-manufacture a historical bug).

## Tests executed (fresh, in this session, from a clean rebuild)

```
$ rm -rf native/occt_bridge/build && cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3
$ cmake --build native/occt_bridge/build -j$(nproc)
(18 targets, 0 errors)
$ ctest --test-dir native/occt_bridge/build --output-on-failure
100% tests passed, 0 tests failed out of 18

$ cargo fmt --all -- --check          # exit 0
$ cargo clippy --workspace --all-targets --all-features -- -D warnings   # exit 0, zero warnings
$ cargo test --workspace
  cad-kernel-api:            23 passed
  cad-occt-bridge (lib):     84 passed
  cad-occt-bridge (adversarial_sweep): 7 passed
  cad-occt-bridge (stage1_bracket):    3 passed
  (all 25 other crates: 0 passed — confirmed by direct source inspection
   to be unedited placeholder stubs, not silently-empty test suites)
```

All counts match `project/gates/stage-1-gate.md` §3 exactly. Additionally:

```
$ for i in $(seq 1 20); do cargo test -p cad-occt-bridge --lib \
    concurrent_fillet_of_a_concave_edge --release; done
20/20 runs: test result: ok. 1 passed; 0 failed
```
(the batch's own evidence was 15 runs in default/debug mode; this review
re-ran 20 more in `--release`, which exercises different optimizer
codegen for the same race, with the same result: 0 failures.)

An **original, independent adversarial concurrency probe** (not present
in the batch's own evidence) was written and run against six
kernel operations the batch's own concurrency investigation did *not*
stress-test — `shell`, `offset`, `sweep`, `loft`, `tessellate`, and the
topology-exploration functions — using the same methodology AICAD-036
used to find the fillet defect (independent contexts on independent
`std::thread::scope` threads, validity-checked, not merely
success-checked). 8 threads x 40 rounds per operation (320 concurrent
builds per operation, 1920 total), release mode: **0 failures across
all six operations.** This is bounded, not exhaustive, evidence (see
"Threading findings" below for what it does and does not establish).
This review's first pass did not commit the probe (reasoning, at the
time, that AICAD-036's own precedent of not committing throwaway
isolation experiments applied) — on reflection, and after a direct
question about whether permitted coverage-adding work had actually been
done, that was too conservative: AGENTS.md's "Autonomously allowed" list
permits adding tests/adversarial cases regardless of whether they find a
bug, so the probe was recreated and committed as
`crates/cad-occt-bridge/tests/concurrency_probe.rs` (see "Patches
applied" below). Final test totals after all patches: 125 Rust tests
(23 + 84 + 9 + 6 + 3, up from the 117 shown above) and 37/37 native
`abi_boundary_test` checks (up from 22) — see "Patches applied" and
"Context lifetime safety fix" below for the exact breakdown.

### Sanitizer results (new evidence this review contributes)

`project/gates/stage-1-gate.md` §4 (item **G2**) and every batch
checkpoint since `STAGE1-A_KERNEL_BOUNDARY.md` have carried an open item:
*"no fresh valgrind run has been performed since Batch 1A."* This review
closes that gap with fresh evidence:

**Valgrind memcheck** (`--leak-check=summary --track-origins=yes`) on
all 9 substantive native test binaries (`lifecycle_test`,
`abi_boundary_test`, `box_cylinder_test`, `boolean_test`,
`fillet_chamfer_test`, `shell_offset_test`, `step_export_test`,
`step_import_test`, `tessellation_test`):
```
ERROR SUMMARY: 0 errors from 0 contexts   (every binary)
LEAK SUMMARY: definitely lost: 0 bytes / indirectly lost: 0 bytes /
              possibly lost: 0 bytes      (every binary)
```
A small "still reachable" residue (16–752 bytes) appears in every
binary — this is OCCT's/Tcl's own well-known static-initialization
allocation, already correctly identified as such in
`STAGE1-A_KERNEL_BOUNDARY.md` §6, not a leak in this bridge's own code.

**AddressSanitizer + UndefinedBehaviorSanitizer** (new — no prior batch
attempted this): rebuilt the full native test suite with GCC
(`-fsanitize=address,undefined -fno-omit-frame-pointer -O1`, clang's ASan
runtime was unavailable in this container so GCC's was used instead) and
re-ran the full `ctest` suite under it:
```
100% tests passed, 0 tests failed out of 18
```
with zero ASan/UBSan reports (no heap/stack buffer overflow, use-after-free,
double-free, signed-integer-overflow, null-pointer, or misaligned-access
diagnostics from any binary). This is genuinely new, stronger evidence
than any prior batch gathered — G2 should be considered **closed** by
this review, not merely re-flagged as still-open.

This does **not** close G1 (exception-containment reachability — see
below): sanitizers detect memory/UB defects, not whether a specific
`catch (const Standard_Failure&)` branch has ever actually executed.

## Kernel-boundary findings (Audit 1)

**No leakage found.** `grep -rlE` for every major OCCT type/class prefix
(`TopoDS_`, `BRepAlgoAPI`, `BRepPrimAPI`, `BRepFilletAPI`,
`BRepOffsetAPI`, `gp_Pnt`, `gp_Trsf`, `STEPControl`, `Standard_Failure`,
`TopExp`, `TopAbs_`, etc.) across every `.rs` file in `crates/` outside
`cad-occt-bridge` returned exactly one hit:
`crates/cad-kernel-api/src/lib.rs:115`, which is a **doc comment**
(`/// A single point-like topological entity (OCCT's \`TopoDS_Vertex\`,
kept nameless here...)`) explaining *why* the type is deliberately
unnamed — not a leaked type. `cad-kernel-api` carries
`#![forbid(unsafe_code)]` crate-wide and defines only opaque,
backend-independent `KernelVertex`/`KernelEdge`/.../`KernelShape`/
`KernelError` types, matching RFC-0002 §3 and `DECISION_LOG.md#DL-5`
exactly. `native/occt_bridge/include/aicad_occt_bridge.h` (the entire C
ABI) was read in full and contains no OCCT class/enum name anywhere —
only `uint64_t`/`uint32_t` handle fields and a project-defined status
enum. `cad-occt-bridge/src/ffi.rs` mirrors the header 1:1 with
`#[repr(C)]` structs of plain integers; nothing OCCT-shaped crosses into
`lib.rs`'s safe wrapper types (`OcctContext`, `Shape<'ctx>`,
`BoundingBox`, `ValidationReport`, `TriangleMesh` — all pure Rust value
types).

Kernel handles are correctly build/epoch-local: `KernelId{context_id,
slot, generation}` and the native `aicad_shape_handle_t` both encode a
generation counter that increments on every release (native
`ShapeTable::Release`), and `context_id` is a per-process monotonic
counter starting at 1 (0 reserved), so a stale or foreign-context handle
can never alias a live one by value alone. Lineage/history capture is
absent from Stage 1 by design (not attempted, not silently frozen) —
consistent with DL-5's "lineage evidence is a later, above-the-kernel
concern." No Stage-4 semantic-reference-resolution policy (fingerprinting,
OCAF-as-identity, etc.) has been implemented or implicitly decided:
`project/experiments/` contains only its own README placeholder, and
`crates/cad-references` is an unedited stub. **Audit 1: no findings.**

## FFI / ownership / lifetime findings (Audit 2)

Verified directly, not assumed: `native/occt_bridge/tests/abi_boundary_test.cpp`
and `lifecycle_test.cpp` exercise invalid-handle, stale-handle (including
"a released handle does NOT alias a newly-created shape that reuses the
same slot, via generation mismatch"), double-release (rejected as
`STALE_HANDLE`, not a crash or silent no-op), foreign-context rejection,
and wrong-thread rejection, each as a real runtime check verified against
a real call, not a design claim. Every one of the 1955 lines of
`aicad_occt_bridge.cpp` wraps its OCCT-calling body in
`try { ... } catch (const Standard_Failure&) { return
OPERATION_FAILED; } catch (...) { return INTERNAL; }` with zero
exceptions — confirmed by reading the file in full, not sampling. No
STL container, `std::string`, or OCCT-owned object crosses the `extern
"C"` boundary; only `#[repr(C)]`-compatible plain-old-data structs and
opaque pointers do (`file_path` is a raw null-terminated `const char*`;
arrays are caller-owned pointer+length pairs). `OcctContext`'s `Drop`
destroys the native context exactly once (Rust ownership guarantees this
runs at most once), and every `Shape<'ctx>` borrows `'ctx`, so the borrow
checker — not just the native `WRONG_THREAD`/context-liveness runtime
check — prevents a context from being dropped while a `Shape` it produced
is still alive. `Shape<'ctx>::drop` releases the shape and explicitly
discards the status (documented rationale: a release failure here would
indicate a bug this crate's own tests would catch directly, not
something to propagate mid-unwind).

The `tessellate`/`tessellation_get` two-call buffer-out-param protocol
(`crates/cad-occt-bridge/src/lib.rs:1006-1053`) is a genuine
buffer-overflow-by-contract API at the native layer (`out_vertices`/
`out_normals` must each be sized `9 * triangle_count` for the *same*
handle's *most recently* cached tessellation) — verified that the only
caller, `Shape::tessellate`, allocates exactly `vec![0.0; 9 *
triangle_count]` from the count `aicad_occt_tessellate` itself just
returned, immediately before the paired `_get` call, with no intervening
call that could invalidate the cache for that handle (contexts are
single-thread-affine, so no other thread can touch this handle's slot in
between). This is safe as actually used, but it is safe **only** because
`cad-occt-bridge` is the sole permitted caller of this native function —
a correct design given DL-5's crate-boundary contract, not an
accidentally-safe one.

**MINOR, now FIXED — double-context-destroy was a genuine native
use-after-free.** `abi_boundary_test.cpp` tested double-*release-shape*
explicitly (line 75: "releasing an already-released handle is rejected
as STALE_HANDLE, not a double-free") but had no equivalent test for
calling `aicad_occt_context_destroy` twice on the same pointer. Reading
the original `CheckContext`/`context_destroy`: after `delete context`, a
second call read `context->owning_thread` on freed memory before its
null-check even applied — a genuine use-after-free **at the raw C ABI
level**, not reachable through the safe Rust API (`OcctContext`'s Drop
runs at most once, by construction) but a real defect in the native ABI
itself. This review's first pass correctly declined to fix it
unilaterally (a correct fix requires an internal architecture decision —
how context memory is owned/reclaimed — that a self-initiated audit
patch should not make alone). An explicit owner decision subsequently
authorized exactly that architecture change; it has now been implemented,
tested, and sanitizer-verified. **See "Context lifetime safety fix"
below for the complete record.**

**Audit 2 conclusion (post-fix)**: no BLOCKER or MAJOR finding. The
generation-based *shape*-handle design was already sound; the
context-pointer-level gap identified above is now closed by the same
class of guarantee (a never-freed control block with an atomically-checked
liveness flag, rather than shape-table slot reuse — see below for why
the two cases warranted different mechanisms). Exception containment
remains complete for every reachable path this review found.

## Context lifetime safety fix (owner-authorized architecture change)

This section is required content per the owner decision that authorized
this fix (message received mid-review, after this review's first pass
had already published Finding 2 as an accepted, unfixed limitation).
Everything below happened *after* the rest of this document's findings
were first written; the fix is committed together with this section.

### Original defect

`struct aicad_occt_context` was a plain heap-allocated object
(`new aicad_occt_context()` in `aicad_occt_context_create`, `delete
context` in `aicad_occt_context_destroy`). `CheckContext` — called first
by every other bridge function — read `context->owning_thread` with no
liveness check at all. Consequences, all real and all verified by this
fix's own new regression tests (see below) against the *pre-fix* code
before it was replaced:
- **Double-destroy**: a second `aicad_occt_context_destroy(context)`
  call dereferenced already-freed memory to read `context->owning_thread`,
  then called `delete` a second time on already-freed memory — a
  textbook double-free, undefined behavior.
- **Use-after-destroy for any other operation**: e.g.
  `aicad_occt_create_box(context, ...)` on a destroyed `context` hit the
  identical use-after-free in `CheckContext` before any of the function's
  own argument validation ran.
- Both were unreachable through `cad-occt-bridge`'s safe Rust API
  (`OcctContext`'s `Drop` runs at most once by Rust ownership, and every
  `Shape<'ctx>` borrows `'ctx` so the borrow checker forbids dropping a
  context while any shape from it is still alive) — the defect lived
  entirely in the native ABI's own contract, reachable only by a caller
  using the raw C functions directly (or by a future, different safe
  wrapper that did not reproduce `cad-occt-bridge`'s own careful
  ownership modeling).

### Alternatives considered

1. **Stable context control block with live/destroyed state, memory
   reused via a slot+generation free-list (mirroring `ShapeTable`
   exactly)** — rejected. `ShapeTable`'s free-list works because a shape
   *handle* is a separate `{context_id, slot, generation}` value the
   caller carries independently of any pointer; a stale handle is
   detected by comparing the caller's carried `generation` against the
   slot's current one. A context *handle* is nothing but the raw
   `aicad_occt_context_t*` pointer itself — there is no separate
   generation value the caller carries alongside it. If a destroyed
   context's memory were recycled for a *different*, legitimately-live
   new context, a caller's stale pointer to the old context would, after
   reuse, address a real, live `aicad_occt_context` — indistinguishable
   from the caller's own context by any check this design could perform,
   silently aliasing someone else's kernel session. This is exactly the
   "released IDs alias newly created geometry" hazard the audit brief
   asks to rule out, and slot-reuse-for-contexts would reintroduce it at
   the context level even while fixing the double-free.
2. **Process-wide live-context registry, `std::unordered_set<void*>` (or
   similar) checked by pointer identity on every call, protected by a
   mutex** — rejected as unnecessarily costly. This would work
   (liveness becomes a lookup keyed by pointer value, safe even for a
   completely foreign pointer, unlike this fix's own approach — see
   "What this does not fix" below), but it puts a process-wide mutex
   acquisition on literally every single bridge call (`CheckContext` is
   the first thing every function does), directly contradicting the
   owner decision's own "prefer avoiding a globally contended lock on
   every bridge call when an equally safe simpler design exists."
   Multiple independent kernel contexts, each meant to be usable
   concurrently from independent threads (Stage-1's own concurrency
   model), would now contend on one global lock for an operation
   (liveness-checking) that has nothing to do with any shared geometry
   state.
3. **Generational/opaque context handle backed by a registry/slab,
   where the value returned to the caller is a `{slot, generation}` pair
   disguised as a pointer (e.g. via pointer tagging or an integer cast to
   a pointer-sized value)** — rejected as more complex than necessary
   for no additional safety benefit over option 4 below, and fragile
   (pointer tagging assumes spare bits are available in real pointer
   values, which is platform-dependent and not guaranteed by the C++
   standard); would also change the opaque type's actual nature in a way
   that could surprise a future maintainer expecting `aicad_occt_context_t*`
   to behave like an ordinary (if opaque) object pointer.
4. **Selected: a stable, permanently-live control block per context,
   *never reused* for a different context, with a single atomic `live`
   flag as the sole safety-critical field, and no per-call lock.** Every
   context created by `aicad_occt_context_create` gets its own
   `aicad_occt_context` object, allocated via `emplace_back()` into a
   process-wide `std::deque<aicad_occt_context>` (chosen specifically
   because `std::deque::emplace_back` never relocates or invalidates the
   address of an already-constructed element, unlike `std::vector` —
   this is the one property this design needs from its container).
   `live` starts `false` on construction; `aicad_occt_context_create`
   sets it `true` (release ordering) only after `id`/`owning_thread` are
   fully initialized and *before* the pointer is ever handed to a
   caller — so no reader can observe a partially-initialized live
   context. `aicad_occt_context_destroy` clears it `false` via
   `compare_exchange_strong` (acq_rel ordering) — the CAS guarantees
   *exactly one* caller ever wins and releases `context->shapes`'s
   actual geometry memory, no matter how many threads call destroy
   concurrently on the same pointer. The `aicad_occt_context` object's
   own (small, fixed-size) memory is never freed for the life of the
   process; the registry's own mutex is taken only inside `Allocate()`
   (i.e. only at context-creation time), never on the per-call hot path.

### Rationale for the selected design

- It is the least complex option that fully closes the in-scope defect:
  no per-call lock, no pointer tagging, no second generation-tracking
  scheme layered on top of a scheme (slot reuse) that does not fit a
  bare-pointer handle.
- It reuses a property this codebase already trusts and has already
  tested extensively for `ShapeTable` — "never actually free memory a
  caller might still hold a stale pointer to; make the *liveness check
  itself* the safety mechanism" — applied at the coarser, much
  lower-frequency granularity of context creation/destruction rather
  than per-shape-operation, where the memory-never-reused property is
  actually *easier* to guarantee soundly (context creation happens
  orders of magnitude less often than shape creation in any realistic
  workload, per `docs/plan`'s own architecture — a kernel context is a
  session-scoped resource, not a per-operation one).
- Never freeing a context's own small control block sounds like it
  trades a use-after-free for a leak, but it does not: the owning
  `std::deque` itself is a function-local `static`, so its destructor —
  and every context object inside it — runs during ordinary C++ static
  teardown at process exit. This review's own valgrind runs (below)
  confirm zero leaked bytes, including after 2000+ context create/destroy
  cycles in one process.

### What this fix does and does not cover

- **Does cover** (verified by the new regression tests below): a context
  pointer this bridge itself issued via `aicad_occt_context_create`,
  used again — by any function, from any thread — after
  `aicad_occt_context_destroy` was called on it, including a second
  `destroy` call itself. All such uses now return
  `AICAD_OCCT_ERR_INVALID_ARGUMENT` deterministically; none dereference
  freed memory, because nothing is ever freed.
- **Does not, and cannot, cover**: a wholly fabricated or foreign
  pointer value that was never returned by `aicad_occt_context_create`
  at all. Dereferencing such a pointer to read `live` is unavoidably
  undefined behavior in any design built on an opaque raw-pointer C ABI
  — the identical, universally-accepted limitation every such API has
  (the C standard library's own `FILE*`, e.g. passed to `fclose`,
  carries the same limitation). This bridge had this property before the
  fix and still has it after; the fix narrows, rather than eliminates,
  the space of unsafe caller behavior, and does so for precisely the
  defect class the owner decision described.

### Implementation

`native/occt_bridge/src/aicad_occt_bridge.cpp`: `struct aicad_occt_context`
now holds `std::atomic<bool> live{false}` plus the pre-existing `id`/
`owning_thread`/`shapes` fields; a new file-local `ContextRegistry`
class (a mutex-guarded `std::deque<aicad_occt_context>`, `Allocate()`
the only operation) backs `aicad_occt_context_create`; `CheckContext`
gained one `live.load(acquire)` check before its existing thread-affinity
check; `aicad_occt_context_destroy` gained the same load plus a
`compare_exchange_strong` before releasing `context->shapes`.
`native/occt_bridge/include/aicad_occt_bridge.h`: rewrote
`aicad_occt_context_destroy`'s and `AICAD_OCCT_ERR_INVALID_ARGUMENT`'s
own doc comments to state the new, actually-true safety contract instead
of the old "undefined at the process level" claim. No function
signature changed; `crates/cad-occt-bridge`'s FFI bindings and safe
wrapper required no changes at all.

### Regression tests added

All in `native/occt_bridge/tests/abi_boundary_test.cpp` (native — the
defect and its fix live entirely below the safe Rust API, so native
tests are the correct and only place these belong), covering every
category the owner decision specified:

| Required case | Test |
|---|---|
| create → destroy | pre-existing `"context_destroy succeeds"`, reconfirmed |
| create → destroy → destroy | `"destroying an already-destroyed context is rejected cleanly, not a double-free"` |
| create → destroy → ordinary operation | `"create_box on a destroyed context is rejected cleanly, not a crash"` |
| stale context with shape/resource operation | `"a shape query against a destroyed context is rejected cleanly, not a crash"` |
| resource destruction after context destruction | `"releasing a shape whose context has been destroyed is rejected cleanly, not a crash"` |
| two independent contexts | `"live_ctx remains fully usable after an unrelated context was destroyed"` + `"live_ctx's own shape is unaffected by doomed_ctx's destruction"` |
| resource/context relationship across the wrong context | pre-existing `FOREIGN_CONTEXT` test, reconfirmed, plus new `"using a live handle against a destroyed context is rejected cleanly, not mistaken for live"` |
| repeated create/destroy cycles | `"repeated-cycle: 2000 create/destroy cycles all succeed cleanly"` + `"...the very first (long-destroyed) context is still safely rejected, never aliased onto a later cycle's context"` |

All pass: `./abi_boundary_test` → 37/37 checks PASSED (up from 22 before
this fix, both counts verified directly against the test binary's own
output, not estimated); the full native suite remains 18/18 (`ctest
--test-dir native/occt_bridge/build`).

### Benchmark result

Per the owner decision's own requirement ("if the chosen design changes
hot-path behavior, add a focused microbenchmark... record the measured
cost"): added to `native/occt_bridge/tests/lifecycle_test.cpp`. Measured
on this review's own container (not a guarantee for other hardware, but
a real, reproduced number, not an estimate):

```
benchmark: one uncontended atomic load+store ~= 1.21 ns; one full
aicad_occt_shape_is_valid call (context-liveness check + real OCCT
work) ~= 668874.17 ns -- the fix's own atomic check is ~0.0% of one
ordinary call's total cost
```

The fix's new per-call cost is one uncontended atomic load (`~1.2 ns`),
against an ordinary bridge call's own OCCT-side work (`~669 µs` for
`BRepCheck_Analyzer`, chosen deliberately as one of this bridge's
*cheaper* operations — most geometry-constructing calls cost
substantially more). The atomic check is roughly **five orders of
magnitude** cheaper than the call it guards; this is not a "small but
real" cost worth trading away for a different design, it is immeasurably
small in context. No hot-path mutex was added at all (the registry's own
mutex is only touched by `Allocate()`, i.e. only at context-creation
time).

### Sanitizer result (post-fix)

Fresh, full-suite runs, performed after this fix (superseding, not
merely repeating, this review's earlier pre-fix sanitizer runs):

```
$ valgrind --leak-check=full ./<each of 17 native test binaries>
ERROR SUMMARY: 0 errors from 0 contexts   (every binary)
definitely/indirectly lost: 0 bytes       (every binary, including
                                            abi_boundary_test's own new
                                            2000-cycle regression test)

$ (GCC ASan+UBSan rebuild) ctest --output-on-failure
100% tests passed, 0 tests failed out of 18
(zero ASan/UBSan reports of any kind, across the full suite)
```

The 2000-create/destroy-cycle regression test's own valgrind run showed
`0 bytes definitely lost` — direct evidence the never-freed-control-block
design does not leak in practice: everything is reclaimed at normal
process exit via the registry's own `static` destructor, matching this
fix's own "Rationale" claim rather than merely asserting it.

### Post-fix second-pass audit result

Per the second-pass requirement's own spirit (triggered here by an
owner-authorized architecture patch rather than a self-discovered
BLOCKER/MAJOR, but treated the same way): the three audit sections this
fix actually touches were re-run against the patched code, not assumed
unaffected.

- **Audit 2 (FFI/ownership/lifetime)** — re-run: all pre-existing checks
  (invalid handle, stale shape handle, foreign context, wrong thread)
  plus all 8 new lifetime checks above pass; fresh valgrind and ASan/UBSan
  both clean. Conclusion updated above: Finding 2 is now FIXED, not just
  documented.
- **Audit 3 (threading)** — re-run: `aicad_occt_context` still declares
  no `unsafe impl Send`/`Sync` (unaffected — the new `std::atomic<bool>`
  field doesn't change this at the Rust FFI boundary, which never sees
  the native struct's layout); the new `ContextRegistry` mutex is
  confirmed, by direct code reading, to be taken only inside `Allocate()`
  — never inside `CheckContext` or any per-shape operation — so the
  "conservative baseline" (no silently-promised arbitrary concurrent
  shared access, no needless lock contention) is preserved, not
  weakened. The benchmark above is the concrete evidence for "negligible
  hot-path cost," not an assumption.
- **Audit 10 (sanitizers)** — re-run in full: see "Sanitizer result"
  above. Both valgrind and ASan/UBSan are clean on the patched code,
  including the new regression tests that specifically exercise the
  previously-unsafe paths.

No other audit section (1, 4–9, 11, 12) touches context lifetime
management, so none required re-running; this was confirmed by checking
that the fix's diff is confined to context creation/destruction/liveness
checking and does not alter shape-table behavior, geometry operations,
STEP handling, or anything outside `aicad_occt_bridge.cpp`'s own context
plumbing and its header's doc comments.

## Threading findings (Audit 3)

`OcctContext` holds a raw pointer field and declares no `unsafe impl
Send`/`unsafe impl Sync` anywhere in the crate (confirmed by `grep -rn
"unsafe impl" crates/ native/` returning zero results workspace-wide) —
Rust's auto-trait rules therefore make `OcctContext` (and, transitively,
`Shape<'ctx>`, which borrows it) neither `Send` nor `Sync` automatically,
at the type level, matching the doc comment's claim and the "conservative
baseline" this audit is instructed to check for. The native
`CheckContext` reads `context->owning_thread` (a `std::thread::id` set
once at construction and never mutated again), so this check itself is
race-free even when called in violation of the affinity contract from a
foreign thread — exactly the situation it exists to catch cleanly rather
than corrupt.

Two real, non-per-context OCCT global-state hazards were found and fixed
by the batch itself, both verified by this review as still fixed:
`STEPControl_Writer`/`Reader`'s shared XSTEP session state (a single
`StepIoMutex`, AICAD-033/035) and `BRepFilletAPI_MakeFillet`'s internal
`ChFi3d` machinery (a single `FilletMutex`, AICAD-036) — the latter
reconfirmed by this review with 20 additional release-mode runs (0
failures) and reviewed line-by-line to confirm the lock genuinely wraps
the entire critical section (construction through `Build()`), not merely
part of it.

**MINOR (downgraded from an initial MAJOR concern by this review's own
new evidence) — concurrency coverage beyond fillet/STEP-I/O is bounded,
not systematic.** The batch's own investigation (AICAD-036) explicitly
disclosed that only `union`/`cut`/`intersect`/`chamfer` were
stress-tested alongside the fillet defect; `sweep`, `loft`, `shell`,
`offset`, `tessellate`, and the topology-exploration functions
(`edge_count`/`get_edge`/`face_count`/`get_face`/vertex and
adjacency queries) were never stress-tested for the same class of
OCCT-internal-global-state defect before this review. This review ran an
original 320-build-per-operation concurrent probe against exactly these
six untested surfaces and found zero failures — real, new evidence that
narrows the risk, but it is one bounded run, not a proof of absence: the
fillet defect itself required specific geometry (a concave/reentrant
edge on a multi-boolean shape, not a bare box) and repeated trials
(~47% failure rate, but only under that specific 3-way-concurrent
condition) to surface at all, so a single 40-round campaign against
simpler fixtures (bare boxes/cylinders) cannot rule out an
equally-narrow defect in, say, `BRepOffsetAPI_ThruSections` or
`BRepOffsetAPI_MakeOffsetShape` under a geometry/concurrency combination
neither this review nor the batch's own testing happened to try.
Recommend, as a Stage-1-hardening (not Stage-1-blocking) condition:
extend the concurrent-stress methodology AICAD-036 established to these
remaining operations using non-trivial (not bare-primitive) fixtures,
and commit the result as a permanent regression the way
`concurrent_fillet_of_a_concave_edge...` already is.

**Audit 3 conclusion**: no BLOCKER. The "conservative baseline" (contexts
not silently promised arbitrary concurrent shared access) holds at both
the type level and the runtime-check level. The one MAJOR-shaped concern
this review went looking for (undiscovered OCCT global-state races in
untested operations) came back with new negative evidence, not
confirmation, so it is recorded as MINOR/residual risk rather than a
blocking finding.

## Geometry correctness findings (Audit 4 / Audit 6)

`stage1_bracket.rs`'s `bracket_is_a_valid_exact_brep_matching_analytic_properties`
test was independently re-derived by hand, not merely re-run, to check
whether its "expected volume" formula is actually correct for the
geometry it claims to model, rather than trusting the code comment:

- Base flange `80×60×10` at origin, wall `80×10×60` at origin: the true
  3D overlap region is `x∈[0,80] ∩ y∈[0,10] ∩ z∈[0,10]` (the wall's Y
  extent intersected with the base's Z extent), volume
  `80·10·10=8000`; union volume `48000+48000-8000=88000` — matches the
  test's own `overlap_volume = BASE_DX * WALL_DY * BASE_DZ` exactly.
- The two base-flange hole pairs (`y=45`, radius 4, drilled along Z
  through 10 units of material) and the two wall hole pairs (`z=35`,
  radius 4, drilled along Y through 10 units of material, after a
  90°-about-X rotation this review confirmed maps `(x,y,z)→(x,z,−y)`,
  correctly turning the cylinder's default +Z axis onto +Y) are all,
  independently verified from their stated centers/radii, geometrically
  disjoint from each other, from the union overlap band, and from the
  fillet/chamfer regions — so the "4 clean cylindrical through-holes,
  each removing `π·r²·10` exactly" claim is not merely assumed, it is
  actually true for these specific placements.
- The fillet is applied to the edge where the wall's back face (plane
  `y=10`, normal +Y) meets the base's top face (plane `z=10`, normal
  +Z) — a true 90° dihedral angle (Y⊥Z), so `r²(1−π/4)` per unit length
  is the exact, not approximate, material added by rounding a
  right-angle reentrant corner. The chamfer edge (wall top face
  `z=60` meets wall front face `y=0`, both box faces, hence also
  exactly 90°) makes `d²/2` per unit length the exact material a
  symmetric chamfer removes. **Both formulas are correct for this exact
  geometry, not merely plausible-looking** — this is a real, checkable
  engineering derivation, not a renamed-primitive test with a fudge
  factor.
- The bounding box, exact mirror-symmetry-on-center-of-mass invariant
  (`com.x == BASE_DX/2` exactly, by construction, independent of the
  fillet/chamfer/hole volumes' own exact values), and the
  greater-than-primitive topology-count assertions (not exact-count
  assertions, correctly avoiding brittle enumeration-order dependence)
  were all reviewed and are sound.

This substantially satisfies Audit 6: the demonstration part is
genuinely nontrivial (union + 4 transformed-cylinder cuts including a
rotated axis + fillet + chamfer), and its verification is analytic/exact,
not render-only or success-only.

The bridge's own operation implementations (Audit 4, sampled across
every operation in the 670-line header and cross-checked against their
1955-line implementation) consistently distinguish construction success
from topological validity (`IsDone()` vs. a caller-invoked
`is_valid()`/`validate()`), correctly reject non-finite/non-positive
inputs before ever calling into OCCT, and use `TopExp::MapShapes`
(de-duplicated) rather than a raw `TopExp_Explorer` for every
"count"/"get" pair — the header/implementation comments show this was
empirically verified (e.g. "24 vs. the correct 12 for a box"), not
assumed, and this review has no reason to doubt that specific claim
given the same pattern is used consistently and correctly throughout.

`adversarial_sweep.rs` (independently re-read and re-run by this review,
not merely trusted from its own report) enforces a real contract
discipline: it explicitly refuses to assert that extreme-but-not-invalid
inputs succeed (an oversized fillet is allowed to fail), while asserting
`KernelError::Internal` must never occur for a merely-extreme input
(reserving `Internal` exclusively for genuine adapter/backend defects) —
this is exactly the AGENTS.md evidence-rule discipline the audit brief
asks reviewers to check for, and it is real, not aspirational: this
review re-ran it and confirms the assertions as written actually enforce
this (not, e.g., accidentally swallowing `Internal` inside a broader
`Err(_) => {}` arm — checked line-by-line).

**MINOR — sweep/loft-specific adversarial cases are documented but not
harness-tested.** The native header's own doc comments state (and the
implementation enforces) that a non-G1-continuous sweep spine or a
mismatched-section-count loft is rejected with `OperationFailed`, but
`adversarial_sweep.rs` does not include sweep/loft in its bounded
campaign (it covers box/cylinder dimensions, boolean tangency/scale,
fillet/chamfer magnitude, and shell/offset delta — not degenerate sweep
paths or loft section mismatches). This is a real, if narrow, coverage
gap relative to the audit brief's Audit 9 list ("degenerate loft
profiles," "degenerate sweep paths"). Non-blocking for Stage 1 (the
Stage-1 proof bracket does not use sweep/loft, and their happy-path
correctness is separately covered by native `sweep_loft_test.cpp` and
Rust unit tests), but recommended for hardening mode.

**Audit 4/6 conclusion**: no BLOCKER or MAJOR. The demonstration part and
its verification are genuine engineering evidence, independently
re-derivable and found correct by this review, not merely internally
self-consistent.

## Silent healing / failure semantics findings (Audit 5)

Searched the entire native and Rust kernel-adapter source for
healing/repair/sewing/tolerance-loosening/retry/fallback patterns. Found
exactly two matches, both of which are explicit statements that **no**
such capability exists yet ("no `heal` operation exists yet," in the
header's `validate()` doc comment, and the same disclosure in
`adversarial_sweep.rs`'s own header comment). No `catch`-then-return-original-shape
pattern, no automatic tolerance broadening, no silent retry-with-different-parameters
loop exists anywhere in the 1955-line implementation (verified by reading
every `catch` block — all 40+ of them follow the identical
`OPERATION_FAILED`/`INTERNAL` two-way mapping with no branch that
manufactures a success). **Audit 5: no findings — this is a clean area,
not merely an unexercised one.**

## STEP interoperability findings (Audit 7)

Read `AICAD-033.md` and `AICAD-035.md` in full and independently
classify the two disclosed verification layers per the audit brief's own
STRONG/MODERATE/WEAK taxonomy, rather than accepting the reports' own
"genuinely independent" framing at face value:

- **Layer 1 (export → re-import through this same bridge, checking
  volume/bbox/face-count/edge-count equality)**: correctly self-disclosed
  by both reports as **WEAK** — same AICAD bridge, same OCCT installation,
  both directions. This is the *only* layer that checks actual re-imported
  geometric properties (volume, bounding box) against the original.
- **Layer 2 (the pure-Python `steputils` parser, applied to the same
  STEP file)**: genuinely independent code with no shared implementation
  with OCCT — a real, different toolchain, satisfying the "different
  toolchain" half of the audit brief's STRONG criterion. However, what it
  actually checks is **exchange-structure/entity-count conformance only**
  (does the file parse as valid ISO-10303-21; do entity-type occurrence
  counts — `ADVANCED_FACE`, `EDGE_CURVE`, `VERTEX_POINT`, etc. — match
  this bridge's own topology counts). It performs **no** independent
  recomputation of volume, bounding box, or any other geometric property,
  and (both reports disclose this correctly) no EXPRESS/AP214
  schema-semantic validation (e.g. it never checks that a
  `CARTESIAN_POINT`'s coordinates are consistent with the `PLANE`/
  `CYLINDRICAL_SURFACE` referencing it).

**MAJOR (classification/framing finding, not a code defect) — the
composite "two disclosed layers" story in `project/gates/stage-1-gate.md`
§2.3/§8 can be read as stronger independent verification than the
underlying evidence actually supports, if a reader does not carefully
separate what each layer checks.** The gate packet's language ("verified
through two disclosed layers... a genuinely independent non-OCCT
structural parse") is not false — both individual-task reports (`AICAD-033.md`,
`AICAD-035.md`) are in fact careful and explicit about exactly this
distinction in their own "What this does/does not prove" sections — but
the *composite claim*, read at the gate-packet level alone, risks
leaving an owner with the impression that the bracket's re-imported
*geometry* (not just its file structure) has been independently
confirmed correct. It has not: **no tool used anywhere in Stage 1,
independent or otherwise, independently recomputes the bracket's volume,
bounding box, or coordinate data from the STEP file and compares it to
the original** — the only volume/bbox comparison (Layer 1) is the
non-independent one. This is precisely the audit brief's warning ("A WEAK
check alone must not be described as strong independent verification")
applied one level up: here it is not a WEAK check being mislabeled STRONG,
but a STRONG-for-structure / WEAK-for-geometry pair of checks whose
combination could be mis-read as fully independent geometric
verification. Recommend, as a Stage-2-or-hardening-mode condition: either
(a) extend the independent parser check to also independently recompute
at least one geometric invariant (e.g. parse `CARTESIAN_POINT` coordinates
for the bracket's known-flat faces and confirm they lie in the expected
planes), or (b) rephrase the gate packet's own top-level summary to state
explicitly, in one sentence, that geometric-fidelity round-trip
verification (as opposed to file-structure verification) is WEAK/
non-independent for all of Stage 1. This is classified MAJOR because it
affects how much confidence the owner should place in a specific,
citable Stage-1 claim, not because the underlying engineering work is
deficient — the individual task reports already contain the accurate,
narrower disclosure this finding asks to be surfaced more prominently.

**Audit 7 conclusion**: one MAJOR (framing/disclosure, not implementation),
correctable without any architecture or test change — a documentation
fix. Both individual STEP task reports (`AICAD-033.md`, `AICAD-035.md`)
were independently confirmed to already contain accurate, non-laundered
disclosure of exactly this limitation in their own text.

## Native robustness / fuzzing findings (Audit 8)

`AICAD-036.md`'s isolation-experiment table (union/cut alone, fillet on
convex edges, chamfer on convex/real-shape edges, concave fillet under
concurrency, the full bracket pipeline under concurrency, and a
zero-concurrency control) was read in full and is genuinely
evidence-based: specific thread/repetition counts, specific failure
counts, and a zero-concurrency control run (300/300 clean) that
correctly isolates the variable under test (concurrency, not the
algorithm itself). The fix (a single `std::mutex` scoped exactly to
`aicad_occt_fillet`, confirmed by the same isolation evidence not to be
needed for `chamfer`) is narrow and evidence-justified, not
"serialize everything to be safe." This review's own 20-repetition
re-run (release mode) and original 6-operation/1920-build concurrent
probe (see "Threading findings" above) corroborate the fix holds and add
new negative evidence for the previously-untested operations.

`adversarial_sweep.rs`'s ~1150 bounded, fixed-seed cases (box/cylinder
scale extremes including `NaN`/`±∞`/`1e-300`/`1e12`; tangent/coincident/
overlapping/disjoint/mixed-scale-`1e-9`-to-`1e6` booleans; impossible
fillet/chamfer magnitudes including negative/zero/`NaN`/10×-oversized;
extreme shell/offset deltas; 500-composition transform chains; unusual
near-degenerate frames) were re-run by this review as part of the
fresh `cargo test --workspace` above (7/7 passed) — this is a real,
reproducible fuzz-adjacent harness, not a report-only claim.

**No hangs, no crashes, no `Internal` misclassifications observed in any
run performed by this review** (the fresh full test suite, the 20-run
fillet-concurrency repeat, the ASan/UBSan-instrumented `ctest` run, or
the original 6-operation concurrent probe). **Regressions added by this
review**: none (no defect was found that warranted one).

**Audit 8 conclusion**: no BLOCKER or MAJOR. See "Threading findings"
above for the one MINOR residual-coverage item (concurrency stress
breadth) and "Geometry correctness findings" for the one MINOR
sweep/loft-adversarial-coverage item.

## Adversarial cases findings (Audit 9)

Covered by the existing harness: zero/near-zero/tiny/huge/mixed-scale
dimensions, tangent/coincident/coplanar-ish booleans (the
`offset_fraction` sweep includes exactly `1.0` = tangent and
`1.0+1e-9` = just-past-tangent), impossible fillet/chamfer, extreme
shell/offset deltas, repeated transforms, unusual frames — all
independently re-run by this review, not merely read. **Not covered**
(consistent with the two MINOR findings above): degenerate/self-
intersecting sweep paths, degenerate loft profiles, open wires used
where a closed wire is expected (partially covered — `make_wire_from_edges`'s
own unit tests separately exercise this, but not inside the bounded
adversarial harness). No BLOCKER-level gap: every adversarial case this
review could construct from already-implemented Stage-1 operations
either matched documented behavior or was already covered.

## Sanitizer / resource safety findings (Audit 10)

See "Sanitizer results" above (valgrind + ASan/UBSan, both clean,
performed fresh by this review). `cargo fmt`/`clippy -D warnings`/full
workspace build+test all reproduced clean from a genuinely fresh
rebuild (`rm -rf native/occt_bridge/build` first), not an incremental
build that might mask a stale-artifact false pass.

## Report claims vs. reality findings (Audit 11)

Sampled and cross-checked `AICAD-033.md`, `AICAD-035.md`, `AICAD-036.md`,
`project/gates/stage-1-gate.md`, and all four batch checkpoints against
the actual current source and this review's own fresh tool runs. Every
sampled test-count, command, and pass/fail claim reproduced exactly (18/18
native, 23+84+7+3 Rust, `cargo fmt`/`clippy` both clean). No instance of
"report says PASS but the test only asserts non-null" was found —
every test this review read in detail (`stage1_bracket.rs`,
`adversarial_sweep.rs`, `abi_boundary_test.cpp`, `lifecycle_test.cpp`)
asserts a specific, checkable property, not merely that a call returned
`OK`. The one framing issue found is the STEP-independence composite
claim already discussed under Audit 7 — a real finding, but about
emphasis/completeness of disclosure at the gate-packet level, not a
fabricated or reproducibility-broken claim (the individual task reports
underneath it are accurate).

**MINOR — `project/reports/AICAD-037.md` does not exist**, although
`project/TASKS.yaml`'s own `AICAD-037` entry specifies
`report: project/reports/AICAD-037.md`. `SESSION_HANDOFF.md` explains
this was a deliberate substitution ("this file + `project/gates/stage-1-gate.md`"
doubles as the report), which is a reasonable call given the gate packet
is itself AICAD-037's own deliverable, but it is an undocumented
deviation from the ticket's own stated `report:` path — a future
mechanical check that expects every `AICAD-NNN.md` report file to exist
would flag this as missing. Recommend adding a one-line stub at that
path pointing to the gate packet, purely for tooling consistency; this
finding does not affect the substance of any evidence.

## Architectural scope-creep findings (Audit 12)

Every one of the 25 non-kernel crates (`cad-references`,
`cad-feature-graph`, `cad-diagnostics`, `cad-runtime`, `cad-compiler`,
`cad-constraints`, `cad-query`, `cad-hir`, `cad-ast`, `cad-lexer`,
`cad-parser`, `cad-types`, `cad-units`, `cad-validation`,
`cad-geometry-api`, `cad-geometry-runtime`, `cad-interchange`,
`cad-artifact`, `cad-provenance`, `cad-requirements`, `cad-assemblies`,
`cad-configurations`, `cad-packages`, `cad-lsp`, `cad-agent-tools`,
`cad-cli`) was directly confirmed (not sampled) to be an unedited 6-line
placeholder (`//! ... placeholder crate created by AICAD-002/AICAD-003
repository scaffolding. No implementation yet.`) with zero test targets,
matching `cargo test --workspace`'s own "0 passed; 0 failed" for each.
`skills/`, `editor/`, `examples/` (beyond the Stage-0 paper example) are
README-only directories. `project/experiments/` (the OCAF-prototyping
area D8 explicitly reserves) contains only its own README, confirming
D8's "internal OCAF usage is prototype-driven, not yet decided" has not
been silently resolved either way. No parser/language semantics, Stage-2
evaluator behavior, feature-DAG semantics, persistent semantic
references, geometry-fingerprint logic, sketch semantics, or solver
semantics exist anywhere in the repository outside `docs/plan/`
(design documents, correctly not implementation) and `rfcs/` (frozen
design decisions, correctly not implementation). **Audit 12: no
findings** — Stage 1 has not silently frozen or pre-implemented any
later-stage architecture.

## Findings summary (all severities)

| # | Severity | Area | File(s) | Summary | Outcome |
|---|---|---|---|---|---|
| 1 | MAJOR | STEP independence framing | `project/gates/stage-1-gate.md` §2.3/§8 | The composite "two disclosed layers" claim can be read as independently verifying re-imported *geometry*, but only file-*structure*/entity-counts are independently verified; the only geometry (volume/bbox) comparison is the non-independent self-round-trip. Individual task reports already disclose this accurately; the gate-packet-level summary does not surface it as sharply. | **Mitigated** — a cross-reference note was added to the top of `stage-1-gate.md` pointing here and stating the narrower claim explicitly; the original evidence is left intact, not rewritten. |
| 2 | MINOR → real defect | FFI lifetime safety | `native/occt_bridge/src/aicad_occt_bridge.cpp` (`aicad_occt_context_destroy`/`CheckContext`), `tests/abi_boundary_test.cpp` | Double-context-destroy, and any operation on a destroyed context, dereferenced freed memory — a genuine native use-after-free, not reachable through the safe Rust API. | **Fixed** (owner-authorized architecture change) — see "Context lifetime safety fix." 15 new regression tests added; fresh valgrind + ASan/UBSan clean. |
| 3 | MINOR | Concurrency coverage breadth | `adversarial_sweep.rs`, AICAD-036 investigation scope | `sweep`/`loft`/`shell`/`offset`/`tessellate`/topology-exploration were never stress-tested for the fillet-class OCCT-global-state defect before this review. | **Addressed** — `crates/cad-occt-bridge/tests/concurrency_probe.rs` added (6 permanent tests, 320 concurrent builds each); still bounded, not exhaustive (see that file's own doc comment). |
| 4 | MINOR | Adversarial-case coverage | `adversarial_sweep.rs` | Degenerate/non-G1 sweep spines and mismatched-section-count lofts were documented and correctly rejected by the implementation but not exercised by the bounded adversarial harness. | **Addressed** — 2 new tests added to `adversarial_sweep.rs` (`degenerate_sweep_spines_never_panic_or_return_internal`, `mismatched_loft_sections_never_panic_or_return_internal`). |
| 5 | MINOR | Process/documentation | `project/TASKS.yaml` (AICAD-037 `report:` path), `project/reports/` | `project/reports/AICAD-037.md` did not exist even though the ticket specifies that path. | **Fixed** — stub added, pointing to the gate packet and this review. |
| 6 | NOTE | Sanitizer coverage (now closed) | native test suite | G2 (no valgrind since Batch 1A) is closed by this review's fresh valgrind + new ASan/UBSan runs, both clean, both re-confirmed after the Finding-2 fix. | Closed. |
| 7 | NOTE | Exception-path coverage | native bridge, all operations | G1 (no confirmed genuine `Standard_Failure` throw reached by any test) remains open; this review did not find a new way to trigger one either, consistent with the batch's own honest disclosure. | Non-blocking, still open. |
| 8 | NOTE | Determinism/performance baselines | `benchmarks/performance/` | Not yet established; correctly out of Stage-1's own ticket scope (`OWNER_DECISIONS.md` D5 remains open and non-blocking for Stage 1, per RFC-0002 §9). | Non-blocking, still open. |

No BLOCKER findings, before or after patching.

## Patches applied

Two rounds, both on top of `a296891`, both committed together with this
document's final revision:

**Round 1 (self-initiated coverage/documentation work)**:
- `project/reports/AICAD-037.md` — added (Finding 5).
- `project/gates/stage-1-gate.md` — added a cross-reference note at the
  top, no other text changed (Finding 1).
- `crates/cad-occt-bridge/tests/concurrency_probe.rs` — added, 6 tests
  (Finding 3).
- `crates/cad-occt-bridge/tests/adversarial_sweep.rs` — 2 tests added,
  plus a missing `Direction3` import fixed (Finding 4).
None of these met the "clear implementation bug" bar the patch policy
reserves its five-step fix cycle for — they are documentation and
test-coverage additions, which AGENTS.md's "Autonomously allowed" list
permits directly. All were rerun after adding: `cargo fmt`/`clippy -D
warnings` clean, full `cargo test --workspace` clean (125 Rust tests
total, up from 117), native `ctest` unaffected (18/18).

**Round 2 (owner-authorized architecture fix)**:
- `native/occt_bridge/include/aicad_occt_bridge.h` — rewrote
  `aicad_occt_context_destroy`'s and `AICAD_OCCT_ERR_INVALID_ARGUMENT`'s
  doc comments to state the new (now-true) safety contract.
- `native/occt_bridge/src/aicad_occt_bridge.cpp` — the context lifetime
  fix itself (Finding 2). See "Context lifetime safety fix" above for
  the full defect/design/rationale/evidence record; that section is
  this patch's own required documentation, not a duplicate of it.
- `native/occt_bridge/tests/abi_boundary_test.cpp` — 15 new regression
  checks (see that section's own table).
- `native/occt_bridge/tests/lifecycle_test.cpp` — a microbenchmark
  measuring the fix's own hot-path cost.
This one *was* a clear implementation bug (a genuine use-after-free),
but fixing it correctly required an internal architecture decision this
review was not positioned to make unilaterally under the patch policy's
own "does not... select between major unresolved architecture
alternatives" boundary — hence the owner authorization, the required
design-alternatives comparison, and the required regression-test/
sanitizer/benchmark evidence, all completed as specified.

## Second-pass requirement

**Triggered by the owner-authorized Round 2 patch** (a real code fix,
unlike Round 1's documentation/coverage additions) and completed: Audits
2, 3, and 10 were re-run against the patched code specifically because
this patch touches context creation/destruction/liveness-checking, which
those three audits' own conclusions depend on. See "Post-fix second-pass
audit result" under "Context lifetime safety fix" above for the detailed
re-run results (all clean); Audits 1, 4–9, 11, 12 do not depend on
context lifetime management and were confirmed, by reading the patch's
own diff, to be untouched by it.

## Known kernel limitations (carried forward, independently reconfirmed)

- `G1`: exception containment's `catch (const Standard_Failure&)`
  branches remain unproven reachable by a genuine OCCT throw (open).
- Sweep/loft: no variable-section sweep, no explicit trihedron control,
  ruled-only loft (documented capability boundary, not a defect).
- Shell/offset: OCCT's own documented limitations (>3-edge vertices,
  offset-vs-curvature limits, C0 BSpline unsupported) apply as-is; no
  attempt was made to work around them, correctly.
- No wall-clock/subprocess watchdog exists for adversarial/fuzz test
  runs; the CI job timeout is the only backstop. Reasonable at Stage 1's
  current fuzz-campaign scale (~1150 + this review's own ~1920 cases,
  seconds of wall time), worth revisiting if campaigns grow substantially.
- No STEP EXPRESS/AP214 schema-semantic conformance check exists at any
  layer (see Audit 7 finding).
- No determinism/performance baseline exists (`OWNER_DECISIONS.md` D5,
  correctly still open and non-blocking for Stage 1).

## Unresolved owner decisions

Unchanged from `project/gates/stage-1-gate.md` §7, independently
re-verified against `project/OWNER_DECISIONS.md`'s own current index:
D3, D5, D10, D11, D12, D15 fully open; D7, D8, D13 partially resolved
with an explicitly-tracked residual item; D1, D2, D4, D6, D9, D14 fully
resolved. None of the open items were found by this review to have been
silently resolved, assumed, or worked around by any Stage-1 code — this
was checked directly (e.g. D8/OCAF via `project/experiments/`'s empty
state, D5/determinism via the absence of any benchmark tolerance
constant anywhere in the kernel adapter) rather than taken on faith from
the gate packet's own §7.

## Recommendation

**PASS.**

This supersedes this review's own first-pass recommendation of PASS WITH
CONDITIONS, issued once, after the patches above, on the basis of the
patched repository's own evidence — not a retained assumption from the
pre-patch pass.

The Stage-1 kernel substrate is architecturally correct against its own
frozen contract (RFC-0002 §3/§4, DL-5): zero OCCT-type leakage above the
adapter boundary, a sound generation-based *shape*-handle design with no
aliasing defect, and — as of the fix documented in "Context lifetime
safety fix" — a sound *context*-lifetime model with the identical
never-alias guarantee extended to context pointers themselves, closing
the one genuine implementation defect (Finding 2) this review found.
Exception containment is complete across every reachable path in the
native implementation. The concurrency contract (`!Send`/`!Sync` at the
type level, runtime-checked single-thread-affinity, no process-wide lock
on any per-call hot path even after the fix — confirmed by a measured
~1.2 ns per-call cost against OCCT's own ~669 µs) holds up under fresh
adversarial testing, including this review's own original 1920-call
concurrent probe (now permanently committed as `concurrency_probe.rs`)
against operations the batch's own investigation never stress-tested.
Zero silent healing/repair exists anywhere in the codebase. The Stage-1
demonstration part is genuinely nontrivial and its verification is
analytic/exact, independently re-derived by hand and confirmed correct,
not merely re-run. Zero Stage-2+ scope creep was found across all 25
non-kernel crates. Fresh valgrind and ASan/UBSan runs — the latter newly
introduced by this review — are clean across the entire native suite,
both before and after the Finding-2 fix, closing the batch's own
long-standing G2 item with direct evidence rather than a re-flagged gap.

Every finding this review raised has now been fixed or mitigated with
concrete, verified evidence, not merely re-classified as acceptable:
Finding 1 (STEP-verification framing) is mitigated by an explicit
cross-reference correcting the composite claim without rewriting the
original evidence; Finding 2 (the context-lifetime use-after-free) is
fixed under an owner-authorized architecture change with a full
alternatives comparison, 15 new regression tests, a measured negligible
performance cost, and clean post-fix sanitizer runs; Findings 3 and 4
(concurrency and sweep/loft adversarial coverage gaps) are addressed
with 8 new permanent tests; Finding 5 (the missing report stub) is
fixed. The three remaining NOTEs (G1's unreached exception path,
determinism/performance baselines) are genuinely non-blocking: they were
correctly out of Stage-1's own ticket scope before this review and
remain so — nothing found during this review's patching work changed
that assessment.

**Non-blocking follow-ups still worth doing in Stage-1 hardening mode or
early Stage 2** (informational, not conditions on this PASS):
- Extend the concurrent-stress methodology in `concurrency_probe.rs` to
  non-trivial (not bare-primitive) fixtures over a longer campaign, the
  way `AICAD-036`'s own fillet investigation eventually needed to, to
  further narrow (bounded evidence can narrow risk but never fully
  eliminate it) the residual risk of an undiscovered OCCT-global-state
  race in an operation this review did not happen to construct the right
  adversarial fixture for.
- If Stage 2 or a later stage needs to cite STEP round-trip fidelity as
  more than structurally verified, extend independent verification to
  recompute at least one geometric invariant outside OCCT (Finding 1's
  underlying gap is mitigated, i.e. honestly disclosed, but not closed).
- G1 (exception-path reachability) and determinism/performance
  baselining remain open, exactly as the batch's own gate packet already
  disclosed; no new urgency was found here.

This recommendation is advisory only. Per `AGENTS.md` and
`project/CURRENT_STAGE.md`, Stage 2 (`AICAD-038` onward) remains blocked
until the owner records a Stage-1 pass decision in
`project/DECISION_LOG.md`. This review does not update
`project/CURRENT_STAGE.md`, does not begin `AICAD-038`, and does not
mark Stage 1 owner-approved.
